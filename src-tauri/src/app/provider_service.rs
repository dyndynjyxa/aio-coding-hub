use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_control::app_gateway_clear_cli_route_runtime_state;
use crate::{blocking, providers};

pub(crate) const PROVIDER_CODEX_CATALOG_EVENT_NAME: &str = "providers:codex_catalog";

#[derive(Clone, Copy, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CodexCatalogEventStatus {
    Updated,
    Failed,
}

#[derive(Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexCatalogEventPayload {
    pub(crate) status: CodexCatalogEventStatus,
}

/// Serializes catalog refresh tasks: each task reads the DB only after acquiring
/// the lock, so the last one to run always writes the freshest catalog and a slow
/// older task cannot overwrite a newer one.
static CODEX_CATALOG_REFRESH_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();

/// Fire-and-forget: the refresh spawns `codex debug models --bundled` (up to 20s), so commands
/// must not wait on it. Success/failure feedback reaches the UI via the codex_catalog event.
pub(crate) fn refresh_codex_catalog_after_routing_change(
    app: &tauri::AppHandle,
    db: crate::db::Db,
    mapping_sources_changed: bool,
) {
    if !mapping_sources_changed {
        return;
    }
    spawn_codex_catalog_refresh(app, db);
}

/// Fire-and-forget catalog refresh; also used after gateway start so a DB that
/// changed while the gateway was off still reaches the Codex catalog without
/// blocking startup.
pub(crate) fn spawn_codex_catalog_refresh<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: crate::db::Db,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = CODEX_CATALOG_REFRESH_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        let refresh_app = app.clone();
        let result = blocking::run("refresh_codex_model_catalog", move || {
            crate::cli_proxy::refresh_codex_model_catalog_if_enabled(&refresh_app, &db)
        })
        .await;

        let status = match result {
            Ok(crate::cli_proxy::CodexCatalogRefreshResult::Updated) => {
                tracing::info!("Codex model capability catalog refreshed after routing change");
                Some(CodexCatalogEventStatus::Updated)
            }
            Ok(crate::cli_proxy::CodexCatalogRefreshResult::NotActive)
            | Ok(crate::cli_proxy::CodexCatalogRefreshResult::Unchanged) => None,
            Err(error) => {
                tracing::warn!(
                    error_code = "CLI_PROXY_CODEX_CATALOG_FAILED",
                    error = %error,
                    "failed to refresh Codex model capability catalog after routing change"
                );
                Some(CodexCatalogEventStatus::Failed)
            }
        };

        if let Some(status) = status {
            crate::app::heartbeat_watchdog::gated_emit(
                &app,
                PROVIDER_CODEX_CATALOG_EVENT_NAME,
                CodexCatalogEventPayload { status },
            );
        }
    });
}

/// Same ordering guarantee as the Codex catalog lock, for the Desktop profile.
static CLAUDE_DESKTOP_MODELS_REFRESH_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();

pub(crate) fn refresh_claude_desktop_models_after_routing_change(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
    cli_key: &str,
) {
    if cli_key == "claude_desktop" {
        spawn_claude_desktop_models_refresh(app, db.clone());
    }
}

/// Fire-and-forget: keeps the Desktop profile's 1M model variants in line with
/// the providers' 1M checkbox. Desktop picks the change up on its next launch.
pub(crate) fn spawn_claude_desktop_models_refresh<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: crate::db::Db,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = CLAUDE_DESKTOP_MODELS_REFRESH_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        let result = blocking::run("refresh_claude_desktop_models", move || {
            crate::cli_proxy::refresh_claude_desktop_models_if_enabled(&app, &db)
        })
        .await;
        match result {
            Ok(true) => tracing::info!("Claude Desktop 1M model variants refreshed"),
            Ok(false) => {}
            Err(error) => tracing::warn!(
                error_code = "CLI_PROXY_CLAUDE_DESKTOP_MODELS_FAILED",
                error = %error,
                "failed to refresh Claude Desktop model list after routing change"
            ),
        }
    });
}

/// Runs `mutation`, comparing the codex routable mapping signature before/after when
/// `tracks_codex_mappings` is set. Returns the mutation result plus whether the mapping
/// sources changed (i.e. the codex catalog needs a refresh).
pub(crate) fn with_codex_mapping_tracking<T>(
    db: &crate::db::Db,
    tracks_codex_mappings: bool,
    mutation: impl FnOnce() -> crate::shared::error::AppResult<T>,
) -> crate::shared::error::AppResult<(T, bool)> {
    let before = tracks_codex_mappings
        .then(|| crate::infra::codex_model_catalog::projection::routable_mapping_signature(db))
        .transpose()?;
    let value = mutation()?;
    let changed = match before {
        Some(before) => {
            before != crate::infra::codex_model_catalog::projection::routable_mapping_signature(db)?
        }
        None => false,
    };
    Ok((value, changed))
}

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderUpsertInput {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: providers::ProviderBaseUrlMode,
    pub auth_mode: Option<providers::ProviderAuthMode>,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cost_multiplier: f64,
    pub priority: Option<i64>,
    pub claude_models: Option<providers::ClaudeModels>,
    pub model_policy: Option<providers::ProviderModelPolicyV1>,
    #[serde(rename = "limit5hUsd", alias = "limit5HUsd")]
    #[specta(rename = "limit5hUsd")]
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<providers::DailyResetMode>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub stream_idle_timeout_seconds: Option<u32>,
    pub extension_values: Option<Vec<providers::ProviderExtensionValuesInput>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProviderRuntimeResetDecision {
    clear_route_runtime_state: bool,
}

fn normalize_provider_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn build_duplicated_provider_name(
    source_name: &str,
    existing_providers: &[providers::ProviderSummary],
) -> String {
    let base_name = format!("{} 副本", source_name.trim());
    let used_names: std::collections::HashSet<String> = existing_providers
        .iter()
        .map(|provider| normalize_provider_name(&provider.name))
        .collect();

    if !used_names.contains(&normalize_provider_name(&base_name)) {
        return base_name;
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base_name} {index}");
        if !used_names.contains(&normalize_provider_name(&candidate)) {
            return candidate;
        }
        index += 1;
    }
}

fn submitted_api_key_changed(
    previous_api_key: Option<&str>,
    submitted_api_key: Option<&str>,
) -> bool {
    let Some(submitted) = submitted_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
    else {
        return false;
    };

    previous_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        != Some(submitted)
}

fn provider_runtime_reset_decision(
    previous: Option<&providers::ProviderSummary>,
    previous_api_key: Option<&str>,
    next: &providers::ProviderSummary,
    submitted_api_key: Option<&str>,
) -> ProviderRuntimeResetDecision {
    let Some(previous) = previous else {
        return ProviderRuntimeResetDecision {
            clear_route_runtime_state: next.enabled,
        };
    };

    let sensitive_config_changed = previous.base_urls != next.base_urls
        || previous.base_url_mode != next.base_url_mode
        || previous.enabled != next.enabled
        || previous.auth_mode != next.auth_mode
        || submitted_api_key_changed(previous_api_key, submitted_api_key)
        || previous.source_provider_id != next.source_provider_id
        || previous.bridge_type != next.bridge_type
        || previous.model_policy != next.model_policy
        || previous.model_policy_status != next.model_policy_status;

    ProviderRuntimeResetDecision {
        clear_route_runtime_state: sensitive_config_changed,
    }
}

pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("providers_list", move || {
        providers::list_by_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: ProviderUpsertInput,
) -> Result<providers::ProviderSummary, String> {
    let ProviderUpsertInput {
        provider_id,
        cli_key,
        name,
        base_urls,
        base_url_mode,
        auth_mode,
        api_key,
        enabled,
        cost_multiplier,
        priority,
        claude_models,
        model_policy,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        tags,
        note,
        source_provider_id,
        bridge_type,
        stream_idle_timeout_seconds,
        extension_values,
    } = input;

    let is_create = provider_id.is_none();
    let name_for_log = name.clone();
    let cli_key_for_log = cli_key.clone();
    let submitted_api_key = api_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let result = blocking::run("provider_upsert", move || {
        let previous = match provider_id {
            Some(id) => {
                let conn = db.open_connection()?;
                Some(providers::get_by_id(&conn, id)?)
            }
            None => None,
        };
        // Updates reject cli_key changes, so `previous.cli_key` never differs here.
        let tracks_codex_mappings = cli_key == "codex";
        let previous_api_key = match provider_id {
            Some(id) => Some(providers::get_api_key_plaintext(&db, id)?),
            None => None,
        };

        let (saved, mapping_sources_changed) =
            with_codex_mapping_tracking(&db, tracks_codex_mappings, || {
                providers::upsert(
                    &db,
                    providers::ProviderUpsertParams {
                        provider_id,
                        cli_key,
                        name,
                        base_urls,
                        base_url_mode,
                        auth_mode,
                        api_key,
                        enabled,
                        cost_multiplier,
                        priority,
                        claude_models,
                        model_policy,
                        limit_5h_usd,
                        limit_daily_usd,
                        daily_reset_mode,
                        daily_reset_time,
                        limit_weekly_usd,
                        limit_monthly_usd,
                        limit_total_usd,
                        tags,
                        note,
                        source_provider_id,
                        bridge_type,
                        stream_idle_timeout_seconds,
                        extension_values,
                    },
                )
            })?;

        let decision = provider_runtime_reset_decision(
            previous.as_ref(),
            previous_api_key.as_deref(),
            &saved,
            submitted_api_key.as_deref(),
        );

        Ok::<_, crate::shared::error::AppError>((saved, decision, mapping_sources_changed))
    })
    .await
    .map_err(String::from);

    if let Ok((ref provider, decision, _)) = result {
        if is_create {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider created"
            );
        } else {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider updated"
            );
        }

        if decision.clear_route_runtime_state {
            let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions = cleared.cleared_sessions,
                cleared_recent_errors = cleared.cleared_recent_errors,
                "provider route runtime state cleared after provider save"
            );
        }
    }

    let (provider, _, mapping_sources_changed) = result?;
    refresh_claude_desktop_models_after_routing_change(&app, &refresh_db, &provider.cli_key);
    refresh_codex_catalog_after_routing_change(&app, refresh_db, mapping_sources_changed);
    Ok(provider)
}

pub(crate) async fn provider_duplicate(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let result = blocking::run("provider_duplicate", move || {
        let source = {
            let conn = db.open_connection()?;
            providers::get_by_id(&conn, provider_id)?
        };
        let siblings = providers::list_by_cli(&db, &source.cli_key)?;
        if source.model_policy_status == providers::ProviderModelPolicyStatus::Invalid {
            return Err(crate::shared::error::AppError::from(
                "SEC_INVALID_INPUT: reset the invalid provider model policy before duplicating",
            ));
        }
        let api_key = if source.auth_mode == "api_key" && source.source_provider_id.is_none() {
            Some(providers::get_api_key_plaintext(&db, provider_id)?)
        } else {
            None
        };

        // Duplicates never join a route, so they cannot change the routable
        // mapping signature — skip the two full policy scans.
        let (provider, mapping_sources_changed) = with_codex_mapping_tracking(&db, false, || {
            providers::duplicate(
                &db,
                source.id,
                providers::ProviderUpsertParams {
                    provider_id: None,
                    cli_key: source.cli_key.clone(),
                    name: build_duplicated_provider_name(&source.name, &siblings),
                    base_urls: source.base_urls.clone(),
                    base_url_mode: source.base_url_mode,
                    auth_mode: match source.auth_mode.as_str() {
                        "oauth" => Some(providers::ProviderAuthMode::Oauth),
                        _ => Some(providers::ProviderAuthMode::ApiKey),
                    },
                    api_key,
                    enabled: source.enabled,
                    cost_multiplier: source.cost_multiplier,
                    priority: None,
                    claude_models: Some(source.claude_models.clone()),
                    model_policy: source.model_policy.clone(),
                    limit_5h_usd: source.limit_5h_usd,
                    limit_daily_usd: source.limit_daily_usd,
                    daily_reset_mode: Some(source.daily_reset_mode),
                    daily_reset_time: Some(source.daily_reset_time.clone()),
                    limit_weekly_usd: source.limit_weekly_usd,
                    limit_monthly_usd: source.limit_monthly_usd,
                    limit_total_usd: source.limit_total_usd,
                    tags: Some(source.tags.clone()),
                    note: Some(source.note.clone()),
                    source_provider_id: source.source_provider_id,
                    bridge_type: source.bridge_type.clone(),
                    stream_idle_timeout_seconds: source.stream_idle_timeout_seconds,
                    extension_values: None,
                },
            )
        })?;
        Ok::<_, crate::shared::error::AppError>((provider, mapping_sources_changed))
    })
    .await
    .map_err(String::from);

    if let Ok((ref provider, _)) = result {
        if provider.enabled {
            let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions = cleared.cleared_sessions,
                cleared_recent_errors = cleared.cleared_recent_errors,
                "provider route runtime state cleared after duplicate"
            );
        }

        tracing::info!(
            provider_id = provider.id,
            cli_key = %provider.cli_key,
            provider_name = %provider.name,
            "provider duplicated"
        );
    }

    let (provider, mapping_sources_changed) = result?;
    refresh_codex_catalog_after_routing_change(&app, refresh_db, mapping_sources_changed);
    Ok(provider)
}

pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let result = blocking::run("provider_set_enabled", move || {
        let tracks_codex_mappings =
            providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found")
            })? == "codex";
        with_codex_mapping_tracking(&db, tracks_codex_mappings, || {
            providers::set_enabled(&db, provider_id, enabled)
        })
    })
    .await
    .map_err(String::from);

    if let Ok((ref provider, _)) = result {
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
        tracing::info!(
            provider_id = provider.id,
            enabled = provider.enabled,
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "provider enabled state changed"
        );
    }

    let (provider, mapping_sources_changed) = result?;
    refresh_claude_desktop_models_after_routing_change(&app, &refresh_db, &provider.cli_key);
    refresh_codex_catalog_after_routing_change(&app, refresh_db, mapping_sources_changed);
    Ok(provider)
}

pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    clear_usage_stats: bool,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let result = blocking::run(
        "provider_delete",
        move || -> crate::shared::error::AppResult<(bool, String, bool)> {
            let cli_key = providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found")
            })?;
            let (_, mapping_sources_changed) =
                with_codex_mapping_tracking(&db, cli_key == "codex", || {
                    providers::delete(&db, provider_id, clear_usage_stats)
                })?;
            Ok((true, cli_key, mapping_sources_changed))
        },
    )
    .await
    .map_err(String::from);

    if let Ok((true, ref cli_key, _)) = result {
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, cli_key);
        tracing::info!(
            provider_id = provider_id,
            cli_key = %cli_key,
            clear_usage_stats = clear_usage_stats,
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "provider deleted"
        );
    }

    let (deleted, cli_key, mapping_sources_changed) = result?;
    refresh_claude_desktop_models_after_routing_change(&app, &refresh_db, &cli_key);
    refresh_codex_catalog_after_routing_change(&app, refresh_db, mapping_sources_changed);
    Ok(deleted)
}

pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("providers_reorder", move || {
        providers::reorder(&db, &cli_key, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref providers) = result {
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = providers.len(),
            "provider pool display order updated"
        );
    }

    result
}

pub(crate) async fn default_route_providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderRouteRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("default_route_providers_list", move || {
        providers::default_route_list(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn default_route_providers_set_order(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderRouteRow>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let result = blocking::run("default_route_providers_set_order", move || {
        with_codex_mapping_tracking(&db, cli_key == "codex", || {
            providers::default_route_set_order(&db, &cli_key, ordered_provider_ids)
        })
    })
    .await
    .map_err(String::from);

    if let Ok((ref rows, _)) = result {
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, &cli_key_for_log);
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = rows.len(),
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "default route provider order updated"
        );
    }

    let (rows, mapping_sources_changed) = result?;
    refresh_claude_desktop_models_after_routing_change(&app, &refresh_db, &cli_key_for_log);
    refresh_codex_catalog_after_routing_change(&app, refresh_db, mapping_sources_changed);
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_upsert_input_deserializes_runtime_camel_case_shape() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "order",
            "authMode": "api_key",
            "apiKey": "k1",
            "enabled": true,
            "costMultiplier": 1.0,
            "priority": 10,
            "claudeModels": null,
            "limit5hUsd": 5.0,
            "limitDailyUsd": 10.0,
            "dailyResetMode": "fixed",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null,
            "tags": ["x"],
            "note": "n",
            "streamIdleTimeoutSeconds": 90
        }))
        .expect("deserialize provider input");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Order);
        assert_eq!(input.auth_mode, Some(providers::ProviderAuthMode::ApiKey));
        assert_eq!(input.limit_5h_usd, Some(5.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Fixed)
        );
        assert_eq!(input.stream_idle_timeout_seconds, Some(90));
    }

    #[test]
    fn provider_upsert_input_accepts_legacy_generated_limit_alias() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "ping",
            "enabled": true,
            "costMultiplier": 1.0,
            "limit5HUsd": 7.0,
            "limitDailyUsd": null,
            "dailyResetMode": "rolling",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null
        }))
        .expect("deserialize provider input legacy alias");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Ping);
        assert_eq!(input.limit_5h_usd, Some(7.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Rolling)
        );
    }

    #[test]
    fn provider_runtime_reset_decision_handles_create_and_non_sensitive_edits() {
        let next = providers::ProviderSummary {
            id: 1,
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            model_policy: Some(providers::ProviderModelPolicyV1::all()),
            model_policy_status: providers::ProviderModelPolicyStatus::Ready,
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: vec![],
            api_key_configured: true,
        };

        assert_eq!(
            provider_runtime_reset_decision(None, None, &next, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut disabled_create = next.clone();
        disabled_create.enabled = false;
        assert_eq!(
            provider_runtime_reset_decision(None, None, &disabled_create, None),
            ProviderRuntimeResetDecision::default()
        );

        let mut previous = next.clone();
        previous.name = "Old Name".to_string();
        previous.note = "old".to_string();
        previous.updated_at = 0;

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("   ")
            ),
            ProviderRuntimeResetDecision::default()
        );

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("sk-existing")
            ),
            ProviderRuntimeResetDecision::default()
        );

        let mut disabled = next.clone();
        disabled.enabled = false;

        assert_eq!(
            provider_runtime_reset_decision(Some(&next), Some("sk-existing"), &disabled, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );
    }

    #[test]
    fn provider_runtime_reset_decision_detects_sensitive_claude_changes() {
        let previous = providers::ProviderSummary {
            id: 1,
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.old.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            model_policy: Some(providers::ProviderModelPolicyV1::all()),
            model_policy_status: providers::ProviderModelPolicyStatus::Ready,
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: vec![],
            api_key_configured: true,
        };

        let mut next = previous.clone();
        next.base_urls = vec!["https://api.new.example.com".to_string()];

        assert_eq!(
            provider_runtime_reset_decision(Some(&previous), Some("sk-old"), &next, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut policy_changed = previous.clone();
        policy_changed.model_policy = Some(providers::ProviderModelPolicyV1 {
            version: 1,
            mode: providers::ProviderModelMode::Selected,
            model_patterns: vec!["claude-sonnet-*".to_string()],
            mappings: vec![],
            supports_1m: false,
        });
        assert_eq!(
            provider_runtime_reset_decision(Some(&previous), Some("sk-old"), &policy_changed, None,),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut legacy = previous.clone();
        legacy.model_policy = None;
        legacy.model_policy_status = providers::ProviderModelPolicyStatus::Legacy;
        assert_eq!(
            provider_runtime_reset_decision(Some(&legacy), Some("sk-old"), &previous, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut next_non_claude = previous.clone();
        next_non_claude.cli_key = "codex".to_string();

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&next_non_claude),
                Some("sk-old"),
                &next_non_claude,
                Some("sk-new")
            ),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );
    }
}
