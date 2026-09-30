//! Usage: Apply committed config imports to the gateway and CLI proxy configurations.

use crate::infra::config_migrate::{self, ConfigBundle, ConfigImportResult};
use crate::shared::error::AppResult;
use crate::{blocking, db, settings};
use tauri::Manager;

pub(crate) async fn config_import(
    app: tauri::AppHandle,
    db: db::Db,
    bundle: ConfigBundle,
) -> AppResult<ConfigImportResult> {
    let gateway_lifecycle = super::gateway_lifecycle_lock::lock().await;
    let result = blocking::run("config_import", {
        let app = app.clone();
        move || import_config_unlocked(&app, &db, bundle)
    })
    .await?;
    drop(gateway_lifecycle);

    #[cfg(windows)]
    let result = {
        let mut result = result;
        if let Err(err) = crate::commands::wsl::wsl_auto_sync_core(&app).await {
            tracing::warn!(error = %err, "config import committed but WSL sync failed");
            result
                .warnings
                .push("WSL 配置同步失败，请在 CLI 管理中重试".to_string());
        }
        result
    };

    Ok(result)
}

// Caller holds GatewayLifecycleLock. An import error leaves the previous runtime
// untouched; sync errors after commit are warnings, not a claimed DB rollback.
pub(crate) fn import_config_unlocked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    bundle: ConfigBundle,
) -> AppResult<ConfigImportResult> {
    let mut result = config_migrate::config_import(app, db, bundle)?;
    let cfg = match settings::read(app) {
        Ok(cfg) => cfg,
        Err(err) => {
            tracing::warn!(error = %err, "config import committed but settings reload failed");
            super::gateway_control::try_app_gateway_set_responses_websocket_enabled(app, false);
            result
                .warnings
                .push("配置已保存，但运行态同步失败，请重启应用".to_string());
            return Ok(result);
        }
    };

    super::gateway_control::try_app_gateway_set_responses_websocket_enabled(
        app,
        cfg.codex_responses_websocket_enabled,
    );
    if app
        .try_state::<super::gateway_state::GatewayState>()
        .is_some()
    {
        // Provider IDs and credentials can change even if the global WS flag did not.
        for cli in ["codex", "claude", "gemini", "grok"] {
            super::gateway_control::app_gateway_clear_cli_route_runtime_state(app, cli);
        }
    }

    let status = super::gateway_runtime_access::try_app_gateway_status(app).unwrap_or_default();
    let origin = status
        .base_url
        .filter(|_| status.running)
        .map(Ok)
        .unwrap_or_else(|| crate::gateway::planned_base_url(&cfg));
    let sync = origin.and_then(|origin| {
        crate::cli_proxy::sync_enabled(app, &origin, status.running)
            .and_then(super::cli_proxy_service::require_success)
    });
    if let Err(err) = sync {
        tracing::warn!(error = %err, "config import committed but CLI proxy sync failed");
        result
            .warnings
            .push("本机客户端配置同步失败，请在 CLI 管理中重试".to_string());
    }
    super::heartbeat_watchdog::gated_emit(
        app,
        crate::gateway::events::GATEWAY_STATUS_EVENT_NAME,
        super::gateway_runtime_access::try_app_gateway_status(app).unwrap_or_default(),
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::MutexGuard;

    struct TestApp {
        app: tauri::App<tauri::test::MockRuntime>,
        db: db::Db,
        _home: tempfile::TempDir,
        _runtime: tokio::runtime::Runtime,
        previous_env: Vec<(&'static str, Option<OsString>)>,
        _lock: MutexGuard<'static, ()>,
    }

    impl TestApp {
        fn new() -> Self {
            let lock = crate::test_support::test_env_lock();
            let home = tempfile::tempdir().unwrap();
            let previous_env = ["AIO_CODING_HUB_HOME_DIR", "AIO_CODING_HUB_DOTDIR_NAME"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect();
            std::env::set_var("AIO_CODING_HUB_HOME_DIR", home.path());
            std::env::set_var("AIO_CODING_HUB_DOTDIR_NAME", ".aio-config-runtime-test");
            crate::test_support::clear_settings_cache();
            let app = tauri::test::mock_app();
            app.manage(crate::resident::ResidentState::default());
            app.manage(crate::app::gateway_state::GatewayState::default());
            let db = crate::db::init(app.handle()).unwrap();
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let running = crate::gateway::runtime::GatewayRuntime::for_app_tests(&runtime);
            running.set_responses_websocket_enabled(true);
            crate::app::gateway_state::with_app_running_gateway_slot_mut(app.handle(), |slot| {
                *slot = Some(running);
            });
            settings::write(
                app.handle(),
                &settings::AppSettings {
                    codex_responses_websocket_enabled: true,
                    ..Default::default()
                },
            )
            .unwrap();
            Self {
                app,
                db,
                _home: home,
                _runtime: runtime,
                previous_env,
                _lock: lock,
            }
        }

        fn snapshot(&self) -> (bool, u64) {
            crate::app::gateway_state::with_app_running_gateway(self.app.handle(), |running| {
                running.unwrap().responses_websocket_snapshot_for_tests()
            })
        }

        fn bundle(&self, enabled: bool) -> ConfigBundle {
            let mut bundle = config_migrate::config_export(self.app.handle(), &self.db).unwrap();
            let mut cfg = settings::read(self.app.handle()).unwrap();
            cfg.codex_responses_websocket_enabled = enabled;
            bundle.settings = serde_json::to_string(&cfg).unwrap();
            bundle
        }
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            for (key, value) in self.previous_env.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            crate::test_support::clear_settings_cache();
        }
    }

    #[test]
    fn import_disables_live_ws_and_invalidates_existing_generation() {
        let test = TestApp::new();
        let before = test.snapshot();
        let result =
            import_config_unlocked(test.app.handle(), &test.db, test.bundle(false)).unwrap();
        assert!(result.warnings.is_empty());
        assert!(!test.snapshot().0);
        assert!(test.snapshot().1 > before.1);
        assert!(
            !settings::read(test.app.handle())
                .unwrap()
                .codex_responses_websocket_enabled
        );
    }

    #[test]
    fn import_invalidates_generations_even_when_ws_setting_is_unchanged() {
        let test = TestApp::new();
        let before = test.snapshot();
        import_config_unlocked(test.app.handle(), &test.db, test.bundle(true)).unwrap();
        assert!(test.snapshot().0);
        assert!(test.snapshot().1 > before.1);
    }

    #[test]
    fn rejected_import_preserves_settings_and_runtime_generation() {
        let test = TestApp::new();
        let before = test.snapshot();
        let mut bundle = test.bundle(false);
        bundle.schema_version = 0;
        assert!(import_config_unlocked(test.app.handle(), &test.db, bundle).is_err());
        assert_eq!(test.snapshot(), before);
        assert!(
            settings::read(test.app.handle())
                .unwrap()
                .codex_responses_websocket_enabled
        );
    }

    #[test]
    fn failed_import_after_settings_write_restores_settings_and_keeps_runtime() {
        let test = TestApp::new();
        let before = test.snapshot();
        let path = crate::grok_config::config_path(test.app.handle()).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[mcp_servers\ninvalid = true\n").unwrap();
        assert!(import_config_unlocked(test.app.handle(), &test.db, test.bundle(false)).is_err());
        assert_eq!(test.snapshot(), before);
        assert!(
            settings::read(test.app.handle())
                .unwrap()
                .codex_responses_websocket_enabled
        );
    }

    #[test]
    fn committed_import_reports_native_sync_failure_without_claiming_rollback() {
        let test = TestApp::new();
        let manifest = crate::app_paths::app_data_dir(test.app.handle())
            .unwrap()
            .join("cli-proxy/codex/manifest.json");
        std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        std::fs::write(&manifest, b"not json").unwrap();
        let result =
            import_config_unlocked(test.app.handle(), &test.db, test.bundle(false)).unwrap();
        assert_eq!(
            result.warnings,
            ["本机客户端配置同步失败，请在 CLI 管理中重试"]
        );
        assert!(!test.snapshot().0);
        assert!(
            !settings::read(test.app.handle())
                .unwrap()
                .codex_responses_websocket_enabled
        );
    }

    #[test]
    fn import_syncs_managed_codex_capability_and_preserves_unknown_fields() {
        let test = TestApp::new();
        let config_path = crate::codex_paths::codex_config_toml_path(test.app.handle()).unwrap();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, "[model_providers.aio]\nunknown = 'keep'\n").unwrap();
        let enabled =
            crate::cli_proxy::set_enabled(test.app.handle(), "codex", true, "http://127.0.0.1:1")
                .unwrap();
        assert!(enabled.ok, "{}", enabled.message);
        let before: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            before["model_providers"]["aio"]["supports_websockets"].as_bool(),
            Some(true)
        );
        let result =
            import_config_unlocked(test.app.handle(), &test.db, test.bundle(false)).unwrap();
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        let after: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            after["model_providers"]["aio"]["supports_websockets"].as_bool(),
            Some(false)
        );
        assert_eq!(
            after["model_providers"]["aio"]["unknown"].as_str(),
            Some("keep")
        );
    }
}
