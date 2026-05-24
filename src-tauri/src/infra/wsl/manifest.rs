//! WSL manifest (config lifecycle): backup, restore, and startup repair.

use crate::shared::error::AppResult;
use crate::shared::fs::{read_file_with_max_len, read_optional_file_with_max_len};

use super::detection::{resolve_wsl_codex_home_host_path, resolve_wsl_home_unc};
use super::shell::write_file_synced;
use super::types::{WslCliBackup, WslDistroManifest};

pub(super) const WSL_MANIFEST_MAX_BYTES: usize = 256 * 1024;
const WSL_MANIFEST_FILE_COUNT_MAX: usize = 256;
pub(super) const WSL_CLIENT_CONFIG_MAX_BYTES: usize = 1024 * 1024;

pub(super) fn wsl_manifests_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<std::path::PathBuf> {
    Ok(crate::infra::app_paths::app_data_dir(app)?.join("wsl-manifests"))
}

pub(super) fn wsl_manifest_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
) -> AppResult<std::path::PathBuf> {
    Ok(wsl_manifests_dir(app)?.join(format!("{distro}.json")))
}

pub(super) fn read_wsl_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
) -> AppResult<Option<WslDistroManifest>> {
    let path = wsl_manifest_path(app, distro)?;
    let Some(content) = read_optional_file_with_max_len(&path, WSL_MANIFEST_MAX_BYTES)? else {
        return Ok(None);
    };
    let manifest: WslDistroManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse WSL manifest for {distro}: {e}"))?;
    Ok(Some(manifest))
}

pub(super) fn write_wsl_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
    manifest: &WslDistroManifest,
) -> AppResult<()> {
    let path = wsl_manifest_path(app, distro)?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize WSL manifest: {e}"))?;
    crate::shared::fs::write_file_atomic(&path, json.as_bytes())
}

pub(super) fn delete_wsl_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
) -> AppResult<()> {
    let path = wsl_manifest_path(app, distro)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("failed to delete WSL manifest for {distro}: {e}"))?;
    }
    Ok(())
}

pub(super) fn read_all_wsl_manifests<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<Vec<WslDistroManifest>> {
    let dir = wsl_manifests_dir(app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut manifests = Vec::new();
    let mut json_files_seen = 0_usize;
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read wsl-manifests dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        json_files_seen += 1;
        if json_files_seen > WSL_MANIFEST_FILE_COUNT_MAX {
            tracing::warn!(
                "too many WSL manifest files in {} (max {})",
                dir.display(),
                WSL_MANIFEST_FILE_COUNT_MAX
            );
            break;
        }
        let bytes = match read_file_with_max_len(&path, WSL_MANIFEST_MAX_BYTES) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("failed to read WSL manifest {}: {e}", path.display());
                continue;
            }
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

pub(super) fn read_wsl_current_values(
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
            let codex_home =
                resolve_wsl_codex_home_host_path(distro).unwrap_or_else(|_| home.join(".codex"));
            // config.toml
            let toml_path = codex_home.join("config.toml");
            let toml_content = read_optional_utf8_file(&toml_path)
                .ok()
                .flatten()
                .unwrap_or_default();
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
            let env_content = read_optional_utf8_file(&env_path)
                .ok()
                .flatten()
                .unwrap_or_default();
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
    let Some(bytes) = read_optional_file_with_max_len(path, WSL_CLIENT_CONFIG_MAX_BYTES)
        .ok()
        .flatten()
    else {
        return std::collections::HashMap::new();
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
pub(super) fn read_json_top_level_str(path: &std::path::Path, key: &str) -> Option<String> {
    let bytes = read_optional_file_with_max_len(path, WSL_CLIENT_CONFIG_MAX_BYTES)
        .ok()
        .flatten()?;
    let val: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    val.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn read_optional_utf8_file(path: &std::path::Path) -> AppResult<Option<String>> {
    let Some(bytes) = read_optional_file_with_max_len(path, WSL_CLIENT_CONFIG_MAX_BYTES)? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes).map_err(|e| {
        format!(
            "SEC_INVALID_INPUT: invalid UTF-8 in {}: {e}",
            path.display()
        )
    })?;
    Ok(Some(text))
}

fn read_existing_utf8_file(path: &std::path::Path) -> AppResult<String> {
    let bytes = read_file_with_max_len(path, WSL_CLIENT_CONFIG_MAX_BYTES)?;
    String::from_utf8(bytes).map_err(|e| {
        format!(
            "SEC_INVALID_INPUT: invalid UTF-8 in {}: {e}",
            path.display()
        )
        .into()
    })
}

/// Extract a value from TOML like `key = "value"`.
///
/// NOTE: This is a simple line-based parser that assumes Codex `config.toml`
/// consists of flat top-level `key = "value"` entries only (no sections, no
/// inline tables, no multi-line values). If the format grows more complex,
/// replace with the `toml` crate.
pub(super) fn extract_toml_value(content: &str, key: &str) -> Option<String> {
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
pub(super) fn extract_env_value(content: &str, key: &str) -> Option<String> {
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

// ── Codex TOML restore helpers ──

pub(super) fn restore_wsl_toml_string_key(
    table: &mut toml::map::Map<String, toml::Value>,
    backup: &WslCliBackup,
    key: &str,
) {
    match backup.original_values.get(key) {
        Some(Some(value)) => {
            table.insert(key.to_string(), toml::Value::String(value.clone()));
        }
        Some(None) | None => {
            table.remove(key);
        }
    }
}

pub(super) fn restore_codex_config_toml(content: &str, backup: &WslCliBackup) -> AppResult<String> {
    let mut parsed = if content.trim().is_empty() {
        toml::Value::Table(Default::default())
    } else {
        toml::from_str::<toml::Value>(content)
            .map_err(|e| format!("failed to parse Codex config.toml: {e}"))?
    };

    let table = parsed.as_table_mut().ok_or_else(|| {
        "failed to restore Codex config.toml: root must be a TOML table".to_string()
    })?;

    for key in ["preferred_auth_method", "model_provider"] {
        restore_wsl_toml_string_key(table, backup, key);
    }

    let remove_model_providers = if let Some(model_providers) = table
        .get_mut("model_providers")
        .and_then(toml::Value::as_table_mut)
    {
        model_providers.remove(super::constants::WSL_CODEX_PROVIDER_KEY);
        model_providers.is_empty()
    } else {
        false
    };

    if remove_model_providers {
        table.remove("model_providers");
    }

    let mut out = toml::to_string_pretty(&parsed)
        .map_err(|e| format!("failed to serialize Codex config.toml: {e}"))?;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

// ── Restore WSL clients (pure Rust via UNC paths) ──

/// Restore a single CLI's config for a distro using the saved backup.
pub(super) fn restore_wsl_cli_backup(
    distro: &str,
    home: &std::path::Path,
    backup: &WslCliBackup,
) -> AppResult<()> {
    match backup.cli_key.as_str() {
        "claude" => {
            let path = home.join(".claude").join("settings.json");
            if !path.exists() {
                return Ok(());
            }
            let bytes = read_file_with_max_len(&path, WSL_CLIENT_CONFIG_MAX_BYTES)?;
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
            let codex_home =
                resolve_wsl_codex_home_host_path(distro).unwrap_or_else(|_| home.join(".codex"));

            // config.toml
            let toml_path = codex_home.join("config.toml");
            if toml_path.exists() {
                let content = read_existing_utf8_file(&toml_path)?;
                let restored = restore_codex_config_toml(&content, backup)?;
                write_file_synced(&toml_path, restored.as_bytes())?;
            }

            // auth.json
            let auth_path = codex_home.join("auth.json");
            if auth_path.exists() {
                let bytes = read_file_with_max_len(&auth_path, WSL_CLIENT_CONFIG_MAX_BYTES)?;
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
            let content = read_existing_utf8_file(&env_path)?;
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
pub fn restore_wsl_clients<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<()> {
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
                    continue; // Don't delete manifest -- retry next startup
                }
            },
        };

        let mut all_ok = true;
        for backup in &manifest.cli_backups {
            if let Err(e) = restore_wsl_cli_backup(distro, &home, backup) {
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
            tracing::warn!("WSL manifest for {distro} kept -- some restores failed, will retry");
        }
    }
    Ok(())
}

// ── Startup repair ──

/// Check for stale manifests at startup and restore if the gateway is dead.
pub fn startup_repair_wsl_manifests<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<()> {
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
            if let Err(e) = restore_wsl_cli_backup(&manifest.distro, &home, backup) {
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
