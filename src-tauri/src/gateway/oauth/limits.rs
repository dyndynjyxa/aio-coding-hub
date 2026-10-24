//! Usage: Shared OAuth quota fetching, parsing and token-bound snapshot persistence.

use crate::blocking;
use crate::domain::provider_oauth_limits::{
    self as domain, OAuthLimitGate, OAuthLimitSnapshotInput, ProviderOAuthCreditBalance,
};
use crate::domain::provider_oauth_quota_estimate::{
    estimate_window, OAuthQuotaWindow, ProviderOAuthQuotaEstimates,
};
use crate::shared::error::db_err;
use rusqlite::OptionalExtension;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

pub(crate) use super::adapters::codex::{apply_codex_quota_headers, CODEX_USAGE_URL};

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthLimitsResult {
    pub limit_short_label: Option<String>,
    pub limit_5h_text: Option<String>,
    pub limit_weekly_text: Option<String>,
    pub limit_5h_reset_at: Option<i64>,
    pub limit_weekly_reset_at: Option<i64>,
    pub reset_credit_available_count: Option<i64>,
    pub credits: Option<ProviderOAuthCreditBalance>,
    pub limit_5h_remaining_percent: Option<f64>,
    pub limit_weekly_remaining_percent: Option<f64>,
    pub routing_limited: bool,
    pub quota_estimates: Option<ProviderOAuthQuotaEstimates>,
    pub limit_window_seconds: (Option<i64>, Option<i64>),
    #[serde(skip)]
    pub usage_limit_reached: bool,
    #[serde(skip)]
    pub estimate_snapshot_identity: Option<(String, i64)>,
}

// One provider lock spans refresh, usage and reset even when refresh rotates the
// token. Completed results remain token-bound and live only while callers wait.
pub(crate) type LimitFlightResult = Option<(String, Result<ProviderOAuthLimitsResult, String>)>;
type LimitFlight = tokio::sync::Mutex<LimitFlightResult>;

fn provider_flight(provider_id: i64) -> Arc<LimitFlight> {
    static FLIGHTS: OnceLock<Mutex<HashMap<i64, Weak<LimitFlight>>>> = OnceLock::new();
    let mut flights = FLIGHTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    flights.retain(|_, flight| flight.strong_count() > 0);
    if let Some(flight) = flights.get(&provider_id).and_then(Weak::upgrade) {
        return flight;
    }
    let flight = Arc::new(tokio::sync::Mutex::new(None));
    flights.insert(provider_id, Arc::downgrade(&flight));
    flight
}

pub(crate) async fn lock_for_reset(
    provider_id: i64,
) -> Result<tokio::sync::OwnedMutexGuard<LimitFlightResult>, String> {
    tokio::time::timeout(
        Duration::from_secs(50),
        provider_flight(provider_id).lock_owned(),
    )
    .await
    .map_err(|_| "OAuth quota reset wait timed out".to_string())
}

pub(crate) async fn fetch_and_save<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    provider_id: i64,
) -> Result<ProviderOAuthLimitsResult, String> {
    let mut result = run_fetch(app, db, provider_id, true)
        .await?
        .ok_or_else(|| "OAuth quota fetch returned no snapshot".to_string())?;
    attach_quota_estimates(db, provider_id, &mut result).await;
    Ok(result)
}

pub(crate) async fn refresh_if_stale<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    provider_id: i64,
) -> Result<(), String> {
    run_fetch(app, db, provider_id, false).await.map(|_| ())
}

async fn needs_refresh(db: &crate::db::Db, provider_id: i64) -> Result<bool, String> {
    blocking::run("oauth_limits_check_freshness", {
        let db = db.clone();
        move || -> crate::shared::error::AppResult<bool> {
            let conn = db.open_connection()?;
            let policy = domain::read_policy(&conn, provider_id)?;
            if policy.oauth_min_remaining_percent.is_none() && !policy.oauth_use_credits {
                return Ok(false);
            }
            let snapshot = domain::read_snapshot(&conn, provider_id)?;
            Ok(snapshot
                .is_none_or(|snapshot| !snapshot.is_fresh(crate::shared::time::now_unix_seconds())))
        }
    })
    .await
    .map_err(Into::<String>::into)
}

async fn run_fetch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    provider_id: i64,
    force: bool,
) -> Result<Option<ProviderOAuthLimitsResult>, String> {
    if !force && !needs_refresh(db, provider_id).await? {
        return Ok(None);
    }
    share_fetch(db, provider_id, |mut details| async move {
        let result = async {
            // Re-read inside the stable provider lock after any preceding reset.
            if !force && !needs_refresh(db, provider_id).await? {
                return Ok(None);
            }
            fetch_and_save_locked(app, db, &mut details).await.map(Some)
        }
        .await;
        (details.oauth_access_token, result)
    })
    .await
}

async fn share_fetch<F, Fut>(
    db: &crate::db::Db,
    provider_id: i64,
    operation: F,
) -> Result<Option<ProviderOAuthLimitsResult>, String>
where
    F: FnOnce(crate::providers::ProviderOAuthDetails) -> Fut,
    Fut: std::future::Future<Output = (String, Result<Option<ProviderOAuthLimitsResult>, String>)>,
{
    let flight = provider_flight(provider_id);
    let mut cached = tokio::time::timeout(Duration::from_secs(50), flight.lock())
        .await
        .map_err(|_| "OAuth quota refresh wait timed out".to_string())?;
    let details = blocking::run("oauth_limits_flight_identity", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    if let Some((token, result)) = cached.as_ref() {
        if *token == details.oauth_access_token {
            return result.clone().map(Some);
        }
    }
    let initial_token = details.oauth_access_token.clone();
    let (token, fetched) = tokio::time::timeout(Duration::from_secs(45), operation(details))
        .await
        .unwrap_or_else(|_| {
            (
                initial_token,
                Err("OAuth quota refresh timed out".to_string()),
            )
        });
    *cached = match &fetched {
        Ok(Some(value)) => Some((token, Ok(value.clone()))),
        Err(err) => Some((token, Err(err.clone()))),
        Ok(None) => None,
    };
    fetched
}

async fn fetch_and_save_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    details: &mut crate::providers::ProviderOAuthDetails,
) -> Result<ProviderOAuthLimitsResult, String> {
    let provider_id = details.id;
    let adapter = super::registry::resolve_oauth_adapter_for_details(details)?;
    let user_agent = if adapter.cli_key() == "claude" {
        crate::gateway::upstream_identity::CLAUDE_CODE_USER_AGENT
    } else {
        super::DEFAULT_OAUTH_USER_AGENT
    };
    let client = super::build_oauth_http_client(app, user_agent, 15, 10)?;
    if oauth_details_can_refresh(details)
        && super::refresh::should_refresh_now(
            details.oauth_expires_at,
            details.oauth_refresh_lead_s,
        )
    {
        match refresh_oauth_details_for_limits(db, &client, details, adapter).await {
            Ok(refreshed) => *details = refreshed,
            Err(err)
                if details
                    .oauth_expires_at
                    .is_some_and(|expiry| expiry > crate::shared::time::now_unix_seconds()) =>
            {
                tracing::warn!(
                    provider_id,
                    "OAuth quota proactive token refresh failed, using valid token: {err}"
                );
            }
            Err(err) => return Err(err),
        }
    }
    let token = effective_oauth_access_token(details, adapter)?;
    let mut expected_revision = read_snapshot_revision(db, provider_id).await?;
    let mut result = match fetch_limits_result_for_details(&client, details, adapter, &token).await
    {
        Ok(result) => result,
        Err(err)
            if should_retry_oauth_limits_after_refresh(&err)
                && oauth_details_can_refresh(details) =>
        {
            *details = refresh_oauth_details_for_limits(db, &client, details, adapter).await?;
            let token = effective_oauth_access_token(details, adapter)?;
            expected_revision = read_snapshot_revision(db, provider_id).await?;
            fetch_limits_result_for_details(&client, details, adapter, &token).await?
        }
        Err(err) => return Err(err),
    };
    save_result(app, db, details, &mut result, expected_revision).await?;
    Ok(result)
}

pub(crate) async fn read_snapshot_revision(
    db: &crate::db::Db,
    provider_id: i64,
) -> Result<i64, String> {
    blocking::run("oauth_limits_read_revision", {
        let db = db.clone();
        move || -> crate::shared::error::AppResult<i64> {
            let conn = db.open_connection()?;
            Ok(domain::read_snapshot(&conn, provider_id)?.map_or(0, |snapshot| snapshot.revision))
        }
    })
    .await
    .map_err(Into::<String>::into)
}

pub(crate) async fn save_result<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    details: &crate::providers::ProviderOAuthDetails,
    result: &mut ProviderOAuthLimitsResult,
    expected_snapshot_revision: i64,
) -> Result<(), String> {
    let provider_id = details.id;
    let routing_limited = blocking::run("oauth_limits_save", {
        let db = db.clone();
        let result = result.clone();
        let expected_token = details.oauth_access_token.clone();
        move || {
            let saved = domain::save_snapshot_if_access_token_matches(
                &db,
                OAuthLimitSnapshotInput {
                    provider_id,
                    limit_short_label: result.limit_short_label.as_deref(),
                    limit_5h_text: result.limit_5h_text.as_deref(),
                    limit_weekly_text: result.limit_weekly_text.as_deref(),
                    limit_5h_reset_at: result.limit_5h_reset_at,
                    limit_weekly_reset_at: result.limit_weekly_reset_at,
                    reset_credit_available_count: result.reset_credit_available_count,
                    limit_5h_remaining_percent: result.limit_5h_remaining_percent,
                    limit_weekly_remaining_percent: result.limit_weekly_remaining_percent,
                    credits: result.credits.as_ref(),
                    usage_limit_reached: result.usage_limit_reached,
                },
                &expected_token,
                expected_snapshot_revision,
            )?;
            if !saved {
                return Err(crate::shared::error::AppError::from(
                    "OAUTH_QUOTA_STALE_SNAPSHOT: OAuth credentials or quota changed while fetching quota"
                        .to_string(),
                ));
            }
            let conn = db.open_connection()?;
            let now_unix = crate::shared::time::now_unix_seconds();
            Ok(matches!(
                domain::gate_snapshot(&conn, provider_id, now_unix)?,
                OAuthLimitGate::Limited { .. }
            ))
        }
    })
    .await
    .map_err(Into::<String>::into)?;
    result.routing_limited = routing_limited;
    result.quota_estimates = None;
    result.estimate_snapshot_identity = Some((
        details.oauth_access_token.clone(),
        expected_snapshot_revision + 1,
    ));
    if !result.routing_limited {
        crate::gateway_control::app_gateway_clear_unavailable_errors(app);
    }
    Ok(())
}

// Enrich UI responses after releasing the quota flight lock. Gateway requests
// only need the saved upstream snapshot and never wait for cost aggregation.
pub(crate) async fn attach_quota_estimates(
    db: &crate::db::Db,
    provider_id: i64,
    result: &mut ProviderOAuthLimitsResult,
) {
    let estimates = blocking::run("oauth_quota_estimates", {
        let db = db.clone();
        let result = result.clone();
        move || -> crate::shared::error::AppResult<Option<ProviderOAuthQuotaEstimates>> {
            let Some((token, revision)) = result.estimate_snapshot_identity else {
                return Ok(None);
            };
            let mut conn = db.open_connection()?;
            let tx = conn
                .transaction()
                .map_err(|e| db_err!("failed to start quota estimate read: {e}"))?;
            let identity = tx
                .query_row(
                    "SELECT p.cli_key, p.oauth_cost_start_at FROM providers p
                     JOIN provider_oauth_limit_snapshots s ON s.provider_id = p.id
                     WHERE p.id = ?1 AND p.auth_mode = 'oauth'
                       AND p.oauth_access_token = ?2 AND s.revision = ?3",
                    rusqlite::params![provider_id, token, revision],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
                )
                .optional()
                .map_err(|e| db_err!("failed to read quota estimate identity: {e}"))?;
            let Some((cli_key, cost_start_at)) = identity else {
                return Ok(None);
            };
            if !matches!(cli_key.as_str(), "codex" | "claude") {
                return Ok(None);
            }
            let now_unix = crate::shared::time::now_unix_seconds();
            Ok(Some(ProviderOAuthQuotaEstimates {
                short: estimate_window(
                    &tx,
                    provider_id,
                    OAuthQuotaWindow {
                        remaining_percent: result.limit_5h_remaining_percent,
                        reset_at: result.limit_5h_reset_at,
                        duration_seconds: result.limit_window_seconds.0,
                        cost_start_at,
                    },
                    now_unix,
                )?,
                weekly: estimate_window(
                    &tx,
                    provider_id,
                    OAuthQuotaWindow {
                        remaining_percent: result.limit_weekly_remaining_percent,
                        reset_at: result.limit_weekly_reset_at,
                        duration_seconds: result.limit_window_seconds.1,
                        cost_start_at,
                    },
                    now_unix,
                )?,
            }))
        }
    })
    .await;
    result.quota_estimates = match estimates {
        Ok(estimates) => estimates,
        Err(err) => {
            tracing::warn!(provider_id, "OAuth quota cost estimate unavailable: {err}");
            None
        }
    };
}

/// Resolve quota ownership without changing the bridge's routing/logging identity.
pub(crate) fn quota_source_provider_id(
    provider_id: i64,
    auth_mode: &str,
    is_bridge: bool,
    source: Option<(i64, &str)>,
) -> Option<i64> {
    let (provider_id, auth_mode) = if is_bridge {
        source?
    } else {
        (provider_id, auth_mode)
    };
    (auth_mode == "oauth").then_some(provider_id)
}

/// Shared by HTTP errors, buffered fake-200 bodies and streaming finalization.
pub(crate) fn record_exhausted(
    db: &crate::db::Db,
    oauth_quota_identity: Option<(i64, &str)>,
    quota_exhausted: bool,
    reset_at: Option<i64>,
) -> bool {
    let Some((provider_id, access_token)) = oauth_quota_identity.filter(|_| quota_exhausted) else {
        return false;
    };
    if let Err(err) = domain::save_exhausted_snapshot_if_access_token_matches(
        db,
        provider_id,
        access_token,
        reset_at,
    ) {
        tracing::warn!(
            provider_id,
            "failed to save OAuth exhausted quota snapshot: {err}"
        );
    }
    true
}

pub(crate) fn codex_account_id(details: &crate::providers::ProviderOAuthDetails) -> Option<String> {
    super::adapters::codex::parse_chatgpt_account_id(details.oauth_id_token.as_deref()).or_else(
        || super::adapters::codex::parse_chatgpt_account_id(Some(&details.oauth_access_token)),
    )
}

pub(crate) async fn fetch_codex_usage_limits(
    client: &reqwest::Client,
    usage_url: &str,
    access_token: &str,
    account_id: Option<&str>,
) -> Result<ProviderOAuthLimitsResult, String> {
    let raw = super::adapters::codex::fetch_codex_usage_payload(
        client,
        usage_url,
        access_token,
        account_id,
    )
    .await?;
    Ok(provider_oauth_limits_result_from_parts(
        "codex",
        None,
        None,
        None,
        Some(&raw),
    ))
}

async fn fetch_limits_result_for_details(
    client: &reqwest::Client,
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn super::provider_trait::OAuthProvider,
    token: &str,
) -> Result<ProviderOAuthLimitsResult, String> {
    if adapter.cli_key() == "codex" {
        return fetch_codex_usage_limits(
            client,
            CODEX_USAGE_URL,
            token,
            codex_account_id(details).as_deref(),
        )
        .await;
    }
    let limits = adapter.fetch_limits(client, token).await?;
    Ok(provider_oauth_limits_result_from_parts(
        adapter.cli_key(),
        limits.limit_short_label.as_deref(),
        limits.limit_5h_text,
        limits.limit_weekly_text,
        limits.raw_json.as_ref(),
    ))
}

pub(crate) fn oauth_details_can_refresh(details: &crate::providers::ProviderOAuthDetails) -> bool {
    details
        .oauth_refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && details
            .oauth_token_uri
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
        && details
            .oauth_client_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
}

pub(crate) fn effective_oauth_access_token(
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
) -> Result<String, String> {
    let token_set = crate::gateway::oauth::provider_trait::OAuthTokenSet {
        access_token: details.oauth_access_token.clone(),
        refresh_token: details.oauth_refresh_token.clone(),
        expires_at: details.oauth_expires_at,
        id_token: details.oauth_id_token.clone(),
    };
    let (token, _) = adapter.resolve_effective_token(&token_set, details.oauth_id_token.as_deref());
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("OAuth access token is empty".to_string());
    }
    Ok(token)
}

pub(crate) async fn refresh_oauth_details_for_limits(
    db: &crate::db::Db,
    client: &reqwest::Client,
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
) -> Result<crate::providers::ProviderOAuthDetails, String> {
    let provider_id = details.id;
    let token_uri = details
        .oauth_token_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing token_uri")?
        .to_string();
    let client_id = details
        .oauth_client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing client_id")?
        .to_string();
    let refresh_token = details
        .oauth_refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("provider missing refresh_token")?
        .to_string();

    let token_set = crate::gateway::oauth::refresh::refresh_provider_token_with_retry(
        client,
        &token_uri,
        &client_id,
        details.oauth_client_secret.as_deref(),
        &refresh_token,
    )
    .await
    .map_err(|e| format!("token refresh failed: {e}"))?;

    let (effective_token, id_token) =
        adapter.resolve_effective_token(&token_set, details.oauth_id_token.as_deref());
    if effective_token.trim().is_empty() {
        return Err("token refresh failed: refreshed access_token is empty".to_string());
    }

    let oauth_provider_type = if details.oauth_provider_type.trim().is_empty() {
        adapter.provider_type().to_string()
    } else {
        details.oauth_provider_type.clone()
    };
    let oauth_client_secret = details.oauth_client_secret.clone();
    let oauth_email = details.oauth_email.clone();
    let new_refresh_token = token_set
        .refresh_token
        .as_deref()
        .or(Some(refresh_token.as_str()))
        .map(str::to_string);
    let expires_at = token_set.expires_at;
    let expected_last_refreshed_at = details.oauth_last_refreshed_at;

    let persisted = blocking::run("provider_oauth_fetch_limits_refresh_save", {
        let db = db.clone();
        let oauth_provider_type = oauth_provider_type.clone();
        let effective_token = effective_token.clone();
        let id_token = id_token.clone();
        let token_uri = token_uri.clone();
        let client_id = client_id.clone();
        let oauth_client_secret = oauth_client_secret.clone();
        let oauth_email = oauth_email.clone();
        let new_refresh_token = new_refresh_token.clone();
        move || {
            crate::providers::update_oauth_tokens_if_last_refreshed_matches(
                &db,
                provider_id,
                "oauth",
                &oauth_provider_type,
                &effective_token,
                new_refresh_token.as_deref(),
                id_token.as_deref(),
                &token_uri,
                &client_id,
                oauth_client_secret.as_deref(),
                expires_at,
                oauth_email.as_deref(),
                expected_last_refreshed_at,
            )
        }
    })
    .await
    .map_err(Into::<String>::into)?;

    if !persisted {
        tracing::info!(
            provider_id,
            "provider_oauth_fetch_limits: refresh CAS conflict, reloading latest tokens"
        );
    }

    blocking::run("provider_oauth_fetch_limits_reload", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)
}

pub(crate) fn should_retry_oauth_limits_after_refresh(err: &str) -> bool {
    err.contains("401 Unauthorized") || err.contains("403 Forbidden")
}

fn parse_credits(body: &serde_json::Value) -> Option<ProviderOAuthCreditBalance> {
    let credits = body.get("credits")?.as_object()?;
    Some(ProviderOAuthCreditBalance {
        has_credits: credits.get("has_credits")?.as_bool()?,
        unlimited: credits.get("unlimited")?.as_bool()?,
        balance: credits
            .get("balance")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    })
}

fn parse_usage_limit_reached(body: &serde_json::Value) -> bool {
    body.pointer("/spend_control/reached")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        || body
            .get("rate_limit_reached_type")
            .is_some_and(|value| !value.is_null())
        || body
            .get("rate_limit_upsell")
            .is_some_and(|value| !value.is_null())
}

fn parse_remaining_percents(cli_key: &str, body: &serde_json::Value) -> (Option<f64>, Option<f64>) {
    match cli_key {
        "codex" => {
            let (primary, secondary) = resolve_rate_windows(body);
            (
                primary.and_then(parse_remaining_percent_from_window),
                secondary.and_then(parse_remaining_percent_from_window),
            )
        }
        "claude" => {
            let parse = |key| {
                body.get(key)
                    .and_then(|window| window.get("utilization"))
                    .and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str()?.parse::<f64>().ok())
                    })
                    .filter(|used| used.is_finite())
                    .map(|used| (100.0 - used).clamp(0.0, 100.0))
            };
            (parse("five_hour"), parse("seven_day"))
        }
        _ => (None, None),
    }
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

fn provider_oauth_limits_result_from_parts(
    cli_key: &str,
    adapter_limit_short_label: Option<&str>,
    parsed_limit_5h_text: Option<String>,
    parsed_limit_weekly_text: Option<String>,
    raw_json: Option<&serde_json::Value>,
) -> ProviderOAuthLimitsResult {
    let limit_short_label = normalize_oauth_short_window_label(cli_key, adapter_limit_short_label);
    let resets = raw_json
        .map(extract_reset_timestamps)
        .unwrap_or((None, None));
    let reset_credit_available_count = (cli_key == "codex")
        .then(|| raw_json.and_then(extract_reset_credit_available_count))
        .flatten();

    // If the adapter already parsed limit texts, use them directly.
    // Otherwise, try to parse from raw_json based on cli_key.
    let (limit_5h_text, limit_weekly_text) =
        if parsed_limit_5h_text.is_some() || parsed_limit_weekly_text.is_some() {
            (parsed_limit_5h_text, parsed_limit_weekly_text)
        } else if let Some(raw) = raw_json {
            match cli_key {
                "codex" => parse_codex_limits(raw),
                "claude" => parse_claude_limits(raw),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

    ProviderOAuthLimitsResult {
        limit_short_label,
        limit_5h_text,
        limit_weekly_text,
        limit_5h_reset_at: resets.0,
        limit_weekly_reset_at: resets.1,
        reset_credit_available_count,
        credits: (cli_key == "codex")
            .then(|| raw_json.and_then(parse_credits))
            .flatten(),
        limit_5h_remaining_percent: raw_json
            .and_then(|raw| parse_remaining_percents(cli_key, raw).0),
        limit_weekly_remaining_percent: raw_json
            .and_then(|raw| parse_remaining_percents(cli_key, raw).1),
        routing_limited: false,
        quota_estimates: None,
        estimate_snapshot_identity: None,
        usage_limit_reached: cli_key == "codex" && raw_json.is_some_and(parse_usage_limit_reached),
        limit_window_seconds: parse_limit_window_seconds(cli_key, raw_json),
    }
}

fn parse_limit_window_seconds(
    cli_key: &str,
    raw_json: Option<&serde_json::Value>,
) -> (Option<i64>, Option<i64>) {
    match cli_key {
        "claude" => (Some(5 * 3600), Some(7 * 24 * 3600)),
        "codex" => {
            let (primary, secondary) = raw_json.map(resolve_rate_windows).unwrap_or((None, None));
            let duration = |window: Option<&serde_json::Value>, fallback| {
                let value = window.and_then(|window| {
                    window
                        .get("limit_window_seconds")
                        .or_else(|| window.get("limitWindowSeconds"))
                });
                match value {
                    Some(value) => value
                        .as_i64()
                        .or_else(|| value.as_str()?.parse().ok())
                        .filter(|seconds| *seconds > 0 && *seconds <= 31 * 24 * 3600),
                    None => Some(fallback),
                }
            };
            (
                duration(primary, 5 * 3600),
                duration(secondary, 7 * 24 * 3600),
            )
        }
        _ => (None, None),
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
fn parse_reset_timestamp_value(value: &serde_json::Value) -> Option<i64> {
    if let Some(timestamp) = value.as_i64().filter(|timestamp| *timestamp > 0) {
        return Some(timestamp);
    }

    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(timestamp) = text.parse::<i64>() {
        return (timestamp > 0).then_some(timestamp);
    }

    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|value| value.timestamp())
        .filter(|timestamp| *timestamp > 0)
}

fn extract_reset_timestamp(window: &serde_json::Value) -> Option<i64> {
    window
        .get("reset_at")
        .or_else(|| window.get("resetAt"))
        .or_else(|| window.get("resets_at"))
        .or_else(|| window.get("resetsAt"))
        .or_else(|| window.get("reset_time"))
        .or_else(|| window.get("resetTime"))
        .and_then(parse_reset_timestamp_value)
}

fn extract_bucket_reset_timestamps(body: &serde_json::Value) -> (Option<i64>, Option<i64>) {
    let Some(buckets) = body.get("buckets").and_then(serde_json::Value::as_array) else {
        return (None, None);
    };

    let mut reset_times: Vec<i64> = buckets.iter().filter_map(extract_reset_timestamp).collect();
    reset_times.sort_unstable();
    reset_times.dedup();

    match (reset_times.first().copied(), reset_times.last().copied()) {
        (Some(first), Some(last)) if first != last => (Some(first), Some(last)),
        (Some(first), _) => (Some(first), None),
        _ => (None, None),
    }
}

fn extract_reset_timestamps(body: &serde_json::Value) -> (Option<i64>, Option<i64>) {
    let (primary, secondary) = resolve_rate_windows(body);
    let resets = (
        primary.and_then(extract_reset_timestamp),
        secondary.and_then(extract_reset_timestamp),
    );
    if resets.0.is_some() || resets.1.is_some() {
        return resets;
    }
    extract_bucket_reset_timestamps(body)
}

fn extract_reset_credit_available_count(body: &serde_json::Value) -> Option<i64> {
    body.get("rate_limit_reset_credits")
        .and_then(|value| value.get("available_count"))
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value >= 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_estimate_durations_follow_upstream_windows_and_keep_invalid_values_unknown() {
        let raw = serde_json::json!({"rate_limit": {
            "primary_window": {"limit_window_seconds": 604800},
            "secondary_window": {"limitWindowSeconds": "7200"}
        }});
        assert_eq!(
            parse_limit_window_seconds("codex", Some(&raw)),
            (Some(604800), Some(7200))
        );
        let invalid = serde_json::json!({"rate_limit": {
            "primary_window": {"limit_window_seconds": 0},
            "secondary_window": {"limit_window_seconds": "unknown"}
        }});
        assert_eq!(
            parse_limit_window_seconds("codex", Some(&invalid)),
            (None, None)
        );
        assert_eq!(
            parse_limit_window_seconds("codex", None),
            (Some(18000), Some(604800))
        );
        assert_eq!(
            parse_limit_window_seconds("claude", None),
            (Some(18000), Some(604800))
        );
        assert_eq!(
            parse_limit_window_seconds("gemini", Some(&raw)),
            (None, None)
        );
        assert_eq!(parse_limit_window_seconds("grok", None), (None, None));
    }

    #[tokio::test]
    async fn quota_estimate_is_ui_only_and_query_failure_preserves_quota() {
        let (_dir, db, provider_id) = quota_test_provider();
        let app = tauri::test::mock_app();
        let now = crate::shared::time::now_unix_seconds();
        db.open_connection().unwrap().execute(
            "INSERT INTO request_logs(trace_id, cli_key, method, path, status, created_at,
              final_provider_id, cost_usd_femto)
             VALUES ('estimate-sample', 'codex', 'POST', '/responses', 200, ?1, ?2, 20000000000000000)",
            rusqlite::params![now - 60, provider_id],
        ).unwrap();
        let details = crate::providers::get_oauth_details(&db, provider_id).unwrap();
        let mut result = parsed_codex(serde_json::json!({"rate_limit": {
            "primary_window": {"used_percent": 20, "reset_at": now + 3600, "limit_window_seconds": 18000},
            "secondary_window": {"used_percent": 25, "reset_at": now + 86400, "limit_window_seconds": 604800}
        }}));
        let revision = read_snapshot_revision(&db, provider_id).await.unwrap();
        save_result(app.handle(), &db, &details, &mut result, revision)
            .await
            .unwrap();
        assert!(result.quota_estimates.is_none());
        attach_quota_estimates(&db, provider_id, &mut result).await;
        let estimates = result.quota_estimates.as_ref().unwrap();
        assert_eq!(
            estimates.short.as_ref().unwrap().estimated_total_usd,
            Some(100.0)
        );
        assert_eq!(
            estimates.weekly.as_ref().unwrap().estimated_remaining_usd,
            Some(60.0)
        );
        assert!(!result.routing_limited);
        let serialized = serde_json::to_value(&result).unwrap();
        assert_eq!(serialized["quota_estimates"]["short"]["sample_count"], 1);
        assert_eq!(
            serialized["limit_window_seconds"],
            serde_json::json!([18000, 604800])
        );
        assert!(serialized.get("estimate_snapshot_identity").is_none());

        {
            let conn = db.open_connection().unwrap();
            conn.execute(
                "UPDATE providers SET oauth_cost_start_at = ?1 WHERE id = ?2",
                rusqlite::params![now - 1, provider_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO request_logs(trace_id, cli_key, method, path, status, created_at,
                  final_provider_id, cost_usd_femto)
                 VALUES ('current-login', 'codex', 'POST', '/responses', 200, ?1, ?2, 1000000000000000)",
                rusqlite::params![now, provider_id],
            )
            .unwrap();
        }
        attach_quota_estimates(&db, provider_id, &mut result).await;
        assert_eq!(
            result
                .quota_estimates
                .as_ref()
                .unwrap()
                .short
                .as_ref()
                .unwrap()
                .estimated_total_usd,
            Some(5.0),
            "the UI must use the current login boundary when aggregating costs"
        );

        db.open_connection()
            .unwrap()
            .execute_batch("DROP TABLE request_logs")
            .unwrap();
        let revision = read_snapshot_revision(&db, provider_id).await.unwrap();
        save_result(app.handle(), &db, &details, &mut result, revision)
            .await
            .unwrap();
        assert!(result.quota_estimates.is_none());
        attach_quota_estimates(&db, provider_id, &mut result).await;
        assert!(result.quota_estimates.is_none());
        assert_eq!(result.limit_window_seconds, (Some(18000), Some(604800)));
        assert_eq!(result.limit_5h_remaining_percent, Some(80.0));
        assert!(!result.routing_limited);
        assert_eq!(
            domain::read_snapshot(&db.open_connection().unwrap(), provider_id)
                .unwrap()
                .unwrap()
                .limit_5h_remaining_percent,
            Some(80.0)
        );
    }

    #[tokio::test]
    async fn quota_estimate_rejects_snapshot_or_login_changes_before_ui_enrichment() {
        let (_dir, db, provider_id) = quota_test_provider();
        let app = tauri::test::mock_app();
        let details = crate::providers::get_oauth_details(&db, provider_id).unwrap();
        let mut result = parsed_codex(serde_json::json!({}));
        let revision = read_snapshot_revision(&db, provider_id).await.unwrap();
        save_result(app.handle(), &db, &details, &mut result, revision)
            .await
            .unwrap();
        attach_quota_estimates(&db, provider_id, &mut result).await;
        assert!(result.quota_estimates.is_some());

        db.open_connection()
            .unwrap()
            .execute(
                "UPDATE providers SET oauth_access_token = 'account-b-token' WHERE id = ?1",
                [provider_id],
            )
            .unwrap();
        attach_quota_estimates(&db, provider_id, &mut result).await;
        assert!(result.quota_estimates.is_none());

        db.open_connection()
            .unwrap()
            .execute(
                "UPDATE providers SET oauth_access_token = 'account-a-token' WHERE id = ?1",
                [provider_id],
            )
            .unwrap();
        domain::clear_snapshot(&db, provider_id).unwrap();
        attach_quota_estimates(&db, provider_id, &mut result).await;
        assert!(result.quota_estimates.is_none());
    }

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
    fn extract_reset_credit_available_count_supports_codex_usage_payload() {
        let body = serde_json::json!({
            "rate_limit": {
                "primary_window": { "used_percent": 25.0 },
                "secondary_window": { "used_percent": 10.0 }
            },
            "rate_limit_reset_credits": {
                "available_count": 3
            }
        });

        assert_eq!(extract_reset_credit_available_count(&body), Some(3));
    }

    #[test]
    fn extract_reset_credit_available_count_ignores_invalid_values() {
        for body in [
            serde_json::json!({}),
            serde_json::json!({ "rate_limit_reset_credits": null }),
            serde_json::json!({ "rate_limit_reset_credits": { "available_count": -1 } }),
            serde_json::json!({ "rate_limit_reset_credits": { "available_count": "3" } }),
        ] {
            assert_eq!(extract_reset_credit_available_count(&body), None);
        }
    }

    #[test]
    fn extract_reset_timestamps_supports_gemini_bucket_reset_time() {
        let body = serde_json::json!({
            "buckets": [
                { "remainingAmount": "0", "resetTime": "2026-03-09T11:00:00Z" },
                { "remainingAmount": "7", "resetTime": "2026-03-16T00:00:00Z" }
            ]
        });

        let resets = extract_reset_timestamps(&body);

        assert_eq!(resets.0, Some(1_773_054_000));
        assert_eq!(resets.1, Some(1_773_619_200));
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

    fn parsed_codex(body: serde_json::Value) -> ProviderOAuthLimitsResult {
        provider_oauth_limits_result_from_parts("codex", None, None, None, Some(&body))
    }

    #[test]
    fn credits_preserve_decimal_string_and_unknown_balance() {
        let balance = "900719925474099312345678.123456789012345678";
        let parsed = parsed_codex(serde_json::json!({"credits": {
            "has_credits": true, "unlimited": false, "balance": balance
        }}));
        let credits = parsed.credits.unwrap();
        assert!(credits.has_credits);
        assert_eq!(credits.balance.as_deref(), Some(balance));
        let zero = parsed_codex(serde_json::json!({"credits": {
            "has_credits": false, "unlimited": false, "balance": "0"
        }}))
        .credits
        .unwrap();
        assert!(!zero.has_credits);
        assert_eq!(zero.balance.as_deref(), Some("0"));
        let unlimited = parsed_codex(serde_json::json!({"credits": {
            "has_credits": false, "unlimited": true, "balance": null
        }}))
        .credits
        .unwrap();
        assert!(unlimited.unlimited);
        assert_eq!(unlimited.balance, None);
        for body in [
            serde_json::json!({}),
            serde_json::json!({"credits":null}),
            serde_json::json!({"credits":{}}),
        ] {
            assert!(parsed_codex(body).credits.is_none());
        }
    }

    #[test]
    fn raw_percent_survives_display_rounding_for_codex_and_claude() {
        let codex = parsed_codex(serde_json::json!({"rate_limit":{
            "primary_window":{"used_percent":99.6},
            "secondary_window":{"used_percent":79.51}
        }}));
        assert_eq!(codex.limit_5h_text.as_deref(), Some("0%"));
        assert!((codex.limit_5h_remaining_percent.unwrap() - 0.4).abs() < 1e-10);
        assert!((codex.limit_weekly_remaining_percent.unwrap() - 20.49).abs() < 1e-10);
        let raw = serde_json::json!({"five_hour":{"utilization":"99.6"}});
        let claude =
            provider_oauth_limits_result_from_parts("claude", None, None, None, Some(&raw));
        assert!((claude.limit_5h_remaining_percent.unwrap() - 0.4).abs() < 1e-10);
    }

    #[test]
    fn hard_limits_follow_wire_payload_without_blocking_ordinary_credit_handoff() {
        assert!(
            !parsed_codex(serde_json::json!({"rate_limit":{"allowed":false,"limit_reached":true}}))
                .usage_limit_reached
        );
        for body in [
            serde_json::json!({"spend_control":{"reached":true}}),
            serde_json::json!({"rate_limit_reached_type":{"type":"workspace_owner_usage_limit_reached"}}),
            serde_json::json!({"rate_limit_reached_type":{"type":"workspace_member_credits_depleted"}}),
            serde_json::json!({"rate_limit_upsell":{"type":"usage_limit"}}),
        ] {
            assert!(parsed_codex(body).usage_limit_reached);
        }
        assert!(!parsed_codex(serde_json::json!({"spend_control":{"reached":false},"rate_limit_reached_type":null,"rate_limit_upsell":null})).usage_limit_reached);
    }

    fn quota_test_provider() -> (tempfile::TempDir, crate::db::Db, i64) {
        use std::sync::atomic::{AtomicI64, Ordering};
        static NEXT_ID: AtomicI64 = AtomicI64::new(91000);
        let provider_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("quota-flight.db")).unwrap();
        db.open_connection().unwrap().execute(
            "INSERT INTO providers(id,cli_key,name,base_url,api_key_plaintext,created_at,updated_at,auth_mode,oauth_provider_type,oauth_access_token,oauth_use_credits) VALUES (?1,'codex','quota-flight','','',1,1,'oauth','codex_oauth','account-a-token',1)",
            [provider_id],
        ).unwrap();
        (dir, db, provider_id)
    }

    #[tokio::test]
    async fn concurrent_failures_share_one_flight_and_next_request_can_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (_dir, db, provider_id) = quota_test_provider();
        let attempts = AtomicUsize::new(0);
        let requests = (0..8).map(|_| {
            share_fetch(&db, provider_id, |details| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                (
                    details.oauth_access_token,
                    Err("quota unavailable".to_string()),
                )
            })
        });
        let results = futures_util::future::join_all(requests).await;
        assert!(results
            .iter()
            .all(|result| result.as_ref().unwrap_err() == "quota unavailable"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        let retried = share_fetch(&db, provider_id, |details| async {
            attempts.fetch_add(1, Ordering::SeqCst);
            (
                details.oauth_access_token,
                Ok(Some(parsed_codex(serde_json::json!({})))),
            )
        })
        .await;
        assert!(retried.unwrap().is_some());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn relogin_reloads_identity_after_waiting_and_never_reuses_old_account_result() {
        let (_dir, db, provider_id) = quota_test_provider();
        for old_result in [
            Ok(Some(parsed_codex(serde_json::json!({})))),
            Err("old account unavailable".to_string()),
        ] {
            db.open_connection()
                .unwrap()
                .execute(
                    "UPDATE providers SET oauth_access_token = 'account-a-token' WHERE id = ?1",
                    [provider_id],
                )
                .unwrap();
            let started = tokio::sync::Notify::new();
            let release = tokio::sync::Notify::new();
            let old_request = share_fetch(&db, provider_id, |details| async {
                started.notify_one();
                release.notified().await;
                (details.oauth_access_token, old_result)
            });
            let new_request = async {
                started.notified().await;
                db.open_connection()
                    .unwrap()
                    .execute(
                        "UPDATE providers SET oauth_access_token = 'account-b-token' WHERE id = ?1",
                        [provider_id],
                    )
                    .unwrap();
                let request = share_fetch(&db, provider_id, |details| async {
                    assert_eq!(details.oauth_access_token, "account-b-token");
                    (
                        details.oauth_access_token,
                        Ok(Some(parsed_codex(
                            serde_json::json!({"rate_limit":{"primary_window":{"used_percent":25}}}),
                        ))),
                    )
                });
                tokio::pin!(request);
                assert!(futures_util::poll!(&mut request).is_pending());
                release.notify_one();
                let result = request.await.unwrap().unwrap();
                assert_eq!(result.limit_5h_remaining_percent, Some(75.0));
            };
            let _ = tokio::join!(old_request, new_request);
        }
    }

    #[tokio::test]
    async fn token_rotation_during_fetch_keeps_waiters_in_one_flight() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (_dir, db, provider_id) = quota_test_provider();
        let started = tokio::sync::Notify::new();
        let release = tokio::sync::Notify::new();
        let duplicate_queries = AtomicUsize::new(0);
        let first = share_fetch(&db, provider_id, |mut details| async {
            assert_eq!(details.oauth_access_token, "account-a-token");
            // Represents a proactive refresh or the 401 retry while the quota
            // operation still owns the provider lock.
            details.oauth_access_token = "refreshed-token".into();
            db.open_connection()
                .unwrap()
                .execute(
                    "UPDATE providers SET oauth_access_token = ?1 WHERE id = ?2",
                    rusqlite::params![details.oauth_access_token, provider_id],
                )
                .unwrap();
            started.notify_one();
            release.notified().await;
            (
                details.oauth_access_token,
                Ok(Some(parsed_codex(
                    serde_json::json!({"rate_limit":{"primary_window":{"used_percent":20}}}),
                ))),
            )
        });
        let second = async {
            started.notified().await;
            let request = share_fetch(&db, provider_id, |details| async {
                duplicate_queries.fetch_add(1, Ordering::SeqCst);
                (
                    details.oauth_access_token,
                    Err("duplicate usage must not run".into()),
                )
            });
            tokio::pin!(request);
            assert!(futures_util::poll!(&mut request).is_pending());
            release.notify_one();
            assert_eq!(
                request.await.unwrap().unwrap().limit_5h_remaining_percent,
                Some(80.0)
            );
        };
        let (first, _) = tokio::join!(first, second);
        assert!(first.is_ok());
        assert_eq!(duplicate_queries.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reset_holds_one_lock_through_token_rotation_consume_and_snapshot_save() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let (_dir, db, provider_id) = quota_test_provider();
        let app = tauri::test::mock_app();
        domain::save_exhausted_snapshot(
            &db,
            provider_id,
            Some(crate::shared::time::now_unix_seconds() + 3600),
        )
        .unwrap();
        let mut reset_guard = lock_for_reset(provider_id).await.unwrap();
        db.open_connection()
            .unwrap()
            .execute(
                "UPDATE providers SET oauth_access_token = 'refreshed-token' WHERE id = ?1",
                [provider_id],
            )
            .unwrap();
        let queried_before_consume = AtomicBool::new(false);
        let query = share_fetch(&db, provider_id, |details| async {
            queried_before_consume.store(true, Ordering::SeqCst);
            (
                details.oauth_access_token,
                Ok(Some(parsed_codex(
                    serde_json::json!({"rate_limit":{"primary_window":{"used_percent":100}}}),
                ))),
            )
        });
        tokio::pin!(query);
        assert!(futures_util::poll!(&mut query).is_pending());
        // After consume succeeds, persist the newly available quota while the
        // same lock excludes every query, including users of the new token.
        let details = crate::providers::get_oauth_details(&db, provider_id).unwrap();
        let revision = read_snapshot_revision(&db, provider_id).await.unwrap();
        let mut refreshed =
            parsed_codex(serde_json::json!({"rate_limit":{"primary_window":{"used_percent":20}}}));
        save_result(app.handle(), &db, &details, &mut refreshed, revision)
            .await
            .unwrap();
        *reset_guard = Some((details.oauth_access_token, Ok(refreshed)));
        drop(reset_guard);
        assert_eq!(
            query.await.unwrap().unwrap().limit_5h_remaining_percent,
            Some(80.0)
        );
        assert!(!queried_before_consume.load(Ordering::SeqCst));
        let saved = domain::read_snapshot(&db.open_connection().unwrap(), provider_id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.limit_5h_remaining_percent, Some(80.0));
        assert!(!saved.usage_limit_reached);
        assert_eq!(saved.revision, revision + 1);
    }

    #[test]
    fn saved_recovery_invalidates_unavailable_cache_without_a_manual_circuit_reset() {
        use tauri::Manager;
        let (_dir, db, provider_id) = quota_test_provider();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let app = tauri::test::mock_app();
        let recent_errors = Arc::new(Mutex::new(
            crate::gateway::proxy::RecentErrorCache::default(),
        ));
        let runtime = crate::gateway::runtime::GatewayRuntime::for_tests(
            &rt,
            Arc::new(crate::session_manager::SessionManager::new()),
            recent_errors.clone(),
        );
        app.manage(crate::app::gateway_state::GatewayState::with_runtime_for_tests(runtime));
        let now = crate::shared::time::now_unix_seconds();
        domain::save_exhausted_snapshot(&db, provider_id, Some(now + 3600)).unwrap();
        recent_errors
            .lock()
            .unwrap()
            .insert_unavailable_for_tests(now, 77, "same-route", 180);
        rt.block_on(async {
            let details = crate::providers::get_oauth_details(&db, provider_id).unwrap();
            let revision = read_snapshot_revision(&db, provider_id).await.unwrap();
            let mut available = parsed_codex(serde_json::json!({
                "rate_limit":{"primary_window":{"used_percent":100}},
                "credits":{"has_credits":true,"unlimited":false,"balance":"12.5"}
            }));
            assert!(
                save_result(app.handle(), &db, &details, &mut available, revision - 1)
                    .await
                    .is_err()
            );
            assert!(recent_errors.lock().unwrap().has_active_error_for_tests(
                now,
                77,
                "same-route"
            ));
            let mut still_limited = parsed_codex(
                serde_json::json!({"rate_limit":{"primary_window":{"used_percent":100}}}),
            );
            save_result(app.handle(), &db, &details, &mut still_limited, revision)
                .await
                .unwrap();
            assert!(still_limited.routing_limited);
            assert!(recent_errors.lock().unwrap().has_active_error_for_tests(
                now,
                77,
                "same-route"
            ));
            save_result(app.handle(), &db, &details, &mut available, revision + 1)
                .await
                .unwrap();
            assert!(!available.routing_limited);
            assert!(!recent_errors.lock().unwrap().has_active_error_for_tests(
                now,
                77,
                "same-route"
            ));
        });
    }

    #[test]
    fn bridge_quota_exhaustion_targets_only_the_real_oauth_source() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("quota-source.db")).unwrap();
        let conn = db.open_connection().unwrap();
        conn.execute_batch("INSERT INTO providers(id,cli_key,name,base_url,api_key_plaintext,created_at,updated_at,auth_mode) VALUES
            (1,'claude','bridge','https://bridge.test','',1,1,'api_key'),
            (2,'codex','source','https://source.test','',1,1,'oauth'),
            (3,'codex','api-source','https://api.test','',1,1,'api_key');").unwrap();
        conn.execute(
            "UPDATE providers SET oauth_access_token='source-token' WHERE id=2",
            [],
        )
        .unwrap();
        drop(conn);
        let source = quota_source_provider_id(1, "api_key", true, Some((2, "oauth")));
        assert_eq!(source, Some(2));
        assert!(record_exhausted(
            &db,
            source.map(|id| (id, "source-token")),
            true,
            None
        ));
        let conn = db.open_connection().unwrap();
        assert!(
            domain::read_snapshot(&conn, 2)
                .unwrap()
                .unwrap()
                .usage_limit_reached
        );
        assert!(domain::read_snapshot(&conn, 1).unwrap().is_none());
        assert_eq!(
            conn.query_row("SELECT auth_mode FROM providers WHERE id=1", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "api_key"
        );
        let non_oauth_source = quota_source_provider_id(1, "oauth", true, Some((3, "api_key")));
        assert_eq!(non_oauth_source, None);
        assert!(!record_exhausted(
            &db,
            non_oauth_source.map(|id| (id, "source-token")),
            true,
            None
        ));
        assert!(domain::read_snapshot(&conn, 3).unwrap().is_none());
        assert_eq!(quota_source_provider_id(1, "oauth", true, None), None);
        assert_eq!(quota_source_provider_id(2, "oauth", false, None), Some(2));
        assert!(!record_exhausted(
            &db,
            Some((2, "source-token")),
            false,
            None
        ));
    }
}
