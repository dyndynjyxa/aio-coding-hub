//! Usage: OAuth refresh decision and execution helpers for provider-integrated OAuth.

use crate::db;
use crate::gateway::oauth::token_exchange::{refresh_access_token, TokenRefreshRequest};
use crate::providers;
use crate::shared::time::now_unix_seconds;

const MIN_REFRESH_RETRY_INTERVAL_SECS: i64 = 30;
const REFRESH_LINEAR_RETRY_MAX_ATTEMPTS: u32 = 3;
const REFRESH_LINEAR_RETRY_BASE_DELAY_SECS: u64 = 2;
const REFRESH_LOOP_INTERVAL_SECS: u64 = 60;
const REFRESH_BATCH_LIMIT: usize = 64;
const QUOTA_CLEAR_BATCH_LIMIT: usize = 128;

pub(crate) fn should_refresh_now(
    expires_at: Option<i64>,
    refresh_lead_s: i64,
    now_unix: i64,
) -> bool {
    match expires_at {
        Some(expires_at) => expires_at.saturating_sub(refresh_lead_s) <= now_unix,
        None => false,
    }
}

pub(crate) fn refreshed_recently(
    last_refreshed_at: Option<i64>,
    now_unix: i64,
    min_interval_secs: i64,
) -> bool {
    match last_refreshed_at {
        Some(ts) => now_unix.saturating_sub(ts) < min_interval_secs,
        None => false,
    }
}

/// Refresh a provider's access token using its refresh_token.
pub(crate) async fn refresh_provider_access_token(
    client: &reqwest::Client,
    provider: &providers::ProviderOAuthRefreshInfo,
) -> crate::shared::error::AppResult<crate::gateway::oauth::token_exchange::OAuthTokenSet> {
    let config = super::providers::config_for_provider_type(&provider.oauth_provider_type)
        .ok_or_else(|| {
            format!(
                "unknown oauth_provider_type: {}",
                provider.oauth_provider_type
            )
        })?;

    let token_uri = provider
        .oauth_token_uri
        .as_deref()
        .unwrap_or(config.token_url);
    let client_id = provider
        .oauth_client_id
        .as_deref()
        .unwrap_or(config.client_id);
    let client_secret = provider
        .oauth_client_secret
        .as_deref()
        .or(config.client_secret);

    let refresh_token = provider
        .oauth_refresh_token
        .as_deref()
        .ok_or_else(|| "no refresh_token on provider".to_string())?;

    let req = TokenRefreshRequest {
        token_uri: token_uri.to_string(),
        client_id: client_id.to_string(),
        client_secret: client_secret.map(str::to_string),
        refresh_token: refresh_token.to_string(),
    };

    refresh_access_token(client, &req).await
}

/// Refresh with linear retry (up to `max_attempts`).
pub(crate) async fn refresh_provider_with_linear_retry(
    client: &reqwest::Client,
    provider: &providers::ProviderOAuthRefreshInfo,
    max_attempts: u32,
) -> crate::shared::error::AppResult<crate::gateway::oauth::token_exchange::OAuthTokenSet> {
    let mut last_err = None;
    for attempt in 0..max_attempts {
        if attempt > 0 {
            let delay_s = REFRESH_LINEAR_RETRY_BASE_DELAY_SECS * (attempt as u64);
            tokio::time::sleep(std::time::Duration::from_secs(delay_s)).await;
        }
        match refresh_provider_access_token(client, provider).await {
            Ok(tokens) => return Ok(tokens),
            Err(e) => {
                tracing::warn!(
                    provider_id = provider.id,
                    attempt = attempt + 1,
                    max = max_attempts,
                    error = %e,
                    "oauth token refresh attempt failed"
                );
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "oauth refresh failed: no attempts made".to_string().into()))
}

/// Background loop that refreshes OAuth tokens on providers.
pub(crate) async fn run_background_refresh_loop(
    app: tauri::AppHandle,
    db: db::Db,
    client: reqwest::Client,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(REFRESH_LOOP_INTERVAL_SECS)).await;

        let now = now_unix_seconds() as i64;

        // 1. Clear expired quota flags.
        if let Err(e) = clear_expired_quotas(&app, &db, now) {
            tracing::error!(error = %e, "failed to clear expired oauth quotas on providers");
        }

        // 2. Refresh providers with expiring tokens.
        if let Err(e) = refresh_due_providers(&app, &db, &client, now).await {
            tracing::error!(error = %e, "failed to refresh oauth tokens on providers");
        }
    }
}

fn clear_expired_quotas(
    app: &tauri::AppHandle,
    db: &db::Db,
    now: i64,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let provider_ids = providers::list_expired_oauth_quotas(&conn, now, QUOTA_CLEAR_BATCH_LIMIT)?;

    for id in &provider_ids {
        if let Err(e) = providers::set_oauth_quota(&conn, *id, false, None) {
            tracing::warn!(
                provider_id = id,
                error = %e,
                "failed to clear oauth quota on provider"
            );
            continue;
        }

        let _ = tauri::Emitter::emit(
            app,
            "provider-oauth-quota",
            serde_json::json!({
                "provider_id": id,
                "quota_exceeded": false,
            }),
        );
    }

    if !provider_ids.is_empty() {
        super::quota_cache::invalidate_all();
    }

    Ok(())
}

async fn refresh_due_providers(
    app: &tauri::AppHandle,
    db: &db::Db,
    client: &reqwest::Client,
    now: i64,
) -> crate::shared::error::AppResult<()> {
    let providers_to_refresh = {
        let conn = db.open_connection()?;
        providers::list_oauth_providers_needing_refresh(&conn, now, REFRESH_BATCH_LIMIT)?
    };

    for provider in &providers_to_refresh {
        if refreshed_recently(
            provider.oauth_last_refreshed_at,
            now,
            MIN_REFRESH_RETRY_INTERVAL_SECS,
        ) {
            continue;
        }

        match refresh_provider_with_linear_retry(
            client,
            provider,
            REFRESH_LINEAR_RETRY_MAX_ATTEMPTS,
        )
        .await
        {
            Ok(tokens) => {
                let conn = db.open_connection()?;
                if let Err(e) = providers::update_oauth_tokens(
                    &conn,
                    provider.id,
                    &tokens.access_token,
                    tokens.id_token.as_deref(),
                    tokens.expires_at,
                    tokens.refresh_token.as_deref(),
                ) {
                    tracing::error!(
                        provider_id = provider.id,
                        error = %e,
                        "failed to store refreshed oauth tokens"
                    );
                    continue;
                }

                let _ = tauri::Emitter::emit(
                    app,
                    "provider-oauth-refreshed",
                    serde_json::json!({
                        "provider_id": provider.id,
                        "expires_at": tokens.expires_at,
                    }),
                );
            }
            Err(e) => {
                let conn = db.open_connection()?;
                let _ = providers::record_oauth_refresh_failure(
                    &conn,
                    provider.id,
                    Some(&e.to_string()),
                );

                let _ = tauri::Emitter::emit(
                    app,
                    "provider-oauth-error",
                    serde_json::json!({
                        "provider_id": provider.id,
                        "error": e.to_string(),
                    }),
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_refresh_when_within_lead_time() {
        let now = 1_000_000;
        assert!(should_refresh_now(Some(now + 100), 200, now));
        assert!(!should_refresh_now(Some(now + 500), 200, now));
        assert!(!should_refresh_now(None, 200, now));
    }

    #[test]
    fn refreshed_recently_guards_against_rapid_retries() {
        let now = 1_000_000;
        assert!(refreshed_recently(Some(now - 10), now, 30));
        assert!(!refreshed_recently(Some(now - 60), now, 30));
        assert!(!refreshed_recently(None, now, 30));
    }
}
