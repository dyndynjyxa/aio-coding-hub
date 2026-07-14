use super::*;
use crate::infra::settings::{self, AppSettings, CodexHomeMode};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::MutexGuard;

static TEST_ENV_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct EnvRestore {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvRestore {
    fn save_once(&mut self, key: &'static str) {
        if self.saved.iter().any(|(k, _)| *k == key) {
            return;
        }
        self.saved.push((key, std::env::var_os(key)));
    }

    fn set_var(&mut self, key: &'static str, value: impl Into<OsString>) {
        self.save_once(key);
        std::env::set_var(key, value.into());
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..).rev() {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

struct CliProxyTestApp {
    _env: EnvRestore,
    _lock: MutexGuard<'static, ()>,
    #[allow(dead_code)]
    home: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
}

impl CliProxyTestApp {
    fn new() -> Self {
        let lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let seq = TEST_ENV_SEQ.fetch_add(1, Ordering::Relaxed);

        let mut env = EnvRestore::default();
        let home_os = home.path().as_os_str().to_os_string();
        env.set_var("AIO_CODING_HUB_HOME_DIR", home_os.clone());
        // app data 目录也使用每测例唯一 dotdir，避免共享真实 HOME 时读到旧 manifest。
        env.set_var(
            "AIO_CODING_HUB_DOTDIR_NAME",
            format!(".aio-coding-hub-cli-proxy-test-{seq}"),
        );
        crate::test_support::clear_settings_cache();

        Self {
            _lock: lock,
            _env: env,
            home,
            app: tauri::test::mock_app(),
        }
    }

    fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
        self.app.handle().clone()
    }
}

fn write_cli_proxy_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    enabled: bool,
    base_origin: Option<&str>,
) {
    write_manifest(
        app,
        cli_key,
        &CliProxyManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            managed_by: MANAGED_BY.to_string(),
            cli_key: cli_key.to_string(),
            enabled,
            base_origin: base_origin.map(str::to_string),
            created_at: 1,
            updated_at: 1,
            files: Vec::new(),
        },
    )
    .expect("write manifest");
}

fn codex_platform_for_tests() -> CodexConfigPlatform {
    CodexConfigPlatform::current()
}

fn write_codex_proxy_files<R: tauri::Runtime>(app: &tauri::AppHandle<R>, base_origin: &str) {
    let config_path = codex_config_path(app).expect("codex config path");
    let auth_path = codex_auth_path(app).expect("codex auth path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config dir");

    let config = build_codex_config_toml(
        None,
        &format!("{base_origin}/v1"),
        codex_platform_for_tests(),
    )
    .expect("build codex config");
    std::fs::write(&config_path, config).expect("write config");

    let auth = build_codex_auth_json(None).expect("build codex auth");
    std::fs::write(&auth_path, auth).expect("write auth");
}

fn write_codex_oauth_proxy_config<R: tauri::Runtime>(app: &tauri::AppHandle<R>, base_origin: &str) {
    let config_path = codex_config_path(app).expect("codex config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config dir");

    let config = build_codex_config_toml_oauth_compatible(
        None,
        &format!("{base_origin}/v1"),
        codex_platform_for_tests(),
    )
    .expect("build codex oauth config");
    std::fs::write(&config_path, config).expect("write config");
}

fn set_custom_codex_home<R: tauri::Runtime>(app: &tauri::AppHandle<R>, codex_home: &Path) {
    let settings = AppSettings {
        codex_home_mode: CodexHomeMode::Custom,
        codex_home_override: codex_home.display().to_string(),
        ..AppSettings::default()
    };
    settings::write(app, &settings).expect("write settings");
}

fn set_codex_oauth_compatible_proxy_mode<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    enabled: bool,
) {
    let mut settings = settings::read(app).unwrap_or_default();
    settings.codex_oauth_compatible_proxy_mode = enabled;
    settings::write(app, &settings).expect("write settings");
}

fn write_codex_direct_files<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    config: &str,
    auth: &str,
) {
    let config_path = codex_config_path(app).expect("codex config path");
    let auth_path = codex_auth_path(app).expect("codex auth path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config dir");
    std::fs::write(&config_path, config).expect("write direct config");
    std::fs::write(&auth_path, auth).expect("write direct auth");
}

fn manifest_entry<'a>(manifest: &'a CliProxyManifest, kind: &str) -> &'a BackupFileEntry {
    manifest
        .files
        .iter()
        .find(|entry| entry.kind == kind)
        .unwrap_or_else(|| panic!("missing manifest entry for kind={kind}"))
}

#[test]
fn enable_grok_proxy_writes_managed_profile_and_preserves_auxiliary_config() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(
        &config_path,
        r#"# preserve top comment
[models]
default = "direct"
session_summary = "summary"
web_search = "search"
image_description = "vision"

[model.direct]
base_url = "https://direct.example/v1"

[model.search]
api_backend = "responses"

[mcp_servers.keep]
command = "npx"
"#,
    )
    .expect("write direct config");
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(crate::grok_config::GrokProxyPreferences {
        model_id: "grok-test-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
        context_window: Some(500_000),
        telemetry: Some(false),
        supports_backend_search: Some(false),
    });
    settings::write(&handle, &app_settings).expect("write preferences");

    let result =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable grok proxy");

    assert!(result.ok, "{}", result.message);
    assert!(result.enabled);
    let updated = std::fs::read_to_string(&config_path).expect("read updated config");
    let document = updated
        .parse::<toml_edit::DocumentMut>()
        .expect("valid updated TOML");
    assert!(updated.starts_with("# preserve top comment"));
    assert_eq!(document["models"]["default"].as_str(), Some("aio"));
    assert_eq!(document["models"]["session_summary"].as_str(), Some("aio"));
    assert_eq!(document["models"]["web_search"].as_str(), Some("aio"));
    assert_eq!(
        document["models"]["image_description"].as_str(),
        Some("aio")
    );
    assert_eq!(
        document["model"]["aio"]["model"].as_str(),
        Some("grok-test-model")
    );
    assert_eq!(
        document["model"]["aio"]["base_url"].as_str(),
        Some("http://127.0.0.1:26543/grok/v1")
    );
    assert_eq!(
        document["model"]["aio"]["api_key"].as_str(),
        Some(PLACEHOLDER_KEY)
    );
    assert_eq!(
        document["model"]["aio"]["api_backend"].as_str(),
        Some("chat_completions")
    );
    assert_eq!(
        document["model"]["aio"]["supports_backend_search"].as_bool(),
        Some(false)
    );
    assert_eq!(
        document["model"]["aio"]["context_window"].as_integer(),
        Some(500_000)
    );
    assert_eq!(document["features"]["telemetry"].as_bool(), Some(false));
    assert_eq!(
        document["mcp_servers"]["keep"]["command"].as_str(),
        Some("npx")
    );

    let manifest = read_manifest(&handle, "grok")
        .expect("read manifest")
        .expect("grok manifest");
    assert!(manifest.enabled);
    let entry = manifest_entry(&manifest, "grok_config_toml");
    assert_eq!(Path::new(&entry.path), config_path);
    assert!(entry.existed);
    assert_eq!(entry.backup_rel.as_deref(), Some("config.toml"));
}

#[test]
fn disable_grok_proxy_restores_only_managed_fields() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(
        &config_path,
        r#"# original comment
[models]
default = "direct"
session_summary = "summary"
web_search = "search"

[model.aio]
model = "original-aio-model"
base_url = "https://original.example/v1"
api_key = "original-placeholder"
api_backend = "responses"
supports_backend_search = false
context_window = 131072

[features]
telemetry = true
"#,
    )
    .expect("write direct config");
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(crate::grok_config::GrokProxyPreferences {
        model_id: "managed-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
        ..Default::default()
    });
    settings::write(&handle, &app_settings).expect("write preferences");

    let enabled =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    crate::grok_config::mutate_path(&config_path, |document| {
        crate::grok_config::set_string(&mut document["models"]["web_search"], "new-search");
        document["model"]["aio"]["user_setting"] = toml_edit::value("keep-me");
        document["mcp_servers"]["added"]["command"] = toml_edit::value("bunx");
        document["user"]["enabled"] = toml_edit::value(true);
        Ok(())
    })
    .expect("write proxy-period changes");
    let mut with_comment = std::fs::read_to_string(&config_path).expect("read proxy config");
    with_comment.push_str("\n# added while proxy\n");
    std::fs::write(&config_path, with_comment).expect("append proxy-period comment");

    let disabled =
        set_enabled(&handle, "grok", false, "http://127.0.0.1:26543").expect("disable grok proxy");

    assert!(disabled.ok, "{}", disabled.message);
    assert!(!disabled.enabled);
    let restored = std::fs::read_to_string(&config_path).expect("read restored config");
    let document = restored
        .parse::<toml_edit::DocumentMut>()
        .expect("valid restored TOML");
    assert!(restored.contains("# original comment"));
    assert!(restored.contains("# added while proxy"));
    assert_eq!(document["models"]["default"].as_str(), Some("direct"));
    assert_eq!(
        document["models"]["session_summary"].as_str(),
        Some("summary")
    );
    assert_eq!(document["models"]["web_search"].as_str(), Some("search"));
    assert_eq!(
        document["model"]["aio"]["model"].as_str(),
        Some("original-aio-model")
    );
    assert_eq!(
        document["model"]["aio"]["base_url"].as_str(),
        Some("https://original.example/v1")
    );
    assert_eq!(
        document["model"]["aio"]["api_key"].as_str(),
        Some("original-placeholder")
    );
    assert_eq!(
        document["model"]["aio"]["api_backend"].as_str(),
        Some("responses")
    );
    assert_eq!(
        document["model"]["aio"]["context_window"].as_integer(),
        Some(131072)
    );
    assert_eq!(
        document["model"]["aio"]["supports_backend_search"].as_bool(),
        Some(false)
    );
    assert_eq!(document["features"]["telemetry"].as_bool(), Some(true));
    assert_eq!(
        document["model"]["aio"]["user_setting"].as_str(),
        Some("keep-me")
    );
    assert_eq!(
        document["mcp_servers"]["added"]["command"].as_str(),
        Some("bunx")
    );
    assert_eq!(document["user"]["enabled"].as_bool(), Some(true));
}

#[test]
fn grok_proxy_status_and_port_sync_track_exact_managed_state() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let preferences = crate::grok_config::GrokProxyPreferences {
        model_id: "grok-port-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::Responses,
        context_window: Some(500_000),
        telemetry: Some(false),
        supports_backend_search: None,
    };
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(preferences);
    settings::write(&handle, &app_settings).expect("write preferences");

    let first_origin = "http://127.0.0.1:26543";
    let enabled = set_enabled(&handle, "grok", true, first_origin).expect("enable grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    let initial_document = std::fs::read_to_string(&config_path)
        .expect("read initial config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid initial TOML");
    assert_eq!(initial_document["models"]["default"].as_str(), Some("aio"));
    assert_eq!(
        initial_document["models"]["session_summary"].as_str(),
        Some("aio")
    );
    assert_eq!(
        initial_document["model"]["aio"]["model"].as_str(),
        Some("grok-port-model")
    );
    assert_eq!(
        initial_document["model"]["aio"]["base_url"].as_str(),
        Some("http://127.0.0.1:26543/grok/v1")
    );
    assert_eq!(
        initial_document["model"]["aio"]["api_key"].as_str(),
        Some(PLACEHOLDER_KEY)
    );
    assert_eq!(
        initial_document["model"]["aio"]["api_backend"].as_str(),
        Some("responses")
    );
    assert_eq!(
        initial_document["model"]["aio"]["supports_backend_search"].as_bool(),
        Some(true)
    );
    assert_eq!(
        initial_document["model"]["aio"]["context_window"].as_integer(),
        Some(500_000)
    );
    assert_eq!(
        initial_document["features"]["telemetry"].as_bool(),
        Some(false)
    );
    assert!(
        initial_document
            .get("models")
            .is_some_and(toml_edit::Item::is_table),
        "models must be a TOML table:\n{initial_document}"
    );
    assert!(
        initial_document
            .get("model")
            .is_some_and(toml_edit::Item::is_table),
        "model must be a TOML table:\n{initial_document}"
    );
    assert_eq!(
        settings::read(&handle)
            .expect("read saved settings")
            .grok_proxy_preferences
            .as_ref()
            .map(|preferences| preferences.model_id.as_str()),
        Some("grok-port-model")
    );
    let saved_preferences = settings::read(&handle)
        .expect("read saved preferences")
        .grok_proxy_preferences
        .expect("saved Grok preferences");
    assert!(crate::grok_config::is_proxy_profile_applied(
        &handle,
        first_origin,
        &saved_preferences,
        PLACEHOLDER_KEY,
    )
    .expect("inspect Grok proxy profile"));
    assert!(grok::is_proxy_config_applied(&handle, first_origin));

    crate::grok_config::mutate_path(&config_path, |document| {
        document["model"]["aio"]["supports_backend_search"] = toml_edit::value(false);
        document["model"]["aio"]["context_window"] = toml_edit::value(100_000);
        document["features"]["telemetry"] = toml_edit::value(true);
        Ok(())
    })
    .expect("drift managed Grok fields");
    assert!(!crate::grok_config::is_proxy_profile_applied(
        &handle,
        first_origin,
        &saved_preferences,
        PLACEHOLDER_KEY,
    )
    .expect("inspect drifted Grok proxy profile"));

    crate::grok_config::apply_proxy_profile(
        &handle,
        first_origin,
        &saved_preferences,
        PLACEHOLDER_KEY,
    )
    .expect("reapply Grok proxy profile");
    assert!(crate::grok_config::is_proxy_profile_applied(
        &handle,
        first_origin,
        &saved_preferences,
        PLACEHOLDER_KEY,
    )
    .expect("inspect reapplied Grok proxy profile"));

    let initial_status = status_all(&handle, Some(first_origin)).expect("initial status");
    let grok_status = initial_status
        .iter()
        .find(|status| status.cli_key == "grok")
        .expect("grok status");
    assert_eq!(grok_status.applied_to_current_gateway, Some(true));

    let next_origin = "http://127.0.0.1:27543";
    let sync_results = sync_enabled(&handle, next_origin, true).expect("sync enabled proxy");
    let grok_result = sync_results
        .iter()
        .find(|result| result.cli_key == "grok")
        .expect("grok sync result");
    assert!(grok_result.ok, "{}", grok_result.message);

    let document = std::fs::read_to_string(config_path)
        .expect("read synced config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid synced TOML");
    assert_eq!(
        document["model"]["aio"]["base_url"].as_str(),
        Some("http://127.0.0.1:27543/grok/v1")
    );
    assert_eq!(
        document["model"]["aio"]["model"].as_str(),
        Some("grok-port-model")
    );
    assert_eq!(
        document["model"]["aio"]["api_backend"].as_str(),
        Some("responses")
    );
    assert_eq!(
        document["model"]["aio"]["supports_backend_search"].as_bool(),
        Some(true)
    );

    let synced_status = status_all(&handle, Some(next_origin)).expect("synced status");
    let grok_status = synced_status
        .iter()
        .find(|status| status.cli_key == "grok")
        .expect("grok status");
    assert_eq!(grok_status.applied_to_current_gateway, Some(true));
}

#[test]
fn updating_grok_preferences_while_proxy_enabled_updates_settings_and_toml() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(crate::grok_config::GrokProxyPreferences {
        model_id: "initial-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::Responses,
        ..Default::default()
    });
    settings::write(&handle, &app_settings).expect("write initial preferences");
    let enabled =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    let updated = crate::grok_config::GrokProxyPreferences {
        model_id: "updated-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
        ..Default::default()
    };
    let state = set_grok_preferences(&handle, updated.clone()).expect("update preferences");

    assert_eq!(state.aio_preferences, Some(updated.clone()));
    assert_eq!(state.effective_preferences, updated);
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    let document = std::fs::read_to_string(config_path)
        .expect("read updated config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid updated TOML");
    assert_eq!(
        document["model"]["aio"]["model"].as_str(),
        Some("updated-model")
    );
    assert_eq!(
        document["model"]["aio"]["api_backend"].as_str(),
        Some("chat_completions")
    );
    assert_eq!(
        document["model"]["aio"]["supports_backend_search"].as_bool(),
        Some(true)
    );
    assert_eq!(
        document["model"]["aio"]["base_url"].as_str(),
        Some("http://127.0.0.1:26543/grok/v1")
    );
}

#[test]
fn updating_grok_preferences_rolls_back_settings_when_toml_update_fails() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let initial_preferences = crate::grok_config::GrokProxyPreferences {
        model_id: "initial-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::Responses,
        ..Default::default()
    };
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(initial_preferences.clone());
    settings::write(&handle, &app_settings).expect("write initial preferences");
    let enabled =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    let invalid_schema =
        b"model = \"not-a-table\"\n\n[models]\ndefault = \"aio\"\nsession_summary = \"aio\"\n";
    std::fs::write(&config_path, invalid_schema).expect("write invalid schema fixture");

    let error = set_grok_preferences(
        &handle,
        crate::grok_config::GrokProxyPreferences {
            model_id: "new-model".to_string(),
            api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
            ..Default::default()
        },
    )
    .expect_err("TOML schema update must fail");

    assert!(error.to_string().contains("GROK_CONFIG_INVALID_SCHEMA"));
    assert_eq!(
        settings::read(&handle)
            .expect("read rolled back settings")
            .grok_proxy_preferences,
        Some(initial_preferences)
    );
    assert_eq!(
        std::fs::read(&config_path).expect("read unchanged TOML"),
        invalid_schema
    );
}

#[test]
fn updating_grok_preferences_while_proxy_disabled_rejects_invalid_toml_without_saving() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let initial_preferences = crate::grok_config::GrokProxyPreferences {
        model_id: "initial-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::Responses,
        ..Default::default()
    };
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(initial_preferences.clone());
    settings::write(&handle, &app_settings).expect("write initial preferences");

    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    let invalid = b"[models\ndefault = broken\n";
    std::fs::write(&config_path, invalid).expect("write invalid config fixture");

    let error = set_grok_preferences(
        &handle,
        crate::grok_config::GrokProxyPreferences {
            model_id: "new-model".to_string(),
            api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
            ..Default::default()
        },
    )
    .expect_err("invalid Grok TOML must block preference updates");

    assert!(error.to_string().contains("GROK_CONFIG_INVALID_TOML"));
    assert_eq!(
        settings::read(&handle)
            .expect("read unchanged settings")
            .grok_proxy_preferences,
        Some(initial_preferences)
    );
    assert_eq!(
        std::fs::read(&config_path).expect("read unchanged TOML"),
        invalid
    );
}

#[test]
fn enable_grok_proxy_preserves_invalid_toml_and_writes_safety_copy() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    let invalid = b"[models\ndefault = broken\n";
    std::fs::write(&config_path, invalid).expect("write invalid config");

    let result =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("attempt enable");

    assert!(!result.ok);
    assert!(!result.enabled);
    assert!(result.message.contains("GROK_CONFIG_INVALID_TOML"));
    assert_eq!(std::fs::read(&config_path).expect("read original"), invalid);
    let safety_path = config_path.with_extension("toml.invalid-backup");
    assert_eq!(
        std::fs::read(safety_path).expect("read safety copy"),
        invalid
    );
    let manifest = read_manifest(&handle, "grok")
        .expect("read manifest")
        .expect("grok manifest");
    assert!(!manifest.enabled);
}

#[test]
fn sync_enabled_rebinds_grok_home_and_restores_old_target() {
    let mut test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let old_home = test_app.home.path().join("grok-old");
    let new_home = test_app.home.path().join("grok-new");
    test_app
        ._env
        .set_var("GROK_HOME", old_home.as_os_str().to_os_string());
    std::fs::create_dir_all(&old_home).expect("create old Grok home");
    let old_config = old_home.join("config.toml");
    std::fs::write(
        &old_config,
        r#"[models]
default = "old-direct"
session_summary = "old-summary"
web_search = "old-search"
"#,
    )
    .expect("write old config");
    let mut app_settings = settings::read(&handle).expect("read settings");
    app_settings.grok_proxy_preferences = Some(crate::grok_config::GrokProxyPreferences {
        model_id: "managed-model".to_string(),
        api_backend: crate::grok_config::GrokApiBackend::Responses,
        ..Default::default()
    });
    settings::write(&handle, &app_settings).expect("write preferences");
    let base_origin = "http://127.0.0.1:26543";
    let enabled = set_enabled(&handle, "grok", true, base_origin).expect("enable proxy");
    assert!(enabled.ok, "{}", enabled.message);
    crate::grok_config::mutate_path(&old_config, |document| {
        document["mcp_servers"]["old-added"]["command"] = toml_edit::value("npx");
        Ok(())
    })
    .expect("add old MCP while proxied");

    std::fs::create_dir_all(&new_home).expect("create new Grok home");
    let new_config = new_home.join("config.toml");
    std::fs::write(
        &new_config,
        r#"[models]
default = "new-direct"
session_summary = "new-summary"
web_search = "new-search"
"#,
    )
    .expect("write new config");
    test_app
        ._env
        .set_var("GROK_HOME", new_home.as_os_str().to_os_string());

    let results = sync_enabled(&handle, base_origin, true).expect("sync after home change");
    let grok_result = results
        .iter()
        .find(|result| result.cli_key == "grok")
        .expect("grok sync result");
    assert!(grok_result.ok, "{}", grok_result.message);

    let old_document = std::fs::read_to_string(&old_config)
        .expect("read restored old config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid old config");
    assert_eq!(
        old_document["models"]["default"].as_str(),
        Some("old-direct")
    );
    assert_eq!(
        old_document["models"]["session_summary"].as_str(),
        Some("old-summary")
    );
    assert_eq!(
        old_document["mcp_servers"]["old-added"]["command"].as_str(),
        Some("npx")
    );

    let new_document = std::fs::read_to_string(&new_config)
        .expect("read managed new config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid new config");
    assert_eq!(new_document["models"]["default"].as_str(), Some("aio"));
    assert_eq!(
        new_document["models"]["session_summary"].as_str(),
        Some("aio")
    );
    assert_eq!(new_document["models"]["web_search"].as_str(), Some("aio"));
    let manifest = read_manifest(&handle, "grok")
        .expect("read rebound manifest")
        .expect("grok manifest");
    assert_eq!(
        Path::new(&manifest_entry(&manifest, "grok_config_toml").path),
        new_config
    );

    let disabled = set_enabled(&handle, "grok", false, base_origin).expect("disable proxy");
    assert!(disabled.ok, "{}", disabled.message);
    let new_document = std::fs::read_to_string(&new_config)
        .expect("read restored new config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid restored new config");
    assert_eq!(
        new_document["models"]["default"].as_str(),
        Some("new-direct")
    );
    assert_eq!(
        new_document["models"]["session_summary"].as_str(),
        Some("new-summary")
    );
}

#[test]
fn first_grok_enable_initializes_preferences_from_existing_default_profile() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(
        config_path,
        r#"[models]
default = "custom-profile"

[model.custom-profile]
model = "existing-model"
api_backend = "chat_completions"
context_window = 262144
supports_backend_search = false

[features]
telemetry = false
"#,
    )
    .expect("write existing config");

    let result =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable Grok proxy");

    assert!(result.ok, "{}", result.message);
    assert_eq!(
        settings::read(&handle)
            .expect("read initialized settings")
            .grok_proxy_preferences,
        Some(crate::grok_config::GrokProxyPreferences {
            model_id: "existing-model".to_string(),
            api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
            context_window: Some(262_144),
            telemetry: Some(false),
            supports_backend_search: Some(false),
        })
    );
}

#[test]
fn missing_grok_config_uses_fallback_and_disable_restores_absence() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    assert!(!config_path.exists());

    let enabled =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable Grok proxy");
    assert!(enabled.ok, "{}", enabled.message);
    assert_eq!(
        settings::read(&handle)
            .expect("read initialized settings")
            .grok_proxy_preferences,
        Some(crate::grok_config::GrokProxyPreferences::default())
    );
    assert!(config_path.exists());

    let disabled =
        set_enabled(&handle, "grok", false, "http://127.0.0.1:26543").expect("disable Grok proxy");
    assert!(disabled.ok, "{}", disabled.message);
    assert!(!config_path.exists());
}

#[test]
fn grok_proxy_round_trip_preserves_unmanaged_inline_tables() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(
        &config_path,
        "models = { custom_search = \"search\" }\nmodel = { custom = { model = \"keep\" } }\n",
    )
    .expect("write inline config");
    let original = std::fs::read(&config_path).expect("read original inline config");

    let enabled =
        set_enabled(&handle, "grok", true, "http://127.0.0.1:26543").expect("enable Grok proxy");
    assert!(enabled.ok, "{}", enabled.message);
    let disabled =
        set_enabled(&handle, "grok", false, "http://127.0.0.1:26543").expect("disable Grok proxy");
    assert!(disabled.ok, "{}", disabled.message);

    assert_eq!(
        std::fs::read(&config_path).expect("read restored inline config"),
        original
    );
}

#[test]
fn startup_repair_marks_applied_grok_proxy_manifest_enabled() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let base_origin = "http://127.0.0.1:26543";
    let enabled = set_enabled(&handle, "grok", true, base_origin).expect("enable Grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    let mut manifest = read_manifest(&handle, "grok")
        .expect("read manifest")
        .expect("grok manifest");
    manifest.enabled = false;
    write_manifest(&handle, "grok", &manifest).expect("simulate interrupted manifest write");

    let repairs = startup_repair_incomplete_enable(&handle).expect("startup repair");
    let grok_repair = repairs
        .iter()
        .find(|result| result.cli_key == "grok")
        .expect("Grok repair result");
    assert!(grok_repair.ok, "{}", grok_repair.message);
    assert!(grok_repair.enabled);
    assert!(
        read_manifest(&handle, "grok")
            .expect("read repaired manifest")
            .expect("grok manifest")
            .enabled
    );
}

#[test]
fn grok_proxy_reapplies_after_exit_restore_keeps_enabled_state() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = crate::grok_config::config_path(&handle).expect("grok config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(
        &config_path,
        "[models]\ndefault = \"direct\"\nsession_summary = \"summary\"\n",
    )
    .expect("write direct config");
    let base_origin = "http://127.0.0.1:26543";
    let enabled = set_enabled(&handle, "grok", true, base_origin).expect("enable Grok proxy");
    assert!(enabled.ok, "{}", enabled.message);

    let restored = restore_enabled_keep_state(&handle).expect("exit restore");
    let grok_restore = restored
        .iter()
        .find(|result| result.cli_key == "grok")
        .expect("Grok restore result");
    assert!(grok_restore.ok, "{}", grok_restore.message);
    let direct = std::fs::read_to_string(&config_path)
        .expect("read direct config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid direct config");
    assert_eq!(direct["models"]["default"].as_str(), Some("direct"));
    assert!(is_enabled(&handle, "grok").expect("enabled state"));

    let synced = sync_enabled(&handle, base_origin, true).expect("startup sync");
    let grok_sync = synced
        .iter()
        .find(|result| result.cli_key == "grok")
        .expect("Grok sync result");
    assert!(grok_sync.ok, "{}", grok_sync.message);
    let managed = std::fs::read_to_string(&config_path)
        .expect("read managed config")
        .parse::<toml_edit::DocumentMut>()
        .expect("valid managed config");
    assert_eq!(managed["models"]["default"].as_str(), Some("aio"));
    assert_eq!(managed["models"]["session_summary"].as_str(), Some("aio"));
}

#[test]
fn read_manifest_rejects_oversized_file() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let root = cli_proxy_root_dir(&handle, "codex").expect("cli proxy root");
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(
        cli_proxy_manifest_path(&root),
        vec![b'x'; CLI_PROXY_MANIFEST_MAX_BYTES + 1],
    )
    .expect("write oversized manifest");

    let err = read_manifest(&handle, "codex").expect_err("oversized manifest should fail");

    assert!(err.to_string().contains("too large"));
}

#[test]
fn backup_for_enable_rejects_oversized_target_file() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = codex_config_path(&handle).expect("codex config path");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config parent");
    std::fs::write(&config_path, vec![b'x'; CLI_PROXY_FILE_MAX_BYTES + 1])
        .expect("write oversized config");

    let err = backup_for_enable(&handle, "codex", "http://127.0.0.1:37123", None)
        .expect_err("oversized target file should fail");

    assert!(err.to_string().contains("too large"));
    assert!(read_manifest(&handle, "codex")
        .expect("read manifest")
        .is_none());
}

#[test]
fn restore_backups_exactly_rejects_oversized_backup_file() {
    let test_app = CliProxyTestApp::new();
    let handle = test_app.handle();
    let config_path = codex_config_path(&handle).expect("codex config path");
    let root = cli_proxy_root_dir(&handle, "codex").expect("cli proxy root");
    let files_dir = cli_proxy_files_dir(&root);
    std::fs::create_dir_all(&files_dir).expect("create files dir");
    std::fs::write(
        files_dir.join("config.toml"),
        vec![b'x'; CLI_PROXY_FILE_MAX_BYTES + 1],
    )
    .expect("write oversized backup");
    let manifest = CliProxyManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: "codex".to_string(),
        enabled: true,
        base_origin: Some("http://127.0.0.1:37123".to_string()),
        created_at: 1,
        updated_at: 1,
        files: vec![BackupFileEntry {
            kind: "unknown_kind".to_string(),
            path: config_path.to_string_lossy().to_string(),
            existed: true,
            backup_rel: Some("config.toml".to_string()),
        }],
    };

    let err = restore_backups_exactly_from_manifest(&handle, &manifest)
        .expect_err("oversized backup should fail");

    assert!(err.to_string().contains("too large"));
}

#[test]
fn codex_proxy_preserves_nested_model_provider_tables_and_order() {
    let input = r#"
model_provider = "aio"
preferred_auth_method = "apikey"

[model_providers.aio]
name = "aio"
base_url = "http://old/v1"
wire_api = "responses"
requires_openai_auth = true

[model_providers.aio.projects."C:\\work"]
trust_level = "trusted"

[other]
foo = "bar"
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("base_url = \"http://new/v1\""), "{s}");
    assert!(
        s.contains("[model_providers.aio.projects.\"C:\\\\work\"]"),
        "{s}"
    );
    assert!(s.contains("trust_level = \"trusted\""), "{s}");

    let base_idx = s.find("[model_providers.aio]").expect("base table exists");
    let nested_idx = s
        .find("[model_providers.aio.projects.\"C:\\\\work\"]")
        .expect("nested table exists");
    assert!(base_idx < nested_idx, "base must appear before nested: {s}");
}

#[test]
fn codex_proxy_preserves_extra_keys_in_base_table() {
    let input = r#"
[model_providers.aio]
name = "aio"
base_url = "http://old/v1"
wire_api = "responses"
requires_openai_auth = true
trusted_roots = ["C:\\work"]
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("base_url = \"http://new/v1\""), "{s}");
    assert!(s.contains("trusted_roots = [\"C:\\\\work\"]"), "{s}");
}

#[test]
fn codex_proxy_dedupes_multiple_base_tables() {
    let input = r#"
[model_providers."aio"]
base_url = "http://old-1/v1"

[model_providers.aio]
base_url = "http://old-2/v1"

[model_providers.aio.projects."C:\\work"]
trust_level = "trusted"
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    let count = s.matches("[model_providers.aio]").count()
        + s.matches("[model_providers.\"aio\"]").count()
        + s.matches("[model_providers.'aio']").count();
    assert_eq!(count, 1, "{s}");
    assert!(s.contains("base_url = \"http://new/v1\""), "{s}");
    assert!(
        s.contains("[model_providers.aio.projects.\"C:\\\\work\"]"),
        "{s}"
    );
}

#[test]
fn codex_oauth_compatible_config_removes_aio_owned_preferred_auth_method() {
    let input = r#"
preferred_auth_method = "apikey"
model = "gpt-5-codex"

[model_providers.aio]
base_url = "http://old/v1"
"#;

    let out = build_codex_config_toml_oauth_compatible(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("model_provider = \"aio\""), "{s}");
    assert!(!s.contains("preferred_auth_method"), "{s}");
    assert!(s.contains("model = \"gpt-5-codex\""), "{s}");
    assert!(s.contains("base_url = \"http://new/v1\""), "{s}");
    assert!(s.contains("requires_openai_auth = true"), "{s}");
}

#[test]
fn codex_oauth_compatible_config_preserves_non_aio_preferred_auth_method() {
    let input = r#"
preferred_auth_method = "chatgpt"

[existing]
foo = "bar"
"#;

    let out = build_codex_config_toml_oauth_compatible(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("preferred_auth_method = \"chatgpt\""), "{s}");
    assert!(s.contains("base_url = \"http://new/v1\""), "{s}");
}

#[test]
fn codex_proxy_inserts_base_table_before_nested_when_missing() {
    let input = r#"
[model_providers.aio.projects."C:\\work"]
trust_level = "trusted"
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    let base_idx = s
        .find("[model_providers.aio]")
        .expect("base table inserted");
    let nested_idx = s
        .find("[model_providers.aio.projects.\"C:\\\\work\"]")
        .expect("nested table exists");
    assert!(base_idx < nested_idx, "base must appear before nested: {s}");
}

#[test]
fn codex_proxy_moves_base_table_before_nested_when_out_of_order() {
    let input = r#"
[model_providers.aio.projects."C:\\work"]
trust_level = "trusted"

[model_providers.aio]
name = "aio"
base_url = "http://old/v1"
wire_api = "responses"
requires_openai_auth = true
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    let base_idx = s.find("[model_providers.aio]").expect("base table exists");
    let nested_idx = s
        .find("[model_providers.aio.projects.\"C:\\\\work\"]")
        .expect("nested table exists");
    assert!(base_idx < nested_idx, "base must appear before nested: {s}");
}

#[test]
fn codex_proxy_adds_windows_sandbox_only_on_windows() {
    let out = build_codex_config_toml(None, "http://new/v1", CodexConfigPlatform::Windows)
        .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("[windows]"), "{s}");
    assert!(s.contains("sandbox = \"elevated\""), "{s}");
}

#[test]
fn codex_proxy_does_not_add_windows_sandbox_on_non_windows() {
    let out =
        build_codex_config_toml(None, "http://new/v1", CodexConfigPlatform::Other).expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(!s.contains("[windows]"), "{s}");
    assert!(!s.contains("sandbox = \"elevated\""), "{s}");
}

#[test]
fn codex_proxy_preserves_existing_windows_block_on_non_windows() {
    let input = r#"
[windows]
sandbox = "elevated"

[existing]
foo = "bar"
"#;

    let out = build_codex_config_toml(
        Some(input.as_bytes().to_vec()),
        "http://new/v1",
        CodexConfigPlatform::Other,
    )
    .expect("build");
    let s = String::from_utf8(out).expect("utf8");

    assert!(s.contains("[windows]"), "{s}");
    assert!(s.contains("sandbox = \"elevated\""), "{s}");
    assert!(s.contains("[existing]"), "{s}");
    assert!(s.contains("foo = \"bar\""), "{s}");
}

#[test]
fn codex_proxy_auth_json_preserves_existing_oauth_fields() {
    let input = r#"{
  "oauth_access_token": "tok-123",
  "oauth_refresh_token": "ref-456",
  "OPENAI_API_KEY": "old-key"
}"#;

    let out = build_codex_auth_json(Some(input.as_bytes().to_vec())).expect("build auth");
    let value: serde_json::Value = serde_json::from_slice(&out).expect("parse output");

    assert_eq!(
        value.get("OPENAI_API_KEY").and_then(|v| v.as_str()),
        Some("aio-coding-hub")
    );
    assert_eq!(
        value.get("oauth_access_token").and_then(|v| v.as_str()),
        Some("tok-123")
    );
    assert_eq!(
        value.get("oauth_refresh_token").and_then(|v| v.as_str()),
        Some("ref-456")
    );
}

#[test]
fn codex_proxy_auth_json_rejects_non_object_root() {
    let input = r#"["not", "an", "object"]"#;
    let err = build_codex_auth_json(Some(input.as_bytes().to_vec())).expect_err("must fail");
    assert!(err
        .to_string()
        .contains("auth.json root must be a JSON object"));
}

#[test]
fn codex_proxy_enable_does_not_partially_write_config_when_auth_json_is_invalid() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    let original_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://api.openai.com/v1"
"#;
    let invalid_auth = r#"{ "tokens": "#;

    write_codex_direct_files(&handle, original_config, invalid_auth);

    let result = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(!result.ok, "enable must fail: {result:?}");
    assert_eq!(
        result.error_code.as_deref(),
        Some("CLI_PROXY_ENABLE_FAILED")
    );
    assert!(
        result.message.contains("CLI_PROXY_INVALID_AUTH_JSON"),
        "unexpected message: {}",
        result.message
    );

    let config_path = codex_config_path(&handle).expect("codex config path");
    let auth_path = codex_auth_path(&handle).expect("codex auth path");
    assert_eq!(
        std::fs::read_to_string(&config_path).expect("read config"),
        original_config,
        "config.toml must stay untouched when a later target fails to parse"
    );
    assert_eq!(
        std::fs::read_to_string(&auth_path).expect("read auth"),
        invalid_auth,
        "auth.json must stay untouched on parse failure"
    );
    assert_eq!(
        std::fs::read_to_string(auth_path.with_extension("json.invalid-backup"))
            .expect("read invalid backup"),
        invalid_auth
    );

    let manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest should preserve backup snapshot");
    assert!(
        !manifest.enabled,
        "failed enable should not mark the proxy as enabled"
    );
}

#[test]
fn status_all_skips_gateway_check_when_gateway_not_running_even_if_codex_is_applied() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    write_cli_proxy_manifest(&handle, "codex", true, Some(base_origin));
    write_codex_proxy_files(&handle, base_origin);

    let rows = status_all(&handle, None).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(codex.enabled);
    assert_eq!(codex.base_origin.as_deref(), Some(base_origin));
    assert_eq!(codex.applied_to_current_gateway, None);
}

#[test]
fn status_all_skips_gateway_check_when_gateway_not_running_even_if_codex_has_drifted() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    write_cli_proxy_manifest(&handle, "codex", true, Some(base_origin));
    write_codex_proxy_files(&handle, "http://127.0.0.1:9999");

    let rows = status_all(&handle, None).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(codex.enabled);
    assert_eq!(codex.base_origin.as_deref(), Some(base_origin));
    assert_eq!(codex.applied_to_current_gateway, None);
}

#[test]
fn status_all_skips_gateway_application_check_for_disabled_codex() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    write_cli_proxy_manifest(&handle, "codex", false, Some(base_origin));
    write_codex_proxy_files(&handle, base_origin);

    let rows = status_all(&handle, None).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(!codex.enabled);
    assert_eq!(codex.base_origin.as_deref(), Some(base_origin));
    assert_eq!(codex.applied_to_current_gateway, None);
}

#[test]
fn status_all_prefers_current_gateway_origin_when_available() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let manifest_origin = "http://127.0.0.1:37123";
    let current_origin = "http://127.0.0.1:37125";

    write_cli_proxy_manifest(&handle, "codex", true, Some(manifest_origin));
    write_codex_proxy_files(&handle, current_origin);

    let rows = status_all(&handle, Some(current_origin)).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(codex.enabled);
    assert_eq!(codex.base_origin.as_deref(), Some(manifest_origin));
    assert_eq!(
        codex.current_gateway_origin.as_deref(),
        Some(current_origin)
    );
    assert_eq!(codex.applied_to_current_gateway, Some(true));
}

#[test]
fn status_all_reports_drift_against_current_gateway_origin() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let manifest_origin = "http://127.0.0.1:37123";
    let current_origin = "http://127.0.0.1:37125";

    write_cli_proxy_manifest(&handle, "codex", true, Some(manifest_origin));
    write_codex_proxy_files(&handle, manifest_origin);

    let rows = status_all(&handle, Some(current_origin)).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(codex.enabled);
    assert_eq!(codex.base_origin.as_deref(), Some(manifest_origin));
    assert_eq!(
        codex.current_gateway_origin.as_deref(),
        Some(current_origin)
    );
    assert_eq!(codex.applied_to_current_gateway, Some(false));
}

#[test]
fn enabling_codex_oauth_compatible_proxy_writes_config_only_and_does_not_create_auth() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    let result = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(result.ok, "{result:?}");

    let config_path = codex_config_path(&handle).expect("config path");
    let auth_path = codex_auth_path(&handle).expect("auth path");
    let config = std::fs::read_to_string(config_path).expect("read config");

    assert!(config.contains("model_provider = \"aio\""), "{config}");
    assert!(
        config.contains(&format!("base_url = \"{base_origin}/v1\"")),
        "{config}"
    );
    assert!(config.contains("requires_openai_auth = true"), "{config}");
    assert!(!config.contains("preferred_auth_method"), "{config}");
    assert!(
        !auth_path.exists(),
        "oauth compatible proxy must not create auth.json at {}",
        auth_path.display()
    );

    let manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    assert!(manifest
        .files
        .iter()
        .any(|entry| entry.kind == "codex_config_toml"));
    assert!(
        !manifest
            .files
            .iter()
            .any(|entry| entry.kind == "codex_auth_json"),
        "oauth compatible manifest should not target auth.json: {manifest:?}"
    );
}

#[test]
fn enabling_codex_oauth_compatible_proxy_preserves_existing_auth_json() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    let config = r#"
preferred_auth_method = "apikey"

[existing]
foo = "bar"
"#;
    let auth = r#"{
  "tokens": { "access": "oauth-token" },
  "last_refresh": 123,
  "profile": "teacher"
}"#;
    write_codex_direct_files(&handle, config, auth);
    let auth_path = codex_auth_path(&handle).expect("auth path");
    let before_auth = std::fs::read_to_string(&auth_path).expect("read auth before");

    let result = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(result.ok, "{result:?}");

    let after_auth = std::fs::read_to_string(&auth_path).expect("read auth after");
    let config_path = codex_config_path(&handle).expect("config path");
    let after_config = std::fs::read_to_string(config_path).expect("read config after");

    assert_eq!(after_auth, before_auth);
    assert!(
        !after_config.contains("preferred_auth_method"),
        "{after_config}"
    );
    assert!(after_config.contains(&format!("base_url = \"{base_origin}/v1\"")));
}

#[test]
fn codex_oauth_compatible_status_uses_config_without_auth_json() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    write_cli_proxy_manifest(&handle, "codex", true, Some(base_origin));
    write_codex_oauth_proxy_config(&handle, base_origin);

    let rows = status_all(&handle, Some(base_origin)).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert!(codex.enabled);
    assert_eq!(codex.applied_to_current_gateway, Some(true));
}

#[test]
fn codex_oauth_compatible_status_reports_drift_when_old_apikey_preference_remains() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    write_cli_proxy_manifest(&handle, "codex", true, Some(base_origin));
    write_codex_proxy_files(&handle, base_origin);

    let rows = status_all(&handle, Some(base_origin)).expect("status_all");
    let codex = rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex row");

    assert_eq!(codex.applied_to_current_gateway, Some(false));
}

#[test]
fn disabling_codex_oauth_compatible_proxy_restores_config_and_leaves_auth_json() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    let config = r#"[existing]
foo = "bar"
"#;
    let auth = r#"{
  "tokens": { "access": "oauth-token" },
  "profile": "teacher"
}"#;
    write_codex_direct_files(&handle, config, auth);
    let auth_path = codex_auth_path(&handle).expect("auth path");

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(enabled.ok, "{enabled:?}");

    let user_changed_auth = r#"{
  "tokens": { "access": "new-oauth-token" },
  "profile": "teacher",
  "user_added": true
}"#;
    std::fs::write(&auth_path, user_changed_auth).expect("write auth user change");

    let disabled = set_enabled(&handle, "codex", false, base_origin).expect("disable codex");
    assert!(disabled.ok, "{disabled:?}");

    let config_after = std::fs::read_to_string(codex_config_path(&handle).expect("config path"))
        .expect("read config after disable");
    let auth_after = std::fs::read_to_string(auth_path).expect("read auth after disable");

    assert!(config_after.contains("[existing]"), "{config_after}");
    assert!(
        !config_after.contains("model_provider = \"aio\""),
        "{config_after}"
    );
    assert_eq!(auth_after, user_changed_auth);
}

#[test]
fn switching_codex_oauth_compatible_proxy_to_normal_mode_adds_auth_backup_and_writes_auth() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";
    set_codex_oauth_compatible_proxy_mode(&handle, true);

    let config = r#"[existing]
foo = "bar"
"#;
    let auth = r#"{
  "tokens": { "access": "oauth-token" },
  "last_refresh": 123,
  "profile": "teacher"
}"#;
    write_codex_direct_files(&handle, config, auth);
    let auth_path = codex_auth_path(&handle).expect("auth path");

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex oauth");
    assert!(enabled.ok, "{enabled:?}");

    let oauth_manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    assert!(
        !oauth_manifest
            .files
            .iter()
            .any(|entry| entry.kind == "codex_auth_json"),
        "oauth compatible manifest should not backup auth: {oauth_manifest:?}"
    );

    set_codex_oauth_compatible_proxy_mode(&handle, false);
    let sync_rows = sync_enabled(&handle, base_origin, true).expect("sync after mode switch");
    let codex_row = sync_rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex sync row");
    assert!(codex_row.ok, "{codex_row:?}");

    let config_after_sync =
        std::fs::read_to_string(codex_config_path(&handle).expect("config path"))
            .expect("read config after sync");
    assert!(
        config_after_sync.contains("preferred_auth_method = \"apikey\""),
        "{config_after_sync}"
    );

    let auth_after_sync = std::fs::read_to_string(&auth_path).expect("read auth after sync");
    let auth_after_sync_json: serde_json::Value =
        serde_json::from_str(&auth_after_sync).expect("parse auth after sync");
    assert_eq!(
        auth_after_sync_json
            .get("OPENAI_API_KEY")
            .and_then(|value| value.as_str()),
        Some(PLACEHOLDER_KEY)
    );
    assert_eq!(
        auth_after_sync_json
            .get("auth_mode")
            .and_then(|value| value.as_str()),
        Some("apikey")
    );
    assert!(auth_after_sync_json.get("tokens").is_none());

    let normal_manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    let auth_entry = manifest_entry(&normal_manifest, "codex_auth_json");
    assert!(auth_entry.existed);
    let root = cli_proxy_root_dir(&handle, "codex").expect("codex root");
    let backup_rel = auth_entry.backup_rel.as_ref().expect("auth backup rel");
    let auth_backup =
        std::fs::read_to_string(cli_proxy_files_dir(&root).join(backup_rel)).expect("read backup");
    let auth_backup_json: serde_json::Value =
        serde_json::from_str(&auth_backup).expect("parse auth backup");
    assert_eq!(
        auth_backup_json
            .get("tokens")
            .and_then(|tokens| tokens.get("access"))
            .and_then(|value| value.as_str()),
        Some("oauth-token")
    );

    let disabled = set_enabled(&handle, "codex", false, base_origin).expect("disable codex");
    assert!(disabled.ok, "{disabled:?}");

    let auth_after_disable = std::fs::read_to_string(auth_path).expect("read auth after disable");
    let auth_after_disable_json: serde_json::Value =
        serde_json::from_str(&auth_after_disable).expect("parse auth after disable");
    assert_eq!(
        auth_after_disable_json
            .get("tokens")
            .and_then(|tokens| tokens.get("access"))
            .and_then(|value| value.as_str()),
        Some("oauth-token")
    );
    assert!(auth_after_disable_json.get("OPENAI_API_KEY").is_none());
    assert!(auth_after_disable_json.get("auth_mode").is_none());
}

#[test]
fn sync_enabled_rebases_codex_manifest_when_codex_home_changes() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    let old_codex_home = app.home.path().join("codex-old");
    let new_codex_home = app.home.path().join("codex-new");

    let old_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://old.example/v1"

[old_section]
marker = "old"
"#;
    let old_auth = r#"{
  "tokens": { "access": "old-token" },
  "profile": "old"
}"#;
    let new_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://new.example/v1"

[new_section]
marker = "new"
"#;
    let new_auth = r#"{
  "tokens": { "access": "new-token" },
  "profile": "new"
}"#;

    set_custom_codex_home(&handle, &old_codex_home);
    write_codex_direct_files(&handle, old_config, old_auth);
    let old_config_path = codex_config_path(&handle).expect("old config path");
    let old_auth_path = codex_auth_path(&handle).expect("old auth path");

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(enabled.ok, "{enabled:?}");

    set_custom_codex_home(&handle, &new_codex_home);
    write_codex_direct_files(&handle, new_config, new_auth);
    let new_config_path = codex_config_path(&handle).expect("new config path");
    let new_auth_path = codex_auth_path(&handle).expect("new auth path");

    assert_ne!(old_config_path, new_config_path);
    assert_ne!(old_auth_path, new_auth_path);

    let sync_rows = sync_enabled(&handle, base_origin, false).expect("sync enabled");
    let codex_row = sync_rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex sync result");
    assert!(codex_row.ok, "{codex_row:?}");
    assert_eq!(codex_row.message, "已重绑 Codex 目录基线，待网关启动后接管");

    let manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    let config_entry = manifest_entry(&manifest, "codex_config_toml");
    let auth_entry = manifest_entry(&manifest, "codex_auth_json");

    assert_eq!(manifest.base_origin.as_deref(), Some(base_origin));
    assert_eq!(PathBuf::from(&config_entry.path), new_config_path);
    assert_eq!(PathBuf::from(&auth_entry.path), new_auth_path);

    let rebound_config = std::fs::read_to_string(&new_config_path).expect("read rebound config");
    let rebound_auth = std::fs::read_to_string(&new_auth_path).expect("read rebound auth");
    let rebound_auth_json: serde_json::Value =
        serde_json::from_str(&rebound_auth).expect("parse rebound auth");

    assert!(
        rebound_config.contains("[new_section]"),
        "offline rebind should keep direct config in target file: {rebound_config}"
    );
    assert!(
        !rebound_config.contains("model_provider = \"aio\""),
        "offline rebind should not rewrite target config to proxy: {rebound_config}"
    );
    assert_eq!(
        rebound_auth_json
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("new"),
        "offline rebind should keep direct auth in target file: {rebound_auth}"
    );
    assert!(
        rebound_auth_json.get("OPENAI_API_KEY").is_none(),
        "offline rebind should not inject proxy auth into target file: {rebound_auth}"
    );

    let root = cli_proxy_root_dir(&handle, "codex").expect("codex root");
    let files_dir = cli_proxy_files_dir(&root);
    let config_backup =
        std::fs::read_to_string(files_dir.join("config.toml")).expect("read config backup");
    let auth_backup =
        std::fs::read_to_string(files_dir.join("auth.json")).expect("read auth backup");
    let auth_backup_json: serde_json::Value =
        serde_json::from_str(&auth_backup).expect("parse auth backup");

    assert!(
        config_backup.contains("[new_section]"),
        "config backup should be rebound to new baseline: {config_backup}"
    );
    assert!(
        !config_backup.contains("[old_section]"),
        "config backup should stop using old baseline: {config_backup}"
    );
    assert_eq!(
        auth_backup_json
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("new"),
        "auth backup should be rebound to new baseline: {auth_backup}"
    );
}

#[test]
fn sync_enabled_rebinds_and_applies_proxy_when_apply_live_true() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    let old_codex_home = app.home.path().join("codex-old");
    let new_codex_home = app.home.path().join("codex-new");

    let old_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://old.example/v1"

[old_section]
marker = "old"
"#;
    let old_auth = r#"{
  "tokens": { "access": "old-token" },
  "profile": "old"
}"#;
    let new_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://new.example/v1"

[new_section]
marker = "new"
"#;
    let new_auth = r#"{
  "tokens": { "access": "new-token" },
  "profile": "new"
}"#;

    set_custom_codex_home(&handle, &old_codex_home);
    write_codex_direct_files(&handle, old_config, old_auth);

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(enabled.ok, "{enabled:?}");

    set_custom_codex_home(&handle, &new_codex_home);
    write_codex_direct_files(&handle, new_config, new_auth);
    let new_config_path = codex_config_path(&handle).expect("new config path");
    let new_auth_path = codex_auth_path(&handle).expect("new auth path");

    let sync_rows = sync_enabled(&handle, base_origin, true).expect("sync enabled");
    let codex_row = sync_rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex sync result");
    assert!(codex_row.ok, "{codex_row:?}");
    assert_eq!(codex_row.message, "已重绑 Codex 目录并写入当前网关配置");

    let rebound_config = std::fs::read_to_string(&new_config_path).expect("read rebound config");
    let rebound_auth = std::fs::read_to_string(&new_auth_path).expect("read rebound auth");
    let rebound_auth_json: serde_json::Value =
        serde_json::from_str(&rebound_auth).expect("parse rebound auth");

    assert!(
        rebound_config.contains("model_provider = \"aio\""),
        "live rebind should rewrite target config to proxy: {rebound_config}"
    );
    assert!(
        rebound_config.contains(&format!("{base_origin}/v1")),
        "live rebind should point target config to current gateway: {rebound_config}"
    );
    assert_eq!(
        rebound_auth_json
            .get("OPENAI_API_KEY")
            .and_then(|value| value.as_str()),
        Some("aio-coding-hub"),
        "live rebind should inject proxy auth into target file: {rebound_auth}"
    );
    assert_eq!(
        rebound_auth_json
            .get("auth_mode")
            .and_then(|value| value.as_str()),
        Some("apikey"),
        "live rebind should mark auth mode for gateway auth: {rebound_auth}"
    );

    let manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    assert_eq!(manifest.base_origin.as_deref(), Some(base_origin));
    assert_eq!(
        PathBuf::from(&manifest_entry(&manifest, "codex_config_toml").path),
        new_config_path
    );
    assert_eq!(
        PathBuf::from(&manifest_entry(&manifest, "codex_auth_json").path),
        new_auth_path
    );
}

#[test]
fn disabling_codex_after_rebind_restores_new_target_path_only() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    let old_codex_home = app.home.path().join("codex-old");
    let new_codex_home = app.home.path().join("codex-new");

    let old_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://old.example/v1"

[old_section]
marker = "old"
"#;
    let old_auth = r#"{
  "tokens": { "access": "old-token" },
  "profile": "old"
}"#;
    let new_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://new.example/v1"

[new_section]
marker = "new"
"#;
    let new_auth = r#"{
  "tokens": { "access": "new-token" },
  "profile": "new"
}"#;

    set_custom_codex_home(&handle, &old_codex_home);
    write_codex_direct_files(&handle, old_config, old_auth);
    let old_config_path = codex_config_path(&handle).expect("old config path");
    let old_auth_path = codex_auth_path(&handle).expect("old auth path");

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(enabled.ok, "{enabled:?}");

    set_custom_codex_home(&handle, &new_codex_home);
    write_codex_direct_files(&handle, new_config, new_auth);
    let new_config_path = codex_config_path(&handle).expect("new config path");
    let new_auth_path = codex_auth_path(&handle).expect("new auth path");

    let sync_rows = sync_enabled(&handle, base_origin, false).expect("sync enabled");
    let codex_row = sync_rows
        .into_iter()
        .find(|row| row.cli_key == "codex")
        .expect("codex sync result");
    assert!(codex_row.ok, "{codex_row:?}");
    assert_eq!(codex_row.message, "已重绑 Codex 目录基线，待网关启动后接管");

    let old_config_before_disable =
        std::fs::read_to_string(&old_config_path).expect("read old config before disable");
    let old_auth_before_disable =
        std::fs::read_to_string(&old_auth_path).expect("read old auth before disable");

    let disabled = set_enabled(&handle, "codex", false, base_origin).expect("disable codex");
    assert!(disabled.ok, "{disabled:?}");

    let old_config_after_disable =
        std::fs::read_to_string(&old_config_path).expect("read old config after disable");
    let old_auth_after_disable =
        std::fs::read_to_string(&old_auth_path).expect("read old auth after disable");
    let new_config_after_disable =
        std::fs::read_to_string(&new_config_path).expect("read new config after disable");
    let new_auth_after_disable =
        std::fs::read_to_string(&new_auth_path).expect("read new auth after disable");
    let new_auth_json: serde_json::Value =
        serde_json::from_str(&new_auth_after_disable).expect("parse new auth after disable");

    assert_eq!(
        old_config_after_disable, old_config_before_disable,
        "old codex_home config should stay untouched after rebind disable"
    );
    assert_eq!(
        old_auth_after_disable, old_auth_before_disable,
        "old codex_home auth should stay untouched after rebind disable"
    );

    assert!(
        new_config_after_disable.contains("[new_section]"),
        "new codex_home config should restore new baseline: {new_config_after_disable}"
    );
    assert!(
        !new_config_after_disable.contains("model_provider = \"aio\""),
        "new codex_home config should no longer point to proxy: {new_config_after_disable}"
    );
    assert_eq!(
        new_auth_json
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("new"),
        "new codex_home auth should restore new baseline: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("tokens").is_some(),
        "new codex_home auth should restore direct tokens: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("OPENAI_API_KEY").is_none(),
        "new codex_home auth should remove proxy API key: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("auth_mode").is_none(),
        "new codex_home auth should remove proxy auth mode: {new_auth_after_disable}"
    );
}

#[test]
fn rebind_codex_home_adopts_existing_proxy_target_and_disable_restores_new_target_path() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    let old_codex_home = app.home.path().join("codex-old");
    let new_codex_home = app.home.path().join("codex-new");

    let old_config = r#"[model_providers.openai]
name = "openai"
base_url = "https://old.example/v1"

[old_section]
marker = "old"
"#;
    let old_auth = r#"{
  "tokens": { "access": "old-token" },
  "profile": "old"
}"#;

    set_custom_codex_home(&handle, &old_codex_home);
    write_codex_direct_files(&handle, old_config, old_auth);
    let old_config_path = codex_config_path(&handle).expect("old config path");
    let old_auth_path = codex_auth_path(&handle).expect("old auth path");

    let enabled = set_enabled(&handle, "codex", true, base_origin).expect("enable codex");
    assert!(enabled.ok, "{enabled:?}");

    let old_proxy_config_before_rebind =
        std::fs::read_to_string(&old_config_path).expect("read old proxy config");
    let old_proxy_auth_before_rebind =
        std::fs::read_to_string(&old_auth_path).expect("read old proxy auth");

    let root = cli_proxy_root_dir(&handle, "codex").expect("codex root");
    let files_dir = cli_proxy_files_dir(&root);
    let config_backup_before_rebind =
        std::fs::read_to_string(files_dir.join("config.toml")).expect("read config backup");
    let auth_backup_before_rebind =
        std::fs::read_to_string(files_dir.join("auth.json")).expect("read auth backup");

    set_custom_codex_home(&handle, &new_codex_home);
    write_codex_proxy_files(&handle, base_origin);
    let new_config_path = codex_config_path(&handle).expect("new config path");
    let new_auth_path = codex_auth_path(&handle).expect("new auth path");

    let rebound = rebind_codex_home_after_change(&handle, base_origin, true).expect("rebind");
    assert!(rebound.ok, "{rebound:?}");
    assert_eq!(rebound.message, "已重绑 Codex 目录并写入当前网关配置");

    let manifest = read_manifest(&handle, "codex")
        .expect("read manifest")
        .expect("manifest exists");
    assert_eq!(manifest.base_origin.as_deref(), Some(base_origin));
    assert_eq!(
        PathBuf::from(&manifest_entry(&manifest, "codex_config_toml").path),
        new_config_path
    );
    assert_eq!(
        PathBuf::from(&manifest_entry(&manifest, "codex_auth_json").path),
        new_auth_path
    );

    let config_backup_after_rebind =
        std::fs::read_to_string(files_dir.join("config.toml")).expect("read config backup");
    let auth_backup_after_rebind =
        std::fs::read_to_string(files_dir.join("auth.json")).expect("read auth backup");
    assert_eq!(
        config_backup_after_rebind, config_backup_before_rebind,
        "adopting an existing proxy target must keep the original direct config backup"
    );
    assert_eq!(
        auth_backup_after_rebind, auth_backup_before_rebind,
        "adopting an existing proxy target must keep the original direct auth backup"
    );

    let disabled = set_enabled(&handle, "codex", false, base_origin).expect("disable codex");
    assert!(disabled.ok, "{disabled:?}");

    let old_config_after_disable =
        std::fs::read_to_string(&old_config_path).expect("read old config after disable");
    let old_auth_after_disable =
        std::fs::read_to_string(&old_auth_path).expect("read old auth after disable");
    let new_config_after_disable =
        std::fs::read_to_string(&new_config_path).expect("read new config after disable");
    let new_auth_after_disable =
        std::fs::read_to_string(&new_auth_path).expect("read new auth after disable");
    let new_auth_json: serde_json::Value =
        serde_json::from_str(&new_auth_after_disable).expect("parse new auth after disable");

    assert_eq!(
        old_config_after_disable, old_proxy_config_before_rebind,
        "old codex_home config should remain untouched after adopt + disable"
    );
    assert_eq!(
        old_auth_after_disable, old_proxy_auth_before_rebind,
        "old codex_home auth should remain untouched after adopt + disable"
    );
    assert!(
        new_config_after_disable.contains("[old_section]"),
        "new codex_home config should restore the original direct baseline: {new_config_after_disable}"
    );
    assert!(
        !new_config_after_disable.contains("model_provider = \"aio\""),
        "new codex_home config should no longer point to proxy after disable: {new_config_after_disable}"
    );
    assert_eq!(
        new_auth_json
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("old"),
        "new codex_home auth should restore the original direct baseline: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("tokens").is_some(),
        "new codex_home auth should restore direct tokens: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("OPENAI_API_KEY").is_none(),
        "new codex_home auth should remove proxy API key after disable: {new_auth_after_disable}"
    );
    assert!(
        new_auth_json.get("auth_mode").is_none(),
        "new codex_home auth should remove proxy auth mode after disable: {new_auth_after_disable}"
    );
}

#[test]
fn claude_proxy_settings_json_rejects_invalid_json() {
    let input = br#"{"env": "#.to_vec();
    let err = build_claude_settings_json(Some(input), "http://127.0.0.1:1717/claude")
        .expect_err("must fail");
    assert!(err.to_string().contains("CLI_PROXY_INVALID_SETTINGS_JSON"));
}

// ── merge-restore tests ─────────────────────────────────────────────────────

fn write_temp(dir: &std::path::Path, name: &str, content: &[u8]) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, content).unwrap();
    p
}

#[test]
fn merge_restore_claude_preserves_user_changes() {
    let tmp = tempfile::tempdir().unwrap();

    // Backup: original file before proxy was enabled
    let backup = write_temp(
        tmp.path(),
        "backup.json",
        br#"{ "model": "opus", "permissions": { "allow": ["Read"] } }"#,
    );

    // Current: user added "language" and proxy injected env keys
    let target = write_temp(
        tmp.path(),
        "settings.json",
        br#"{
  "model": "opus",
  "language": "zh-CN",
  "permissions": { "allow": ["Read", "Write"] },
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:37123/claude",
    "ANTHROPIC_AUTH_TOKEN": "aio-coding-hub",
    "MCP_TIMEOUT": "30000"
  }
}"#,
    );

    merge_restore_claude_settings_json(&target, &backup).unwrap();

    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();

    // User's added "language" is preserved
    assert_eq!(result.get("language").unwrap().as_str(), Some("zh-CN"));
    // User's updated permissions are preserved
    let allow = result["permissions"]["allow"].as_array().unwrap();
    assert_eq!(allow.len(), 2);
    // Proxy keys are removed (backup didn't have them)
    let env = result.get("env").unwrap().as_object().unwrap();
    assert!(!env.contains_key("ANTHROPIC_BASE_URL"));
    assert!(!env.contains_key("ANTHROPIC_AUTH_TOKEN"));
    // User's other env keys are preserved
    assert_eq!(env.get("MCP_TIMEOUT").unwrap().as_str(), Some("30000"));
}

#[test]
fn merge_restore_claude_restores_original_env_keys() {
    let tmp = tempfile::tempdir().unwrap();

    // Backup: had original ANTHROPIC_BASE_URL
    let backup = write_temp(
        tmp.path(),
        "backup.json",
        br#"{ "env": { "ANTHROPIC_BASE_URL": "https://api.anthropic.com" } }"#,
    );

    // Current: proxy replaced the URL
    let target = write_temp(
        tmp.path(),
        "settings.json",
        br#"{ "env": { "ANTHROPIC_BASE_URL": "http://127.0.0.1:37123/claude", "ANTHROPIC_AUTH_TOKEN": "aio" } }"#,
    );

    merge_restore_claude_settings_json(&target, &backup).unwrap();

    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    let env = result.get("env").unwrap().as_object().unwrap();
    assert_eq!(
        env.get("ANTHROPIC_BASE_URL").unwrap().as_str(),
        Some("https://api.anthropic.com")
    );
    assert!(!env.contains_key("ANTHROPIC_AUTH_TOKEN"));
}

#[test]
fn merge_restore_codex_auth_preserves_user_changes() {
    let tmp = tempfile::tempdir().unwrap();

    // Backup: had OAuth tokens
    let backup = write_temp(
        tmp.path(),
        "backup.json",
        br#"{ "tokens": { "access": "tok-123" }, "last_refresh": 1234, "custom_key": "keep" }"#,
    );

    // Current: proxy replaced auth, user added a new key
    let target = write_temp(
        tmp.path(),
        "auth.json",
        br#"{ "OPENAI_API_KEY": "aio-coding-hub", "auth_mode": "apikey", "user_added": "hello" }"#,
    );

    merge_restore_codex_auth_json(&target, &backup).unwrap();

    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    // Proxy keys removed
    assert!(result.get("OPENAI_API_KEY").is_none());
    assert!(result.get("auth_mode").is_none());
    // OAuth tokens restored from backup
    assert!(result.get("tokens").is_some());
    assert!(result.get("last_refresh").is_some());
    // User's addition preserved
    assert_eq!(result.get("user_added").unwrap().as_str(), Some("hello"));
}

#[test]
fn merge_restore_gemini_env_preserves_user_changes() {
    let tmp = tempfile::tempdir().unwrap();

    // Backup: had original API key
    let backup = write_temp(
        tmp.path(),
        "backup.env",
        b"GEMINI_API_KEY=original-key\nCUSTOM_VAR=keep\n",
    );

    // Current: proxy replaced keys, user added new var
    let target = write_temp(
        tmp.path(),
        ".env",
        b"GOOGLE_GEMINI_BASE_URL=http://127.0.0.1:37123/gemini\nGEMINI_API_KEY=aio-coding-hub\nUSER_VAR=hello\n",
    );

    merge_restore_gemini_env(&target, &backup).unwrap();

    let result = std::fs::read_to_string(&target).unwrap();
    // Proxy base URL removed (backup didn't have it)
    assert!(!result.contains("GOOGLE_GEMINI_BASE_URL"));
    // Original API key restored
    assert!(result.contains("GEMINI_API_KEY=original-key"));
    // User's addition preserved
    assert!(result.contains("USER_VAR=hello"));
}

#[test]
fn merge_restore_codex_config_preserves_user_changes() {
    let tmp = tempfile::tempdir().unwrap();

    // Backup: no proxy config, just user settings
    let backup = write_temp(
        tmp.path(),
        "backup.toml",
        b"[model_providers.openai]\nname = \"openai\"\nbase_url = \"https://api.openai.com/v1\"\n",
    );

    // Current: proxy added its config, user added a new section
    let target = write_temp(
        tmp.path(),
        "config.toml",
        b"model_provider = \"aio\"\npreferred_auth_method = \"apikey\"\n\n[model_providers.openai]\nname = \"openai\"\nbase_url = \"https://api.openai.com/v1\"\n\n[model_providers.aio]\nname = \"aio\"\nbase_url = \"http://127.0.0.1:37123/v1\"\nwire_api = \"responses\"\nrequires_openai_auth = true\n\n[user_section]\nfoo = \"bar\"\n\n[windows]\nsandbox = \"elevated\"\n",
    );

    merge_restore_codex_config_toml(&target, &backup).unwrap();

    let result = std::fs::read_to_string(&target).unwrap();
    // Proxy root keys removed (check for the root-level assignment, not table names)
    assert!(
        !result.contains("model_provider = \"aio\""),
        "model_provider root key should be removed: {result}"
    );
    assert!(
        !result.contains("preferred_auth_method"),
        "preferred_auth_method should be removed: {result}"
    );
    // Proxy provider section removed
    assert!(!result.contains("[model_providers.aio]"));
    // Proxy windows sandbox removed
    assert!(!result.contains("[windows]"));
    assert!(!result.contains("sandbox"));
    // User's openai section preserved
    assert!(result.contains("[model_providers.openai]"));
    assert!(result.contains("base_url = \"https://api.openai.com/v1\""));
    // User's custom section preserved
    assert!(result.contains("[user_section]"));
    assert!(result.contains("foo = \"bar\""));
}

/// Simulates the app lifecycle that causes the "修复" button to appear:
///
/// 1. Enable proxy → config files point to gateway
/// 2. `restore_enabled_keep_state` (exit cleanup) → config files restored to direct mode,
///    but manifest still has `enabled = true`
/// 3. `status_all` with the same gateway origin → `applied_to_current_gateway == false`
///    (this is the drifted state that shows the "修复" button)
/// 4. `sync_enabled` (the fix) → config files re-applied
/// 5. `status_all` → `applied_to_current_gateway == true` (drift resolved)
#[test]
fn sync_enabled_resolves_drift_after_restore_enabled_keep_state() {
    let app = CliProxyTestApp::new();
    let handle = app.handle();
    let base_origin = "http://127.0.0.1:37123";

    // Step 1: Enable the codex proxy so config files point to the gateway.
    let enable_result = set_enabled(&handle, "codex", true, base_origin).expect("enable");
    assert!(enable_result.ok, "enable should succeed: {enable_result:?}");

    // Verify proxy is applied.
    let rows = status_all(&handle, Some(base_origin)).expect("status_all after enable");
    let codex = rows.iter().find(|r| r.cli_key == "codex").expect("codex");
    assert_eq!(codex.applied_to_current_gateway, Some(true));

    // Step 2: Simulate exit cleanup — restores config to direct mode, keeps enabled=true.
    let restore_results = restore_enabled_keep_state(&handle).expect("restore");
    let codex_restore = restore_results
        .iter()
        .find(|r| r.cli_key == "codex")
        .expect("codex restore");
    assert!(
        codex_restore.ok,
        "restore should succeed: {codex_restore:?}"
    );

    // Step 3: After restore, status_all should report drift (the bug scenario).
    let rows = status_all(&handle, Some(base_origin)).expect("status_all after restore");
    let codex = rows.iter().find(|r| r.cli_key == "codex").expect("codex");
    assert!(codex.enabled, "manifest should still be enabled");
    assert_eq!(
        codex.applied_to_current_gateway,
        Some(false),
        "config was restored to direct mode, so drift is expected"
    );

    // Step 4: sync_enabled (the fix applied at autostart) should re-apply proxy config.
    let sync_results = sync_enabled(&handle, base_origin, true).expect("sync");
    let codex_sync = sync_results
        .iter()
        .find(|r| r.cli_key == "codex")
        .expect("codex sync");
    assert!(codex_sync.ok, "sync should succeed: {codex_sync:?}");

    // Step 5: Drift should now be resolved.
    let rows = status_all(&handle, Some(base_origin)).expect("status_all after sync");
    let codex = rows.iter().find(|r| r.cli_key == "codex").expect("codex");
    assert!(codex.enabled);
    assert_eq!(
        codex.applied_to_current_gateway,
        Some(true),
        "sync_enabled should resolve the drift"
    );
}
