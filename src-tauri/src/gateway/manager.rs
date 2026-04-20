use crate::shared::mutex_ext::MutexExt;
use crate::{
    circuit_breaker, db, provider_circuit_breakers, providers, request_logs, session_manager,
    settings, wsl,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use super::codex_session_id::CodexSessionIdCache;
use super::events::{GatewayLogEvent, GATEWAY_LOG_EVENT_NAME, GATEWAY_STATUS_EVENT_NAME};
use super::listen;
use super::proxy::{GatewayErrorCode, ProviderBaseUrlPingCache, RecentErrorCache};
use super::routes::build_router;
use super::util::now_unix_seconds;
use super::{GatewayProviderCircuitStatus, GatewayStatus};

struct RunningGateway {
    port: u16,
    base_url: String,
    listen_addr: String,
    circuit: Arc<circuit_breaker::CircuitBreaker>,
    session: Arc<session_manager::SessionManager>,
    recent_errors: Arc<Mutex<RecentErrorCache>>,
    shutdown: oneshot::Sender<()>,
    task: tauri::async_runtime::JoinHandle<()>,
    log_task: tauri::async_runtime::JoinHandle<()>,
    circuit_task: tauri::async_runtime::JoinHandle<()>,
    oauth_refresh_shutdown: tokio::sync::watch::Sender<bool>,
    oauth_refresh_task: tauri::async_runtime::JoinHandle<()>,
}

struct ResolvedGatewayBinding {
    bind_host: String,
    base_host: String,
    fixed_port: Option<u16>,
}

type RunningGatewayHandles = (
    oneshot::Sender<()>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    tauri::async_runtime::JoinHandle<()>,
    tokio::sync::watch::Sender<bool>,
    tauri::async_runtime::JoinHandle<()>,
);

pub(crate) struct GatewayStartResult {
    pub(crate) status: GatewayStatus,
    pub(crate) effective_preferred_port: u16,
}

#[derive(Default)]
pub struct GatewayManager {
    running: Option<RunningGateway>,
}

#[derive(Clone)]
pub(super) struct GatewayAppState {
    pub(super) app: tauri::AppHandle,
    pub(super) db: db::Db,
    pub(super) log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    pub(super) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(super) session: Arc<session_manager::SessionManager>,
    pub(super) codex_session_cache: Arc<Mutex<CodexSessionIdCache>>,
    pub(super) recent_errors: Arc<Mutex<RecentErrorCache>>,
    pub(super) latency_cache: Arc<Mutex<ProviderBaseUrlPingCache>>,
}

impl GatewayAppState {
    pub(super) fn client(&self) -> reqwest::Client {
        super::http_client::get()
    }

    #[cfg(test)]
    pub(super) fn current_client() -> reqwest::Client {
        super::http_client::get()
    }
}

fn port_candidates(preferred: Option<u16>) -> impl Iterator<Item = u16> {
    let mut candidates = Vec::with_capacity(
        (settings::MAX_GATEWAY_PORT - settings::DEFAULT_GATEWAY_PORT + 2) as usize,
    );

    if let Some(p) = preferred {
        if p > 0 {
            candidates.push(p);
        }
    }

    for port in settings::DEFAULT_GATEWAY_PORT..=settings::MAX_GATEWAY_PORT {
        if candidates.first().copied() == Some(port) {
            continue;
        }
        candidates.push(port);
    }

    candidates.into_iter()
}

fn bind_host_port(bind_host: &str, port: u16) -> Option<std::net::TcpListener> {
    let std_listener = std::net::TcpListener::bind((bind_host, port)).ok()?;
    std_listener.set_nonblocking(true).ok()?;
    Some(std_listener)
}

fn bind_first_available(
    bind_host: &str,
    preferred: Option<u16>,
) -> crate::shared::error::AppResult<(u16, std::net::TcpListener)> {
    for port in port_candidates(preferred) {
        if let Some(std_listener) = bind_host_port(bind_host, port) {
            return Ok((port, std_listener));
        }
    }

    Err(format!(
        "no available port in range {}..{} for host {bind_host}",
        settings::DEFAULT_GATEWAY_PORT,
        settings::MAX_GATEWAY_PORT
    )
    .into())
}

fn resolve_gateway_binding(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<ResolvedGatewayBinding> {
    let (bind_host, fixed_port) = match cfg.gateway_listen_mode {
        settings::GatewayListenMode::Localhost => ("127.0.0.1".to_string(), None),
        settings::GatewayListenMode::Lan => ("0.0.0.0".to_string(), None),
        settings::GatewayListenMode::WslAuto => (wsl::resolve_wsl_host(cfg), None),
        settings::GatewayListenMode::Custom => {
            let parsed = listen::parse_custom_listen_address(&cfg.gateway_custom_listen_address)?;
            (parsed.host, parsed.port)
        }
    };

    let base_host = match cfg.gateway_listen_mode {
        settings::GatewayListenMode::Lan => "127.0.0.1".to_string(),
        settings::GatewayListenMode::Custom if listen::is_wildcard_host(&bind_host) => {
            "127.0.0.1".to_string()
        }
        _ => bind_host.clone(),
    };

    Ok(ResolvedGatewayBinding {
        bind_host,
        base_host,
        fixed_port,
    })
}

pub(crate) fn planned_base_url(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<String> {
    let binding = resolve_gateway_binding(cfg)?;
    let port = binding.fixed_port.unwrap_or(cfg.preferred_port);
    Ok(format!(
        "http://{}",
        listen::format_host_port(&binding.base_host, port)
    ))
}

pub(crate) fn listen_rebind_required(
    previous: &settings::AppSettings,
    next: &settings::AppSettings,
) -> bool {
    if previous.preferred_port != next.preferred_port
        || previous.gateway_listen_mode != next.gateway_listen_mode
        || previous.gateway_custom_listen_address != next.gateway_custom_listen_address
    {
        return true;
    }

    if next.gateway_listen_mode == settings::GatewayListenMode::WslAuto
        && (previous.wsl_host_address_mode != next.wsl_host_address_mode
            || previous.wsl_custom_host_address != next.wsl_custom_host_address)
    {
        return true;
    }

    false
}

impl GatewayManager {
    fn start_with_config_inner(
        &mut self,
        app: &tauri::AppHandle,
        db: db::Db,
        cfg: &settings::AppSettings,
        preferred_port: Option<u16>,
    ) -> crate::shared::error::AppResult<GatewayStartResult> {
        if self.running.is_some() {
            let status = self.status();
            let effective_preferred_port = status.port.unwrap_or(cfg.preferred_port);
            return Ok(GatewayStartResult {
                status,
                effective_preferred_port,
            });
        }

        let requested_port = preferred_port
            .filter(|p| *p > 0)
            .unwrap_or(cfg.preferred_port.max(settings::DEFAULT_GATEWAY_PORT));

        let binding = resolve_gateway_binding(cfg)?;
        let bind_host = binding.bind_host;
        let fixed_port = binding.fixed_port;

        let (port, std_listener) = if let Some(port) = fixed_port {
            let listener = bind_host_port(&bind_host, port)
                .ok_or_else(|| format!("failed to bind {bind_host}:{port}"))?;
            (port, listener)
        } else {
            bind_first_available(&bind_host, Some(requested_port))?
        };

        let listen_addr = listen::format_host_port(&bind_host, port);
        let base_host = binding.base_host;
        let base_url = format!("http://{}", listen::format_host_port(&base_host, port));
        let bind_addr = std_listener
            .local_addr()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));

        if fixed_port.is_none() && port != requested_port {
            let payload = GatewayLogEvent {
                level: "warn",
                error_code: GatewayErrorCode::PortInUse.as_str(),
                message: format!("端口 {requested_port} 被占用，已自动切换到 {port}"),
                requested_port,
                bound_port: port,
                base_url: base_url.clone(),
            };
            crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_LOG_EVENT_NAME, payload);
        }

        // Initialize the global HTTP client with proxy settings from config.
        super::http_client::sync_runtime_context(port, &bind_host, &base_host);
        let proxy_url = if cfg.upstream_proxy_enabled {
            super::http_client::build_effective_proxy_url(
                Some(cfg.upstream_proxy_url.as_str()),
                Some(cfg.upstream_proxy_username.as_str()),
                Some(cfg.upstream_proxy_password.as_str()),
            )
            .map_err(|e| format!("{}: {e}", GatewayErrorCode::HttpClientInit.as_str()))?
        } else {
            None
        };
        super::http_client::init(proxy_url.as_deref())
            .map_err(|e| format!("{}: {e}", GatewayErrorCode::HttpClientInit.as_str()))?;

        let (log_tx, log_task) = request_logs::start_buffered_writer(app.clone(), db.clone());
        let (circuit_tx, circuit_task) =
            provider_circuit_breakers::start_buffered_writer(db.clone());

        let circuit_initial = match provider_circuit_breakers::load_all(&db) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("circuit breaker state load failed, using defaults: {}", err);
                Default::default()
            }
        };

        let circuit_config = circuit_breaker::CircuitBreakerConfig {
            failure_threshold: cfg.circuit_breaker_failure_threshold.max(1),
            open_duration_secs: (cfg.circuit_breaker_open_duration_minutes as i64)
                .saturating_mul(60),
        };
        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_config,
            circuit_initial,
            Some(circuit_tx),
        ));
        let circuit_for_manager = circuit.clone();
        let session = Arc::new(session_manager::SessionManager::new());
        let codex_session_cache = Arc::new(Mutex::new(CodexSessionIdCache::default()));
        let recent_errors = Arc::new(Mutex::new(RecentErrorCache::default()));
        let latency_cache = Arc::new(Mutex::new(ProviderBaseUrlPingCache::default()));

        let state_app = app.clone();

        let state = GatewayAppState {
            app: app.clone(),
            db: db.clone(),
            log_tx,
            circuit,
            session: session.clone(),
            codex_session_cache,
            recent_errors: recent_errors.clone(),
            latency_cache,
        };

        let app = build_router(state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // Spawn the background OAuth token refresh loop.
        let (oauth_refresh_shutdown_tx, oauth_refresh_shutdown_rx) =
            tokio::sync::watch::channel(false);
        let oauth_refresh_task = super::oauth::refresh_loop::spawn(db, oauth_refresh_shutdown_rx);

        let task = tauri::async_runtime::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(l) => l,
                Err(err) => {
                    tracing::error!(bind_addr = %bind_addr, "gateway listener initialization failed: {}", err);
                    return;
                }
            };

            let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });

            if let Err(err) = serve.await {
                tracing::error!(bind_addr = %bind_addr, "gateway server runtime error: {}", err);
            }
        });

        self.running = Some(RunningGateway {
            port,
            base_url,
            listen_addr,
            circuit: circuit_for_manager,
            session,
            recent_errors: recent_errors.clone(),
            shutdown: shutdown_tx,
            task,
            log_task,
            circuit_task,
            oauth_refresh_shutdown: oauth_refresh_shutdown_tx,
            oauth_refresh_task,
        });

        let status = self.status();
        crate::app::heartbeat_watchdog::gated_emit(&state_app, GATEWAY_STATUS_EVENT_NAME, &status);
        Ok(GatewayStartResult {
            status,
            effective_preferred_port: port,
        })
    }

    pub fn status(&self) -> GatewayStatus {
        match &self.running {
            Some(r) => GatewayStatus {
                running: true,
                port: Some(r.port),
                base_url: Some(r.base_url.clone()),
                listen_addr: Some(r.listen_addr.clone()),
            },
            None => GatewayStatus {
                running: false,
                port: None,
                base_url: None,
                listen_addr: None,
            },
        }
    }

    pub fn active_sessions(
        &self,
        now_unix: i64,
        limit: usize,
    ) -> Vec<session_manager::ActiveSessionSnapshot> {
        match &self.running {
            Some(r) => r.session.list_active(now_unix, limit),
            None => Vec::new(),
        }
    }

    pub fn clear_cli_session_bindings(&self, cli_key: &str) -> usize {
        match &self.running {
            Some(r) => r.session.clear_cli_bindings(cli_key),
            None => 0,
        }
    }

    pub fn start(
        &mut self,
        app: &tauri::AppHandle,
        db: db::Db,
        preferred_port: Option<u16>,
    ) -> crate::shared::error::AppResult<GatewayStatus> {
        let cfg = settings::read(app)?;
        let requested_port = preferred_port
            .filter(|p| *p > 0)
            .unwrap_or(cfg.preferred_port.max(settings::DEFAULT_GATEWAY_PORT));
        let start_result = self.start_with_config_inner(app, db, &cfg, preferred_port)?;

        if start_result.effective_preferred_port != requested_port
            && requested_port == cfg.preferred_port
        {
            if let Ok(mut current) = settings::read(app) {
                if current.preferred_port != start_result.effective_preferred_port {
                    current.preferred_port = start_result.effective_preferred_port;
                    let _ = settings::write(app, &current);
                }
            }
        }

        Ok(start_result.status)
    }

    pub fn start_with_config(
        &mut self,
        app: &tauri::AppHandle,
        db: db::Db,
        cfg: &settings::AppSettings,
        preferred_port: Option<u16>,
    ) -> crate::shared::error::AppResult<GatewayStartResult> {
        self.start_with_config_inner(app, db, cfg, preferred_port)
    }

    pub fn circuit_status(
        &self,
        app: &tauri::AppHandle,
        db: &db::Db,
        cli_key: &str,
    ) -> crate::shared::error::AppResult<Vec<GatewayProviderCircuitStatus>> {
        let provider_ids: Vec<i64> = providers::list_by_cli(db, cli_key)?
            .into_iter()
            .map(|p| p.id)
            .collect();

        if provider_ids.is_empty() {
            return Ok(Vec::new());
        }

        let now_unix = now_unix_seconds() as i64;

        if let Some(r) = &self.running {
            return Ok(provider_ids
                .into_iter()
                .map(|provider_id| {
                    let check = r.circuit.should_allow(provider_id, now_unix);
                    let snap = check.after;
                    GatewayProviderCircuitStatus {
                        provider_id,
                        state: snap.state.as_str().to_string(),
                        failure_count: snap.failure_count,
                        failure_threshold: snap.failure_threshold,
                        open_until: snap.open_until,
                        cooldown_until: snap.cooldown_until,
                    }
                })
                .collect());
        }

        let persisted = provider_circuit_breakers::load_all(db).unwrap_or_default();
        let cfg = settings::read(app)?;
        let failure_threshold = cfg.circuit_breaker_failure_threshold.max(1);

        Ok(provider_ids
            .into_iter()
            .map(|provider_id| {
                if let Some(item) = persisted.get(&provider_id) {
                    let failure_count = item.failure_timestamps.len().min(u32::MAX as usize) as u32;
                    let expired = item.state == circuit_breaker::CircuitState::Open
                        && item.open_until.map(|t| now_unix >= t).unwrap_or(true);
                    if expired {
                        return GatewayProviderCircuitStatus {
                            provider_id,
                            state: circuit_breaker::CircuitState::HalfOpen.as_str().to_string(),
                            failure_count,
                            failure_threshold,
                            open_until: None,
                            cooldown_until: None,
                        };
                    }
                    GatewayProviderCircuitStatus {
                        provider_id,
                        state: item.state.as_str().to_string(),
                        failure_count,
                        failure_threshold,
                        open_until: item.open_until,
                        cooldown_until: None,
                    }
                } else {
                    GatewayProviderCircuitStatus {
                        provider_id,
                        state: circuit_breaker::CircuitState::Closed.as_str().to_string(),
                        failure_count: 0,
                        failure_threshold,
                        open_until: None,
                        cooldown_until: None,
                    }
                }
            })
            .collect())
    }

    pub fn circuit_reset_provider(
        &self,
        db: &db::Db,
        provider_id: i64,
    ) -> crate::shared::error::AppResult<()> {
        if provider_id <= 0 {
            return Err("SEC_INVALID_INPUT: provider_id must be > 0"
                .to_string()
                .into());
        }

        if let Some(r) = &self.running {
            let now_unix = now_unix_seconds() as i64;
            r.circuit.reset(provider_id, now_unix);
            r.recent_errors.lock_or_recover().clear();
        }

        let _ = provider_circuit_breakers::delete_by_provider_id(db, provider_id)?;
        Ok(())
    }

    pub fn circuit_reset_cli(
        &self,
        db: &db::Db,
        cli_key: &str,
    ) -> crate::shared::error::AppResult<usize> {
        let provider_ids: Vec<i64> = providers::list_by_cli(db, cli_key)?
            .into_iter()
            .map(|p| p.id)
            .collect();

        if provider_ids.is_empty() {
            return Ok(0);
        }

        if let Some(r) = &self.running {
            let now_unix = now_unix_seconds() as i64;
            for provider_id in &provider_ids {
                r.circuit.reset(*provider_id, now_unix);
            }
            r.recent_errors.lock_or_recover().clear();
        }

        let _ = provider_circuit_breakers::delete_by_provider_ids(db, &provider_ids)?;
        Ok(provider_ids.len())
    }

    pub fn update_circuit_config(&self, failure_threshold: u32, open_duration_secs: i64) {
        if let Some(r) = &self.running {
            r.circuit
                .update_config(circuit_breaker::CircuitBreakerConfig {
                    failure_threshold,
                    open_duration_secs,
                });
        }
    }

    pub fn take_running(&mut self) -> Option<RunningGatewayHandles> {
        self.running.take().map(|r| {
            // Signal the OAuth refresh loop to stop.
            let _ = r.oauth_refresh_shutdown.send(true);
            (
                r.shutdown,
                r.task,
                r.log_task,
                r.circuit_task,
                r.oauth_refresh_shutdown,
                r.oauth_refresh_task,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{listen_rebind_required, GatewayAppState, GatewayManager, RunningGateway};
    use crate::{circuit_breaker, providers, session_manager};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::sync::oneshot;

    fn build_running_gateway(
        rt: &tokio::runtime::Runtime,
        session: Arc<session_manager::SessionManager>,
        recent_errors: Arc<Mutex<crate::gateway::proxy::RecentErrorCache>>,
    ) -> RunningGateway {
        let circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
            circuit_breaker::CircuitBreakerConfig::default(),
            HashMap::new(),
            None,
        ));

        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (oauth_refresh_shutdown_tx, _oauth_refresh_shutdown_rx) =
            tokio::sync::watch::channel(false);
        RunningGateway {
            port: 1,
            base_url: "http://127.0.0.1:1".to_string(),
            listen_addr: "127.0.0.1:1".to_string(),
            circuit,
            session,
            recent_errors,
            shutdown: shutdown_tx,
            task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            log_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            circuit_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
            oauth_refresh_shutdown: oauth_refresh_shutdown_tx,
            oauth_refresh_task: tauri::async_runtime::JoinHandle::Tokio(rt.spawn(async {})),
        }
    }

    fn spawn_http_proxy_server() -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind proxy listener");
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
        let session = Arc::new(session_manager::SessionManager::new());
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

        let removed = manager.clear_cli_session_bindings("claude");
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
        let session = Arc::new(session_manager::SessionManager::new());
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

        manager
            .circuit_reset_provider(&db, provider_id)
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
        let session = Arc::new(session_manager::SessionManager::new());
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

        let reset_count = manager.circuit_reset_cli(&db, "claude").expect("reset cli");

        assert_eq!(reset_count, 2);
        let cached = recent_errors
            .lock()
            .expect("lock recent_errors")
            .has_active_error_for_tests(now_unix, 88, "fp-cli-reset");
        assert!(!cached);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn gateway_app_state_client_follows_hot_reloaded_proxy() {
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
}
