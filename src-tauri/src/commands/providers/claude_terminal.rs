use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::shared::mutex_ext::MutexExt;
use crate::{base_url_probe, blocking, providers};
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::Manager;
use tauri_plugin_clipboard_manager::ClipboardExt;

const ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";
const ENV_DISABLE_ERROR_REPORTING: &str = "DISABLE_ERROR_REPORTING";
const ENV_DISABLE_TELEMETRY: &str = "DISABLE_TELEMETRY";
const ENV_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
const ENV_ANTHROPIC_BASE_URL: &str = "ANTHROPIC_BASE_URL";
const ENV_ANTHROPIC_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
const CLAUDE_LAUNCHER_DIR_NAME: &str = "claude-launchers";
const CLAUDE_LAUNCHER_ARTIFACT_TTL_SECS: u64 = 60 * 60;

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_claude_terminal_launch_command(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<String, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let gateway_base_origin = blocking::run("provider_claude_terminal_launch_gateway_origin", {
        let app = app.clone();
        let db = db.clone();
        move || ensure_gateway_base_origin(&app, &db)
    })
    .await?;

    blocking::run("provider_claude_terminal_launch_command", move || {
        let launch = providers::claude_terminal_launch_context(&db, provider_id)?;
        let claude_base_url = build_claude_gateway_base_url(&gateway_base_origin, provider_id);
        create_claude_terminal_launch_command(
            &app,
            provider_id,
            &claude_base_url,
            &launch.api_key_plaintext,
        )
    })
    .await
    .map_err(Into::into)
}

fn ensure_gateway_base_origin(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
) -> crate::shared::error::AppResult<String> {
    let state = app.state::<GatewayState>();
    let mut manager = state.0.lock_or_recover();

    let mut status = manager.status();
    if !status.running {
        status = manager.start(app, db.clone(), None)?;
    }

    drop(manager);

    crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_STATUS_EVENT_NAME, status.clone());

    status
        .base_url
        .ok_or_else(|| "SYSTEM_ERROR: gateway base_url missing".to_string().into())
}

fn build_claude_gateway_base_url(gateway_base_origin: &str, provider_id: i64) -> String {
    format!(
        "{}/claude/_aio/provider/{provider_id}",
        gateway_base_origin.trim_end_matches('/')
    )
}

fn is_claude_launcher_artifact_file_name(name: &str) -> bool {
    name.starts_with("claude_") || name.starts_with("aio_claude_launcher_")
}

fn claude_launch_artifact_paths(
    dir: &Path,
    provider_id: i64,
    pid: u32,
    now: i64,
) -> (PathBuf, PathBuf) {
    let config_path = dir.join(format!("claude_{provider_id}_{pid}_{now}.json"));
    let script_path = if cfg!(target_os = "windows") {
        dir.join(format!("aio_claude_launcher_{provider_id}_{pid}_{now}.ps1"))
    } else {
        dir.join(format!("aio_claude_launcher_{provider_id}_{pid}_{now}.sh"))
    };
    (config_path, script_path)
}

fn claude_launcher_artifacts_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    let dir = crate::infra::app_paths::app_data_dir(app)?.join(CLAUDE_LAUNCHER_DIR_NAME);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("SYSTEM_ERROR: create claude launcher dir failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(dir)
}

fn prune_stale_claude_launch_artifacts(dir: &Path, now: std::time::SystemTime) {
    let ttl = std::time::Duration::from_secs(CLAUDE_LAUNCHER_ARTIFACT_TTL_SECS);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_claude_launcher_artifact_file_name(name) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified_at) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified_at) else {
            continue;
        };
        if age > ttl {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn write_claude_launcher_file(
    path: &Path,
    content: impl AsRef<[u8]>,
    _executable: bool,
) -> crate::shared::error::AppResult<()> {
    std::fs::write(path, content)
        .map_err(|e| format!("SYSTEM_ERROR: write launcher asset failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if _executable { 0o700 } else { 0o600 };
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    Ok(())
}

fn create_claude_terminal_launch_command<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider_id: i64,
    base_url: &str,
    api_key_plaintext: &str,
) -> crate::shared::error::AppResult<String> {
    let now = crate::shared::time::now_unix_seconds();
    let pid = std::process::id();
    let artifact_dir = claude_launcher_artifacts_dir(app)?;
    prune_stale_claude_launch_artifacts(&artifact_dir, std::time::SystemTime::now());
    let (config_path, script_path) =
        claude_launch_artifact_paths(&artifact_dir, provider_id, pid, now);

    let settings_json = build_claude_settings_json(base_url, api_key_plaintext)?;
    write_claude_launcher_file(&config_path, settings_json, false)
        .map_err(|e| format!("SYSTEM_ERROR: write claude settings failed: {e}"))?;

    let (script_content, launch_command) = build_claude_launch_assets(&script_path, &config_path);
    if let Err(err) = write_claude_launcher_file(&script_path, script_content, true) {
        let _ = std::fs::remove_file(&config_path);
        return Err(format!("SYSTEM_ERROR: write launch script failed: {err}").into());
    }

    Ok(launch_command)
}

fn build_claude_launch_assets(script_path: &Path, config_path: &Path) -> (String, String) {
    if cfg!(target_os = "windows") {
        let script_content = build_claude_launcher_powershell_script(config_path, script_path);
        let launch_command = build_powershell_launch_command(script_path);
        (script_content, launch_command)
    } else {
        let script_content = build_claude_launcher_bash_script(config_path, script_path);
        let launch_command = build_bash_launch_command(script_path);
        (script_content, launch_command)
    }
}

fn build_claude_settings_json(
    base_url: &str,
    api_key_plaintext: &str,
) -> crate::shared::error::AppResult<String> {
    let value = json!({
        "env": {
            ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
            ENV_DISABLE_ERROR_REPORTING: "1",
            ENV_DISABLE_TELEMETRY: "1",
            ENV_MCP_TIMEOUT: "60000",
            ENV_ANTHROPIC_BASE_URL: base_url,
            ENV_ANTHROPIC_AUTH_TOKEN: api_key_plaintext,
        }
    });

    serde_json::to_string_pretty(&value)
        .map_err(|e| format!("SYSTEM_ERROR: serialize claude settings failed: {e}").into())
}

fn build_claude_launcher_bash_script(config_path: &Path, script_path: &Path) -> String {
    let config_var = bash_single_quote(&config_path.to_string_lossy());
    let script_var = bash_single_quote(&script_path.to_string_lossy());

    format!(
        "#!/bin/bash\n\
config_path={config_var}\n\
script_path={script_var}\n\
cleanup() {{\n\
  rm -f \"$config_path\" \"$script_path\"\n\
}}\n\
trap cleanup EXIT INT TERM HUP\n\
echo \"Using provider-specific claude config:\"\n\
echo \"$config_path\"\n\
claude --settings \"$config_path\"\n\
cleanup\n\
trap - EXIT INT TERM HUP\n\
exec bash --norc --noprofile\n"
    )
}

fn build_claude_launcher_powershell_script(config_path: &Path, script_path: &Path) -> String {
    let config_var = powershell_single_quote(&config_path.to_string_lossy());
    let script_var = powershell_single_quote(&script_path.to_string_lossy());

    format!(
        "$configPath = {config_var}\n\
$scriptPath = {script_var}\n\
try {{\n\
  Write-Output \"Using provider-specific claude config:\"\n\
  Write-Output $configPath\n\
  claude --settings $configPath\n\
}} finally {{\n\
  Remove-Item -LiteralPath $configPath -ErrorAction SilentlyContinue\n\
  Remove-Item -LiteralPath $scriptPath -ErrorAction SilentlyContinue\n\
}}\n"
    )
}

fn build_bash_launch_command(script_path: &Path) -> String {
    format!("bash {}", bash_single_quote(&script_path.to_string_lossy()))
}

fn build_powershell_launch_command(script_path: &Path) -> String {
    format!(
        "powershell -NoLogo -NoExit -ExecutionPolicy Bypass -File {}",
        windows_double_quote(&script_path.to_string_lossy())
    )
}

fn bash_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn powershell_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

fn windows_double_quote(value: &str) -> String {
    format!("\"{value}\"")
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_copy_api_key_to_clipboard(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let api_key = blocking::run(
        "provider_copy_api_key_to_clipboard",
        move || -> crate::shared::error::AppResult<String> {
            let conn = db.open_connection()?;
            let provider = providers::get_by_id(&conn, provider_id)?;
            if provider.auth_mode != "api_key" || provider.source_provider_id.is_some() {
                return Err("SEC_INVALID_INPUT: provider does not own a direct api_key"
                    .to_string()
                    .into());
            }

            let api_key = providers::get_api_key_plaintext(&db, provider_id)?;
            if api_key.trim().is_empty() {
                return Err("SEC_INVALID_INPUT: provider api_key is not configured"
                    .to_string()
                    .into());
            }

            Ok(api_key)
        },
    )
    .await?;

    app.clipboard().write_text(api_key).map_err(|err| {
        format!("SYSTEM_ERROR: failed to write provider api_key to clipboard: {err}")
    })?;
    tracing::info!(provider_id, "provider api_key copied to clipboard");
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn base_url_ping_ms(base_url: String) -> Result<u64, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("aio-coding-hub-ping/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("PING_HTTP_CLIENT_INIT: {e}"))?;
    base_url_probe::probe_base_url_ms(&client, &base_url, std::time::Duration::from_secs(3)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "windows"))]
    use std::process::{Command, Stdio};
    #[cfg(not(target_os = "windows"))]
    use tempfile::tempdir;

    #[test]
    fn bash_single_quote_escapes_single_quote() {
        assert_eq!(bash_single_quote("a'b"), "'a'\"'\"'b'");
    }

    #[test]
    fn powershell_single_quote_escapes_single_quote() {
        assert_eq!(powershell_single_quote("a'b"), "'a''b'");
    }

    #[test]
    fn build_settings_contains_required_envs() {
        let json_text = build_claude_settings_json("https://example.com", "sk-test").unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_text).unwrap();
        let env = value
            .get("env")
            .and_then(|v| v.as_object())
            .expect("env object");

        assert_eq!(
            env.get(ENV_CLAUDE_DISABLE_NONESSENTIAL_TRAFFIC)
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_DISABLE_ERROR_REPORTING)
                .and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_DISABLE_TELEMETRY).and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            env.get(ENV_MCP_TIMEOUT).and_then(|v| v.as_str()),
            Some("60000")
        );
        assert_eq!(
            env.get(ENV_ANTHROPIC_BASE_URL).and_then(|v| v.as_str()),
            Some("https://example.com")
        );
        assert_eq!(
            env.get(ENV_ANTHROPIC_AUTH_TOKEN).and_then(|v| v.as_str()),
            Some("sk-test")
        );
    }

    #[test]
    fn build_claude_gateway_base_url_trims_trailing_slash() {
        let url = build_claude_gateway_base_url("http://127.0.0.1:18080/", 12);
        assert_eq!(url, "http://127.0.0.1:18080/claude/_aio/provider/12");
    }

    #[test]
    fn bash_launch_script_includes_cleanup_and_claude_settings() {
        let config_path = Path::new("/tmp/claude_x.json");
        let script_path = Path::new("/tmp/aio_launcher.sh");
        let script = build_claude_launcher_bash_script(config_path, script_path);

        assert!(script.contains("cleanup() {"));
        assert!(script.contains("trap cleanup EXIT INT TERM HUP"));
        assert!(script.contains("claude --settings \"$config_path\""));
        assert!(script.contains("cleanup"));
        assert!(script.contains("exec bash --norc --noprofile"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn bash_launch_script_cleans_sensitive_files_before_shell_handoff() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("claude.json");
        let script_path = temp.path().join("launcher.sh");
        let fake_claude_path = temp.path().join("claude");
        let output_path = temp.path().join("claude-args.txt");

        fs::write(&config_path, "{}").expect("write config");
        fs::write(
            &script_path,
            build_claude_launcher_bash_script(&config_path, &script_path),
        )
        .expect("write script");
        fs::write(
            &fake_claude_path,
            "#!/bin/bash\nprintf '%s\n' \"$@\" > \"$OUTPUT_PATH\"\nexit 0\n",
        )
        .expect("write fake claude");

        let mut perms = fs::metadata(&fake_claude_path)
            .expect("fake claude metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_claude_path, perms).expect("chmod fake claude");

        let path_env = match std::env::var("PATH") {
            Ok(path) => format!("{}:{}", temp.path().display(), path),
            Err(_) => temp.path().display().to_string(),
        };
        let status = Command::new("bash")
            .arg(&script_path)
            .env("PATH", path_env)
            .env("OUTPUT_PATH", &output_path)
            .stdin(Stdio::null())
            .status()
            .expect("run launcher");

        assert!(status.success());
        assert!(!config_path.exists(), "config file should be removed");
        assert!(!script_path.exists(), "launcher script should be removed");

        let claude_args = fs::read_to_string(&output_path).expect("read fake claude args");
        assert!(claude_args.contains("--settings"));
        assert!(claude_args.contains(config_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn powershell_launch_script_includes_cleanup_and_claude_settings() {
        let config_path = Path::new(r"C:\\Temp\\claude_x.json");
        let script_path = Path::new(r"C:\\Temp\\aio_launcher.ps1");
        let script = build_claude_launcher_powershell_script(config_path, script_path);

        assert!(script.contains("Write-Output \"Using provider-specific claude config:\""));
        assert!(script.contains("claude --settings $configPath"));
        assert!(
            script.contains("Remove-Item -LiteralPath $configPath -ErrorAction SilentlyContinue")
        );
        assert!(
            script.contains("Remove-Item -LiteralPath $scriptPath -ErrorAction SilentlyContinue")
        );
    }

    #[test]
    fn powershell_launch_command_uses_expected_flags() {
        let script_path = Path::new(r"C:\\Temp\\aio_launcher.ps1");
        let command = build_powershell_launch_command(script_path);

        assert!(command.starts_with("powershell -NoLogo -NoExit -ExecutionPolicy Bypass -File"));
        assert!(command.contains("\"C:\\\\Temp\\\\aio_launcher.ps1\""));
    }

    #[test]
    fn claude_launch_artifact_paths_use_requested_directory() {
        let dir = Path::new("/tmp/aio-launchers");
        let (config_path, script_path) = claude_launch_artifact_paths(dir, 9, 77, 1234);

        assert_eq!(config_path, dir.join("claude_9_77_1234.json"));
        if cfg!(target_os = "windows") {
            assert_eq!(script_path, dir.join("aio_claude_launcher_9_77_1234.ps1"));
        } else {
            assert_eq!(script_path, dir.join("aio_claude_launcher_9_77_1234.sh"));
        }
    }

    #[test]
    fn detects_claude_launcher_artifact_file_names() {
        assert!(is_claude_launcher_artifact_file_name("claude_1_2_3.json"));
        assert!(is_claude_launcher_artifact_file_name(
            "aio_claude_launcher_1_2_3.sh"
        ));
        assert!(!is_claude_launcher_artifact_file_name("providers.json"));
    }
}
