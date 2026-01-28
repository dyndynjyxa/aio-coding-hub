//! Usage: Discover installed CLIs and manage related local config (infra adapter).

use crate::shared::fs::{read_optional_file, write_file_atomic_if_changed};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::Manager;

const ENV_KEY_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
const ENV_KEY_DISABLE_ERROR_REPORTING: &str = "DISABLE_ERROR_REPORTING";

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
) -> Result<std::process::Output, String> {
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
                    .map_err(|e| format!("failed to collect output {label}: {e}"));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{label} timed out after {}ms", timeout.as_millis()));
                }
                std::thread::sleep(CMD_POLL_INTERVAL);
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to wait for {label}: {e}"));
            }
        }
    }
}

fn home_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .home_dir()
        .map_err(|e| format!("failed to resolve home dir: {e}"))
}

fn claude_config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(home_dir(app)?.join(".claude"))
}

fn claude_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(claude_config_dir(app)?.join("settings.json"))
}

fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> serde_json::Value {
    match bytes {
        Some(b) => serde_json::from_slice::<serde_json::Value>(&b)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

fn json_to_bytes(value: &serde_json::Value, hint: &str) -> Result<Vec<u8>, String> {
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

fn read_claude_env(settings_path: &Path) -> Result<(Option<u64>, bool), String> {
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
) -> Result<serde_json::Value, String> {
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

fn write_claude_env(
    app: &tauri::AppHandle,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> Result<(), String> {
    let settings_path = claude_settings_path(app)?;
    let current = read_optional_file(&settings_path)?;
    let root = json_root_from_bytes(current);
    let patched = patch_claude_env(root, mcp_timeout_ms, disable_error_reporting)?;
    let bytes = json_to_bytes(&patched, "claude/settings.json")?;
    let _ = write_file_atomic_if_changed(&settings_path, &bytes)?;
    Ok(())
}

fn exe_names_for(cmd: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        return vec![
            format!("{cmd}.exe"),
            format!("{cmd}.cmd"),
            format!("{cmd}.bat"),
            cmd.to_string(),
        ];
    }
    #[cfg(not(windows))]
    {
        vec![cmd.to_string()]
    }
}

fn find_exe_in_dir(dir: &Path, names: &[String]) -> Option<PathBuf> {
    for name in names {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
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

fn scan_executable(app: &tauri::AppHandle, cmd: &str) -> Result<Option<PathBuf>, String> {
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

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/opt/homebrew/bin"));
        candidates.push(PathBuf::from("/usr/local/bin"));
        candidates.push(PathBuf::from("/usr/bin"));
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from("/usr/local/bin"));
        candidates.push(PathBuf::from("/usr/bin"));
        candidates.push(PathBuf::from("/bin"));
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

fn is_fish_shell(shell: &Path) -> bool {
    shell
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("fish") || v.eq_ignore_ascii_case("fish.exe"))
        .unwrap_or(false)
}

fn run_in_login_shell(shell: &Path, script: &str) -> Result<String, String> {
    let mut cmd = Command::new(shell);

    #[cfg(not(windows))]
    {
        if is_fish_shell(shell) {
            cmd.arg("-l").arg("-c").arg(script);
        } else {
            cmd.arg("-lc").arg(script);
        }
    }

    #[cfg(windows)]
    {
        return Err(format!(
            "login shell resolution is not supported on windows (shell={})",
            shell.display()
        ));
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
            "unknown error".to_string()
        } else {
            msg
        });
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn resolve_executable_via_login_shell(cmd: &str) -> Result<Option<PathBuf>, String> {
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
    if candidate.exists() {
        return Ok(Some(candidate));
    }

    Ok(None)
}

fn run_version(exe: &Path) -> Result<String, String> {
    let mut cmd = Command::new(exe);
    cmd.arg("--version");

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
        "unknown error".to_string()
    } else {
        msg
    })
}

fn cli_probe(app: &tauri::AppHandle, cmd: &str) -> Result<CliProbeResult, String> {
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
            Err(err) => error = Some(err),
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

pub fn claude_info_get(app: &tauri::AppHandle) -> Result<ClaudeCliInfo, String> {
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

pub fn codex_info_get(app: &tauri::AppHandle) -> Result<SimpleCliInfo, String> {
    let probe = cli_probe(app, "codex")?;
    Ok(SimpleCliInfo {
        found: probe.found,
        executable_path: probe.executable_path,
        version: probe.version,
        error: probe.error,
        shell: probe.shell,
        resolved_via: probe.resolved_via,
    })
}

pub fn gemini_info_get(app: &tauri::AppHandle) -> Result<SimpleCliInfo, String> {
    let probe = cli_probe(app, "gemini")?;
    Ok(SimpleCliInfo {
        found: probe.found,
        executable_path: probe.executable_path,
        version: probe.version,
        error: probe.error,
        shell: probe.shell,
        resolved_via: probe.resolved_via,
    })
}

pub fn claude_env_set(
    app: &tauri::AppHandle,
    mcp_timeout_ms: Option<u64>,
    disable_error_reporting: bool,
) -> Result<ClaudeEnvState, String> {
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
