//! Usage: Check installed CLI versions against npm and run CLI updates.

use crate::shared::http_body::read_text_with_limit;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::task::JoinHandle;

const NPM_LATEST_TIMEOUT: Duration = Duration::from_secs(10);
const NPM_INSTALL_TIMEOUT: Duration = Duration::from_secs(120);
const NPM_INSTALL_OUTPUT_STREAM_LIMIT: usize = 32 * 1024;
const NPM_INSTALL_OUTPUT_READ_CHUNK_SIZE: usize = 8 * 1024;
const NPM_LATEST_RESPONSE_BODY_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CliVersionCheck {
    pub cli_key: String,
    pub npm_package: String,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CliUpdateResult {
    pub cli_key: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Extract the leading semver-like portion from a version string.
/// e.g. "2.1.90 (Claude Code)" → "2.1.90", "v1.0.0-beta" → "1.0.0-beta"
fn extract_semver(raw: &str) -> &str {
    let s = raw.trim().trim_start_matches('v');
    // Take characters until we hit a space or any char that can't be part of semver
    let end = s.find([' ', '(', ')']).unwrap_or(s.len());
    s[..end].trim_end_matches(|c: char| !c.is_ascii_alphanumeric())
}

fn npm_package_for_cli_key(cli_key: &str) -> Option<&'static str> {
    match cli_key.trim().to_ascii_lowercase().as_str() {
        "claude" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@openai/codex"),
        "gemini" => Some("@google/gemini-cli"),
        _ => None,
    }
}

fn unsupported_cli_key_error(cli_key: &str) -> String {
    format!("unsupported cli_key: {cli_key}")
}

async fn fetch_latest_version(npm_package: &str) -> Result<String, String> {
    let url = format!("https://registry.npmjs.org/{npm_package}/latest");
    let client = reqwest::Client::builder()
        .timeout(NPM_LATEST_TIMEOUT)
        .build()
        .map_err(|e| format!("failed to build npm registry client: {e}"))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("failed to fetch latest npm version: {e}"))?;
    let response = response
        .error_for_status()
        .map_err(|e| format!("npm registry returned error: {e}"))?;

    let body = read_text_with_limit(
        response,
        NPM_LATEST_RESPONSE_BODY_LIMIT,
        "npm registry response",
    )
    .await
    .map_err(|e| format!("failed to read npm registry response: {e}"))?;
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse npm registry response: {e}"))?;
    payload
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "npm registry response missing version".to_string())
}

pub async fn cli_check_latest_version(app: &tauri::AppHandle, cli_key: String) -> CliVersionCheck {
    let normalized_cli_key = cli_key.trim().to_ascii_lowercase();
    let Some(npm_package) = npm_package_for_cli_key(&normalized_cli_key) else {
        return CliVersionCheck {
            cli_key: normalized_cli_key.clone(),
            npm_package: String::new(),
            installed_version: None,
            latest_version: None,
            update_available: false,
            error: Some(unsupported_cli_key_error(&normalized_cli_key)),
        };
    };

    let installed = crate::cli_manager::simple_cli_info_get(app, &normalized_cli_key);
    let installed_version = installed
        .as_ref()
        .ok()
        .and_then(|info| info.version.clone());
    let installed_error = match installed {
        Ok(info) => info
            .error
            .map(|error| format!("failed to probe installed version: {error}")),
        Err(error) => Some(format!("failed to probe installed version: {error}")),
    };

    match fetch_latest_version(npm_package).await {
        Ok(latest_version) => {
            let update_available = installed_version
                .as_ref()
                .map(|installed| {
                    let installed_clean = extract_semver(installed);
                    let latest_clean = extract_semver(&latest_version);
                    installed_clean != latest_clean
                })
                .unwrap_or(false);

            CliVersionCheck {
                cli_key: normalized_cli_key,
                npm_package: npm_package.to_string(),
                installed_version,
                latest_version: Some(latest_version),
                update_available,
                error: installed_error,
            }
        }
        Err(error) => CliVersionCheck {
            cli_key: normalized_cli_key,
            npm_package: npm_package.to_string(),
            installed_version,
            latest_version: None,
            update_available: false,
            error: Some(match installed_error {
                Some(installed_error) => format!("{installed_error}; {error}"),
                None => error,
            }),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LimitedCommandOutput {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

impl LimitedCommandOutput {
    fn empty(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            truncated: false,
            limit,
        }
    }
}

async fn read_limited_output<R>(
    mut reader: R,
    limit: usize,
) -> std::io::Result<LimitedCommandOutput>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::with_capacity(limit.min(NPM_INSTALL_OUTPUT_READ_CHUNK_SIZE));
    let mut truncated = false;
    let mut chunk = [0_u8; NPM_INSTALL_OUTPUT_READ_CHUNK_SIZE];

    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(bytes.len());
        if remaining > 0 {
            let keep = read.min(remaining);
            bytes.extend_from_slice(&chunk[..keep]);
            if keep < read {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }

    Ok(LimitedCommandOutput {
        bytes,
        truncated,
        limit,
    })
}

fn render_limited_output(output: &LimitedCommandOutput, stream_name: &str) -> String {
    let mut rendered = String::from_utf8_lossy(&output.bytes).trim().to_string();
    if output.truncated {
        if !rendered.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&format!(
            "[{stream_name} truncated after {} bytes]",
            output.limit
        ));
    }
    rendered
}

fn join_command_output(stdout: &LimitedCommandOutput, stderr: &LimitedCommandOutput) -> String {
    let stdout = render_limited_output(stdout, "stdout");
    let stderr = render_limited_output(stderr, "stderr");
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    }
}

fn npm_executable_names() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        vec!["npm.cmd", "npm.bat", "npm.exe", "npm"]
    }
    #[cfg(not(windows))]
    {
        vec!["npm"]
    }
}

fn prefer_sibling_npm_path(cli_executable: &Path) -> Option<PathBuf> {
    let parent = cli_executable.parent()?;
    for candidate in npm_executable_names() {
        let path = parent.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn resolve_npm_executable(app: &tauri::AppHandle, cli_key: &str) -> Result<PathBuf, String> {
    let cli_info = crate::cli_manager::simple_cli_info_get(app, cli_key)
        .map_err(|e| format!("failed to resolve {cli_key} executable: {e}"))?;
    if let Some(cli_executable_path) = cli_info.executable_path.as_deref() {
        if let Some(npm_path) = prefer_sibling_npm_path(Path::new(cli_executable_path)) {
            return Ok(npm_path);
        }
    }

    let npm_info = crate::cli_manager::simple_cli_info_get(app, "npm")
        .map_err(|e| format!("failed to resolve npm executable: {e}"))?;
    npm_info
        .executable_path
        .map(PathBuf::from)
        .ok_or_else(|| "failed to locate npm executable".to_string())
}

fn prepend_command_path(command: &mut Command, dir: &Path) {
    let key = "PATH";
    let separator = if cfg!(windows) { ";" } else { ":" };
    let current = std::env::var(key).unwrap_or_default();
    let prefix = dir.to_string_lossy();
    if current.is_empty() {
        command.env(key, prefix.as_ref());
    } else {
        command.env(key, format!("{prefix}{separator}{current}"));
    }
}

fn build_cli_update_command(
    app: &tauri::AppHandle,
    cli_key: &str,
    npm_package: &str,
) -> Result<Command, String> {
    let npm_path = resolve_npm_executable(app, cli_key)?;
    let package_spec = format!("{npm_package}@latest");

    #[cfg(windows)]
    let mut command = {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(&npm_path);
        cmd.args(["install", "-g", &package_spec]);
        cmd
    };

    #[cfg(not(windows))]
    let mut command = {
        let mut cmd = Command::new(&npm_path);
        cmd.args(["install", "-g", &package_spec]);
        cmd
    };

    if let Some(parent) = npm_path.parent() {
        prepend_command_path(&mut command, parent);
    }

    Ok(command)
}

type OutputReadTask = JoinHandle<std::io::Result<LimitedCommandOutput>>;

async fn collect_output_task(
    task: Option<OutputReadTask>,
    stream_name: &str,
) -> Result<LimitedCommandOutput, String> {
    let Some(task) = task else {
        return Ok(LimitedCommandOutput::empty(NPM_INSTALL_OUTPUT_STREAM_LIMIT));
    };

    match task.await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(format!("failed to read npm update {stream_name}: {error}")),
        Err(error) => Err(format!(
            "failed to join npm update {stream_name} reader: {error}"
        )),
    }
}

async fn collect_update_output(
    stdout_task: Option<OutputReadTask>,
    stderr_task: Option<OutputReadTask>,
) -> Result<(LimitedCommandOutput, LimitedCommandOutput), String> {
    let stdout = collect_output_task(stdout_task, "stdout").await?;
    let stderr = collect_output_task(stderr_task, "stderr").await?;
    Ok((stdout, stderr))
}

pub async fn cli_update(app: &tauri::AppHandle, cli_key: String) -> CliUpdateResult {
    let normalized_cli_key = cli_key.trim().to_ascii_lowercase();
    let Some(npm_package) = npm_package_for_cli_key(&normalized_cli_key) else {
        return CliUpdateResult {
            cli_key: normalized_cli_key.clone(),
            success: false,
            output: String::new(),
            error: Some(unsupported_cli_key_error(&normalized_cli_key)),
        };
    };

    let mut command = match build_cli_update_command(app, &normalized_cli_key, npm_package) {
        Ok(command) => command,
        Err(error) => {
            return CliUpdateResult {
                cli_key: normalized_cli_key,
                success: false,
                output: String::new(),
                error: Some(error),
            };
        }
    };
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.kill_on_drop(true);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let spawn_result = command.spawn();
    let mut child = match spawn_result {
        Ok(child) => child,
        Err(error) => {
            return CliUpdateResult {
                cli_key: normalized_cli_key,
                success: false,
                output: String::new(),
                error: Some(format!("failed to start npm update: {error}")),
            }
        }
    };

    let stdout_task = child
        .stdout
        .take()
        .map(|stdout| tokio::spawn(read_limited_output(stdout, NPM_INSTALL_OUTPUT_STREAM_LIMIT)));
    let stderr_task = child
        .stderr
        .take()
        .map(|stderr| tokio::spawn(read_limited_output(stderr, NPM_INSTALL_OUTPUT_STREAM_LIMIT)));

    let wait_result = tokio::time::timeout(NPM_INSTALL_TIMEOUT, child.wait()).await;
    match wait_result {
        Ok(Ok(status)) => {
            let output_result = collect_update_output(stdout_task, stderr_task).await;
            let (stdout, stderr) = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return CliUpdateResult {
                        cli_key: normalized_cli_key,
                        success: false,
                        output: String::new(),
                        error: Some(error),
                    };
                }
            };
            let combined_output = join_command_output(&stdout, &stderr);
            if status.success() {
                CliUpdateResult {
                    cli_key: normalized_cli_key,
                    success: true,
                    output: combined_output,
                    error: None,
                }
            } else {
                CliUpdateResult {
                    cli_key: normalized_cli_key,
                    success: false,
                    output: combined_output,
                    error: Some(format!(
                        "npm update failed with exit code {:?}",
                        status.code()
                    )),
                }
            }
        }
        Ok(Err(error)) => {
            let _ = collect_update_output(stdout_task, stderr_task).await;
            CliUpdateResult {
                cli_key: normalized_cli_key,
                success: false,
                output: String::new(),
                error: Some(format!("failed while waiting for npm update: {error}")),
            }
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = collect_update_output(stdout_task, stderr_task).await;
            CliUpdateResult {
                cli_key: normalized_cli_key,
                success: false,
                output: String::new(),
                error: Some(format!(
                    "npm update timed out after {}s",
                    NPM_INSTALL_TIMEOUT.as_secs()
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_semver_strips_suffix_and_prefix() {
        assert_eq!(extract_semver("2.1.90 (Claude Code)"), "2.1.90");
        assert_eq!(extract_semver("v2.1.90"), "2.1.90");
        assert_eq!(extract_semver("2.1.90"), "2.1.90");
        assert_eq!(extract_semver("1.0.0-beta.1"), "1.0.0-beta.1");
        assert_eq!(extract_semver("  v3.0.0  "), "3.0.0");
    }

    #[test]
    fn npm_package_mapping_matches_supported_clis() {
        assert_eq!(
            npm_package_for_cli_key("claude"),
            Some("@anthropic-ai/claude-code")
        );
        assert_eq!(npm_package_for_cli_key("codex"), Some("@openai/codex"));
        assert_eq!(
            npm_package_for_cli_key("gemini"),
            Some("@google/gemini-cli")
        );
        assert_eq!(npm_package_for_cli_key("unknown"), None);
    }

    #[test]
    fn join_command_output_combines_stdout_and_stderr() {
        let stdout = LimitedCommandOutput {
            bytes: b"done\n".to_vec(),
            truncated: false,
            limit: 32,
        };
        let stderr = LimitedCommandOutput {
            bytes: b"warn\n".to_vec(),
            truncated: false,
            limit: 32,
        };
        assert_eq!(join_command_output(&stdout, &stderr), "done\nwarn");

        assert_eq!(
            join_command_output(&stdout, &LimitedCommandOutput::empty(32)),
            "done"
        );
        assert_eq!(
            join_command_output(&LimitedCommandOutput::empty(32), &stderr),
            "warn"
        );
    }

    #[tokio::test]
    async fn read_limited_output_drains_reader_but_keeps_bounded_prefix() {
        let input = std::io::Cursor::new(vec![b'a'; 20]);
        let output = read_limited_output(input, 8).await.expect("read output");

        assert_eq!(output.bytes, vec![b'a'; 8]);
        assert!(output.truncated);
        assert_eq!(output.limit, 8);
        assert_eq!(
            render_limited_output(&output, "stdout"),
            "aaaaaaaa\n[stdout truncated after 8 bytes]"
        );
    }

    #[test]
    fn join_command_output_marks_each_truncated_stream() {
        let stdout = LimitedCommandOutput {
            bytes: b"done".to_vec(),
            truncated: true,
            limit: 4,
        };
        let stderr = LimitedCommandOutput {
            bytes: b"warn".to_vec(),
            truncated: true,
            limit: 4,
        };

        assert_eq!(
            join_command_output(&stdout, &stderr),
            "done\n[stdout truncated after 4 bytes]\nwarn\n[stderr truncated after 4 bytes]"
        );
    }

    #[test]
    fn prefer_sibling_npm_path_uses_same_bin_dir_as_cli() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cli_path = dir.path().join("codex");
        let npm_path = dir
            .path()
            .join(if cfg!(windows) { "npm.cmd" } else { "npm" });

        std::fs::write(&cli_path, "").expect("write cli");
        std::fs::write(&npm_path, "").expect("write npm");

        assert_eq!(prefer_sibling_npm_path(&cli_path), Some(npm_path));
    }
}
