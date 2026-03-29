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
    _lock: MutexGuard<'static, ()>,
    _env: EnvRestore,
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

fn set_custom_codex_home<R: tauri::Runtime>(app: &tauri::AppHandle<R>, codex_home: &Path) {
    let settings = AppSettings {
        codex_home_mode: CodexHomeMode::Custom,
        codex_home_override: codex_home.display().to_string(),
        ..AppSettings::default()
    };
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
