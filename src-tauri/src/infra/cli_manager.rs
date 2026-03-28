//! Usage: Discover installed CLIs and manage related local config (infra adapter).

use crate::shared::fs::{read_optional_file, write_file_atomic_if_changed};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ENV_KEY_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
const ENV_KEY_DISABLE_ERROR_REPORTING: &str = "DISABLE_ERROR_REPORTING";

#[cfg(not(windows))]
const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(2);
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const CMD_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeCliInfo {
    pub found: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
    pub shell: Option<String>,
    pub resolved_via: String,
    pub config_dir: String,
    pub settings_path: String,
    pub mcp_timeout_ms: Option<u64>,
    pub disable_error_reporting: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimpleCliInfo {
    pub found: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
    pub shell: Option<String>,
    pub resolved_via: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeEnvState {
    pub config_dir: String,
    pub settings_path: String,
    pub mcp_timeout_ms: Option<u64>,
    pub disable_error_reporting: bool,
}

#[derive(Debug)]
struct CliProbeResult {
    found: bool,
    executable_path: Option<String>,
    version: Option<String>,
    error: Option<String>,
    shell: Option<String>,
    resolved_via: String,
}

fn command_output_with_timeout(
    mut cmd: Command,
    timeout: Duration,
    label: String,
) -> crate::shared::error::AppResult<std::process::Output> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to execute {label}: {e}"))?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|e| format!("failed to collect output {label}: {e}").into());
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{label} timed out after {}ms", timeout.as_millis()).into());
                }
                std::thread::sleep(CMD_POLL_INTERVAL);
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to wait for {label}: {e}").into());
            }
        }
    }
}

fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

fn claude_config_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(home_dir(app)?.join(".claude"))
}

fn claude_settings_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(claude_config_dir(app)?.join("settings.json"))
}

fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> serde_json::Value {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

fn json_to_bytes(
    value: &serde_json::Value,
    hint: &str,
) -> crate::shared::error::AppResult<Vec<u8>> {
    let mut out =
        serde_json::to_vec_pretty(value).map_err(|e| format!("failed to serialize {hint}: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

fn ensure_json_object_root(mut root: serde_json::Value) -> serde_json::Value {
    if root.is_object() {
        return root;
    }
    root = serde_json::json!({});
    root
}

fn env_string_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
        _ => None,
    }
}

fn read_claude_env(settings_path: &Path) -> crate::shared::error::AppResult<(Option<u64>, bool)> {
    let Some(bytes) = read_optional_file(settings_path)? else {
        return Ok((None, false));
    };

    let value = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_else(|_| {
        // Keep best-effort: invalid json -> treat as empty (we'll overwrite on write).
        serde_json::json!({})
    });

    let Some(env) = value.get("env").and_then(|v| v.as_object()) else {
        return Ok((None, false));
    };

    let mcp_timeout_ms = env
        .get(ENV_KEY_MCP_TIMEOUT)
        .and_then(env_string_value)
        .and_then(|s| s.parse::<u64>().ok());

    let disable_error_reporting = env.contains_key(ENV_KEY_DISABLE_ERROR_REPORTING);

    Ok((mcp_timeout_ms, disable_error_reporting))
}

fn patch_claude_env(
    root: serde_json::Value,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> crate::shared::error::AppResult<serde_json::Value> {
    let mut root = ensure_json_object_root(root);
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "settings.json root must be a JSON object".to_string())?;

    let env = obj
        .entry("env")
        .or_insert_with(|| serde_json::Value::Object(Default::default()));
    if !env.is_object() {
        *env = serde_json::Value::Object(Default::default());
    }
    let env = env
        .as_object_mut()
        .ok_or_else(|| "settings.json env must be an object".to_string())?;

    match mcp_timeout_ms.filter(|v| *v > 0) {
        Some(v) => {
            env.insert(
                ENV_KEY_MCP_TIMEOUT.to_string(),
                serde_json::Value::String(v.to_string()),
            );
        }
        None => {
            env.remove(ENV_KEY_MCP_TIMEOUT);
        }
    }

    if disable_error_reporting {
        env.insert(
            ENV_KEY_DISABLE_ERROR_REPORTING.to_string(),
            serde_json::Value::String("1".to_string()),
        );
    } else {
        env.remove(ENV_KEY_DISABLE_ERROR_REPORTING);
    }

    Ok(root)
}

fn write_claude_env<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> crate::shared::error::AppResult<()> {
    let settings_path = claude_settings_path(app)?;
    let current = read_optional_file(&settings_path)?;
    let root = json_root_from_bytes(current);
    let patched = patch_claude_env(root, mcp_timeout_ms, disable_error_reporting)?;
    let bytes = json_to_bytes(&patched, "claude/settings.json")?;
    let _ = write_file_atomic_if_changed(&settings_path, &bytes)?;

    if let Some(backup_path) = crate::cli_proxy::backup_file_path_for_enabled_manifest(
        app,
        "claude",
        "claude_settings_json",
        "settings.json",
    )? {
        let backup_current = read_optional_file(&backup_path)?;
        let backup_root = json_root_from_bytes(backup_current);
        let backup_patched =
            patch_claude_env(backup_root, mcp_timeout_ms, disable_error_reporting)?;
        let backup_bytes = json_to_bytes(&backup_patched, "claude/settings.json backup")?;
        let _ = write_file_atomic_if_changed(&backup_path, &backup_bytes)?;
    }

    Ok(())
}

fn exe_names_for(cmd: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        vec![
            format!("{cmd}.exe"),
            format!("{cmd}.cmd"),
            format!("{cmd}.bat"),
            cmd.to_string(),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![cmd.to_string()]
    }
}

fn is_path_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_exe_in_dir(dir: &Path, names: &[String]) -> Option<PathBuf> {
    for name in names {
        let p = dir.join(name);
        if is_path_executable(&p) {
            return Some(p);
        }
    }
    None
}

/// Platform-specific system directories that GUI-launched processes may lack in PATH.
///
/// On macOS / Linux, apps launched from Dock / .desktop inherit a minimal PATH
/// (typically `/usr/bin:/bin`). This list ensures we also search Homebrew, system,
/// and common package-manager locations.
#[cfg(not(windows))]
fn platform_extra_path_dirs() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
    }
    #[cfg(target_os = "linux")]
    {
        &["/usr/local/bin", "/usr/bin", "/bin"]
    }
}

fn find_exe_in_path(names: &[String]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let raw = path.to_string_lossy().to_string();
    let sep = if cfg!(windows) { ';' } else { ':' };
    for part in raw.split(sep) {
        let dir = PathBuf::from(part);
        if let Some(p) = find_exe_in_dir(&dir, names) {
            return Some(p);
        }
    }
    None
}

fn scan_executable(
    app: &tauri::AppHandle,
    cmd: &str,
) -> crate::shared::error::AppResult<Option<PathBuf>> {
    let names = exe_names_for(cmd);
    if let Some(p) = find_exe_in_path(&names) {
        return Ok(Some(p));
    }

    let home = home_dir(app)?;
    let mut candidates: Vec<PathBuf> = vec![
        home.join(".local").join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".pnpm-global").join("bin"),
        home.join(".volta").join("bin"),
        home.join(".asdf").join("shims"),
        home.join(".bun").join("bin"),
        home.join("n").join("bin"),
        home.join(".cargo").join("bin"),
    ];

    #[cfg(not(windows))]
    for dir in platform_extra_path_dirs() {
        candidates.push(PathBuf::from(dir));
    }

    #[cfg(windows)]
    {
        candidates.push(PathBuf::from(r"C:\Program Files\nodejs"));
        candidates.push(PathBuf::from(r"C:\Program Files (x86)\nodejs"));
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(appdata).join("npm"));
        }
    }

    for dir in candidates {
        if let Some(p) = find_exe_in_dir(&dir, &names) {
            return Ok(Some(p));
        }
    }

    #[cfg(not(windows))]
    {
        // Best-effort: scan nvm bins (~/.nvm/versions/node/*/bin)
        let nvm_root = home.join(".nvm").join("versions").join("node");
        if nvm_root.exists() {
            if let Ok(entries) = std::fs::read_dir(&nvm_root) {
                for (idx, entry) in entries.flatten().enumerate() {
                    if idx > 30 {
                        break;
                    }
                    let p = entry.path().join("bin");
                    if let Some(exe) = find_exe_in_dir(&p, &names) {
                        return Ok(Some(exe));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn shell_env_path() -> Option<PathBuf> {
    std::env::var_os("SHELL").map(PathBuf::from)
}

#[cfg(not(windows))]
fn is_fish_shell(shell: &Path) -> bool {
    shell
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("fish") || v.eq_ignore_ascii_case("fish.exe"))
        .unwrap_or(false)
}

fn run_in_login_shell(shell: &Path, script: &str) -> crate::shared::error::AppResult<String> {
    #[cfg(windows)]
    {
        let _ = script;
        Err(format!(
            "login shell resolution is not supported on windows (shell={})",
            shell.display()
        )
        .into())
    }

    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(shell);
        if is_fish_shell(shell) {
            cmd.arg("-l").arg("-c").arg(script);
        } else {
            cmd.arg("-lc").arg(script);
        }

        let out = command_output_with_timeout(
            cmd,
            LOGIN_SHELL_TIMEOUT,
            format!("login shell {}", shell.display()),
        )?;
        if !out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            return Err(if msg.is_empty() {
                "unknown error"
            } else {
                &msg
            }
            .to_string()
            .into());
        }

        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

fn resolve_executable_via_login_shell(
    cmd: &str,
) -> crate::shared::error::AppResult<Option<PathBuf>> {
    let Some(shell) = shell_env_path() else {
        return Ok(None);
    };
    if !shell.exists() {
        return Ok(None);
    }

    let script = format!("command -v {cmd}");
    let out = run_in_login_shell(&shell, &script)?;
    let first = out.lines().next().unwrap_or("").trim().to_string();
    if first.is_empty() {
        return Ok(None);
    }

    let candidate = PathBuf::from(first);
    if is_path_executable(&candidate) {
        return Ok(Some(candidate));
    }

    Ok(None)
}

fn run_version(exe: &Path) -> crate::shared::error::AppResult<String> {
    let mut cmd = Command::new(exe);
    cmd.arg("--version");

    // GUI-launched processes on macOS/Linux inherit a minimal PATH that often
    // lacks Homebrew / nvm / system dirs. Prepend the standard locations so that
    // shebang-based CLIs (#!/usr/bin/env node) can resolve their runtime.
    #[cfg(not(windows))]
    {
        let extra = platform_extra_path_dirs().join(":");
        cmd.env(
            "PATH",
            format!("{}:{}", extra, std::env::var("PATH").unwrap_or_default()),
        );
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let out =
        command_output_with_timeout(cmd, VERSION_TIMEOUT, format!("{} --version", exe.display()))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        let first = stdout.lines().next().unwrap_or("").trim().to_string();
        if !first.is_empty() {
            return Ok(first);
        }
        if !stdout.is_empty() {
            return Ok(stdout);
        }
        return Ok("unknown".to_string());
    }

    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(if msg.is_empty() {
        "unknown error"
    } else {
        &msg
    }
    .to_string()
    .into())
}

fn cli_probe(app: &tauri::AppHandle, cmd: &str) -> crate::shared::error::AppResult<CliProbeResult> {
    let shell = std::env::var("SHELL").ok();

    let (exe, resolved_via) = match resolve_executable_via_login_shell(cmd) {
        Ok(Some(p)) => (Some(p), "login_shell".to_string()),
        Ok(None) => (scan_executable(app, cmd)?, "path_scan".to_string()),
        Err(_) => (scan_executable(app, cmd)?, "path_scan".to_string()),
    };

    let mut found = false;
    let mut executable_path: Option<String> = None;
    let mut version: Option<String> = None;
    let mut error: Option<String> = None;

    if let Some(exe) = exe {
        found = true;
        executable_path = Some(exe.to_string_lossy().to_string());
        match run_version(&exe) {
            Ok(v) => version = Some(v),
            Err(err) => error = Some(err.to_string()),
        }
    }

    Ok(CliProbeResult {
        found,
        executable_path,
        version,
        error,
        shell,
        resolved_via,
    })
}

pub fn claude_info_get(app: &tauri::AppHandle) -> crate::shared::error::AppResult<ClaudeCliInfo> {
    let config_dir = claude_config_dir(app)?;
    let settings_path = claude_settings_path(app)?;
    let (mcp_timeout_ms, disable_error_reporting) = read_claude_env(&settings_path)?;

    let probe = cli_probe(app, "claude")?;

    Ok(ClaudeCliInfo {
        found: probe.found,
        executable_path: probe.executable_path,
        version: probe.version,
        error: probe.error,
        shell: probe.shell,
        resolved_via: probe.resolved_via,
        config_dir: config_dir.to_string_lossy().to_string(),
        settings_path: settings_path.to_string_lossy().to_string(),
        mcp_timeout_ms,
        disable_error_reporting,
    })
}

pub fn codex_info_get(app: &tauri::AppHandle) -> crate::shared::error::AppResult<SimpleCliInfo> {
    simple_cli_info_get(app, "codex")
}

pub fn gemini_info_get(app: &tauri::AppHandle) -> crate::shared::error::AppResult<SimpleCliInfo> {
    simple_cli_info_get(app, "gemini")
}

pub fn simple_cli_info_get(
    app: &tauri::AppHandle,
    cmd: &str,
) -> crate::shared::error::AppResult<SimpleCliInfo> {
    let probe = cli_probe(app, cmd)?;
    Ok(SimpleCliInfo {
        found: probe.found,
        executable_path: probe.executable_path,
        version: probe.version,
        error: probe.error,
        shell: probe.shell,
        resolved_via: probe.resolved_via,
    })
}

pub fn claude_env_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> crate::shared::error::AppResult<ClaudeEnvState> {
    write_claude_env(app, mcp_timeout_ms, disable_error_reporting)?;
    let config_dir = claude_config_dir(app)?;
    let settings_path = claude_settings_path(app)?;
    let (mcp_timeout_ms, disable_error_reporting) = read_claude_env(&settings_path)?;

    Ok(ClaudeEnvState {
        config_dir: config_dir.to_string_lossy().to_string(),
        settings_path: settings_path.to_string_lossy().to_string(),
        mcp_timeout_ms,
        disable_error_reporting,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn find_exe_in_dir_ignores_directory_named_like_command() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("codex")).expect("create command-like directory");

        let names = vec!["codex".to_string()];
        assert_eq!(find_exe_in_dir(dir.path(), &names), None);
    }

    #[cfg(unix)]
    #[test]
    fn find_exe_in_dir_ignores_non_executable_file_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("codex");
        fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write file");

        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).expect("set non-executable permissions");

        let names = vec!["codex".to_string()];
        assert_eq!(find_exe_in_dir(dir.path(), &names), None);
    }

    #[cfg(unix)]
    #[test]
    fn find_exe_in_dir_accepts_executable_file_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("codex");
        fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write file");

        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("set executable permissions");

        let names = vec!["codex".to_string()];
        assert_eq!(find_exe_in_dir(dir.path(), &names), Some(path));
    }
}
