use super::runtime::{GatewayRuntime, GatewayRuntimeHandles};

#[derive(Default)]
pub struct GatewayManager {
    pub(crate) running: Option<GatewayRuntime>,
}

impl GatewayManager {
    pub fn take_running(&mut self) -> Option<GatewayRuntimeHandles> {
        self.running.take().map(GatewayRuntime::into_handles)
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayManager;
    use crate::gateway::binder::listen_rebind_required;
    use crate::gateway::control_service::GatewayControlService;
    use crate::gateway::runtime::GatewayAppState;
    use crate::gateway::runtime::GatewayRuntime;
    use crate::gateway::GatewayStatus;
    use crate::providers;
    use std::io::{Read, Write};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    struct HttpClientTestGuard {
        _env_guard: MutexGuard<'static, ()>,
    }

    impl Drop for HttpClientTestGuard {
        fn drop(&mut self) {
            let _ = crate::gateway::http_client::apply_proxy(None);
        }
    }

    fn http_client_test_guard() -> HttpClientTestGuard {
        let env_guard = crate::test_support::test_env_lock();
        let _ = crate::gateway::http_client::apply_proxy(None);
        HttpClientTestGuard {
            _env_guard: env_guard,
        }
    }

    fn build_running_gateway(
        rt: &tokio::runtime::Runtime,
        session: Arc<crate::session_manager::SessionManager>,
        recent_errors: Arc<Mutex<crate::gateway::proxy::RecentErrorCache>>,
    ) -> crate::gateway::runtime::GatewayRuntime {
        crate::gateway::runtime::GatewayRuntime::for_tests(rt, session, recent_errors)
    }

    fn spawn_http_proxy_server() -> (String, mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind proxy listener");
        let addr = listener.local_addr().expect("proxy addr");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buf = [0_u8; 4096];
            let size = stream.read(&mut buf).expect("read request");
            let request = String::from_utf8_lossy(&buf[..size]).to_string();
            tx.send(request).expect("send request");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("write response");
        });

        (format!("http://127.0.0.1:{}", addr.port()), rx)
    }

    fn default_settings() -> crate::settings::AppSettings {
        crate::settings::AppSettings::default()
    }

    #[test]
    fn listen_rebind_required_when_listen_mode_changes() {
        let previous = default_settings();
        let mut next = previous.clone();
        next.gateway_listen_mode = crate::settings::GatewayListenMode::Lan;

        assert!(listen_rebind_required(&previous, &next));
    }

    #[test]
    fn listen_rebind_required_when_custom_listen_address_changes() {
        let mut previous = default_settings();
        previous.gateway_listen_mode = crate::settings::GatewayListenMode::Custom;
        previous.gateway_custom_listen_address = "127.0.0.1:37123".to_string();
        let mut next = previous.clone();
        next.gateway_custom_listen_address = "0.0.0.0:37123".to_string();

        assert!(listen_rebind_required(&previous, &next));
    }

    #[test]
    fn listen_rebind_required_when_wsl_host_binding_changes_under_wsl_auto() {
        let mut previous = default_settings();
        previous.gateway_listen_mode = crate::settings::GatewayListenMode::WslAuto;
        let mut next = previous.clone();
        next.wsl_host_address_mode = crate::settings::WslHostAddressMode::Custom;
        next.wsl_custom_host_address = "172.20.80.1".to_string();

        assert!(listen_rebind_required(&previous, &next));
    }

    #[test]
    fn listen_rebind_not_required_for_non_listener_settings_only() {
        let previous = default_settings();
        let mut next = previous.clone();
        next.upstream_proxy_enabled = true;
        next.upstream_proxy_url = "http://127.0.0.1:7890".to_string();
        next.enable_cache_anomaly_monitor = !previous.enable_cache_anomaly_monitor;

        assert!(!listen_rebind_required(&previous, &next));
    }

    #[test]
    fn clear_cli_session_bindings_removes_only_target_cli_when_running() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let session = Arc::new(crate::session_manager::SessionManager::new());
        let recent_errors = Arc::new(Mutex::new(
            crate::gateway::proxy::RecentErrorCache::default(),
        ));
        let now_unix = 100;

        session.bind_sort_mode(
            "claude",
            "session_a",
            Some(1),
            Some(vec![101, 102]),
            now_unix,
        );
        session.bind_sort_mode("claude", "session_b", None, None, now_unix);
        session.bind_sort_mode("codex", "session_c", Some(2), Some(vec![201]), now_unix);

        assert_eq!(
            session.get_bound_sort_mode_id("claude", "session_a", now_unix),
            Some(Some(1))
        );

        let manager = GatewayManager {
            running: Some(build_running_gateway(&rt, session.clone(), recent_errors)),
        };

        let removed = manager
            .running
            .as_ref()
            .expect("running gateway")
            .clear_cli_session_bindings("claude");
        assert_eq!(removed, 2);

        assert_eq!(
            session.get_bound_sort_mode_id("claude", "session_a", now_unix),
            None
        );
        assert_eq!(
            session.get_bound_sort_mode_id("claude", "session_b", now_unix),
            None
        );
        assert_eq!(
            session.get_bound_sort_mode_id("codex", "session_c", now_unix),
            Some(Some(2))
        );
    }

    #[test]
    fn clear_cli_route_runtime_state_clears_sessions_and_recent_error_cache() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let session = Arc::new(crate::session_manager::SessionManager::new());
        let recent_errors = Arc::new(Mutex::new(
            crate::gateway::proxy::RecentErrorCache::default(),
        ));
        let now_unix = 100;

        session.bind_sort_mode(
            "codex",
            "session_a",
            Some(1),
            Some(vec![101, 102]),
            now_unix,
        );
        session.bind_sort_mode("claude", "session_b", Some(2), Some(vec![201]), now_unix);

        {
            let mut cache = recent_errors.lock().expect("lock recent_errors");
            cache.insert_unavailable_for_tests(now_unix, 77, "fp-codex", 30);
            cache.insert_unavailable_for_tests(now_unix, 88, "fp-claude", 30);
        }

        let manager = GatewayManager {
            running: Some(build_running_gateway(
                &rt,
                session.clone(),
                recent_errors.clone(),
            )),
        };

        let cleared = manager
            .running
            .as_ref()
            .expect("running gateway")
            .clear_cli_route_runtime_state("codex");
        assert_eq!(cleared.cleared_sessions, 1);
        assert_eq!(cleared.cleared_recent_errors, 2);

        assert_eq!(
            session.get_bound_sort_mode_id("codex", "session_a", now_unix),
            None
        );
        assert_eq!(
            session.get_bound_sort_mode_id("claude", "session_b", now_unix),
            Some(Some(2))
        );

        let cache = recent_errors.lock().expect("lock recent_errors");
        assert!(!cache.has_active_error_for_tests(now_unix, 77, "fp-codex"));
        assert!(!cache.has_active_error_for_tests(now_unix, 88, "fp-claude"));
    }

    fn insert_provider(db: &crate::db::Db, cli_key: &str, name: &str) -> i64 {
        providers::upsert(
            db,
            providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: cli_key.to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.com".to_string()],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some("k".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(100),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: Some(providers::DailyResetMode::Fixed),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert provider")
        .id
    }

    #[test]
    fn circuit_reset_provider_clears_recent_error_cache() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("gateway_manager_reset_provider.db");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let session = Arc::new(crate::session_manager::SessionManager::new());
        let recent_errors = Arc::new(Mutex::new(
            crate::gateway::proxy::RecentErrorCache::default(),
        ));
        let now_unix = 100;

        {
            let mut cache = recent_errors.lock().expect("lock recent_errors");
            cache.insert_unavailable_for_tests(now_unix, 77, "fp-provider-reset", 30);
        }

        let provider_id = insert_provider(&db, "claude", "Claude Reset");
        let manager = GatewayManager {
            running: Some(build_running_gateway(&rt, session, recent_errors.clone())),
        };

        GatewayControlService::circuit_reset_provider(manager.running.as_ref(), &db, provider_id)
            .expect("reset provider");

        let cached = recent_errors
            .lock()
            .expect("lock recent_errors")
            .has_active_error_for_tests(now_unix, 77, "fp-provider-reset");
        assert!(!cached);
    }

    #[test]
    fn circuit_reset_cli_clears_recent_error_cache() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("gateway_manager_reset_cli.db");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let session = Arc::new(crate::session_manager::SessionManager::new());
        let recent_errors = Arc::new(Mutex::new(
            crate::gateway::proxy::RecentErrorCache::default(),
        ));
        let now_unix = 100;

        {
            let mut cache = recent_errors.lock().expect("lock recent_errors");
            cache.insert_unavailable_for_tests(now_unix, 88, "fp-cli-reset", 30);
        }

        insert_provider(&db, "claude", "Claude Reset A");
        insert_provider(&db, "claude", "Claude Reset B");

        let manager = GatewayManager {
            running: Some(build_running_gateway(&rt, session, recent_errors.clone())),
        };

        let reset_count =
            GatewayControlService::circuit_reset_cli(manager.running.as_ref(), &db, "claude")
                .expect("reset cli");

        assert_eq!(reset_count, 2);
        let cached = recent_errors
            .lock()
            .expect("lock recent_errors")
            .has_active_error_for_tests(now_unix, 88, "fp-cli-reset");
        assert!(!cached);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_app_state_client_follows_hot_reloaded_proxy() {
        let _guard = http_client_test_guard();
        let (proxy_a, rx_a) = spawn_http_proxy_server();
        let (proxy_b, rx_b) = spawn_http_proxy_server();

        crate::gateway::http_client::sync_runtime_context(37123, "127.0.0.1", "127.0.0.1");
        crate::gateway::http_client::init(Some(&proxy_a)).expect("init proxy a");

        let response = GatewayAppState::current_client()
            .get("http://example.com/")
            .send()
            .await
            .expect("request via proxy a");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let first_request = rx_a
            .recv_timeout(Duration::from_secs(3))
            .expect("proxy a should receive request");
        assert!(first_request.starts_with("GET http://example.com/ HTTP/1.1"));

        crate::gateway::http_client::apply_proxy(Some(&proxy_b)).expect("switch to proxy b");

        let response = GatewayAppState::current_client()
            .get("http://example.com/")
            .send()
            .await
            .expect("request via proxy b");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let second_request = rx_b
            .recv_timeout(Duration::from_secs(3))
            .expect("proxy b should receive request");
        assert!(second_request.starts_with("GET http://example.com/ HTTP/1.1"));
    }

    #[test]
    fn status_defaults_to_stopped_when_runtime_absent() {
        let manager = GatewayManager::default();
        let status = manager
            .running
            .as_ref()
            .map(GatewayRuntime::status)
            .unwrap_or_default();

        assert_eq!(
            status,
            GatewayStatus {
                running: false,
                port: None,
                base_url: None,
                listen_addr: None,
            }
        );
    }
}
