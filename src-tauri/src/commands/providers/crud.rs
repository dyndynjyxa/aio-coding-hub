use crate::app_state::{ensure_db_ready, DbInitState, GatewayState};
use crate::shared::mutex_ext::MutexExt;
use crate::{blocking, providers};

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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProviderRuntimeResetDecision {
    clear_session_bindings: bool,
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
        return ProviderRuntimeResetDecision::default();
    };

    let sensitive_config_changed = previous.base_urls != next.base_urls
        || previous.base_url_mode != next.base_url_mode
        || previous.auth_mode != next.auth_mode
        || submitted_api_key_changed(previous_api_key, submitted_api_key)
        || previous.source_provider_id != next.source_provider_id
        || previous.bridge_type != next.bridge_type;

    ProviderRuntimeResetDecision {
        clear_session_bindings: sensitive_config_changed,
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("providers_list", move || {
        providers::list_by_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    gateway_state: tauri::State<'_, GatewayState>,
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
    } = input;

    let is_create = provider_id.is_none();
    let name_for_log = name.clone();
    let cli_key_for_log = cli_key.clone();
    let submitted_api_key = api_key.clone();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_upsert", move || {
        let previous = match provider_id {
            Some(id) => {
                let conn = db.open_connection()?;
                Some(providers::get_by_id(&conn, id)?)
            }
            None => None,
        };
        let previous_api_key = match provider_id {
            Some(id) => Some(providers::get_api_key_plaintext(&db, id)?),
            None => None,
        };

        let saved = providers::upsert(
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
            },
        )?;

        let decision = provider_runtime_reset_decision(
            previous.as_ref(),
            previous_api_key.as_deref(),
            &saved,
            submitted_api_key.as_deref(),
        );

        Ok::<_, crate::shared::error::AppError>((saved, decision))
    })
    .await
    .map_err(Into::into);

    if let Ok((ref provider, decision)) = result {
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

        if decision.clear_session_bindings {
            let cleared_sessions = {
                let manager = gateway_state.0.lock_or_recover();
                manager.clear_cli_session_bindings(&provider.cli_key)
            };
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions,
                "provider runtime bindings cleared after sensitive update"
            );
        }
    }

    result.map(|(provider, _)| provider)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_duplicate(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_duplicate", move || {
        let conn = db.open_connection()?;
        let source = providers::get_by_id(&conn, provider_id)?;
        let siblings = providers::list_by_cli(&db, &source.cli_key)?;
        let api_key = if source.auth_mode == "api_key" && source.source_provider_id.is_none() {
            Some(providers::get_api_key_plaintext(&db, provider_id)?)
        } else {
            None
        };

        providers::upsert(
            &db,
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
            },
        )
    })
    .await
    .map_err(Into::into);

    if let Ok(ref provider) = result {
        tracing::info!(
            provider_id = provider.id,
            cli_key = %provider.cli_key,
            provider_name = %provider.name,
            "provider duplicated"
        );
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("provider_set_enabled", move || {
        providers::set_enabled(&db, provider_id, enabled)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref provider) = result {
        tracing::info!(
            provider_id = provider.id,
            enabled = provider.enabled,
            "provider enabled state changed"
        );
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run(
        "provider_delete",
        move || -> crate::shared::error::AppResult<bool> {
            providers::delete(&db, provider_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into);

    if let Ok(true) = result {
        tracing::info!(provider_id = provider_id, "provider deleted");
    }

    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    gateway_state: tauri::State<'_, GatewayState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let result = blocking::run("providers_reorder", move || {
        providers::reorder(&db, &cli_key, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref providers) = result {
        // Provider order changes must invalidate session-bound provider_order (default TTL=300s).
        let cleared = {
            let manager = gateway_state.0.lock_or_recover();
            manager.clear_cli_session_bindings(&cli_key_for_log)
        };
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = providers.len(),
            cleared_sessions = cleared,
            "providers reordered"
        );
    }

    result
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
    fn provider_runtime_reset_decision_ignores_create_and_non_sensitive_edits() {
        let next = providers::ProviderSummary {
            id: 1,
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
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
            api_key_configured: true,
        };

        assert_eq!(
            provider_runtime_reset_decision(None, None, &next, None),
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
            api_key_configured: true,
        };

        let mut next = previous.clone();
        next.base_urls = vec!["https://api.new.example.com".to_string()];

        assert_eq!(
            provider_runtime_reset_decision(Some(&previous), Some("sk-old"), &next, None),
            ProviderRuntimeResetDecision {
                clear_session_bindings: true,
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
                clear_session_bindings: true,
            }
        );
    }
}
