use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;

use super::oauth::{
    effective_oauth_access_token, oauth_details_can_refresh, refresh_oauth_details_for_limits,
    should_retry_oauth_limits_after_refresh,
};

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthLimitsResult {
    pub limit_short_label: Option<String>,
    pub limit_5h_text: Option<String>,
    pub limit_weekly_text: Option<String>,
    pub limit_5h_reset_at: Option<i64>,
    pub limit_weekly_reset_at: Option<i64>,
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_fetch_limits(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<ProviderOAuthLimitsResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let mut details = blocking::run("provider_oauth_fetch_limits_load", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(&details)?;
    let client = crate::gateway::oauth::build_oauth_http_client(
        &format!("aio-coding-hub-oauth-command/{}", env!("CARGO_PKG_VERSION")),
        15,
        10,
    )?;

    if oauth_details_can_refresh(&details)
        && crate::gateway::oauth::refresh::should_refresh_now(
            details.oauth_expires_at,
            details.oauth_refresh_lead_s,
        )
    {
        match refresh_oauth_details_for_limits(&db, &client, &details, adapter).await {
            Ok(refreshed) => details = refreshed,
            Err(err) => {
                let now_unix = crate::shared::time::now_unix_seconds();
                let still_valid = details
                    .oauth_expires_at
                    .map(|expires_at| expires_at > now_unix)
                    .unwrap_or(false);
                if still_valid {
                    tracing::warn!(
                        provider_id = details.id,
                        cli_key = %details.cli_key,
                        "provider_oauth_fetch_limits: proactive refresh failed, using existing token: {err}"
                    );
                } else {
                    return Err(err);
                }
            }
        }
    }

    let token = effective_oauth_access_token(&details, adapter)?;
    let limits = match adapter.fetch_limits(&client, &token).await {
        Ok(limits) => limits,
        Err(err) => {
            let err_str = format!("fetch_limits failed: {err}");
            if should_retry_oauth_limits_after_refresh(&err_str)
                && oauth_details_can_refresh(&details)
            {
                let refreshed =
                    refresh_oauth_details_for_limits(&db, &client, &details, adapter).await?;
                let refreshed_token = effective_oauth_access_token(&refreshed, adapter)?;
                adapter
                    .fetch_limits(&client, &refreshed_token)
                    .await
                    .map_err(|retry_err| format!("fetch_limits failed: {retry_err}"))?
            } else {
                return Err(err_str);
            }
        }
    };

    let limit_short_label =
        normalize_oauth_short_window_label(adapter.cli_key(), limits.limit_short_label.as_deref());

    // If the adapter already parsed limit texts, use them directly.
    // Otherwise, try to parse from raw_json based on cli_key.
    let (limit_5h_text, limit_weekly_text, limit_5h_reset_at, limit_weekly_reset_at) =
        if limits.limit_5h_text.is_some() || limits.limit_weekly_text.is_some() {
            let resets = limits
                .raw_json
                .as_ref()
                .map(extract_reset_timestamps)
                .unwrap_or((None, None));
            (
                limits.limit_5h_text.clone(),
                limits.limit_weekly_text.clone(),
                resets.0,
                resets.1,
            )
        } else if let Some(ref raw) = limits.raw_json {
            let cli_key = adapter.cli_key();
            let (text_5h, text_weekly) = match cli_key {
                "codex" => parse_codex_limits(raw),
                "claude" => parse_claude_limits(raw),
                _ => (None, None),
            };
            let resets = extract_reset_timestamps(raw);
            (text_5h, text_weekly, resets.0, resets.1)
        } else {
            (None, None, None, None)
        };

    Ok(ProviderOAuthLimitsResult {
        limit_short_label,
        limit_5h_text,
        limit_weekly_text,
        limit_5h_reset_at,
        limit_weekly_reset_at,
    })
}

fn default_oauth_short_window_label(cli_key: &str) -> Option<String> {
    match cli_key {
        "codex" | "claude" => Some("5h".to_string()),
        "gemini" => Some("短窗".to_string()),
        _ => None,
    }
}

fn normalize_oauth_short_window_label(
    cli_key: &str,
    adapter_label: Option<&str>,
) -> Option<String> {
    let adapter_label = adapter_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    match cli_key {
        "gemini" => Some("短窗".to_string()),
        _ => adapter_label.or_else(|| default_oauth_short_window_label(cli_key)),
    }
}

fn parse_remaining_percent_from_window(window: &serde_json::Value) -> Option<f64> {
    if !window.is_object() {
        return None;
    }
    if let Some(used) = window
        .get("used_percent")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            window
                .get("usedPercent")
                .and_then(serde_json::Value::as_f64)
        })
    {
        let remaining = (100.0 - used).clamp(0.0, 100.0);
        return Some(remaining);
    }
    let remaining = window
        .get("remaining_count")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            window
                .get("remainingCount")
                .and_then(serde_json::Value::as_f64)
        });
    let total = window
        .get("total_count")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| window.get("totalCount").and_then(serde_json::Value::as_f64));
    match (remaining, total) {
        (Some(rem), Some(t)) if t > 0.0 => Some((rem / t * 100.0).clamp(0.0, 100.0)),
        _ => None,
    }
}

fn format_percent_label(value: f64) -> String {
    format!("{:.0}%", value.clamp(0.0, 100.0))
}

fn resolve_rate_windows(
    body: &serde_json::Value,
) -> (Option<&serde_json::Value>, Option<&serde_json::Value>) {
    let rate_limit = body.get("rate_limit").unwrap_or(body);
    let primary = rate_limit
        .get("primary_window")
        .or_else(|| rate_limit.get("primaryWindow"))
        .or_else(|| body.get("five_hour"))
        .or_else(|| body.get("5_hour_window"))
        .or_else(|| body.get("fiveHourWindow"));
    let secondary = rate_limit
        .get("secondary_window")
        .or_else(|| rate_limit.get("secondaryWindow"))
        .or_else(|| body.get("seven_day"))
        .or_else(|| body.get("weekly_window"))
        .or_else(|| body.get("weeklyWindow"));
    (primary, secondary)
}

fn parse_codex_limits(body: &serde_json::Value) -> (Option<String>, Option<String>) {
    let (primary, secondary) = resolve_rate_windows(body);

    let limit_5h = primary
        .and_then(parse_remaining_percent_from_window)
        .map(format_percent_label);
    let limit_weekly = secondary
        .and_then(parse_remaining_percent_from_window)
        .map(format_percent_label);
    (limit_5h, limit_weekly)
}

fn parse_claude_limits(body: &serde_json::Value) -> (Option<String>, Option<String>) {
    fn extract_utilization(window: &serde_json::Value) -> Option<f64> {
        window
            .get("utilization")
            .and_then(serde_json::Value::as_f64)
            .or_else(|| {
                window
                    .get("utilization")
                    .and_then(serde_json::Value::as_str)?
                    .parse::<f64>()
                    .ok()
            })
    }

    let limit_5h = body
        .get("five_hour")
        .and_then(extract_utilization)
        .map(|used| format_percent_label(100.0 - used));
    let limit_weekly = body
        .get("seven_day")
        .and_then(extract_utilization)
        .map(|used| format_percent_label(100.0 - used));
    (limit_5h, limit_weekly)
}
fn extract_reset_timestamp(window: &serde_json::Value) -> Option<i64> {
    window
        .get("reset_at")
        .or_else(|| window.get("resetAt"))
        .or_else(|| window.get("resets_at"))
        .or_else(|| window.get("resetsAt"))
        .and_then(serde_json::Value::as_i64)
}

fn extract_reset_timestamps(body: &serde_json::Value) -> (Option<i64>, Option<i64>) {
    let (primary, secondary) = resolve_rate_windows(body);
    (
        primary.and_then(extract_reset_timestamp),
        secondary.and_then(extract_reset_timestamp),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_oauth_short_window_label_forces_gemini_to_short_window() {
        assert_eq!(
            normalize_oauth_short_window_label("gemini", Some("1h")).as_deref(),
            Some("短窗")
        );
        assert_eq!(
            normalize_oauth_short_window_label("gemini", None).as_deref(),
            Some("短窗")
        );
        assert_eq!(
            normalize_oauth_short_window_label("codex", Some("custom")).as_deref(),
            Some("custom")
        );
    }

    #[test]
    fn resolve_rate_windows_prefers_rate_limit_windows_and_supports_fallback_shapes() {
        let nested = serde_json::json!({
            "rate_limit": {
                "primaryWindow": { "remaining_count": 1, "total_count": 2 },
                "secondary_window": { "remaining_count": 3, "total_count": 4 }
            },
            "five_hour": { "remaining_count": 9, "total_count": 10 },
            "weekly_window": { "remaining_count": 8, "total_count": 10 }
        });
        let (primary, secondary) = resolve_rate_windows(&nested);
        assert_eq!(
            primary.and_then(parse_remaining_percent_from_window),
            Some(50.0)
        );
        assert_eq!(
            secondary.and_then(parse_remaining_percent_from_window),
            Some(75.0)
        );

        let fallback = serde_json::json!({
            "five_hour": { "remaining_count": 2, "total_count": 8 },
            "weekly_window": { "remaining_count": 1, "total_count": 4 }
        });
        let (primary, secondary) = resolve_rate_windows(&fallback);
        assert_eq!(
            primary.and_then(parse_remaining_percent_from_window),
            Some(25.0)
        );
        assert_eq!(
            secondary.and_then(parse_remaining_percent_from_window),
            Some(25.0)
        );
    }

    #[test]
    fn parse_codex_limits_supports_five_hour_fallback_window_shape() {
        let body = serde_json::json!({
            "five_hour": { "remaining_count": 1, "total_count": 2 },
            "weekly_window": { "remaining_count": 3, "total_count": 4 }
        });

        let (limit_5h, limit_weekly) = parse_codex_limits(&body);

        assert_eq!(limit_5h.as_deref(), Some("50%"));
        assert_eq!(limit_weekly.as_deref(), Some("75%"));
    }

    #[test]
    fn oauth_limits_fetch_error_requires_refresh_on_auth_failures() {
        assert!(should_retry_oauth_limits_after_refresh(
            "fetch_limits failed: claude limits fetch status: 401 Unauthorized"
        ));
        assert!(should_retry_oauth_limits_after_refresh(
            "fetch_limits failed: codex limits fetch status: 403 Forbidden"
        ));
    }

    #[test]
    fn oauth_limits_fetch_error_ignores_non_auth_failures() {
        assert!(!should_retry_oauth_limits_after_refresh(
            "fetch_limits failed: claude limits fetch status: 500 Internal Server Error"
        ));
        assert!(!should_retry_oauth_limits_after_refresh(
            "fetch_limits failed: gemini limits fetch could not resolve a quota project"
        ));
    }
}
