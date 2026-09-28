use super::*;

struct TestHome {
    previous: Option<std::ffi::OsString>,
    _home: tempfile::TempDir,
}

impl TestHome {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("AIO_CODING_HUB_TEST_HOME");
        std::env::set_var("AIO_CODING_HUB_TEST_HOME", home.path());
        crate::test_support::clear_settings_cache();
        Self {
            previous,
            _home: home,
        }
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("AIO_CODING_HUB_TEST_HOME", value),
            None => std::env::remove_var("AIO_CODING_HUB_TEST_HOME"),
        }
        crate::test_support::clear_settings_cache();
    }
}

fn claude_applied(app: &tauri::AppHandle<tauri::test::MockRuntime>, origin: &str) -> bool {
    cli_proxy::status_all(app, Some(origin))
        .unwrap()
        .into_iter()
        .find(|row| row.cli_key == "claude")
        .unwrap()
        .applied_to_current_gateway
        == Some(true)
}

// Keep the guard outside block_on so shutdown also runs after a failed assertion.
struct GatewayCleanup(tauri::AppHandle<tauri::test::MockRuntime>);

impl Drop for GatewayCleanup {
    fn drop(&mut self) {
        tauri::async_runtime::block_on(super::super::cleanup::stop_gateway_best_effort_unlocked(
            &self.0,
        ));
    }
}

fn lifecycle_app() -> tauri::App<tauri::test::MockRuntime> {
    use tauri::Manager;
    let app = tauri::test::mock_app();
    app.manage(super::super::gateway_state::GatewayState::default());
    app.manage(DbInitState::default());
    app
}

fn available_low_port() -> u16 {
    (10000..30000)
        .find(|port| std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .expect("available unprivileged port below default")
}

async fn assert_live_gateway(status: &crate::gateway::GatewayStatus) {
    assert!(status.running);
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("{}/health", status.base_url.as_ref().unwrap()))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
}

fn settings_update(port: u16) -> super::super::settings_service::SettingsUpdate {
    serde_json::from_value(serde_json::json!({
        "preferredPort": port,
        "autoStart": false,
        "logRetentionDays": 7,
        "failoverMaxAttemptsPerProvider": 2,
        "failoverMaxProvidersToTry": 3,
    }))
    .unwrap()
}

#[test]
fn cli_disable_waits_for_gateway_lifecycle_transaction() {
    let _lock = crate::test_support::test_env_lock();
    let _home = TestHome::new();
    // Keep the process-wide test home lock outside the async future.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let app = tauri::test::mock_app();
            let app = app.handle();
            let origin = "http://127.0.0.1:37123";
            cli_proxy::set_enabled(app, "claude", true, origin).unwrap();

            let lifecycle = super::super::gateway_lifecycle_lock::lock().await;
            let app_for_disable = app.clone();
            let mut disable = tokio::spawn(async move {
                super::super::cli_proxy_service::cli_proxy_set_disabled_impl(
                    app_for_disable,
                    None,
                    "claude".into(),
                )
                .await
            });
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(20), &mut disable)
                    .await
                    .is_err()
            );
            assert!(claude_applied(app, origin));
            drop(lifecycle);
            assert!(disable.await.unwrap().unwrap().ok);
            assert!(!claude_applied(app, origin));
        });
}

#[test]
fn implicit_start_respects_low_port_and_cli_disable_keeps_listener() {
    use tauri::Manager;
    let _lock = crate::test_support::test_env_lock();
    let _home = TestHome::new();
    let app = lifecycle_app();
    let app = app.handle();
    let _cleanup = GatewayCleanup(app.clone());
    let preferred_port = available_low_port();
    crate::settings::write(
        app,
        &crate::settings::AppSettings {
            preferred_port,
            ..Default::default()
        },
    )
    .unwrap();
    let origin = format!("http://127.0.0.1:{preferred_port}");
    assert!(
        cli_proxy::set_enabled(app, "claude", true, &origin)
            .unwrap()
            .ok
    );

    tauri::async_runtime::block_on(async {
        let state = app.state::<DbInitState>();
        let enabled = cli_proxy_set_enabled_impl(app.clone(), state.inner(), "claude".into(), true)
            .await
            .unwrap();
        assert!(enabled.ok);
        let status = super::super::gateway_runtime_access::app_gateway_status(app);
        assert_eq!(status.port, Some(preferred_port));
        assert_eq!(status.base_url.as_deref(), Some(origin.as_str()));
        assert_eq!(
            crate::settings::read(app).unwrap().preferred_port,
            preferred_port
        );
        assert!(claude_applied(app, &origin));
        assert_live_gateway(&status).await;

        let disabled =
            cli_proxy_set_enabled_impl(app.clone(), state.inner(), "claude".into(), false)
                .await
                .unwrap();
        assert!(disabled.ok);
        assert!(!claude_applied(app, &origin));
        assert_live_gateway(&super::super::gateway_runtime_access::app_gateway_status(
            app,
        ))
        .await;
    });
}

#[test]
fn occupied_preferred_port_syncs_cli_to_final_listener() {
    use tauri::Manager;
    let _lock = crate::test_support::test_env_lock();
    let _home = TestHome::new();
    let app = lifecycle_app();
    let app = app.handle();
    let _cleanup = GatewayCleanup(app.clone());
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let preferred_port = occupied.local_addr().unwrap().port();
    crate::settings::write(
        app,
        &crate::settings::AppSettings {
            preferred_port,
            ..Default::default()
        },
    )
    .unwrap();
    let original_origin = format!("http://127.0.0.1:{preferred_port}");
    assert!(
        cli_proxy::set_enabled(app, "claude", true, &original_origin)
            .unwrap()
            .ok
    );
    tauri::async_runtime::block_on(async {
        let state = app.state::<DbInitState>();
        let enabled = cli_proxy_set_enabled_impl(app.clone(), state.inner(), "claude".into(), true)
            .await
            .unwrap();
        assert!(enabled.ok);
        let status = super::super::gateway_runtime_access::app_gateway_status(app);
        assert_ne!(status.port, Some(preferred_port));
        assert_eq!(
            Some(crate::settings::read(app).unwrap().preferred_port),
            status.port
        );
        assert!(claude_applied(app, status.base_url.as_deref().unwrap()));
        assert_live_gateway(&status).await;
    });
}

#[test]
fn settings_transaction_restores_listener_after_failed_rebind_then_syncs_successful_rebind() {
    use tauri::Manager;
    let _lock = crate::test_support::test_env_lock();
    let _home = TestHome::new();
    let app = lifecycle_app();
    let app = app.handle();
    let _cleanup = GatewayCleanup(app.clone());
    let preferred_port = available_low_port();
    crate::settings::write(
        app,
        &crate::settings::AppSettings {
            preferred_port,
            ..Default::default()
        },
    )
    .unwrap();
    let original_origin = format!("http://127.0.0.1:{preferred_port}");
    assert!(
        cli_proxy::set_enabled(app, "claude", true, &original_origin)
            .unwrap()
            .ok
    );
    tauri::async_runtime::block_on(async {
        let state = app.state::<DbInitState>();
        cli_proxy_set_enabled_impl(app.clone(), state.inner(), "claude".into(), true)
            .await
            .unwrap();
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut update = settings_update(preferred_port);
        update.gateway_listen_mode = Some(crate::settings::GatewayListenMode::Custom);
        update.gateway_custom_listen_address = Some(occupied.local_addr().unwrap().to_string());
        let error = super::super::settings_service::apply_settings_update(
            app.clone(),
            state.inner(),
            update,
        )
        .await
        .unwrap_err();
        assert!(error.contains("重绑失败"), "{error}");
        let status = super::super::gateway_runtime_access::app_gateway_status(app);
        assert_eq!(status.base_url.as_deref(), Some(original_origin.as_str()));
        let settings = crate::settings::read(app).unwrap();
        assert_eq!(settings.preferred_port, preferred_port);
        assert_eq!(
            settings.gateway_listen_mode,
            crate::settings::GatewayListenMode::Localhost
        );
        assert!(claude_applied(app, &original_origin));
        assert_live_gateway(&status).await;

        let next_port = available_low_port();
        let (result, _) = super::super::settings_service::apply_settings_update(
            app.clone(),
            state.inner(),
            settings_update(next_port),
        )
        .await
        .unwrap();
        assert!(result.runtime.gateway_rebound);
        assert!(result.runtime.cli_proxy_synced);
        assert_eq!(result.runtime.gateway_status.port, Some(next_port));
        let origin = result.runtime.gateway_status.base_url.as_deref().unwrap();
        assert!(claude_applied(app, origin));
        assert_live_gateway(&result.runtime.gateway_status).await;
    });
}

#[test]
fn reset_retains_app_data_when_cli_restore_fails() {
    use tauri::Manager;
    let _lock = crate::test_support::test_env_lock();
    let _home = TestHome::new();
    let app = lifecycle_app();
    let app = app.handle();
    let _cleanup = GatewayCleanup(app.clone());
    crate::settings::write(
        app,
        &crate::settings::AppSettings {
            preferred_port: available_low_port(),
            ..Default::default()
        },
    )
    .unwrap();
    tauri::async_runtime::block_on(async {
        let state = app.state::<DbInitState>();
        let enabled = cli_proxy_set_enabled_impl(app.clone(), state.inner(), "claude".into(), true)
            .await
            .unwrap();
        assert!(enabled.ok);
        let app_data = crate::app_paths::app_data_dir(app).unwrap();
        let manifest = app_data.join("cli-proxy/claude/manifest.json");
        let valid_manifest = std::fs::read(&manifest).unwrap();
        std::fs::write(&manifest, b"{broken").unwrap();
        let settings_path = app_data.join("settings.json");
        let db_path = crate::db::db_path(app).unwrap();
        let error =
            super::super::data_management_service::reset_app_data(app.clone(), state.inner())
                .await
                .unwrap_err();
        assert!(error.to_string().contains("CLI_PROXY_FAILED"));
        assert!(settings_path.exists());
        assert!(db_path.exists());
        assert_eq!(std::fs::read(&manifest).unwrap(), b"{broken");
        assert!(!super::super::gateway_runtime_access::app_gateway_status(app).running);

        std::fs::write(&manifest, valid_manifest).unwrap();
        assert!(
            super::super::data_management_service::reset_app_data(app.clone(), state.inner())
                .await
                .unwrap()
        );
        assert!(!settings_path.exists());
        assert!(!db_path.exists());
    });
}
