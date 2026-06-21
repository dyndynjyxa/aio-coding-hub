//! Usage: Gateway start and circuit-control orchestration.

use crate::{
    app::plugin_service, app::plugins::runtime_executor::RuntimeGatewayPluginExecutor,
    circuit_breaker, db, provider_circuit_breakers, providers, session_manager, settings,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use super::background_tasks::GatewayBackgroundTasks;
use super::binder::{bind_exact, bind_first_available, resolve_gateway_binding};
use super::codex_session_id::CodexSessionIdCache;
use super::events::{GatewayLogEvent, GATEWAY_LOG_EVENT_NAME, GATEWAY_STATUS_EVENT_NAME};
use super::proxy::{GatewayErrorCode, ProviderBaseUrlPingCache, RecentErrorCache};
use super::routes::build_router;
use super::runtime::{GatewayAppState, GatewayRuntime, GatewayRuntimeInit};
use super::util::now_unix_seconds;
use super::GatewayProviderCircuitStatus;

pub(crate) struct GatewayStartResult {
    pub(crate) status: super::GatewayStatus,
    pub(crate) effective_preferred_port: u16,
}

pub(crate) struct GatewayControlService;

impl GatewayControlService {
    pub(crate) fn start(
        running: &mut Option<GatewayRuntime>,
        app: &tauri::AppHandle,
        db: db::Db,
        cfg: &settings::AppSettings,
        preferred_port: Option<u16>,
    ) -> crate::shared::error::AppResult<GatewayStartResult> {
        if let Some(runtime) = running.as_ref() {
            let status = runtime.status();
            let effective_preferred_port = status.port.unwrap_or(cfg.preferred_port);
            return Ok(GatewayStartResult {
                status,
                effective_preferred_port,
            });
        }

        let requested_port = preferred_port
            .filter(|port| *port > 0)
            .unwrap_or(cfg.preferred_port.max(settings::DEFAULT_GATEWAY_PORT));

        let binding = resolve_gateway_binding(cfg)?;
        let (port, std_listener) = if let Some(port) = binding.fixed_port {
            let listener = bind_exact(&binding.bind_host, port)?;
            (port, listener)
        } else {
            bind_first_available(&binding.bind_host, Some(requested_port))?
        };

        let listen_addr = super::listen::format_host_port(&binding.bind_host, port);
        let base_url = format!(
            "http://{}",
            super::listen::format_host_port(&binding.base_host, port)
        );
        let bind_addr = std_listener
            .local_addr()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));

        emit_port_fallback_log(
            app,
            binding.fixed_port,
            requested_port,
            port,
            base_url.clone(),
        );
        configure_http_client(
            cfg,
            port,
            binding.bind_host.as_str(),
            binding.base_host.as_str(),
        )?;

        let background_tasks = GatewayBackgroundTasks::start(app.clone(), db.clone());
        let circuit = build_circuit_breaker(&db, cfg, background_tasks.circuit_persist_tx());
        let session = Arc::new(session_manager::SessionManager::new());
        let recent_errors = Arc::new(Mutex::new(RecentErrorCache::default()));
        let plugin_pipeline = load_gateway_plugin_pipeline(&db);

        let state = GatewayAppState {
            app: app.clone(),
            db: db.clone(),
            log_tx: background_tasks.log_tx(),
            circuit: circuit.clone(),
            session: session.clone(),
            codex_session_cache: Arc::new(Mutex::new(CodexSessionIdCache::default())),
            recent_errors: recent_errors.clone(),
            latency_cache: Arc::new(Mutex::new(ProviderBaseUrlPingCache::default())),
            plugin_pipeline: plugin_pipeline.clone(),
        };
        let router = build_router(state);
        let (shutdown, shutdown_rx) = oneshot::channel::<()>();
        let task = tauri::async_runtime::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(listener) => listener,
                Err(err) => {
                    tracing::error!(
                        bind_addr = %bind_addr,
                        "gateway listener initialization failed: {}",
                        err
                    );
                    return;
                }
            };

            let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });

            if let Err(err) = serve.await {
                tracing::error!(bind_addr = %bind_addr, "gateway server runtime error: {}", err);
            }
        });

        let runtime = GatewayRuntime::new(GatewayRuntimeInit {
            port,
            base_url,
            listen_addr,
            circuit,
            session,
            recent_errors,
            shutdown,
            task,
            background_tasks,
            plugin_pipeline: plugin_pipeline.clone(),
        });
        let status = runtime.status();
        *running = Some(runtime);
        crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_STATUS_EVENT_NAME, &status);

        Ok(GatewayStartResult {
            status,
            effective_preferred_port: port,
        })
    }

    pub(crate) fn circuit_status(
        running: Option<&GatewayRuntime>,
        app: &tauri::AppHandle,
        db: &db::Db,
        cli_key: &str,
    ) -> crate::shared::error::AppResult<Vec<GatewayProviderCircuitStatus>> {
        let provider_ids = provider_ids_for_cli(db, cli_key)?;
        if provider_ids.is_empty() {
            return Ok(Vec::new());
        }

        let now_unix = now_unix_seconds() as i64;
        if let Some(runtime) = running {
            return Ok(runtime.circuit_status(&provider_ids, now_unix));
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
                        && item.open_until.map(|ts| now_unix >= ts).unwrap_or(true);
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

    pub(crate) fn refresh_plugins(running: Option<&GatewayRuntime>, db: &db::Db) {
        let Some(runtime) = running else {
            return;
        };
        match plugin_service::enabled_plugins_for_gateway(db) {
            Ok(plugins) => {
                let plugin_count = plugins.len();
                runtime.refresh_plugin_pipeline(plugins);
                tracing::info!(plugin_count, "refreshed gateway plugin pipeline");
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to refresh gateway plugin pipeline; keeping previous snapshot"
                );
            }
        }
    }

    pub(crate) fn circuit_reset_provider(
        running: Option<&GatewayRuntime>,
        db: &db::Db,
        provider_id: i64,
    ) -> crate::shared::error::AppResult<()> {
        if provider_id <= 0 {
            return Err("SEC_INVALID_INPUT: provider_id must be > 0"
                .to_string()
                .into());
        }

        if let Some(runtime) = running {
            runtime.circuit_reset_provider(provider_id, now_unix_seconds() as i64);
        }

        let _ = provider_circuit_breakers::delete_by_provider_id(db, provider_id)?;
        Ok(())
    }

    pub(crate) fn circuit_reset_cli(
        running: Option<&GatewayRuntime>,
        db: &db::Db,
        cli_key: &str,
    ) -> crate::shared::error::AppResult<usize> {
        let provider_ids = provider_ids_for_cli(db, cli_key)?;
        if provider_ids.is_empty() {
            return Ok(0);
        }

        if let Some(runtime) = running {
            runtime.circuit_reset_cli(&provider_ids, now_unix_seconds() as i64);
        }

        let _ = provider_circuit_breakers::delete_by_provider_ids(db, &provider_ids)?;
        Ok(provider_ids.len())
    }
}

fn load_gateway_plugin_pipeline(
    db: &db::Db,
) -> Arc<super::plugins::pipeline::GatewayPluginPipeline> {
    match plugin_service::enabled_plugins_for_gateway(db) {
        Ok(plugins) if plugins.is_empty() => {
            super::plugins::pipeline::GatewayPluginPipeline::empty_shared()
        }
        Ok(plugins) => {
            tracing::info!(
                plugin_count = plugins.len(),
                "loaded enabled gateway plugins"
            );
            Arc::new(
                super::plugins::pipeline::GatewayPluginPipeline::for_runtime(
                    plugins,
                    Arc::new(RuntimeGatewayPluginExecutor::default()),
                    super::plugins::pipeline::GatewayPluginPipelineConfig::default(),
                ),
            )
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to load gateway plugins; continuing with empty plugin pipeline"
            );
            super::plugins::pipeline::GatewayPluginPipeline::empty_shared()
        }
    }
}

fn provider_ids_for_cli(db: &db::Db, cli_key: &str) -> crate::shared::error::AppResult<Vec<i64>> {
    Ok(providers::list_by_cli(db, cli_key)?
        .into_iter()
        .map(|provider| provider.id)
        .collect())
}

fn emit_port_fallback_log(
    app: &tauri::AppHandle,
    fixed_port: Option<u16>,
    requested_port: u16,
    bound_port: u16,
    base_url: String,
) {
    if fixed_port.is_none() && bound_port != requested_port {
        let payload = GatewayLogEvent {
            level: "warn",
            error_code: GatewayErrorCode::PortInUse.as_str(),
            message: format!("端口 {requested_port} 被占用，已自动切换到 {bound_port}"),
            requested_port,
            bound_port,
            base_url,
        };
        crate::app::heartbeat_watchdog::gated_emit(app, GATEWAY_LOG_EVENT_NAME, payload);
    }
}

fn configure_http_client(
    cfg: &settings::AppSettings,
    port: u16,
    bind_host: &str,
    base_host: &str,
) -> crate::shared::error::AppResult<()> {
    let context = super::http_client::runtime_self_check_context(port, bind_host, base_host);
    let proxy_url = if cfg.upstream_proxy_enabled {
        super::http_client::build_effective_proxy_url(
            Some(cfg.upstream_proxy_url.as_str()),
            Some(cfg.upstream_proxy_username.as_str()),
            Some(cfg.upstream_proxy_password.as_str()),
        )
        .map_err(|err| format!("{}: {err}", GatewayErrorCode::HttpClientInit.as_str()))?
    } else {
        None
    };
    super::http_client::validate_proxy_with_context(proxy_url.as_deref(), &context)
        .map_err(|err| format!("{}: {err}", GatewayErrorCode::HttpClientInit.as_str()))?;
    super::http_client::sync_runtime_context(port, bind_host, base_host);
    super::http_client::init(proxy_url.as_deref())
        .map_err(|err| format!("{}: {err}", GatewayErrorCode::HttpClientInit.as_str()).into())
}

fn build_circuit_breaker(
    db: &db::Db,
    cfg: &settings::AppSettings,
    persist_tx: tokio::sync::mpsc::Sender<circuit_breaker::CircuitPersistedState>,
) -> Arc<circuit_breaker::CircuitBreaker> {
    let circuit_initial = match provider_circuit_breakers::load_all(db) {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!("circuit breaker state load failed, using defaults: {}", err);
            Default::default()
        }
    };

    let circuit_config = circuit_breaker::CircuitBreakerConfig {
        failure_threshold: cfg.circuit_breaker_failure_threshold.max(1),
        open_duration_secs: (cfg.circuit_breaker_open_duration_minutes as i64).saturating_mul(60),
    };
    Arc::new(circuit_breaker::CircuitBreaker::new(
        circuit_config,
        circuit_initial,
        Some(persist_tx),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_http_client_rejects_runtime_self_loop_proxy() {
        let cfg = settings::AppSettings {
            upstream_proxy_enabled: true,
            upstream_proxy_url: "http://127.0.0.1:37123".to_string(),
            ..settings::AppSettings::default()
        };

        let err = configure_http_client(&cfg, 37123, "127.0.0.1", "127.0.0.1")
            .expect_err("runtime self-loop proxy should be rejected")
            .to_string();

        assert!(err.contains(GatewayErrorCode::HttpClientInit.as_str()));
        assert!(err.contains("self-loop"));
    }
}
