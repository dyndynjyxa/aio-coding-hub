//! Usage: Check installed CLI versions against npm and run CLI updates.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const NPM_LATEST_TIMEOUT: Duration = Duration::from_secs(10);
const NPM_INSTALL_TIMEOUT: Duration = Duration::from_secs(120);

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

    let payload: serde_json::Value = response
        .json()
        .await
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
                    let installed_clean = installed.trim_start_matches('v');
                    let latest_clean = latest_version.trim_start_matches('v');
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

fn join_command_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
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
    let child = match spawn_result {
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

    let wait_result = tokio::time::timeout(NPM_INSTALL_TIMEOUT, child.wait_with_output()).await;
    match wait_result {
        Ok(Ok(output)) => {
            let combined_output = join_command_output(&output.stdout, &output.stderr);
            if output.status.success() {
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
                        output.status.code()
                    )),
                }
            }
        }
        Ok(Err(error)) => CliUpdateResult {
            cli_key: normalized_cli_key,
            success: false,
            output: String::new(),
            error: Some(format!("failed while waiting for npm update: {error}")),
        },
        Err(_) => CliUpdateResult {
            cli_key: normalized_cli_key,
            success: false,
            output: String::new(),
            error: Some(format!(
                "npm update timed out after {}s",
                NPM_INSTALL_TIMEOUT.as_secs()
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(join_command_output(b"done\n", b"warn\n"), "done\nwarn");
        assert_eq!(join_command_output(b"done\n", b""), "done");
        assert_eq!(join_command_output(b"", b"warn\n"), "warn");
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
