//! Usage: Windows WSL detection and per-distro client configuration helpers.

use crate::mcp_sync::McpServerForSync;
use crate::prompt_sync;
use crate::settings;
use crate::shared::error::{AppError, AppResult};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
pub struct WslDetection {
    pub detected: bool,
    pub distros: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslDistroConfigStatus {
    pub distro: String,
    pub claude: bool,
    pub codex: bool,
    pub gemini: bool,
    pub claude_mcp: bool,
    pub codex_mcp: bool,
    pub gemini_mcp: bool,
    pub claude_prompt: bool,
    pub codex_prompt: bool,
    pub gemini_prompt: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureCliReport {
    pub cli_key: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureDistroReport {
    pub distro: String,
    pub ok: bool,
    pub results: Vec<WslConfigureCliReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureReport {
    pub ok: bool,
    pub message: String,
    pub distros: Vec<WslConfigureDistroReport>,
}

#[cfg(windows)]
fn hide_window_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn hide_window_cmd(program: &str) -> Command {
    Command::new(program)
}

fn decode_utf16_le(mut bytes: &[u8]) -> String {
    // BOM (FF FE)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        bytes = &bytes[2..];
    }

    let len = bytes.len() - (bytes.len() % 2);
    let mut u16s = Vec::with_capacity(len / 2);
    for chunk in bytes[..len].chunks_exact(2) {
        u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    String::from_utf16_lossy(&u16s)
}

fn bash_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

/// Resolve the user HOME directory inside a WSL distro, returned as a Windows UNC path.
///
/// Example: distro `"Ubuntu"` → `\\wsl$\Ubuntu\home\diao`
pub fn resolve_wsl_home_unc(distro: &str) -> AppResult<PathBuf> {
    if !cfg!(windows) {
        return Err(AppError::new(
            "WSL_ERROR",
            "WSL is only available on Windows",
        ));
    }

    let output = hide_window_cmd("wsl")
        .args([
            "-d",
            distro,
            "--",
            "bash",
            "-lc",
            r#"getent passwd "$(whoami)" | cut -d: -f6"#,
        ])
        .output()
        .map_err(|e| AppError::new("WSL_ERROR", format!("failed to run wsl.exe: {e}")))?;

    if !output.status.success() {
        return Err(AppError::new(
            "WSL_ERROR",
            format!("wsl command failed for distro: {distro}"),
        ));
    }

    let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if home.is_empty() || !home.starts_with('/') {
        return Err(AppError::new(
            "WSL_ERROR",
            format!("invalid HOME for distro {distro}: {home}"),
        ));
    }

    // Build UNC path: \\wsl$\<distro><home_path_with_backslashes>
    let unc = format!(r"\\wsl$\{}{}", distro, home.replace('/', "\\"));
    Ok(PathBuf::from(unc))
}

/// Validate that a distro name is in the detected WSL distros list.
pub fn validate_distro(distro: &str) -> AppResult<()> {
    let trimmed = distro.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "distro is required"));
    }
    let detection = detect();
    if !detection.distros.iter().any(|d| d == trimmed) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("unknown WSL distro: {trimmed}"),
        ));
    }
    Ok(())
}

// ── WSL manifest (config lifecycle) ────────────────────────────────
// These types and functions are only called from `#[cfg(windows)]` blocks
// (cleanup.rs, lib.rs, configure_clients), so they appear unused on Linux CI.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslCliBackup {
    pub cli_key: String,
    /// Keys injected by AIO and the values written.
    pub injected_keys: std::collections::HashMap<String, String>,
    /// Original values before injection. `None` means key did not exist.
    pub original_values: std::collections::HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslDistroManifest {
    pub schema_version: u32,
    pub distro: String,
    pub configured: bool,
    pub proxy_origin: String,
    pub configured_at: i64,
    /// Cached UNC path to WSL home dir, so restore doesn't need to spawn wsl.exe.
    #[serde(default)]
    pub wsl_home_unc: Option<String>,
    pub cli_backups: Vec<WslCliBackup>,
}

fn wsl_manifests_dir(app: &tauri::AppHandle) -> AppResult<std::path::PathBuf> {
    Ok(crate::infra::app_paths::app_data_dir(app)?.join("wsl-manifests"))
}

fn wsl_manifest_path(app: &tauri::AppHandle, distro: &str) -> AppResult<std::path::PathBuf> {
    Ok(wsl_manifests_dir(app)?.join(format!("{distro}.json")))
}

fn read_wsl_manifest(app: &tauri::AppHandle, distro: &str) -> AppResult<Option<WslDistroManifest>> {
    let path = wsl_manifest_path(app, distro)?;
    let Some(content) = crate::shared::fs::read_optional_file(&path)? else {
        return Ok(None);
    };
    let manifest: WslDistroManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse WSL manifest for {distro}: {e}"))?;
    Ok(Some(manifest))
}

fn write_wsl_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    manifest: &WslDistroManifest,
) -> AppResult<()> {
    let path = wsl_manifest_path(app, distro)?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize WSL manifest: {e}"))?;
    crate::shared::fs::write_file_atomic(&path, json.as_bytes())
}

fn delete_wsl_manifest(app: &tauri::AppHandle, distro: &str) -> AppResult<()> {
    let path = wsl_manifest_path(app, distro)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("failed to delete WSL manifest for {distro}: {e}"))?;
    }
    Ok(())
}

fn read_all_wsl_manifests(app: &tauri::AppHandle) -> AppResult<Vec<WslDistroManifest>> {
    let dir = wsl_manifests_dir(app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut manifests = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read wsl-manifests dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        match serde_json::from_slice::<WslDistroManifest>(&bytes) {
            Ok(m) => manifests.push(m),
            Err(e) => {
                tracing::warn!("failed to parse WSL manifest {}: {e}", path.display());
            }
        }
    }
    Ok(manifests)
}

// ── Capture original values (pure Rust via UNC paths) ──

fn read_wsl_current_values(
    distro: &str,
    cli_key: &str,
) -> AppResult<std::collections::HashMap<String, Option<String>>> {
    let home = resolve_wsl_home_unc(distro)?;
    let mut map = std::collections::HashMap::new();

    match cli_key {
        "claude" => {
            let path = home.join(".claude").join("settings.json");
            let env = read_json_nested_str_map(&path, "env");
            map.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                env.get("ANTHROPIC_BASE_URL").cloned(),
            );
            map.insert(
                "ANTHROPIC_AUTH_TOKEN".to_string(),
                env.get("ANTHROPIC_AUTH_TOKEN").cloned(),
            );
        }
        "codex" => {
            let codex_home = home.join(".codex");
            // config.toml
            let toml_path = codex_home.join("config.toml");
            let toml_content = std::fs::read_to_string(&toml_path).unwrap_or_default();
            map.insert(
                "preferred_auth_method".to_string(),
                extract_toml_value(&toml_content, "preferred_auth_method"),
            );
            map.insert(
                "model_provider".to_string(),
                extract_toml_value(&toml_content, "model_provider"),
            );
            // auth.json
            let auth_path = codex_home.join("auth.json");
            let auth = read_json_top_level_str(&auth_path, "OPENAI_API_KEY");
            map.insert("OPENAI_API_KEY".to_string(), auth);
        }
        "gemini" => {
            let env_path = home.join(".gemini").join(".env");
            let env_content = std::fs::read_to_string(&env_path).unwrap_or_default();
            map.insert(
                "GOOGLE_GEMINI_BASE_URL".to_string(),
                extract_env_value(&env_content, "GOOGLE_GEMINI_BASE_URL"),
            );
            map.insert(
                "GEMINI_API_KEY".to_string(),
                extract_env_value(&env_content, "GEMINI_API_KEY"),
            );
        }
        _ => {}
    }

    Ok(map)
}

/// Read a JSON file, return a map of string values from a nested object key.
fn read_json_nested_str_map(
    path: &std::path::Path,
    key: &str,
) -> std::collections::HashMap<String, String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return std::collections::HashMap::new(),
    };
    let val: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return std::collections::HashMap::new(),
    };
    let Some(obj) = val.get(key).and_then(|v| v.as_object()) else {
        return std::collections::HashMap::new();
    };
    obj.iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect()
}

/// Read a top-level string value from a JSON file.
fn read_json_top_level_str(path: &std::path::Path, key: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let val: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    val.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a value from TOML like `key = "value"`.
fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim().trim_matches('"');
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

/// Extract a value from .env like `KEY=value`.
fn extract_env_value(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let check = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        if let Some(val) = check.strip_prefix(&prefix) {
            let val = val.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Write file and force sync to disk (critical for UNC/9P paths during exit).
fn write_file_synced(path: &std::path::Path, data: &[u8]) -> AppResult<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)
        .map_err(|e| format!("failed to create {}: {e}", path.display()))?;
    file.write_all(data)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("failed to sync {}: {e}", path.display()))?;
    Ok(())
}

// ── Restore WSL clients (pure Rust via UNC paths) ──

/// Restore a single CLI's config for a distro using the saved backup.
fn restore_wsl_cli_backup(home: &std::path::Path, backup: &WslCliBackup) -> AppResult<()> {
    match backup.cli_key.as_str() {
        "claude" => {
            let path = home.join(".claude").join("settings.json");
            if !path.exists() {
                return Ok(());
            }
            let bytes = std::fs::read(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let mut data: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;

            if let Some(env) = data.get_mut("env").and_then(|v| v.as_object_mut()) {
                for key in ["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"] {
                    match backup.original_values.get(key) {
                        Some(Some(val)) => {
                            env.insert(key.to_string(), serde_json::Value::String(val.clone()));
                        }
                        Some(None) | None => {
                            env.remove(key);
                        }
                    }
                }
            }

            let out = serde_json::to_string_pretty(&data)
                .map_err(|e| format!("failed to serialize: {e}"))?;
            write_file_synced(&path, format!("{out}\n").as_bytes())?;
        }
        "codex" => {
            let codex_home = home.join(".codex");

            // config.toml
            let toml_path = codex_home.join("config.toml");
            if toml_path.exists() {
                let content = std::fs::read_to_string(&toml_path)
                    .map_err(|e| format!("failed to read {}: {e}", toml_path.display()))?;
                let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

                for key in ["preferred_auth_method", "model_provider"] {
                    let prefix = key.to_string();
                    // Remove existing lines for this key
                    lines.retain(|l| {
                        let trimmed = l.trim();
                        !(trimmed.starts_with(&prefix)
                            && trimmed[prefix.len()..].trim_start().starts_with('='))
                    });
                    // Re-insert if original had a value
                    if let Some(Some(val)) = backup.original_values.get(key) {
                        lines.push(format!("{key} = \"{val}\""));
                    }
                }

                let out = lines.join("\n");
                let toml_out = if out.ends_with('\n') { out } else { out + "\n" };
                write_file_synced(&toml_path, toml_out.as_bytes())?;
            }

            // auth.json
            let auth_path = codex_home.join("auth.json");
            if auth_path.exists() {
                let bytes = std::fs::read(&auth_path)
                    .map_err(|e| format!("failed to read {}: {e}", auth_path.display()))?;
                let mut data: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("failed to parse {}: {e}", auth_path.display()))?;

                if let Some(obj) = data.as_object_mut() {
                    match backup.original_values.get("OPENAI_API_KEY") {
                        Some(Some(val)) => {
                            obj.insert(
                                "OPENAI_API_KEY".to_string(),
                                serde_json::Value::String(val.clone()),
                            );
                        }
                        Some(None) | None => {
                            obj.remove("OPENAI_API_KEY");
                        }
                    }
                }

                let out = serde_json::to_string_pretty(&data)
                    .map_err(|e| format!("failed to serialize: {e}"))?;
                write_file_synced(&auth_path, format!("{out}\n").as_bytes())?;
            }
        }
        "gemini" => {
            let env_path = home.join(".gemini").join(".env");
            if !env_path.exists() {
                return Ok(());
            }
            let content = std::fs::read_to_string(&env_path)
                .map_err(|e| format!("failed to read {}: {e}", env_path.display()))?;
            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

            for key in ["GOOGLE_GEMINI_BASE_URL", "GEMINI_API_KEY"] {
                let prefix = format!("{key}=");
                // Remove existing lines for this key
                lines.retain(|l| {
                    let trimmed = l.trim();
                    if trimmed.starts_with('#') {
                        return true;
                    }
                    let check = trimmed.strip_prefix("export ").unwrap_or(trimmed);
                    !check.starts_with(&prefix)
                });
                // Re-insert if original had a value
                if let Some(Some(val)) = backup.original_values.get(key) {
                    lines.push(format!("{key}={val}"));
                }
            }

            let out = lines.join("\n");
            let env_out = if out.ends_with('\n') { out } else { out + "\n" };
            write_file_synced(&env_path, env_out.as_bytes())?;
        }
        _ => {}
    }
    Ok(())
}

/// Restore WSL client configurations using saved manifests.
pub fn restore_wsl_clients(app: &tauri::AppHandle) -> AppResult<()> {
    let manifests = read_all_wsl_manifests(app)?;
    if manifests.is_empty() {
        return Ok(());
    }

    for manifest in &manifests {
        let distro = &manifest.distro;

        // Use cached UNC path; fall back to resolving (which needs wsl.exe)
        let home = match &manifest.wsl_home_unc {
            Some(p) => std::path::PathBuf::from(p),
            None => match resolve_wsl_home_unc(distro) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        "WSL restore skipped for {distro}: no cached home and resolve failed: {e}"
                    );
                    continue; // Don't delete manifest — retry next startup
                }
            },
        };

        let mut all_ok = true;
        for backup in &manifest.cli_backups {
            if let Err(e) = restore_wsl_cli_backup(&home, backup) {
                tracing::warn!("WSL restore failed for {} in {distro}: {e}", backup.cli_key);
                all_ok = false;
            } else {
                tracing::info!("WSL restore succeeded for {} in {distro}", backup.cli_key);
            }
        }

        // Only delete manifest if all restores succeeded
        if all_ok {
            if let Err(e) = delete_wsl_manifest(app, distro) {
                tracing::warn!("failed to delete WSL manifest for {distro}: {e}");
            }
        } else {
            tracing::warn!("WSL manifest for {distro} kept — some restores failed, will retry");
        }
    }
    Ok(())
}

/// Fallback restore: remove known injected keys without relying on manifest.
#[allow(dead_code)]
pub fn restore_wsl_clients_fallback(_app: &tauri::AppHandle, distros: &[String]) -> AppResult<()> {
    for distro in distros {
        let home = match resolve_wsl_home_unc(distro) {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("WSL fallback restore: cannot resolve home for {distro}: {e}");
                continue;
            }
        };

        // Claude: remove injected keys from settings.json
        let claude_path = home.join(".claude").join("settings.json");
        if claude_path.exists() {
            if let Ok(bytes) = std::fs::read(&claude_path) {
                if let Ok(mut data) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(env) = data.get_mut("env").and_then(|v| v.as_object_mut()) {
                        env.remove("ANTHROPIC_BASE_URL");
                        env.remove("ANTHROPIC_AUTH_TOKEN");
                    }
                    if let Ok(out) = serde_json::to_string_pretty(&data) {
                        let _ = std::fs::write(&claude_path, format!("{out}\n"));
                    }
                }
            }
        }

        // Codex: remove injected keys
        let codex_home = home.join(".codex");
        let toml_path = codex_home.join("config.toml");
        if toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&toml_path) {
                let lines: Vec<&str> = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        !t.starts_with("preferred_auth_method") && !t.starts_with("model_provider")
                    })
                    .collect();
                let _ = std::fs::write(&toml_path, lines.join("\n") + "\n");
            }
        }
        let auth_path = codex_home.join("auth.json");
        if auth_path.exists() {
            if let Ok(bytes) = std::fs::read(&auth_path) {
                if let Ok(mut data) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(obj) = data.as_object_mut() {
                        obj.remove("OPENAI_API_KEY");
                    }
                    if let Ok(out) = serde_json::to_string_pretty(&data) {
                        let _ = std::fs::write(&auth_path, format!("{out}\n"));
                    }
                }
            }
        }

        // Gemini: remove injected keys from .env
        let gemini_env = home.join(".gemini").join(".env");
        if gemini_env.exists() {
            if let Ok(content) = std::fs::read_to_string(&gemini_env) {
                let lines: Vec<&str> = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        let check = t.strip_prefix("export ").unwrap_or(t);
                        !check.starts_with("GOOGLE_GEMINI_BASE_URL=")
                            && !check.starts_with("GEMINI_API_KEY=")
                    })
                    .collect();
                let _ = std::fs::write(&gemini_env, lines.join("\n") + "\n");
            }
        }
    }
    Ok(())
}

// ── Startup repair ──

/// Check for stale manifests at startup and restore if the gateway is dead.
pub fn startup_repair_wsl_manifests(app: &tauri::AppHandle) -> AppResult<()> {
    let manifests = read_all_wsl_manifests(app)?;
    if manifests.is_empty() {
        return Ok(());
    }

    for manifest in &manifests {
        let origin = &manifest.proxy_origin;
        // Extract port from proxy_origin (e.g. "http://172.x.x.x:12345")
        let port_alive = origin
            .rsplit(':')
            .next()
            .and_then(|p| p.trim_end_matches('/').parse::<u16>().ok())
            .map(|port| {
                // Quick check: try connecting to the port
                std::net::TcpStream::connect_timeout(
                    &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    std::time::Duration::from_millis(500),
                )
                .is_ok()
            })
            .unwrap_or(false);

        if port_alive {
            tracing::debug!(
                "WSL manifest for {} still alive (proxy_origin={origin}), keeping",
                manifest.distro
            );
            continue;
        }

        tracing::info!(
            "WSL manifest for {} has dead gateway (proxy_origin={origin}), restoring",
            manifest.distro
        );

        let home = match &manifest.wsl_home_unc {
            Some(p) => std::path::PathBuf::from(p),
            None => match resolve_wsl_home_unc(&manifest.distro) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        "startup WSL restore skipped for {}: no cached home and resolve failed: {e}",
                        manifest.distro
                    );
                    continue;
                }
            },
        };

        for backup in &manifest.cli_backups {
            if let Err(e) = restore_wsl_cli_backup(&home, backup) {
                tracing::warn!(
                    "startup WSL restore failed for {} in {}: {e}",
                    backup.cli_key,
                    manifest.distro
                );
            }
        }
        if let Err(e) = delete_wsl_manifest(app, &manifest.distro) {
            tracing::warn!(
                "failed to delete stale WSL manifest for {}: {e}",
                manifest.distro
            );
        }
    }
    Ok(())
}

pub fn detect() -> WslDetection {
    let mut out = WslDetection {
        detected: false,
        distros: Vec::new(),
    };

    if !cfg!(windows) {
        return out;
    }

    let output = hide_window_cmd("wsl").args(["--list", "--quiet"]).output();
    let Ok(output) = output else {
        return out;
    };
    if !output.status.success() {
        return out;
    }

    let decoded = decode_utf16_le(&output.stdout);
    for line in decoded.lines() {
        let mut distro = line.trim().to_string();
        distro = distro.trim_matches(&['\0', '\r'][..]).trim().to_string();
        if distro.is_empty() {
            continue;
        }
        if distro.starts_with("Windows") {
            continue;
        }
        out.distros.push(distro);
    }

    out.detected = !out.distros.is_empty();
    out
}

/// Resolve the host address that WSL distros should use to reach the gateway.
///
/// This is used by:
/// - Gateway listen mode `wsl_auto` (bind host)
/// - WSL client configuration (base origin host)
pub fn resolve_wsl_host(cfg: &settings::AppSettings) -> String {
    match cfg.wsl_host_address_mode {
        settings::WslHostAddressMode::Custom => {
            let addr = cfg.wsl_custom_host_address.trim();
            if addr.is_empty() {
                "127.0.0.1".to_string()
            } else {
                addr.to_string()
            }
        }
        settings::WslHostAddressMode::Auto => {
            host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
        }
    }
}

pub fn host_ipv4_best_effort() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }

    let output = hide_window_cmd("ipconfig").output().ok()?;
    let stdout = {
        let utf8 = String::from_utf8_lossy(&output.stdout).to_string();
        if utf8.contains('\0') {
            let decoded = decode_utf16_le(&output.stdout);
            let trimmed = decoded.trim().to_string();
            if trimmed.is_empty() {
                utf8
            } else {
                trimmed
            }
        } else {
            utf8
        }
    };
    use std::net::Ipv4Addr;

    let mut in_wsl_adapter = false;
    for raw_line in stdout.lines() {
        let line = raw_line.trim().trim_matches('\0');

        if line.contains("vEthernet (WSL)")
            || line.contains("vEthernet(WSL)")
            || line.contains("Ethernet adapter vEthernet (WSL)")
        {
            in_wsl_adapter = true;
            continue;
        }

        // Adapter section boundary (English + Chinese output). If localized, we keep scanning until we see IPv4.
        if in_wsl_adapter
            && line.ends_with(':')
            && (line.contains("adapter") || line.contains("适配器"))
            && !line.contains("WSL")
        {
            break;
        }

        if !in_wsl_adapter {
            continue;
        }

        if line.contains("IPv4") || line.contains("IP Address") {
            let Some((_, tail)) = line.rsplit_once(':').or_else(|| line.rsplit_once('：')) else {
                continue;
            };
            let ip = tail.trim();
            if ip.is_empty() || ip.contains(':') {
                continue;
            }
            if ip.parse::<Ipv4Addr>().is_ok() {
                return Some(ip.to_string());
            }
        }
    }

    None
}

fn run_wsl_bash_script(distro: &str, script: &str) -> crate::shared::error::AppResult<()> {
    let mut cmd = hide_window_cmd("wsl");
    cmd.args(["-d", distro, "bash"]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn wsl: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("failed to write wsl stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for wsl: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // wsl.exe on non-English Windows may emit UTF-16LE warnings on stderr;
    // bash/python errors inside the distro are UTF-8.  Try UTF-8 first and
    // fall back to UTF-16LE when null bytes are present (a strong indicator).
    let stderr_raw = &output.stderr;
    let stderr = {
        let utf8 = String::from_utf8_lossy(stderr_raw).trim().to_string();
        if utf8.contains('\0') {
            let decoded = decode_utf16_le(stderr_raw);
            let trimmed = decoded.trim().to_string();
            if trimmed.is_empty() {
                utf8
            } else {
                trimmed
            }
        } else {
            utf8
        }
    };
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!(
        "WSL_ERROR: {}",
        if msg.is_empty() {
            "unknown error"
        } else {
            &msg
        }
    )
    .into())
}

/// Execute a bash script inside a WSL distro and capture its stdout.
fn run_wsl_bash_script_capture(
    distro: &str,
    script: &str,
) -> crate::shared::error::AppResult<String> {
    let mut cmd = hide_window_cmd("wsl");
    cmd.args(["-d", distro, "bash"]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn wsl: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("failed to write wsl stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for wsl: {e}"))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr_raw = &output.stderr;
    let stderr = {
        let utf8 = String::from_utf8_lossy(stderr_raw).trim().to_string();
        if utf8.contains('\0') {
            let decoded = decode_utf16_le(stderr_raw);
            let trimmed = decoded.trim().to_string();
            if trimmed.is_empty() {
                utf8
            } else {
                trimmed
            }
        } else {
            utf8
        }
    };
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!(
        "WSL_ERROR: {}",
        if msg.is_empty() {
            "unknown error"
        } else {
            &msg
        }
    )
    .into())
}

/// Read a file from WSL using base64 encoding. Returns None if file does not exist.
fn read_wsl_file(distro: &str, path_expr: &str) -> AppResult<Option<Vec<u8>>> {
    use base64::Engine;

    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
if [ ! -f "$target" ]; then
  echo "AIO_WSL_FILE_NOT_FOUND"
  exit 0
fi
base64 -w0 "$target"
echo ""
"#
    );
    let stdout = run_wsl_bash_script_capture(distro, &script)?;
    let trimmed = stdout.trim();
    if trimmed == "AIO_WSL_FILE_NOT_FOUND" {
        return Ok(None);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| format!("WSL_ERROR: base64 decode failed: {e}"))?;
    Ok(Some(bytes))
}

/// Atomically write a file into WSL (backup + tmp + mv).
fn write_wsl_file(distro: &str, path_expr: &str, content: &[u8]) -> AppResult<()> {
    use base64::Engine;

    let b64 = base64::engine::general_purpose::STANDARD.encode(content);
    let path_escaped = bash_single_quote(path_expr);
    let b64_escaped = bash_single_quote(&b64);

    let script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

target={path_escaped}
dir="$(dirname "$target")"
mkdir -p "$dir"

ts="$(date +%s)"
if [ -f "$target" ]; then
  cp -a "$target" "$target.bak.$ts"
fi

tmp_path="$(mktemp "${{target}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

echo {b64_escaped} | base64 -d > "$tmp_path"

if [ -f "$target" ]; then
  chmod --reference="$target" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$target"
trap - EXIT
"#
    );
    run_wsl_bash_script(distro, &script)
}

fn remove_wsl_file(distro: &str, path_expr: &str) -> AppResult<()> {
    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
rm -f -- "$target"
"#
    );
    run_wsl_bash_script(distro, &script)
}

// ── MCP/Prompt WSL sync data structures ──

/// MCP sync data for all CLIs, used when syncing to WSL.
pub struct WslMcpSyncData {
    pub claude: Vec<McpServerForSync>,
    pub codex: Vec<McpServerForSync>,
    pub gemini: Vec<McpServerForSync>,
}

/// Prompt sync data for all CLIs, used when syncing to WSL.
pub struct WslPromptSyncData {
    pub claude_content: Option<String>,
    pub codex_content: Option<String>,
    pub gemini_content: Option<String>,
}

const WSL_PROMPT_MANIFEST_SCHEMA_VERSION: u32 = 1;
const WSL_PROMPT_MANAGED_BY: &str = "aio-coding-hub";

// ── WSL MCP manifest ──

/// Tracks which MCP server keys were synced to a WSL distro for a specific CLI,
/// so we can properly remove them on the next sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslMcpManifest {
    distro: String,
    cli_key: String,
    managed_keys: Vec<String>,
    updated_at: i64,
}

fn wsl_mcp_manifest_path(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    let dir = crate::app_paths::app_data_dir(app)?
        .join("wsl-mcp-sync")
        .join(distro)
        .join(cli_key);
    Ok(dir.join("manifest.json"))
}

fn read_wsl_mcp_manifest(app: &tauri::AppHandle, distro: &str, cli_key: &str) -> Vec<String> {
    let path = match wsl_mcp_manifest_path(app, distro, cli_key) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_slice::<WslMcpManifest>(&bytes) {
        Ok(m) => m.managed_keys,
        Err(_) => Vec::new(),
    }
}

fn write_wsl_mcp_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    managed_keys: &[String],
) -> AppResult<()> {
    let path = wsl_mcp_manifest_path(app, distro, cli_key)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create wsl-mcp-sync dir: {e}"))?;
    }
    let manifest = WslMcpManifest {
        distro: distro.to_string(),
        cli_key: cli_key.to_string(),
        managed_keys: managed_keys.to_vec(),
        updated_at: crate::shared::time::now_unix_seconds(),
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize wsl mcp manifest: {e}"))?;
    std::fs::write(&path, json.as_bytes())
        .map_err(|e| format!("failed to write wsl mcp manifest: {e}"))?;
    Ok(())
}

// ── WSL prompt manifest ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslPromptSyncFileEntry {
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WslPromptManifest {
    schema_version: u32,
    managed_by: String,
    distro: String,
    cli_key: String,
    enabled: bool,
    created_at: i64,
    updated_at: i64,
    file: WslPromptSyncFileEntry,
}

fn wsl_prompt_sync_root_dir(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    Ok(crate::app_paths::app_data_dir(app)?
        .join("wsl-prompt-sync")
        .join(distro)
        .join(cli_key))
}

fn wsl_prompt_files_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("files")
}

fn wsl_prompt_manifest_path(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<std::path::PathBuf> {
    Ok(wsl_prompt_sync_root_dir(app, distro, cli_key)?.join("manifest.json"))
}

fn read_wsl_prompt_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
) -> AppResult<Option<WslPromptManifest>> {
    let path = wsl_prompt_manifest_path(app, distro, cli_key)?;
    let bytes = match std::fs::read(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!("failed to read WSL prompt manifest: {err}").into());
        }
    };

    let manifest: WslPromptManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("failed to parse WSL prompt manifest: {e}"))?;
    if manifest.managed_by != WSL_PROMPT_MANAGED_BY {
        return Err(format!(
            "WSL prompt manifest managed_by mismatch: expected {WSL_PROMPT_MANAGED_BY}, got {}",
            manifest.managed_by
        )
        .into());
    }
    Ok(Some(manifest))
}

fn write_wsl_prompt_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    manifest: &WslPromptManifest,
) -> AppResult<()> {
    let root = wsl_prompt_sync_root_dir(app, distro, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create WSL prompt sync dir: {e}"))?;
    let path = wsl_prompt_manifest_path(app, distro, cli_key)?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize WSL prompt manifest: {e}"))?;
    std::fs::write(&path, json.as_bytes())
        .map_err(|e| format!("failed to write WSL prompt manifest: {e}"))?;
    Ok(())
}

// ── Path adaptation for WSL ──

/// Strip Windows executable extensions (.cmd, .bat, .exe) from a command name.
fn strip_win_exe_ext(cmd: &str) -> &str {
    for ext in &[".cmd", ".bat", ".exe", ".CMD", ".BAT", ".EXE"] {
        if let Some(stripped) = cmd.strip_suffix(ext) {
            return stripped;
        }
    }
    cmd
}

/// Check if a path looks like a Windows absolute path (e.g., `C:\...`).
fn is_windows_absolute_path(p: &str) -> bool {
    let bytes = p.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && (bytes[1] == b':')
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Adapt MCP servers for WSL: strip .cmd/.bat/.exe extensions from commands,
/// and convert Windows absolute paths to WSL `/mnt/` mount paths in command,
/// args, cwd, and env values.
pub fn adapt_mcp_servers_for_wsl(servers: &[McpServerForSync]) -> Vec<McpServerForSync> {
    servers
        .iter()
        .map(|s| {
            let mut adapted = McpServerForSync {
                server_key: s.server_key.clone(),
                transport: s.transport.clone(),
                command: s.command.clone(),
                args: s.args.clone(),
                env: s.env.clone(),
                cwd: s.cwd.clone(),
                url: s.url.clone(),
                headers: s.headers.clone(),
            };

            // Adapt command
            if let Some(ref cmd) = adapted.command {
                let stripped = strip_win_exe_ext(cmd);
                if is_windows_absolute_path(stripped) {
                    // Cannot resolve at build time; skip this server's command conversion.
                    // WSL users will likely have the tool installed natively.
                    // Just use the basename without extension.
                    let basename = stripped.rsplit(['\\', '/']).next().unwrap_or(stripped);
                    adapted.command = Some(basename.to_string());
                } else {
                    adapted.command = Some(stripped.to_string());
                }
            }

            // Adapt args: convert any Windows absolute paths to WSL /mnt/ paths
            adapted.args = adapted
                .args
                .iter()
                .map(|arg| {
                    if is_windows_absolute_path(arg) {
                        win_path_to_wsl_mount(arg)
                    } else {
                        arg.clone()
                    }
                })
                .collect();

            // Adapt cwd
            if let Some(ref cwd) = adapted.cwd {
                if is_windows_absolute_path(cwd) {
                    // Convert backslashes to forward slashes for wslpath compatibility.
                    // Use /mnt/<drive>/... heuristic since we can't call wslpath at build time.
                    let converted = win_path_to_wsl_mount(cwd);
                    adapted.cwd = Some(converted);
                }
            }

            // Adapt env values: convert any Windows absolute paths to WSL /mnt/ paths
            adapted.env = adapted
                .env
                .iter()
                .map(|(k, v)| {
                    let converted = if is_windows_absolute_path(v) {
                        win_path_to_wsl_mount(v)
                    } else {
                        v.clone()
                    };
                    (k.clone(), converted)
                })
                .collect();

            adapted
        })
        .collect()
}

/// Best-effort Windows path to WSL /mnt/ path conversion.
/// E.g., `C:\Users\foo\bar` → `/mnt/c/Users/foo/bar`
fn win_path_to_wsl_mount(win_path: &str) -> String {
    let bytes = win_path.as_bytes();
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let rest = &win_path[2..].replace('\\', "/");
        format!("/mnt/{drive}{rest}")
    } else {
        win_path.replace('\\', "/")
    }
}

// ── WSL MCP sync ──

/// Sync MCP configuration for a single CLI to a WSL distro.
/// Uses the existing `build_next_bytes` to merge servers into the config.
fn sync_wsl_mcp_for_cli(
    distro: &str,
    cli_key: &str,
    servers: &[McpServerForSync],
    managed_keys: &[String],
) -> AppResult<Vec<String>> {
    if !matches!(cli_key, "claude" | "codex" | "gemini") {
        return Err(format!("unknown cli_key: {cli_key}").into());
    }

    // Resolve the config file path inside WSL (handles CODEX_HOME, etc.)
    let resolve_script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
codex_home_raw="${{CODEX_HOME:-$HOME/.codex}}"
p="$codex_home_raw"
if [ "$codex_home_raw" = "~" ]; then p="$HOME"
elif [ "${{codex_home_raw#~/}}" != "$codex_home_raw" ]; then p="$HOME/${{codex_home_raw#~/}}"
elif [ "${{codex_home_raw#/}}" = "$codex_home_raw" ]; then p="$HOME/$codex_home_raw"
fi
case {cli_key} in
  claude) echo "$HOME/.claude.json" ;;
  codex) echo "$p/config.toml" ;;
  gemini) echo "$HOME/.gemini/settings.json" ;;
esac
"#,
        cli_key = bash_single_quote(cli_key)
    );

    let resolved_path = run_wsl_bash_script_capture(distro, &resolve_script)?;
    let resolved_path = resolved_path.trim();

    // Read current config from WSL
    let current = read_wsl_file(distro, resolved_path)?;

    // Build merged config using existing infrastructure
    let next_bytes = crate::mcp_sync::build_next_bytes(cli_key, current, managed_keys, servers)
        .map_err(|e| format!("WSL MCP build failed for {cli_key}: {e}"))?;

    // Write back to WSL
    write_wsl_file(distro, resolved_path, &next_bytes)?;

    // Return list of keys we now manage
    let mut keys: Vec<String> = servers.iter().map(|s| s.server_key.clone()).collect();
    keys.sort();
    keys.dedup();
    Ok(keys)
}

// ── WSL Prompt sync ──

/// Resolve the prompt target file path inside a WSL distro.
/// Returns an absolute path like `/home/user/.claude/CLAUDE.md`.
fn resolve_wsl_prompt_path(distro: &str, cli_key: &str) -> AppResult<String> {
    let resolve_script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
codex_home_raw="${{CODEX_HOME:-$HOME/.codex}}"
p="$codex_home_raw"
if [ "$codex_home_raw" = "~" ]; then p="$HOME"
elif [ "${{codex_home_raw#~/}}" != "$codex_home_raw" ]; then p="$HOME/${{codex_home_raw#~/}}"
elif [ "${{codex_home_raw#/}}" = "$codex_home_raw" ]; then p="$HOME/$codex_home_raw"
fi
case {cli_key} in
  claude) echo "$HOME/.claude/CLAUDE.md" ;;
  codex) echo "$p/AGENTS.md" ;;
  gemini) echo "$HOME/.gemini/GEMINI.md" ;;
esac
"#,
        cli_key = bash_single_quote(cli_key)
    );
    let resolved = run_wsl_bash_script_capture(distro, &resolve_script)?;
    let resolved = resolved.trim().to_string();
    if resolved.is_empty() || !resolved.starts_with('/') {
        return Err(format!("failed to resolve prompt path for {cli_key}: {resolved}").into());
    }
    Ok(resolved)
}

/// Sync a prompt file for a single CLI to a WSL distro.
fn backup_wsl_prompt_for_enable(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    target_path: &str,
    existing: Option<WslPromptManifest>,
) -> AppResult<WslPromptManifest> {
    let root = wsl_prompt_sync_root_dir(app, distro, cli_key)?;
    let files_dir = wsl_prompt_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create WSL prompt files dir: {e}"))?;

    let existing_bytes = read_wsl_file(distro, target_path)?;
    let backup_rel = if let Some(bytes) = existing_bytes {
        let backup_name = std::path::Path::new(target_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("prompt.md")
            .to_string();
        let backup_path = files_dir.join(&backup_name);
        std::fs::write(&backup_path, bytes)
            .map_err(|e| format!("failed to write WSL prompt backup: {e}"))?;
        Some(backup_name)
    } else {
        None
    };

    let now = crate::shared::time::now_unix_seconds();
    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);
    Ok(WslPromptManifest {
        schema_version: WSL_PROMPT_MANIFEST_SCHEMA_VERSION,
        managed_by: WSL_PROMPT_MANAGED_BY.to_string(),
        distro: distro.to_string(),
        cli_key: cli_key.to_string(),
        enabled: false,
        created_at,
        updated_at: now,
        file: WslPromptSyncFileEntry {
            path: target_path.to_string(),
            existed: backup_rel.is_some(),
            backup_rel,
        },
    })
}

fn restore_wsl_prompt_from_manifest(
    app: &tauri::AppHandle,
    distro: &str,
    manifest: &WslPromptManifest,
) -> AppResult<()> {
    let target_path = manifest.file.path.as_str();
    if manifest.file.existed {
        let Some(backup_rel) = manifest.file.backup_rel.as_ref() else {
            return Err("WSL prompt restore backup missing".into());
        };
        let backup_root = wsl_prompt_files_dir(&wsl_prompt_sync_root_dir(
            app,
            &manifest.distro,
            &manifest.cli_key,
        )?);
        let backup_path = backup_root.join(backup_rel);
        let bytes = std::fs::read(&backup_path)
            .map_err(|e| format!("failed to read WSL prompt backup: {e}"))?;
        return write_wsl_file(distro, target_path, &bytes);
    }

    remove_wsl_file(distro, target_path)
}

fn sync_wsl_prompt_for_cli(
    app: &tauri::AppHandle,
    distro: &str,
    cli_key: &str,
    content: Option<&str>,
) -> AppResult<()> {
    if !matches!(cli_key, "claude" | "codex" | "gemini") {
        return Err(format!("unknown cli_key: {cli_key}").into());
    }

    // Resolve to an absolute path inside WSL (e.g. /home/user/.codex/AGENTS.md)
    // Must NOT pass $HOME as a literal string — bash_single_quote would prevent expansion.
    let target_path = resolve_wsl_prompt_path(distro, cli_key)?;
    let trimmed = content.map(str::trim).filter(|value| !value.is_empty());
    let existing = read_wsl_prompt_manifest(app, distro, cli_key)?;

    match trimmed {
        Some(content) => {
            let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);
            let mut manifest = if should_backup {
                backup_wsl_prompt_for_enable(app, distro, cli_key, &target_path, existing.clone())?
            } else {
                existing.ok_or_else(|| "WSL prompt manifest missing while enabled".to_string())?
            };

            if should_backup {
                write_wsl_prompt_manifest(app, distro, cli_key, &manifest)?;
            }

            let bytes = prompt_sync::prompt_content_to_bytes(content);
            write_wsl_file(distro, &target_path, &bytes)?;

            manifest.enabled = true;
            manifest.updated_at = crate::shared::time::now_unix_seconds();
            manifest.file.path = target_path;
            write_wsl_prompt_manifest(app, distro, cli_key, &manifest)
        }
        None => {
            let Some(mut manifest) = existing else {
                return Ok(());
            };
            if !manifest.enabled {
                return Ok(());
            }

            restore_wsl_prompt_from_manifest(app, distro, &manifest)?;
            manifest.enabled = false;
            manifest.updated_at = crate::shared::time::now_unix_seconds();
            write_wsl_prompt_manifest(app, distro, cli_key, &manifest)
        }
    }
}

// ── Data gathering ──

/// Gather MCP sync data from the database for all CLIs.
pub fn gather_mcp_sync_data(conn: &rusqlite::Connection) -> AppResult<WslMcpSyncData> {
    let gather_for_cli = |cli_key: &str| -> AppResult<Vec<McpServerForSync>> {
        let servers = crate::mcp::list_enabled_for_cli(conn, cli_key)?;
        Ok(adapt_mcp_servers_for_wsl(&servers))
    };

    Ok(WslMcpSyncData {
        claude: gather_for_cli("claude")?,
        codex: gather_for_cli("codex")?,
        gemini: gather_for_cli("gemini")?,
    })
}

/// Gather prompt sync data from the database for all CLIs.
pub fn gather_prompt_sync_data(conn: &rusqlite::Connection) -> AppResult<WslPromptSyncData> {
    let get_for_cli = |cli_key: &str| -> AppResult<Option<String>> {
        let Some(workspace_id) = crate::workspaces::active_id_by_cli(conn, cli_key)? else {
            return Ok(None);
        };
        let content: Option<String> = conn
            .query_row(
                r#"
SELECT content
FROM prompts
WHERE workspace_id = ?1 AND enabled = 1
ORDER BY updated_at DESC, id DESC
LIMIT 1
"#,
                rusqlite::params![workspace_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("DB_ERROR: failed to query enabled prompt for {cli_key}: {e}"))?;
        Ok(content)
    };

    Ok(WslPromptSyncData {
        claude_content: get_for_cli("claude")?,
        codex_content: get_for_cli("codex")?,
        gemini_content: get_for_cli("gemini")?,
    })
}

fn configure_wsl_claude(distro: &str, proxy_origin: &str) -> crate::shared::error::AppResult<()> {
    let base_url = format!("{proxy_origin}/claude");
    let base_url = bash_single_quote(&base_url);
    let auth_token = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

mkdir -p "$HOME/.claude"
config_path="$HOME/.claude/settings.json"

if [ -L "$config_path" ]; then
  echo "Refusing to modify: $config_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

base_url={base_url}
auth_token={auth_token}

ts="$(date +%s)"
if [ -f "$config_path" ]; then
  cp -a "$config_path" "$config_path.bak.$ts"
fi

tmp_path="$(mktemp "${{config_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

if command -v jq >/dev/null 2>&1; then
  if [ -s "$config_path" ]; then
    if ! jq -e 'type=="object" and (.env==null or (.env|type)=="object")' "$config_path" >/dev/null; then
      echo "Refusing to modify: $config_path must be a JSON object and env must be an object (or null)." >&2
      exit 2
    fi

    jq --arg base_url "$base_url" --arg auth_token "$auth_token" '
      .env = (.env // {{}})
      | .env.ANTHROPIC_BASE_URL = $base_url
      | .env.ANTHROPIC_AUTH_TOKEN = $auth_token
    ' "$config_path" > "$tmp_path"
  else
    jq -n --arg base_url "$base_url" --arg auth_token "$auth_token" '{{env:{{ANTHROPIC_BASE_URL:$base_url, ANTHROPIC_AUTH_TOKEN:$auth_token}}}}' > "$tmp_path"
  fi

  jq -e --arg base_url "$base_url" --arg auth_token "$auth_token" '
    .env.ANTHROPIC_BASE_URL == $base_url and .env.ANTHROPIC_AUTH_TOKEN == $auth_token
  ' "$tmp_path" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
  python3 - "$base_url" "$auth_token" "$config_path" "$tmp_path" <<'PY'
import json
import sys
from pathlib import Path

base_url, auth_token, src, dst = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]

data = {{}}
try:
    text = Path(src).read_text(encoding="utf-8")
    if text.strip():
        data = json.loads(text)
except FileNotFoundError:
    data = {{}}
except Exception as e:
    sys.stderr.write(f"Failed to parse existing settings.json: {{e}}\n")
    sys.exit(2)

if not isinstance(data, dict):
    sys.stderr.write("settings.json must be a JSON object\\n")
    sys.exit(2)

env = data.get("env")
if env is None:
    env = {{}}
if not isinstance(env, dict):
    sys.stderr.write("settings.json env must be a JSON object\\n")
    sys.exit(2)

env["ANTHROPIC_BASE_URL"] = base_url
env["ANTHROPIC_AUTH_TOKEN"] = auth_token
data["env"] = env

Path(dst).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

  python3 - "$base_url" "$auth_token" "$tmp_path" <<'PY'
import json
import sys
from pathlib import Path

base_url, auth_token, path = sys.argv[1], sys.argv[2], sys.argv[3]
payload = json.loads(Path(path).read_text(encoding="utf-8"))
ok = (
    isinstance(payload, dict)
    and isinstance(payload.get("env"), dict)
    and payload["env"].get("ANTHROPIC_BASE_URL") == base_url
    and payload["env"].get("ANTHROPIC_AUTH_TOKEN") == auth_token
)
if not ok:
    sys.stderr.write("Sanity check failed for generated settings.json\\n")
    sys.exit(2)
PY
else
  if [ -s "$config_path" ]; then
    echo "Missing jq/python3; cannot safely merge existing $config_path" >&2
    exit 2
  fi

  cat > "$tmp_path" <<EOF
{{
  "env": {{
    "ANTHROPIC_BASE_URL": "$base_url",
    "ANTHROPIC_AUTH_TOKEN": "$auth_token"
  }}
}}
EOF
fi

if [ ! -s "$tmp_path" ]; then
  echo "Sanity check failed: generated settings.json is empty" >&2
  exit 2
fi

if [ -f "$config_path" ]; then
  chmod --reference="$config_path" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$config_path"
trap - EXIT

claude_json_path="$HOME/.claude.json"

if [ -L "$claude_json_path" ]; then
  echo "Skipping: $claude_json_path is a symlink." >&2
else
  cj_tmp="$(mktemp "${{claude_json_path}}.tmp.XXXXXX")"
  cj_cleanup() {{ rm -f "$cj_tmp"; }}
  trap cj_cleanup EXIT

  if command -v jq >/dev/null 2>&1; then
    if [ -s "$claude_json_path" ]; then
      jq '.hasCompletedOnboarding = true' "$claude_json_path" > "$cj_tmp"
    else
      jq -n '{{hasCompletedOnboarding: true}}' > "$cj_tmp"
    fi
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$claude_json_path" "$cj_tmp" <<'PY'
import json, sys
from pathlib import Path
src, dst = sys.argv[1], sys.argv[2]
data = {{}}
try:
    text = Path(src).read_text(encoding="utf-8")
    if text.strip():
        data = json.loads(text)
except FileNotFoundError:
    pass
if not isinstance(data, dict):
    data = {{}}
data["hasCompletedOnboarding"] = True
Path(dst).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
  else
    if [ -s "$claude_json_path" ]; then
      echo "Missing jq/python3; cannot safely merge existing $claude_json_path" >&2
    else
      echo '{{"hasCompletedOnboarding":true}}' > "$cj_tmp"
    fi
  fi

  if [ -s "$cj_tmp" ]; then
    if [ -f "$claude_json_path" ]; then
      chmod --reference="$claude_json_path" "$cj_tmp" 2>/dev/null || true
    fi
    mv -f "$cj_tmp" "$claude_json_path"
  fi
  trap - EXIT
fi
"#
    );

    run_wsl_bash_script(distro, &script)
}

fn configure_wsl_codex(distro: &str, proxy_origin: &str) -> crate::shared::error::AppResult<()> {
    let base_url = format!("{proxy_origin}/v1");
    let base_url = bash_single_quote(&base_url);
    let provider_key = bash_single_quote("aio");
    let api_key = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

	HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
	export HOME

		codex_home_raw="${{CODEX_HOME:-$HOME/.codex}}"
		codex_home="$codex_home_raw"
		if [ "$codex_home_raw" = "~" ]; then
		  codex_home="$HOME"
		elif [ "${{codex_home_raw#~/}}" != "$codex_home_raw" ]; then
		  codex_home="$HOME/${{codex_home_raw#~/}}"
		elif [ "${{codex_home_raw#~\\}}" != "$codex_home_raw" ]; then
		  codex_home="$HOME/${{codex_home_raw#~\\}}"
		elif [ "${{codex_home_raw#/}}" = "$codex_home_raw" ]; then
		  codex_home="$HOME/$codex_home_raw"
		fi

	mkdir -p "$codex_home"
	config_path="$codex_home/config.toml"
	auth_path="$codex_home/auth.json"

if [ -L "$config_path" ]; then
  echo "Refusing to modify: $config_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi
if [ -L "$auth_path" ]; then
  echo "Refusing to modify: $auth_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

base_url={base_url}
provider_key={provider_key}
api_key={api_key}

ts="$(date +%s)"
[ -f "$config_path" ] && cp -a "$config_path" "$config_path.bak.$ts"
[ -f "$auth_path" ] && cp -a "$auth_path" "$auth_path.bak.$ts"

tmp_config="$(mktemp "${{config_path}}.tmp.XXXXXX")"
tmp_auth="$(mktemp "${{auth_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_config" "$tmp_auth"; }}
trap cleanup EXIT

if [ -s "$config_path" ]; then
  awk -v provider_key="$provider_key" -v base_url="$base_url" '
    BEGIN {{ in_root=1; seen_pref=0; seen_model=0; skipping=0 }}
    function ltrim(s) {{ sub(/^[[:space:]]+/, "", s); return s }}
    function rtrim(s) {{ sub(/[[:space:]]+$/, "", s); return s }}
    function extract_header(s) {{
      if (match(s, /^\[[^\]]+\]/)) {{
        return substr(s, RSTART, RLENGTH)
      }}
      return s
    }}
    function is_target_section(h, pk) {{
      header = extract_header(h)
      base1 = "[model_providers." pk "]"
      base2 = "[model_providers.\"" pk "\"]"
      base3 = "[model_providers.'"'"'" pk "'"'"']"
      prefix1 = "[model_providers." pk "."
      prefix2 = "[model_providers.\"" pk "\"."
      prefix3 = "[model_providers.'"'"'" pk "'"'"'."
      return (header == base1 || header == base2 || header == base3 || index(header, prefix1) == 1 || index(header, prefix2) == 1 || index(header, prefix3) == 1)
    }}
    {{
      line=$0
      trimmed=rtrim(ltrim(line))

      # skipping check BEFORE comment check to delete comments inside skipped section
      if (skipping) {{
        if (substr(trimmed, 1, 1) == "[") {{
          if (is_target_section(trimmed, provider_key)) {{
            next
          }}
          skipping=0
        }} else {{
          next
        }}
      }}

      if (trimmed ~ /^#/) {{ print line; next }}

      if (in_root && substr(trimmed, 1, 1) == "[") {{
        inserted=0
        if (!seen_pref) {{ print "preferred_auth_method = \"apikey\""; seen_pref=1; inserted=1 }}
        if (!seen_model) {{ print "model_provider = \"" provider_key "\""; seen_model=1; inserted=1 }}
        if (inserted) print ""
        in_root=0
      }}

      if (is_target_section(trimmed, provider_key)) {{
        skipping=1
        next
      }}

      if (in_root && trimmed ~ /^preferred_auth_method[[:space:]]*=/) {{
        if (!seen_pref) {{ print "preferred_auth_method = \"apikey\""; seen_pref=1 }}
        next
      }}
      if (in_root && trimmed ~ /^model_provider[[:space:]]*=/) {{
        if (!seen_model) {{ print "model_provider = \"" provider_key "\""; seen_model=1 }}
        next
      }}

      print line
    }}
    END {{
      if (in_root) {{
        if (!seen_pref) print "preferred_auth_method = \"apikey\""
        if (!seen_model) print "model_provider = \"" provider_key "\""
      }}
      print ""
      print "[model_providers." provider_key "]"
      print "name = \"" provider_key "\""
      print "base_url = \"" base_url "\""
      print "wire_api = \"responses\""
      print "requires_openai_auth = true"
    }}
  ' "$config_path" > "$tmp_config"
else
  cat > "$tmp_config" <<EOF
preferred_auth_method = "apikey"
model_provider = "$provider_key"

[model_providers.$provider_key]
name = "$provider_key"
base_url = "$base_url"
wire_api = "responses"
requires_openai_auth = true
EOF
fi

if [ ! -s "$tmp_config" ]; then
  echo "Sanity check failed: generated config.toml is empty" >&2
  exit 2
fi
grep -qF 'preferred_auth_method = "apikey"' "$tmp_config" || {{ echo "Sanity check failed: missing preferred_auth_method" >&2; exit 2; }}
grep -qF "model_provider = \"$provider_key\"" "$tmp_config" || {{ echo "Sanity check failed: missing model_provider" >&2; exit 2; }}
grep -qF "base_url = \"$base_url\"" "$tmp_config" || {{ echo "Sanity check failed: missing provider base_url" >&2; exit 2; }}

count_section="$(awk -v pk="$provider_key" '
  BEGIN {{ c=0 }}
  function extract_header(s) {{
    if (match(s, /^\[[^\]]+\]/)) {{
      return substr(s, RSTART, RLENGTH)
    }}
    return s
  }}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    sub(/[[:space:]]+$/, "", line)
    if (line ~ /^#/) next
    if (substr(line, 1, 1) != "[") next
    header = extract_header(line)
    base1 = "[model_providers." pk "]"
    base2 = "[model_providers.\"" pk "\"]"
    base3 = "[model_providers.'"'"'" pk "'"'"']"
    if (header == base1 || header == base2 || header == base3) c++
  }}
  END {{ print c }}
' "$tmp_config")"
if [ "$count_section" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one [model_providers.$provider_key] section, got $count_section" >&2
  exit 2
fi

if command -v jq >/dev/null 2>&1; then
  if [ -s "$auth_path" ]; then
    if ! jq -e 'type=="object"' "$auth_path" >/dev/null; then
      echo "Refusing to modify: $auth_path must be a JSON object." >&2
      exit 2
    fi
    jq --arg api_key "$api_key" '.OPENAI_API_KEY = $api_key' "$auth_path" > "$tmp_auth"
  else
    jq -n --arg api_key "$api_key" '{{OPENAI_API_KEY:$api_key}}' > "$tmp_auth"
  fi
  jq -e --arg api_key "$api_key" '.OPENAI_API_KEY == $api_key' "$tmp_auth" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
  python3 - "$api_key" "$auth_path" "$tmp_auth" <<'PY'
import json
import sys
from pathlib import Path

api_key, src, dst = sys.argv[1], sys.argv[2], sys.argv[3]
data = {{}}
try:
    text = Path(src).read_text(encoding="utf-8")
    if text.strip():
        data = json.loads(text)
except FileNotFoundError:
    data = {{}}
except Exception as e:
    sys.stderr.write(f"Failed to parse existing auth.json: {{e}}\n")
    sys.exit(2)

if not isinstance(data, dict):
    sys.stderr.write("auth.json must be a JSON object\n")
    sys.exit(2)

data["OPENAI_API_KEY"] = api_key
Path(dst).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
else
  if [ -s "$auth_path" ]; then
    echo "Missing jq/python3; cannot safely merge existing $auth_path" >&2
    exit 2
  fi
  cat > "$tmp_auth" <<EOF
{{"OPENAI_API_KEY":"$api_key"}}
EOF
fi

if [ ! -s "$tmp_auth" ]; then
  echo "Sanity check failed: generated auth.json is empty" >&2
  exit 2
fi
grep -qF '"OPENAI_API_KEY"' "$tmp_auth" || {{ echo "Sanity check failed: missing OPENAI_API_KEY" >&2; exit 2; }}

if [ -f "$config_path" ]; then
  chmod --reference="$config_path" "$tmp_config" 2>/dev/null || true
fi
if [ -f "$auth_path" ]; then
  chmod --reference="$auth_path" "$tmp_auth" 2>/dev/null || true
fi

if mv -f "$tmp_config" "$config_path"; then
  if mv -f "$tmp_auth" "$auth_path"; then
    trap - EXIT
    exit 0
  fi

  echo "Failed to write $auth_path; attempting to rollback $config_path" >&2
  if [ -f "$config_path.bak.$ts" ]; then
    if cp -a "$config_path.bak.$ts" "$config_path"; then
      echo "Rollback successful: restored $config_path from backup" >&2
    else
      echo "CRITICAL: Rollback failed! Manual recovery needed: cp $config_path.bak.$ts $config_path" >&2
    fi
  else
    echo "WARNING: No backup found for $config_path, moving to $config_path.failed.$ts" >&2
    mv -f "$config_path" "$config_path.failed.$ts" 2>/dev/null || echo "CRITICAL: Failed to move config to .failed" >&2
  fi
  exit 1
fi

echo "Failed to write $config_path" >&2
exit 1
"#
    );

    run_wsl_bash_script(distro, &script)
}

fn configure_wsl_gemini(distro: &str, proxy_origin: &str) -> crate::shared::error::AppResult<()> {
    let base_url = format!("{proxy_origin}/gemini");
    let base_url = bash_single_quote(&base_url);
    let api_key = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

mkdir -p "$HOME/.gemini"
env_path="$HOME/.gemini/.env"

if [ -L "$env_path" ]; then
  echo "Refusing to modify: $env_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

gemini_base_url={base_url}
api_key={api_key}

ts="$(date +%s)"
[ -f "$env_path" ] && cp -a "$env_path" "$env_path.bak.$ts"

tmp_path="$(mktemp "${{env_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

if [ -f "$env_path" ]; then
  awk -v gemini_base_url="$gemini_base_url" -v api_key="$api_key" '
    BEGIN {{ seen_base=0; seen_key=0 }}
    function ltrim(s) {{ sub(/^[[:space:]]+/, "", s); return s }}
    {{
      line=$0
      trimmed=ltrim(line)
      if (trimmed ~ /^#/) {{ print line; next }}

      prefix=""
      rest=trimmed
      if (rest ~ /^export[[:space:]]+/) {{
        prefix="export "
        sub(/^export[[:space:]]+/, "", rest)
      }}

      if (rest ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) {{
        if (!seen_base) {{
          print prefix "GOOGLE_GEMINI_BASE_URL=" gemini_base_url
          seen_base=1
        }}
        next
      }}
      if (rest ~ /^GEMINI_API_KEY[[:space:]]*=/) {{
        if (!seen_key) {{
          print prefix "GEMINI_API_KEY=" api_key
          seen_key=1
        }}
        next
      }}

      print line
    }}
    END {{
      if (!seen_base) print "GOOGLE_GEMINI_BASE_URL=" gemini_base_url
      if (!seen_key) print "GEMINI_API_KEY=" api_key
    }}
  ' "$env_path" > "$tmp_path"
else
  cat > "$tmp_path" <<EOF
GOOGLE_GEMINI_BASE_URL=$gemini_base_url
GEMINI_API_KEY=$api_key
EOF
fi

if [ ! -s "$tmp_path" ]; then
  echo "Sanity check failed: generated .env is empty" >&2
  exit 2
fi

count_base="$(awk '
  BEGIN{{c=0}}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) c++
  }}
  END{{print c}}
' "$tmp_path")"
if [ "$count_base" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one GOOGLE_GEMINI_BASE_URL, got $count_base" >&2
  exit 2
fi

count_key="$(awk '
  BEGIN{{c=0}}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GEMINI_API_KEY[[:space:]]*=/) c++
  }}
  END{{print c}}
' "$tmp_path")"
if [ "$count_key" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one GEMINI_API_KEY, got $count_key" >&2
  exit 2
fi

actual_base="$(awk '
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) {{
      sub(/^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/, "", line)
      sub(/[[:space:]]+$/, "", line)
      print line
      exit
    }}
  }}
' "$tmp_path")"
if [ "$actual_base" != "$gemini_base_url" ]; then
  echo "Sanity check failed: GOOGLE_GEMINI_BASE_URL mismatch" >&2
  exit 2
fi

actual_key="$(awk '
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GEMINI_API_KEY[[:space:]]*=/) {{
      sub(/^GEMINI_API_KEY[[:space:]]*=/, "", line)
      sub(/[[:space:]]+$/, "", line)
      print line
      exit
    }}
  }}
' "$tmp_path")"
if [ "$actual_key" != "$api_key" ]; then
  echo "Sanity check failed: GEMINI_API_KEY mismatch" >&2
  exit 2
fi

if [ -f "$env_path" ]; then
  chmod --reference="$env_path" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$env_path"
trap - EXIT
"#
    );

    run_wsl_bash_script(distro, &script)
}

pub fn get_config_status(distros: &[String]) -> Vec<WslDistroConfigStatus> {
    if !cfg!(windows) {
        return Vec::new();
    }

    const STATUS_SCRIPT: &str = r#"
# Normalize HOME: Windows environment may inject HOME=C:\Users\...
home_from_getent="$(getent passwd "$(whoami)" | cut -d: -f6 2>/dev/null || true)"
if [ -n "$home_from_getent" ]; then
  HOME="$home_from_getent"
fi
export HOME

claude=0
codex=0
gemini=0
claude_mcp=0
codex_mcp=0
gemini_mcp=0
claude_prompt=0
codex_prompt=0
gemini_prompt=0

[ -f "$HOME/.claude/settings.json" ] && claude=1

CODEX_HOME_RAW="${CODEX_HOME:-$HOME/.codex}"
p="$CODEX_HOME_RAW"
if [ "$CODEX_HOME_RAW" = "~" ]; then
  p="$HOME"
elif [ "${CODEX_HOME_RAW#~/}" != "$CODEX_HOME_RAW" ]; then
  p="$HOME/${CODEX_HOME_RAW#~/}"
elif [ "${CODEX_HOME_RAW#~\\}" != "$CODEX_HOME_RAW" ]; then
  p="$HOME/${CODEX_HOME_RAW#~\\}"
elif [ "${CODEX_HOME_RAW#/}" = "$CODEX_HOME_RAW" ]; then
  p="$HOME/$CODEX_HOME_RAW"
fi

[ -f "$p/config.toml" ] && codex=1
[ -f "$HOME/.gemini/.env" ] && gemini=1

# Check MCP: claude uses .claude.json mcpServers, codex uses config.toml mcp_servers, gemini uses settings.json mcpServers
if [ -f "$HOME/.claude.json" ] && command -v grep >/dev/null 2>&1; then
  grep -q '"mcpServers"' "$HOME/.claude.json" 2>/dev/null && claude_mcp=1
fi
if [ -f "$p/config.toml" ] && command -v grep >/dev/null 2>&1; then
  grep -q '\[mcp_servers\.' "$p/config.toml" 2>/dev/null && codex_mcp=1
fi
if [ -f "$HOME/.gemini/settings.json" ] && command -v grep >/dev/null 2>&1; then
  grep -q '"mcpServers"' "$HOME/.gemini/settings.json" 2>/dev/null && gemini_mcp=1
fi

# Check Prompt files
[ -f "$HOME/.claude/CLAUDE.md" ] && claude_prompt=1
[ -f "$p/AGENTS.md" ] && codex_prompt=1
[ -f "$HOME/.gemini/GEMINI.md" ] && gemini_prompt=1

printf 'AIO_WSL_STATUS=%s%s%s%s%s%s%s%s%s\n' "$claude" "$codex" "$gemini" "$claude_mcp" "$codex_mcp" "$gemini_mcp" "$claude_prompt" "$codex_prompt" "$gemini_prompt"
"#;

    #[derive(Default)]
    struct StatusBits {
        claude: bool,
        codex: bool,
        gemini: bool,
        claude_mcp: bool,
        codex_mcp: bool,
        gemini_mcp: bool,
        claude_prompt: bool,
        codex_prompt: bool,
        gemini_prompt: bool,
    }

    fn parse_status_bits(text: &str) -> Option<StatusBits> {
        let slice = match text.split_once("AIO_WSL_STATUS=") {
            Some((_, tail)) => tail,
            None => text,
        };
        let mut bits = slice.chars().filter(|c| *c == '0' || *c == '1');
        Some(StatusBits {
            claude: bits.next()? == '1',
            codex: bits.next()? == '1',
            gemini: bits.next()? == '1',
            claude_mcp: bits.next()? == '1',
            codex_mcp: bits.next()? == '1',
            gemini_mcp: bits.next()? == '1',
            claude_prompt: bits.next()? == '1',
            codex_prompt: bits.next()? == '1',
            gemini_prompt: bits.next()? == '1',
        })
    }

    let mut out = Vec::new();
    for distro in distros {
        let bits: StatusBits = (|| -> Option<StatusBits> {
            let mut cmd = hide_window_cmd("wsl");
            cmd.args(["-d", distro, "bash"]);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!(distro = distro, error = %err, "WSL config status spawn failed");
                    return None;
                }
            };

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(STATUS_SCRIPT.as_bytes());
            }

            let output = match child.wait_with_output() {
                Ok(o) => o,
                Err(err) => {
                    tracing::warn!(distro = distro, error = %err, "WSL config status wait failed");
                    return None;
                }
            };

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = {
                    let utf8 = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    if utf8.contains('\0') {
                        let decoded = decode_utf16_le(&output.stderr);
                        let trimmed = decoded.trim().to_string();
                        if trimmed.is_empty() {
                            utf8
                        } else {
                            trimmed
                        }
                    } else {
                        utf8
                    }
                };
                tracing::warn!(
                    distro = distro,
                    code = ?output.status.code(),
                    stdout = stdout,
                    stderr = stderr,
                    "WSL config status script failed"
                );
                return None;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            match parse_status_bits(&stdout) {
                Some(v) => Some(v),
                None => {
                    tracing::warn!(
                        distro = distro,
                        stdout = stdout.trim().to_string(),
                        "WSL config status output parse failed"
                    );
                    None
                }
            }
        })()
        .unwrap_or_default();

        out.push(WslDistroConfigStatus {
            distro: distro.clone(),
            claude: bits.claude,
            codex: bits.codex,
            gemini: bits.gemini,
            claude_mcp: bits.claude_mcp,
            codex_mcp: bits.codex_mcp,
            gemini_mcp: bits.gemini_mcp,
            claude_prompt: bits.claude_prompt,
            codex_prompt: bits.codex_prompt,
            gemini_prompt: bits.gemini_prompt,
        });
    }

    out
}

fn wsl_target_enabled(targets: &settings::WslTargetCli, cli_key: &str) -> bool {
    match cli_key {
        "claude" => targets.claude,
        "codex" => targets.codex,
        "gemini" => targets.gemini,
        _ => false,
    }
}

pub fn configure_clients(
    app: &tauri::AppHandle,
    distros: &[String],
    targets: &settings::WslTargetCli,
    proxy_origin: &str,
    mcp_data: Option<&WslMcpSyncData>,
    prompt_data: Option<&WslPromptSyncData>,
) -> WslConfigureReport {
    if !cfg!(windows) {
        return WslConfigureReport {
            ok: false,
            message: "WSL configuration is only available on Windows".to_string(),
            distros: Vec::new(),
        };
    }

    let mut distro_reports = Vec::new();
    let mut success_ops = 0usize;
    let mut error_ops = 0usize;

    for distro in distros {
        let mut results = Vec::new();
        let mut cli_backups = Vec::new();

        // Load existing manifest so we don't overwrite original_values on repeated calls
        let existing_manifest = read_wsl_manifest(app, distro).unwrap_or(None);
        let existing_backups: std::collections::HashMap<&str, &WslCliBackup> = existing_manifest
            .as_ref()
            .map(|m| {
                m.cli_backups
                    .iter()
                    .map(|b| (b.cli_key.as_str(), b))
                    .collect()
            })
            .unwrap_or_default();

        // ── Auth configuration (with original-value capture) ──
        for (cli_key, enabled, configure_fn) in [
            (
                "claude",
                targets.claude,
                configure_wsl_claude as fn(&str, &str) -> AppResult<()>,
            ),
            (
                "codex",
                targets.codex,
                configure_wsl_codex as fn(&str, &str) -> AppResult<()>,
            ),
            (
                "gemini",
                targets.gemini,
                configure_wsl_gemini as fn(&str, &str) -> AppResult<()>,
            ),
        ] {
            if !enabled {
                continue;
            }
            // If we already have a backup for this CLI (from a prior call), preserve
            // the original_values; otherwise capture fresh ones now.
            let original_values = if let Some(prev) = existing_backups.get(cli_key) {
                prev.original_values.clone()
            } else {
                read_wsl_current_values(distro, cli_key).unwrap_or_default()
            };

            match configure_fn(distro, proxy_origin) {
                Ok(()) => {
                    // Record what we injected
                    let injected_keys = match cli_key {
                        "claude" => {
                            let mut m = std::collections::HashMap::new();
                            m.insert(
                                "ANTHROPIC_BASE_URL".to_string(),
                                format!("{proxy_origin}/claude"),
                            );
                            m.insert(
                                "ANTHROPIC_AUTH_TOKEN".to_string(),
                                "aio-coding-hub".to_string(),
                            );
                            m
                        }
                        "codex" => {
                            let mut m = std::collections::HashMap::new();
                            m.insert("preferred_auth_method".to_string(), "api-key".to_string());
                            m.insert("model_provider".to_string(), "openai".to_string());
                            m.insert("OPENAI_API_KEY".to_string(), "aio-coding-hub".to_string());
                            m
                        }
                        "gemini" => {
                            let mut m = std::collections::HashMap::new();
                            m.insert(
                                "GOOGLE_GEMINI_BASE_URL".to_string(),
                                format!("{proxy_origin}/gemini"),
                            );
                            m.insert("GEMINI_API_KEY".to_string(), "aio-coding-hub".to_string());
                            m
                        }
                        _ => std::collections::HashMap::new(),
                    };
                    cli_backups.push(WslCliBackup {
                        cli_key: cli_key.to_string(),
                        injected_keys,
                        original_values,
                    });
                    results.push(WslConfigureCliReport {
                        cli_key: cli_key.to_string(),
                        ok: true,
                        message: "ok".to_string(),
                    });
                }
                Err(err) => {
                    results.push(WslConfigureCliReport {
                        cli_key: cli_key.to_string(),
                        ok: false,
                        message: err.to_string(),
                    });
                }
            }
        }

        // ── MCP sync ──
        if let Some(mcp) = mcp_data {
            for (cli_key, servers) in [
                ("claude", &mcp.claude),
                ("codex", &mcp.codex),
                ("gemini", &mcp.gemini),
            ] {
                if !wsl_target_enabled(targets, cli_key) {
                    continue;
                }
                let managed_keys = read_wsl_mcp_manifest(app, distro, cli_key);
                if servers.is_empty() && managed_keys.is_empty() {
                    continue;
                }
                match sync_wsl_mcp_for_cli(distro, cli_key, servers, &managed_keys) {
                    Ok(new_keys) => {
                        if let Err(e) = write_wsl_mcp_manifest(app, distro, cli_key, &new_keys) {
                            tracing::warn!(
                                distro = distro,
                                cli_key = cli_key,
                                "failed to write WSL MCP manifest: {e}"
                            );
                        }
                        results.push(WslConfigureCliReport {
                            cli_key: format!("{cli_key}_mcp"),
                            ok: true,
                            message: format!("ok ({} servers)", new_keys.len()),
                        });
                    }
                    Err(err) => {
                        results.push(WslConfigureCliReport {
                            cli_key: format!("{cli_key}_mcp"),
                            ok: false,
                            message: err.to_string(),
                        });
                    }
                }
            }
        }

        // ── Prompt sync ──
        if let Some(prompts) = prompt_data {
            for (cli_key, content) in [
                ("claude", prompts.claude_content.as_deref()),
                ("codex", prompts.codex_content.as_deref()),
                ("gemini", prompts.gemini_content.as_deref()),
            ] {
                if !wsl_target_enabled(targets, cli_key) {
                    continue;
                }
                match sync_wsl_prompt_for_cli(app, distro, cli_key, content) {
                    Ok(()) => {
                        results.push(WslConfigureCliReport {
                            cli_key: format!("{cli_key}_prompt"),
                            ok: true,
                            message: "ok".to_string(),
                        });
                    }
                    Err(err) => {
                        results.push(WslConfigureCliReport {
                            cli_key: format!("{cli_key}_prompt"),
                            ok: false,
                            message: err.to_string(),
                        });
                    }
                }
            }
        }

        // ── Write manifest for this distro ──
        if !cli_backups.is_empty() {
            let wsl_home_unc = resolve_wsl_home_unc(distro)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let manifest = WslDistroManifest {
                schema_version: 1,
                distro: distro.clone(),
                configured: true,
                proxy_origin: proxy_origin.to_string(),
                configured_at: crate::shared::time::now_unix_seconds(),
                wsl_home_unc,
                cli_backups,
            };
            if let Err(e) = write_wsl_manifest(app, distro, &manifest) {
                tracing::warn!("failed to write WSL manifest for {distro}: {e}");
            }
        }

        let distro_ok = results.iter().all(|r| r.ok);
        success_ops += results.iter().filter(|r| r.ok).count();
        error_ops += results.iter().filter(|r| !r.ok).count();

        distro_reports.push(WslConfigureDistroReport {
            distro: distro.clone(),
            ok: distro_ok,
            results,
        });
    }

    let message = if error_ops > 0 {
        format!(
            "已配置：{success_ops} 项；失败：{error_ops} 项（可展开查看每个 distro 的详细结果）"
        )
    } else {
        format!("配置成功：{success_ops} 项")
    };

    WslConfigureReport {
        ok: success_ops > 0,
        message,
        distros: distro_reports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_server(
        command: Option<&str>,
        args: Vec<&str>,
        cwd: Option<&str>,
        env: Vec<(&str, &str)>,
    ) -> McpServerForSync {
        McpServerForSync {
            server_key: "test".to_string(),
            transport: "stdio".to_string(),
            command: command.map(|s| s.to_string()),
            args: args.into_iter().map(|s| s.to_string()).collect(),
            env: env
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            cwd: cwd.map(|s| s.to_string()),
            url: None,
            headers: BTreeMap::new(),
        }
    }

    #[test]
    fn test_win_path_to_wsl_mount_drive_letter() {
        assert_eq!(
            win_path_to_wsl_mount(r"C:\Users\foo\bar"),
            "/mnt/c/Users/foo/bar"
        );
        assert_eq!(win_path_to_wsl_mount(r"D:\tools\cli"), "/mnt/d/tools/cli");
    }

    #[test]
    fn test_win_path_to_wsl_mount_non_absolute() {
        assert_eq!(win_path_to_wsl_mount(r".\relative\path"), "./relative/path");
    }

    #[test]
    fn test_strip_win_exe_ext() {
        assert_eq!(strip_win_exe_ext("npx.cmd"), "npx");
        assert_eq!(strip_win_exe_ext("server.exe"), "server");
        assert_eq!(strip_win_exe_ext("run.bat"), "run");
        assert_eq!(strip_win_exe_ext("npx"), "npx");
    }

    #[test]
    fn test_adapt_converts_args_with_windows_paths() {
        let servers = vec![make_server(
            Some("npx.cmd"),
            vec!["-y", "@mcp/server-fs", r"C:\Users\diao\Documents"],
            Some(r"C:\Users\diao\project"),
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(adapted[0].command.as_deref(), Some("npx"));
        assert_eq!(adapted[0].args[0], "-y");
        assert_eq!(adapted[0].args[1], "@mcp/server-fs");
        assert_eq!(adapted[0].args[2], "/mnt/c/Users/diao/Documents");
        assert_eq!(adapted[0].cwd.as_deref(), Some("/mnt/c/Users/diao/project"));
    }

    #[test]
    fn test_adapt_converts_env_with_windows_paths() {
        let servers = vec![make_server(
            Some("node"),
            vec![],
            None,
            vec![
                ("NODE_PATH", r"C:\Users\diao\node_modules"),
                ("API_KEY", "sk-abc123"),
            ],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(
            adapted[0].env.get("NODE_PATH").unwrap(),
            "/mnt/c/Users/diao/node_modules"
        );
        // Non-path values should remain unchanged
        assert_eq!(adapted[0].env.get("API_KEY").unwrap(), "sk-abc123");
    }

    #[test]
    fn test_adapt_leaves_non_windows_args_unchanged() {
        let servers = vec![make_server(
            Some("node"),
            vec!["--port", "3000", "/home/user/script.js"],
            None,
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(
            adapted[0].args,
            vec!["--port", "3000", "/home/user/script.js"]
        );
    }

    #[test]
    fn test_adapt_command_windows_absolute_path_uses_basename() {
        let servers = vec![make_server(
            Some(r"C:\Program Files\tool\server.exe"),
            vec![],
            None,
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(adapted[0].command.as_deref(), Some("server"));
    }

    #[test]
    fn test_wsl_manifest_roundtrip() {
        let manifest = WslDistroManifest {
            schema_version: 1,
            distro: "Ubuntu".to_string(),
            configured: true,
            proxy_origin: "http://172.20.0.1:12345".to_string(),
            configured_at: 1700000000,
            wsl_home_unc: Some(r"\\wsl$\Ubuntu\home\testuser".to_string()),
            cli_backups: vec![
                WslCliBackup {
                    cli_key: "claude".to_string(),
                    injected_keys: [
                        (
                            "ANTHROPIC_BASE_URL".to_string(),
                            "http://172.20.0.1:12345/claude".to_string(),
                        ),
                        (
                            "ANTHROPIC_AUTH_TOKEN".to_string(),
                            "aio-coding-hub".to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    original_values: [
                        (
                            "ANTHROPIC_BASE_URL".to_string(),
                            Some("https://api.anthropic.com".to_string()),
                        ),
                        (
                            "ANTHROPIC_AUTH_TOKEN".to_string(),
                            Some("sk-old-token".to_string()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                WslCliBackup {
                    cli_key: "gemini".to_string(),
                    injected_keys: [("GEMINI_API_KEY".to_string(), "aio-coding-hub".to_string())]
                        .into_iter()
                        .collect(),
                    original_values: [("GEMINI_API_KEY".to_string(), Some("old-key".to_string()))]
                        .into_iter()
                        .collect(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize");
        let deserialized: WslDistroManifest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.schema_version, 1);
        assert_eq!(deserialized.distro, "Ubuntu");
        assert_eq!(deserialized.proxy_origin, "http://172.20.0.1:12345");
        assert_eq!(deserialized.cli_backups.len(), 2);
        assert_eq!(deserialized.cli_backups[0].cli_key, "claude");
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_BASE_URL"),
            Some(&Some("https://api.anthropic.com".to_string()))
        );
    }

    #[test]
    fn test_wsl_manifest_with_null_originals() {
        let manifest = WslDistroManifest {
            schema_version: 1,
            distro: "Debian".to_string(),
            configured: true,
            proxy_origin: "http://172.20.0.1:9999".to_string(),
            configured_at: 1700000000,
            wsl_home_unc: None,
            cli_backups: vec![WslCliBackup {
                cli_key: "claude".to_string(),
                injected_keys: [
                    (
                        "ANTHROPIC_BASE_URL".to_string(),
                        "http://172.20.0.1:9999/claude".to_string(),
                    ),
                    (
                        "ANTHROPIC_AUTH_TOKEN".to_string(),
                        "aio-coding-hub".to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
                original_values: [
                    ("ANTHROPIC_BASE_URL".to_string(), None),
                    ("ANTHROPIC_AUTH_TOKEN".to_string(), None),
                ]
                .into_iter()
                .collect(),
            }],
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize");
        // Verify null is present in JSON
        assert!(
            json.contains("null"),
            "JSON should contain null for missing original values"
        );

        let deserialized: WslDistroManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_BASE_URL"),
            Some(&None)
        );
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_AUTH_TOKEN"),
            Some(&None)
        );
    }

    #[test]
    fn test_extract_toml_value() {
        let content = r#"
preferred_auth_method = "api-key"
model_provider = "openai"
model = "o3"
"#;
        assert_eq!(
            extract_toml_value(content, "preferred_auth_method"),
            Some("api-key".to_string())
        );
        assert_eq!(
            extract_toml_value(content, "model_provider"),
            Some("openai".to_string())
        );
        assert_eq!(extract_toml_value(content, "nonexistent"), None);
    }

    #[test]
    fn test_extract_env_value() {
        let content = r#"
# comment line
GOOGLE_GEMINI_BASE_URL=http://localhost:1234/gemini
GEMINI_API_KEY=aio-coding-hub
OTHER_VAR=keep
"#;
        assert_eq!(
            extract_env_value(content, "GOOGLE_GEMINI_BASE_URL"),
            Some("http://localhost:1234/gemini".to_string())
        );
        assert_eq!(
            extract_env_value(content, "GEMINI_API_KEY"),
            Some("aio-coding-hub".to_string())
        );
        assert_eq!(extract_env_value(content, "MISSING"), None);
    }

    #[test]
    fn test_extract_env_value_with_export() {
        let content = "export GEMINI_API_KEY=my-key\n";
        assert_eq!(
            extract_env_value(content, "GEMINI_API_KEY"),
            Some("my-key".to_string())
        );
    }
}
