//! Usage: Settings-related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::gateway::events::GATEWAY_STATUS_EVENT_NAME;
use crate::shared::mutex_ext::MutexExt;
use crate::{blocking, cli_proxy, resident, settings};
use tauri::Manager;

fn read_settings_for_update<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<settings::AppSettings> {
    settings::read(app).map_err(|err| {
        format!(
            "SETTINGS_RECOVERY_REQUIRED: settings.json could not be read; fix or restore it before saving: {err}"
        )
        .into()
    })
}

fn write_settings_view<R, F>(
    app: &tauri::AppHandle<R>,
    mutate: F,
) -> crate::shared::error::AppResult<SettingsView>
where
    R: tauri::Runtime,
    F: FnOnce(&mut settings::AppSettings) -> crate::shared::error::AppResult<()>,
{
    let mut settings = read_settings_for_update(app)?;
    settings.schema_version = settings::SCHEMA_VERSION;
    mutate(&mut settings)?;
    settings::write(app, &settings).map(|value| SettingsView::from(&value))
}

/// Encapsulates all fields for the `settings_set` command.
#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsUpdate {
    pub preferred_port: u16,
    pub show_home_heatmap: Option<bool>,
    pub show_home_usage: Option<bool>,
    pub home_usage_period: Option<settings::HomeUsagePeriod>,
    pub gateway_listen_mode: Option<settings::GatewayListenMode>,
    pub gateway_custom_listen_address: Option<String>,
    pub auto_start: bool,
    pub start_minimized: Option<bool>,
    pub tray_enabled: Option<bool>,
    pub enable_cli_proxy_startup_recovery: Option<bool>,
    pub log_retention_days: u32,
    pub provider_cooldown_seconds: Option<u32>,
    pub provider_base_url_ping_cache_ttl_seconds: Option<u32>,
    pub upstream_first_byte_timeout_seconds: Option<u32>,
    pub upstream_stream_idle_timeout_seconds: Option<u32>,
    pub upstream_request_timeout_non_streaming_seconds: Option<u32>,
    pub intercept_anthropic_warmup_requests: Option<bool>,
    pub enable_thinking_signature_rectifier: Option<bool>,
    pub enable_thinking_budget_rectifier: Option<bool>,
    pub enable_billing_header_rectifier: Option<bool>,
    pub enable_claude_metadata_user_id_injection: Option<bool>,
    pub enable_cache_anomaly_monitor: Option<bool>,
    pub enable_task_complete_notify: Option<bool>,
    pub enable_notification_sound: Option<bool>,
    pub enable_response_fixer: Option<bool>,
    pub response_fixer_fix_encoding: Option<bool>,
    pub response_fixer_fix_sse_format: Option<bool>,
    pub response_fixer_fix_truncated_json: Option<bool>,
    pub verbose_provider_error: Option<bool>,
    pub failover_max_attempts_per_provider: u32,
    pub failover_max_providers_to_try: u32,
    pub circuit_breaker_failure_threshold: Option<u32>,
    pub circuit_breaker_open_duration_minutes: Option<u32>,
    pub update_releases_url: Option<String>,
    pub wsl_auto_config: Option<bool>,
    pub wsl_target_cli: Option<settings::WslTargetCli>,
    pub cli_priority_order: Option<Vec<String>>,
    pub wsl_host_address_mode: Option<settings::WslHostAddressMode>,
    pub wsl_custom_host_address: Option<String>,
    pub codex_home_mode: Option<settings::CodexHomeMode>,
    pub codex_home_override: Option<String>,
    pub cx2cc_fallback_model_opus: Option<String>,
    pub cx2cc_fallback_model_sonnet: Option<String>,
    pub cx2cc_fallback_model_haiku: Option<String>,
    pub cx2cc_fallback_model_main: Option<String>,
    pub cx2cc_model_reasoning_effort: Option<String>,
    pub cx2cc_service_tier: Option<String>,
    pub cx2cc_disable_response_storage: Option<bool>,
    pub cx2cc_enable_reasoning_to_thinking: Option<bool>,
    pub cx2cc_drop_stop_sequences: Option<bool>,
    pub cx2cc_clean_schema: Option<bool>,
    pub cx2cc_filter_batch_tool: Option<bool>,
    pub upstream_proxy_enabled: Option<bool>,
    pub upstream_proxy_url: Option<String>,
    pub upstream_proxy_username: Option<String>,
    pub upstream_proxy_password: Option<SensitiveStringUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case", tag = "mode", content = "value")]
pub(crate) enum SensitiveStringUpdate {
    Preserve,
    Clear,
    Replace(String),
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct SettingsView {
    pub schema_version: u32,
    pub preferred_port: u16,
    pub show_home_heatmap: bool,
    pub show_home_usage: bool,
    pub home_usage_period: settings::HomeUsagePeriod,
    pub gateway_listen_mode: settings::GatewayListenMode,
    pub gateway_custom_listen_address: String,
    pub wsl_auto_config: bool,
    pub wsl_target_cli: settings::WslTargetCli,
    pub cli_priority_order: Vec<String>,
    pub wsl_host_address_mode: settings::WslHostAddressMode,
    pub wsl_custom_host_address: String,
    pub codex_home_mode: settings::CodexHomeMode,
    pub codex_home_override: String,
    pub auto_start: bool,
    pub start_minimized: bool,
    pub tray_enabled: bool,
    pub enable_cli_proxy_startup_recovery: bool,
    pub log_retention_days: u32,
    pub provider_cooldown_seconds: u32,
    pub provider_base_url_ping_cache_ttl_seconds: u32,
    pub upstream_first_byte_timeout_seconds: u32,
    pub upstream_stream_idle_timeout_seconds: u32,
    pub upstream_request_timeout_non_streaming_seconds: u32,
    pub update_releases_url: String,
    pub failover_max_attempts_per_provider: u32,
    pub failover_max_providers_to_try: u32,
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_open_duration_minutes: u32,
    pub enable_circuit_breaker_notice: bool,
    pub verbose_provider_error: bool,
    pub intercept_anthropic_warmup_requests: bool,
    pub enable_thinking_signature_rectifier: bool,
    pub enable_thinking_budget_rectifier: bool,
    pub enable_billing_header_rectifier: bool,
    pub enable_codex_session_id_completion: bool,
    pub enable_claude_metadata_user_id_injection: bool,
    pub enable_cache_anomaly_monitor: bool,
    pub enable_task_complete_notify: bool,
    pub enable_notification_sound: bool,
    pub enable_response_fixer: bool,
    pub response_fixer_fix_encoding: bool,
    pub response_fixer_fix_sse_format: bool,
    pub response_fixer_fix_truncated_json: bool,
    pub response_fixer_max_json_depth: u32,
    pub response_fixer_max_fix_size: u32,
    pub cx2cc_fallback_model_opus: String,
    pub cx2cc_fallback_model_sonnet: String,
    pub cx2cc_fallback_model_haiku: String,
    pub cx2cc_fallback_model_main: String,
    pub cx2cc_model_reasoning_effort: String,
    pub cx2cc_service_tier: String,
    pub cx2cc_disable_response_storage: bool,
    pub cx2cc_enable_reasoning_to_thinking: bool,
    pub cx2cc_drop_stop_sequences: bool,
    pub cx2cc_clean_schema: bool,
    pub cx2cc_filter_batch_tool: bool,
    pub upstream_proxy_enabled: bool,
    pub upstream_proxy_url: String,
    pub upstream_proxy_username: String,
    pub upstream_proxy_password_configured: bool,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct SettingsMutationRuntime {
    pub gateway_rebound: bool,
    pub cli_proxy_synced: bool,
    pub wsl_auto_sync_triggered: bool,
    pub gateway_status: crate::gateway::GatewayStatus,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct SettingsMutationResult {
    pub settings: SettingsView,
    pub runtime: SettingsMutationRuntime,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayRectifierSettingsUpdate {
    pub verbose_provider_error: bool,
    pub intercept_anthropic_warmup_requests: bool,
    pub enable_thinking_signature_rectifier: bool,
    pub enable_thinking_budget_rectifier: bool,
    pub enable_billing_header_rectifier: bool,
    pub enable_claude_metadata_user_id_injection: bool,
    pub enable_response_fixer: bool,
    pub response_fixer_fix_encoding: bool,
    pub response_fixer_fix_sse_format: bool,
    pub response_fixer_fix_truncated_json: bool,
    pub response_fixer_max_json_depth: u32,
    pub response_fixer_max_fix_size: u32,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CircuitBreakerNoticeUpdate {
    pub enable_circuit_breaker_notice: bool,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexSessionIdCompletionUpdate {
    pub enable_codex_session_id_completion: bool,
}

#[derive(Debug, Clone)]
struct SettingsRuntimePlan {
    gateway_rebind_required: bool,
    cli_proxy_sync_required: bool,
    #[cfg(windows)]
    wsl_auto_sync_required: bool,
}

impl From<&settings::AppSettings> for SettingsView {
    fn from(value: &settings::AppSettings) -> Self {
        Self {
            schema_version: value.schema_version,
            preferred_port: value.preferred_port,
            show_home_heatmap: value.show_home_heatmap,
            show_home_usage: value.show_home_usage,
            home_usage_period: value.home_usage_period,
            gateway_listen_mode: value.gateway_listen_mode,
            gateway_custom_listen_address: value.gateway_custom_listen_address.clone(),
            wsl_auto_config: value.wsl_auto_config,
            wsl_target_cli: value.wsl_target_cli,
            cli_priority_order: value.cli_priority_order.clone(),
            wsl_host_address_mode: value.wsl_host_address_mode,
            wsl_custom_host_address: value.wsl_custom_host_address.clone(),
            codex_home_mode: value.codex_home_mode,
            codex_home_override: value.codex_home_override.clone(),
            auto_start: value.auto_start,
            start_minimized: value.start_minimized,
            tray_enabled: value.tray_enabled,
            enable_cli_proxy_startup_recovery: value.enable_cli_proxy_startup_recovery,
            log_retention_days: value.log_retention_days,
            provider_cooldown_seconds: value.provider_cooldown_seconds,
            provider_base_url_ping_cache_ttl_seconds: value
                .provider_base_url_ping_cache_ttl_seconds,
            upstream_first_byte_timeout_seconds: value.upstream_first_byte_timeout_seconds,
            upstream_stream_idle_timeout_seconds: value.upstream_stream_idle_timeout_seconds,
            upstream_request_timeout_non_streaming_seconds: value
                .upstream_request_timeout_non_streaming_seconds,
            update_releases_url: value.update_releases_url.clone(),
            failover_max_attempts_per_provider: value.failover_max_attempts_per_provider,
            failover_max_providers_to_try: value.failover_max_providers_to_try,
            circuit_breaker_failure_threshold: value.circuit_breaker_failure_threshold,
            circuit_breaker_open_duration_minutes: value.circuit_breaker_open_duration_minutes,
            enable_circuit_breaker_notice: value.enable_circuit_breaker_notice,
            verbose_provider_error: value.verbose_provider_error,
            intercept_anthropic_warmup_requests: value.intercept_anthropic_warmup_requests,
            enable_thinking_signature_rectifier: value.enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier: value.enable_thinking_budget_rectifier,
            enable_billing_header_rectifier: value.enable_billing_header_rectifier,
            enable_codex_session_id_completion: value.enable_codex_session_id_completion,
            enable_claude_metadata_user_id_injection: value
                .enable_claude_metadata_user_id_injection,
            enable_cache_anomaly_monitor: value.enable_cache_anomaly_monitor,
            enable_task_complete_notify: value.enable_task_complete_notify,
            enable_notification_sound: value.enable_notification_sound,
            enable_response_fixer: value.enable_response_fixer,
            response_fixer_fix_encoding: value.response_fixer_fix_encoding,
            response_fixer_fix_sse_format: value.response_fixer_fix_sse_format,
            response_fixer_fix_truncated_json: value.response_fixer_fix_truncated_json,
            response_fixer_max_json_depth: value.response_fixer_max_json_depth,
            response_fixer_max_fix_size: value.response_fixer_max_fix_size,
            cx2cc_fallback_model_opus: value.cx2cc_fallback_model_opus.clone(),
            cx2cc_fallback_model_sonnet: value.cx2cc_fallback_model_sonnet.clone(),
            cx2cc_fallback_model_haiku: value.cx2cc_fallback_model_haiku.clone(),
            cx2cc_fallback_model_main: value.cx2cc_fallback_model_main.clone(),
            cx2cc_model_reasoning_effort: value.cx2cc_model_reasoning_effort.clone(),
            cx2cc_service_tier: value.cx2cc_service_tier.clone(),
            cx2cc_disable_response_storage: value.cx2cc_disable_response_storage,
            cx2cc_enable_reasoning_to_thinking: value.cx2cc_enable_reasoning_to_thinking,
            cx2cc_drop_stop_sequences: value.cx2cc_drop_stop_sequences,
            cx2cc_clean_schema: value.cx2cc_clean_schema,
            cx2cc_filter_batch_tool: value.cx2cc_filter_batch_tool,
            upstream_proxy_enabled: value.upstream_proxy_enabled,
            upstream_proxy_url: value.upstream_proxy_url.clone(),
            upstream_proxy_username: value.upstream_proxy_username.clone(),
            upstream_proxy_password_configured: !value.upstream_proxy_password.is_empty(),
        }
    }
}

impl SettingsRuntimePlan {
    fn from_settings(previous: &settings::AppSettings, next: &settings::AppSettings) -> Self {
        let gateway_rebind_required = crate::gateway::listen_rebind_required(previous, next);
        let codex_home_changed = previous.codex_home_mode != next.codex_home_mode
            || previous.codex_home_override != next.codex_home_override;
        let cli_proxy_sync_required = gateway_rebind_required || codex_home_changed;
        #[cfg(windows)]
        let wsl_auto_sync_required = next.wsl_auto_config
            && next.gateway_listen_mode != settings::GatewayListenMode::Localhost
            && (previous.wsl_auto_config != next.wsl_auto_config
                || previous.wsl_target_cli != next.wsl_target_cli
                || previous.wsl_host_address_mode != next.wsl_host_address_mode
                || previous.wsl_custom_host_address != next.wsl_custom_host_address
                || codex_home_changed
                || gateway_rebind_required);
        Self {
            gateway_rebind_required,
            cli_proxy_sync_required,
            #[cfg(windows)]
            wsl_auto_sync_required,
        }
    }
}

fn apply_sensitive_string_update(
    update: Option<SensitiveStringUpdate>,
    previous: String,
) -> String {
    match update.unwrap_or(SensitiveStringUpdate::Preserve) {
        SensitiveStringUpdate::Preserve => previous,
        SensitiveStringUpdate::Clear => String::new(),
        SensitiveStringUpdate::Replace(value) => value,
    }
}

fn sync_runtime_side_effects(
    app: &tauri::AppHandle,
    next_settings: &settings::AppSettings,
) -> Result<(), String> {
    if let Some(resident) = app.try_state::<resident::ResidentState>() {
        resident.set_tray_enabled(next_settings.tray_enabled);
    }

    if let Some(gw) = app.try_state::<GatewayState>() {
        let manager = gw.0.lock_or_recover();
        manager.update_circuit_config(
            next_settings.circuit_breaker_failure_threshold.max(1),
            (next_settings.circuit_breaker_open_duration_minutes as i64).saturating_mul(60),
        );
    }

    crate::gateway::http_client::sync_from_settings(next_settings)?;
    Ok(())
}

fn current_gateway_status(app: &tauri::AppHandle) -> crate::gateway::GatewayStatus {
    let state = app.state::<GatewayState>();
    let manager = state.0.lock_or_recover();
    manager.status()
}

async fn start_gateway_with_settings(
    app: &tauri::AppHandle,
    db_state: &DbInitState,
    next_settings: &settings::AppSettings,
) -> Result<crate::gateway::GatewayStartResult, String> {
    let db = ensure_db_ready(app.clone(), db_state).await?;
    let next_settings = next_settings.clone();
    let start_result = blocking::run("settings_set_gateway_start", {
        let app = app.clone();
        let db = db.clone();
        move || {
            let state = app.state::<GatewayState>();
            let mut manager = state.0.lock_or_recover();
            manager.start_with_config(&app, db, &next_settings, Some(next_settings.preferred_port))
        }
    })
    .await?;

    crate::app::heartbeat_watchdog::gated_emit(
        app,
        GATEWAY_STATUS_EVENT_NAME,
        start_result.status.clone(),
    );
    Ok(start_result)
}

async fn write_settings_snapshot(
    app: &tauri::AppHandle,
    next_settings: &settings::AppSettings,
) -> Result<settings::AppSettings, String> {
    let next_settings = next_settings.clone();
    blocking::run("settings_set_write", {
        let app = app.clone();
        move || settings::write(&app, &next_settings)
    })
    .await
    .map_err(Into::into)
}

async fn restore_previous_runtime(
    app: &tauri::AppHandle,
    db_state: &DbInitState,
    previous_settings: &settings::AppSettings,
    previous_gateway_status: &crate::gateway::GatewayStatus,
) -> crate::gateway::GatewayStatus {
    let _ = sync_runtime_side_effects(app, previous_settings);

    if !previous_gateway_status.running {
        return current_gateway_status(app);
    }

    crate::app::cleanup::stop_gateway_best_effort(app).await;
    match start_gateway_with_settings(app, db_state, previous_settings).await {
        Ok(result) => result.status,
        Err(err) => {
            tracing::error!(
                error = %err,
                "settings update rollback failed to restore previous gateway runtime"
            );
            current_gateway_status(app)
        }
    }
}

async fn rollback_settings_transaction(
    app: &tauri::AppHandle,
    db_state: &DbInitState,
    previous_settings: &settings::AppSettings,
    previous_gateway_status: &crate::gateway::GatewayStatus,
) -> crate::gateway::GatewayStatus {
    let rollback_result = blocking::run("settings_set_rollback", {
        let app = app.clone();
        let previous_settings = previous_settings.clone();
        move || settings::write(&app, &previous_settings)
    })
    .await;

    if let Err(rollback_error) = rollback_result {
        tracing::error!(
            error = %rollback_error,
            "settings update rollback failed to restore settings.json"
        );
    }

    restore_previous_runtime(app, db_state, previous_settings, previous_gateway_status).await
}

async fn sync_cli_proxy_for_settings(
    app: &tauri::AppHandle,
    base_origin: String,
    apply_live: bool,
) -> bool {
    match blocking::run("settings_set_cli_proxy_sync", {
        let app = app.clone();
        move || cli_proxy::sync_enabled(&app, &base_origin, apply_live)
    })
    .await
    {
        Ok(results) => {
            let failed_count = results.iter().filter(|row| !row.ok).count();
            if failed_count > 0 {
                tracing::warn!(
                    failed_count,
                    total = results.len(),
                    apply_live,
                    "settings update cli proxy sync completed with partial failures"
                );
            }
            failed_count == 0
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                apply_live,
                "settings update cli proxy sync failed"
            );
            false
        }
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_get(app: tauri::AppHandle) -> Result<SettingsView, String> {
    blocking::run("settings_get", move || {
        settings::read(&app).map(|value| SettingsView::from(&value))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    update: SettingsUpdate,
) -> Result<SettingsMutationResult, String> {
    settings_set_impl(app, db_state.inner(), update).await
}

pub(crate) async fn settings_set_impl(
    app: tauri::AppHandle,
    db_state: &DbInitState,
    update: SettingsUpdate,
) -> Result<SettingsMutationResult, String> {
    let SettingsUpdate {
        preferred_port,
        show_home_heatmap,
        show_home_usage,
        home_usage_period,
        gateway_listen_mode,
        gateway_custom_listen_address,
        auto_start,
        start_minimized,
        tray_enabled,
        enable_cli_proxy_startup_recovery,
        log_retention_days,
        provider_cooldown_seconds,
        provider_base_url_ping_cache_ttl_seconds,
        upstream_first_byte_timeout_seconds,
        upstream_stream_idle_timeout_seconds,
        upstream_request_timeout_non_streaming_seconds,
        intercept_anthropic_warmup_requests,
        enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier,
        enable_billing_header_rectifier,
        enable_claude_metadata_user_id_injection,
        enable_cache_anomaly_monitor,
        enable_task_complete_notify,
        enable_notification_sound,
        enable_response_fixer,
        response_fixer_fix_encoding,
        response_fixer_fix_sse_format,
        response_fixer_fix_truncated_json,
        verbose_provider_error,
        failover_max_attempts_per_provider,
        failover_max_providers_to_try,
        circuit_breaker_failure_threshold,
        circuit_breaker_open_duration_minutes,
        update_releases_url,
        wsl_auto_config,
        wsl_target_cli,
        cli_priority_order,
        wsl_host_address_mode,
        wsl_custom_host_address,
        codex_home_mode,
        codex_home_override,
        cx2cc_fallback_model_opus,
        cx2cc_fallback_model_sonnet,
        cx2cc_fallback_model_haiku,
        cx2cc_fallback_model_main,
        cx2cc_model_reasoning_effort,
        cx2cc_service_tier,
        cx2cc_disable_response_storage,
        cx2cc_enable_reasoning_to_thinking,
        cx2cc_drop_stop_sequences,
        cx2cc_clean_schema,
        cx2cc_filter_batch_tool,
        upstream_proxy_enabled,
        upstream_proxy_url,
        upstream_proxy_username,
        upstream_proxy_password,
    } = update;

    let app_for_work = app.clone();
    let (previous_settings, candidate_settings) = blocking::run(
        "settings_set",
        move || -> crate::shared::error::AppResult<(
            settings::AppSettings,
            settings::AppSettings,
        )> {
            let previous = read_settings_for_update(&app_for_work)?;
            let update_releases_url =
                update_releases_url.unwrap_or(previous.update_releases_url.clone());
            let tray_enabled = tray_enabled.unwrap_or(previous.tray_enabled);
            let start_minimized = start_minimized.unwrap_or(previous.start_minimized);
            let enable_cli_proxy_startup_recovery = enable_cli_proxy_startup_recovery
                .unwrap_or(previous.enable_cli_proxy_startup_recovery);
            let provider_cooldown_seconds =
                provider_cooldown_seconds.unwrap_or(previous.provider_cooldown_seconds);
            let gateway_listen_mode = gateway_listen_mode.unwrap_or(previous.gateway_listen_mode);
            let show_home_heatmap = show_home_heatmap.unwrap_or(previous.show_home_heatmap);
            let show_home_usage = show_home_usage.unwrap_or(previous.show_home_usage);
            let home_usage_period = home_usage_period.unwrap_or(previous.home_usage_period);
            let gateway_custom_listen_address = gateway_custom_listen_address
                .unwrap_or(previous.gateway_custom_listen_address.clone())
                .trim()
                .to_string();
            let wsl_auto_config = wsl_auto_config.unwrap_or(previous.wsl_auto_config);
            let wsl_target_cli = wsl_target_cli.unwrap_or(previous.wsl_target_cli);
            let cli_priority_order =
                cli_priority_order.unwrap_or(previous.cli_priority_order.clone());
            let wsl_host_address_mode =
                wsl_host_address_mode.unwrap_or(previous.wsl_host_address_mode);
            let wsl_custom_host_address = wsl_custom_host_address
                .unwrap_or(previous.wsl_custom_host_address.clone())
                .trim()
                .to_string();
            let codex_home_mode = codex_home_mode.unwrap_or(previous.codex_home_mode);
            let codex_home_override = codex_home_override
                .unwrap_or(previous.codex_home_override.clone())
                .trim()
                .to_string();
            let cx2cc_fallback_model_opus = cx2cc_fallback_model_opus
                .unwrap_or(previous.cx2cc_fallback_model_opus.clone())
                .trim()
                .to_string();
            if cx2cc_fallback_model_opus.is_empty() {
                return Err("cx2cc_fallback_model_opus cannot be empty".into());
            }
            let cx2cc_fallback_model_sonnet = cx2cc_fallback_model_sonnet
                .unwrap_or(previous.cx2cc_fallback_model_sonnet.clone())
                .trim()
                .to_string();
            if cx2cc_fallback_model_sonnet.is_empty() {
                return Err("cx2cc_fallback_model_sonnet cannot be empty".into());
            }
            let cx2cc_fallback_model_haiku = cx2cc_fallback_model_haiku
                .unwrap_or(previous.cx2cc_fallback_model_haiku.clone())
                .trim()
                .to_string();
            if cx2cc_fallback_model_haiku.is_empty() {
                return Err("cx2cc_fallback_model_haiku cannot be empty".into());
            }
            let cx2cc_fallback_model_main = cx2cc_fallback_model_main
                .unwrap_or(previous.cx2cc_fallback_model_main.clone())
                .trim()
                .to_string();
            if cx2cc_fallback_model_main.is_empty() {
                return Err("cx2cc_fallback_model_main cannot be empty".into());
            }
            let cx2cc_model_reasoning_effort =
                cx2cc_model_reasoning_effort.unwrap_or(previous.cx2cc_model_reasoning_effort.clone());
            let cx2cc_service_tier =
                cx2cc_service_tier.unwrap_or(previous.cx2cc_service_tier.clone());
            let cx2cc_disable_response_storage =
                cx2cc_disable_response_storage.unwrap_or(previous.cx2cc_disable_response_storage);
            let cx2cc_enable_reasoning_to_thinking = cx2cc_enable_reasoning_to_thinking
                .unwrap_or(previous.cx2cc_enable_reasoning_to_thinking);
            let cx2cc_drop_stop_sequences =
                cx2cc_drop_stop_sequences.unwrap_or(previous.cx2cc_drop_stop_sequences);
            let cx2cc_clean_schema = cx2cc_clean_schema.unwrap_or(previous.cx2cc_clean_schema);
            let cx2cc_filter_batch_tool =
                cx2cc_filter_batch_tool.unwrap_or(previous.cx2cc_filter_batch_tool);
            let upstream_proxy_enabled =
                upstream_proxy_enabled.unwrap_or(previous.upstream_proxy_enabled);
            let upstream_proxy_url = upstream_proxy_url
                .unwrap_or(previous.upstream_proxy_url.clone())
                .trim()
                .to_string();
            let upstream_proxy_username = upstream_proxy_username
                .unwrap_or(previous.upstream_proxy_username.clone())
                .trim()
                .to_string();
            let upstream_proxy_password =
                apply_sensitive_string_update(upstream_proxy_password, previous.upstream_proxy_password.clone());
            if upstream_proxy_enabled && upstream_proxy_url.is_empty() {
                return Err(
                    "upstream_proxy_url cannot be empty when upstream proxy is enabled".into(),
                );
            }
            let provider_base_url_ping_cache_ttl_seconds = provider_base_url_ping_cache_ttl_seconds
                .unwrap_or(previous.provider_base_url_ping_cache_ttl_seconds);
            let upstream_first_byte_timeout_seconds = upstream_first_byte_timeout_seconds
                .unwrap_or(previous.upstream_first_byte_timeout_seconds);
            let upstream_stream_idle_timeout_seconds = upstream_stream_idle_timeout_seconds
                .unwrap_or(previous.upstream_stream_idle_timeout_seconds);
            let upstream_request_timeout_non_streaming_seconds =
                upstream_request_timeout_non_streaming_seconds
                    .unwrap_or(previous.upstream_request_timeout_non_streaming_seconds);
            let intercept_anthropic_warmup_requests = intercept_anthropic_warmup_requests
                .unwrap_or(previous.intercept_anthropic_warmup_requests);
            let enable_thinking_signature_rectifier = enable_thinking_signature_rectifier
                .unwrap_or(previous.enable_thinking_signature_rectifier);
            let enable_thinking_budget_rectifier = enable_thinking_budget_rectifier
                .unwrap_or(previous.enable_thinking_budget_rectifier);
            let enable_billing_header_rectifier =
                enable_billing_header_rectifier.unwrap_or(previous.enable_billing_header_rectifier);
            let enable_claude_metadata_user_id_injection = enable_claude_metadata_user_id_injection
                .unwrap_or(previous.enable_claude_metadata_user_id_injection);
            let enable_cache_anomaly_monitor =
                enable_cache_anomaly_monitor.unwrap_or(previous.enable_cache_anomaly_monitor);
            let enable_task_complete_notify =
                enable_task_complete_notify.unwrap_or(previous.enable_task_complete_notify);
            let enable_notification_sound =
                enable_notification_sound.unwrap_or(previous.enable_notification_sound);
            let enable_response_fixer =
                enable_response_fixer.unwrap_or(previous.enable_response_fixer);
            let response_fixer_fix_encoding =
                response_fixer_fix_encoding.unwrap_or(previous.response_fixer_fix_encoding);
            let response_fixer_fix_sse_format =
                response_fixer_fix_sse_format.unwrap_or(previous.response_fixer_fix_sse_format);
            let response_fixer_fix_truncated_json = response_fixer_fix_truncated_json
                .unwrap_or(previous.response_fixer_fix_truncated_json);
            let verbose_provider_error =
                verbose_provider_error.unwrap_or(previous.verbose_provider_error);
            let circuit_breaker_failure_threshold = circuit_breaker_failure_threshold
                .unwrap_or(previous.circuit_breaker_failure_threshold);
            let circuit_breaker_open_duration_minutes = circuit_breaker_open_duration_minutes
                .unwrap_or(previous.circuit_breaker_open_duration_minutes);
            let next_auto_start = crate::app::autostart::reconcile_auto_start(
                &app_for_work,
                previous.auto_start,
                auto_start,
                false,
            );

            let settings = settings::AppSettings {
                schema_version: settings::SCHEMA_VERSION,
                preferred_port,
                show_home_heatmap,
                show_home_usage,
                home_usage_period,
                gateway_listen_mode,
                gateway_custom_listen_address,
                wsl_auto_config,
                wsl_target_cli,
                cli_priority_order,
                wsl_host_address_mode,
                wsl_custom_host_address,
                codex_home_mode,
                codex_home_override,
                auto_start: next_auto_start,
                start_minimized,
                tray_enabled,
                enable_cli_proxy_startup_recovery,
                log_retention_days,
                provider_cooldown_seconds,
                provider_base_url_ping_cache_ttl_seconds,
                upstream_first_byte_timeout_seconds,
                upstream_stream_idle_timeout_seconds,
                upstream_request_timeout_non_streaming_seconds,
                update_releases_url,
                failover_max_attempts_per_provider,
                failover_max_providers_to_try,
                circuit_breaker_failure_threshold,
                circuit_breaker_open_duration_minutes,
                enable_circuit_breaker_notice: previous.enable_circuit_breaker_notice,
                verbose_provider_error,
                intercept_anthropic_warmup_requests,
                enable_thinking_signature_rectifier,
                enable_thinking_budget_rectifier,
                enable_billing_header_rectifier,
                enable_codex_session_id_completion: previous.enable_codex_session_id_completion,
                enable_claude_metadata_user_id_injection,
                enable_cache_anomaly_monitor,
                enable_task_complete_notify,
                enable_notification_sound,
                enable_response_fixer,
                response_fixer_fix_encoding,
                response_fixer_fix_sse_format,
                response_fixer_fix_truncated_json,
                response_fixer_max_json_depth: previous.response_fixer_max_json_depth,
                response_fixer_max_fix_size: previous.response_fixer_max_fix_size,
                cx2cc_fallback_model_opus,
                cx2cc_fallback_model_sonnet,
                cx2cc_fallback_model_haiku,
                cx2cc_fallback_model_main,
                cx2cc_model_reasoning_effort,
                cx2cc_service_tier,
                cx2cc_disable_response_storage,
                cx2cc_enable_reasoning_to_thinking,
                cx2cc_drop_stop_sequences,
                cx2cc_clean_schema,
                cx2cc_filter_batch_tool,
                upstream_proxy_enabled,
                upstream_proxy_url,
                upstream_proxy_username,
                upstream_proxy_password,
            };

            crate::gateway::http_client::validate_proxy_for_settings(&settings)?;
            Ok((previous, settings))
        },
    )
    .await?;

    let previous_gateway_status = current_gateway_status(&app);
    let runtime_plan = SettingsRuntimePlan::from_settings(&previous_settings, &candidate_settings);

    let mut gateway_status = current_gateway_status(&app);
    let mut gateway_rebound = false;
    let mut committed_settings = candidate_settings.clone();
    if runtime_plan.gateway_rebind_required && previous_gateway_status.running {
        crate::app::cleanup::stop_gateway_best_effort(&app).await;
        match start_gateway_with_settings(&app, db_state, &committed_settings).await {
            Ok(start_result) => {
                committed_settings.preferred_port = start_result.effective_preferred_port;
                gateway_status = start_result.status;
                gateway_rebound = true;
            }
            Err(rebind_error) => {
                tracing::error!(
                    error = %rebind_error,
                    "settings update failed during gateway rebind; restoring previous runtime"
                );
                restore_previous_runtime(
                    &app,
                    db_state,
                    &previous_settings,
                    &previous_gateway_status,
                )
                .await;
                return Err(format!(
                    "监听地址未生效，新的运行态重绑失败：{rebind_error}"
                ));
            }
        }
    } else if previous_gateway_status.running {
        gateway_status = current_gateway_status(&app);
    }

    let final_settings = match write_settings_snapshot(&app, &committed_settings).await {
        Ok(written) => written,
        Err(write_error) => {
            if gateway_rebound {
                rollback_settings_transaction(
                    &app,
                    db_state,
                    &previous_settings,
                    &previous_gateway_status,
                )
                .await;
            }
            return Err(if gateway_rebound {
                format!("监听地址重绑成功，但写入设置失败，已恢复旧配置：{write_error}")
            } else {
                format!("保存设置失败：{write_error}")
            });
        }
    };

    if let Err(sync_error) = sync_runtime_side_effects(&app, &final_settings) {
        if gateway_rebound {
            rollback_settings_transaction(
                &app,
                db_state,
                &previous_settings,
                &previous_gateway_status,
            )
            .await;
            return Err(format!(
                "监听地址重绑成功，但运行态提交失败，已恢复旧配置：{sync_error}"
            ));
        }

        let _ = write_settings_snapshot(&app, &previous_settings).await;
        let _ = sync_runtime_side_effects(&app, &previous_settings);
        return Err(format!("保存设置失败：{sync_error}"));
    }

    let cli_proxy_synced = if runtime_plan.cli_proxy_sync_required {
        let base_origin = if gateway_status.running {
            gateway_status.base_url.clone().unwrap_or_else(|| {
                format!(
                    "http://127.0.0.1:{}",
                    gateway_status.port.unwrap_or(final_settings.preferred_port)
                )
            })
        } else {
            crate::gateway::planned_base_url(&final_settings)?
        };
        sync_cli_proxy_for_settings(&app, base_origin, gateway_status.running).await
    } else {
        false
    };

    #[cfg(windows)]
    let wsl_auto_sync_triggered = if runtime_plan.wsl_auto_sync_required {
        match wsl_auto_sync_after_settings(&app).await {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!("WSL auto-sync after settings change failed: {}", err);
                false
            }
        }
    } else {
        false
    };
    #[cfg(not(windows))]
    let wsl_auto_sync_triggered = false;

    tracing::info!(
        preferred_port = final_settings.preferred_port,
        auto_start = final_settings.auto_start,
        tray_enabled = final_settings.tray_enabled,
        gateway_rebound,
        cli_proxy_synced,
        wsl_auto_sync_triggered,
        "settings updated"
    );

    Ok(SettingsMutationResult {
        settings: SettingsView::from(&final_settings),
        runtime: SettingsMutationRuntime {
            gateway_rebound,
            cli_proxy_synced,
            wsl_auto_sync_triggered,
            gateway_status,
        },
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_gateway_rectifier_set(
    app: tauri::AppHandle,
    update: GatewayRectifierSettingsUpdate,
) -> Result<SettingsView, String> {
    let app_for_work = app.clone();
    let result = blocking::run("settings_gateway_rectifier_set", move || {
        write_settings_view(&app_for_work, move |settings| {
            settings.verbose_provider_error = update.verbose_provider_error;
            settings.intercept_anthropic_warmup_requests =
                update.intercept_anthropic_warmup_requests;
            settings.enable_thinking_signature_rectifier =
                update.enable_thinking_signature_rectifier;
            settings.enable_thinking_budget_rectifier = update.enable_thinking_budget_rectifier;
            settings.enable_billing_header_rectifier = update.enable_billing_header_rectifier;
            settings.enable_claude_metadata_user_id_injection =
                update.enable_claude_metadata_user_id_injection;
            settings.enable_response_fixer = update.enable_response_fixer;
            settings.response_fixer_fix_encoding = update.response_fixer_fix_encoding;
            settings.response_fixer_fix_sse_format = update.response_fixer_fix_sse_format;
            settings.response_fixer_fix_truncated_json = update.response_fixer_fix_truncated_json;
            settings.response_fixer_max_json_depth = update.response_fixer_max_json_depth;
            settings.response_fixer_max_fix_size = update.response_fixer_max_fix_size;
            Ok(())
        })
    })
    .await
    .map_err(Into::into);

    if let Ok(ref settings) = result {
        tracing::info!(
            verbose_provider_error = settings.verbose_provider_error,
            intercept_anthropic_warmup_requests = settings.intercept_anthropic_warmup_requests,
            enable_thinking_signature_rectifier = settings.enable_thinking_signature_rectifier,
            enable_thinking_budget_rectifier = settings.enable_thinking_budget_rectifier,
            enable_billing_header_rectifier = settings.enable_billing_header_rectifier,
            enable_claude_metadata_user_id_injection =
                settings.enable_claude_metadata_user_id_injection,
            enable_response_fixer = settings.enable_response_fixer,
            "gateway rectifier settings updated"
        );
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_circuit_breaker_notice_set(
    app: tauri::AppHandle,
    update: CircuitBreakerNoticeUpdate,
) -> Result<SettingsView, String> {
    let app_for_work = app.clone();
    blocking::run("settings_circuit_breaker_notice_set", move || {
        write_settings_view(&app_for_work, move |settings| {
            settings.enable_circuit_breaker_notice = update.enable_circuit_breaker_notice;
            Ok(())
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn settings_codex_session_id_completion_set(
    app: tauri::AppHandle,
    update: CodexSessionIdCompletionUpdate,
) -> Result<SettingsView, String> {
    let app_for_work = app.clone();
    blocking::run("settings_codex_session_id_completion_set", move || {
        write_settings_view(&app_for_work, move |settings| {
            settings.enable_codex_session_id_completion = update.enable_codex_session_id_completion;
            Ok(())
        })
    })
    .await
    .map_err(Into::into)
}

/// Background WSL sync triggered after settings change.
/// Delegates to the shared `wsl_auto_sync_core` which handles all precondition checks.
#[cfg(windows)]
async fn wsl_auto_sync_after_settings(app: &tauri::AppHandle) -> Result<(), String> {
    super::wsl::wsl_auto_sync_core(app).await
}
