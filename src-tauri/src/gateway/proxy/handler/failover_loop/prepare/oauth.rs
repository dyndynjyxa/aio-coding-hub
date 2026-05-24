//! Usage: OAuth credential resolution helpers for `failover_loop`.

use crate::gateway::util::now_unix_seconds;

pub(super) fn resolve_oauth_adapter_for_provider(
    cli_key: &str,
    provider_id: i64,
    oauth_provider_type: Option<&str>,
) -> crate::shared::error::AppResult<
    &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
> {
    crate::gateway::oauth::registry::resolve_oauth_adapter(
        cli_key,
        provider_id,
        oauth_provider_type,
    )
    .map_err(Into::into)
}

/// Resolve the effective API credential for a provider.
/// For `api_key` mode, returns the plaintext key.
/// For `oauth` mode, checks token freshness and refreshes inline if needed.
pub(super) async fn resolve_effective_credential<R: tauri::Runtime>(
    state: &crate::gateway::runtime::GatewayAppState<R>,
    cli_key: &str,
    provider: &crate::providers::ProviderForGateway,
) -> crate::shared::error::AppResult<String> {
    if provider.auth_mode != "oauth" {
        let api_key = provider.api_key_plaintext.trim();
        if api_key.is_empty() {
            return Err("SEC_INVALID_INPUT: provider api_key is empty"
                .to_string()
                .into());
        }
        return Ok(api_key.to_string());
    }

    let details = crate::providers::get_oauth_details(&state.db, provider.id)?;
    if details.cli_key != cli_key {
        return Err(format!(
            "SEC_INVALID_STATE: oauth details cli_key mismatch for provider_id={} (expected={cli_key}, actual={})",
            provider.id, details.cli_key
        )
        .into());
    }
    let oauth_adapter = resolve_oauth_adapter_for_provider(
        cli_key,
        provider.id,
        Some(details.oauth_provider_type.as_str()),
    )?;

    let raw_token = details.oauth_access_token.trim().to_string();
    if raw_token.is_empty() {
        return Err("SEC_INVALID_INPUT: oauth access_token is empty"
            .to_string()
            .into());
    }

    let token = raw_token;
    let now_unix = now_unix_seconds() as i64;
    if crate::gateway::oauth::refresh::should_refresh_now(
        details.oauth_expires_at,
        details.oauth_refresh_lead_s,
    ) {
        if let (Some(ref refresh_token), Some(ref token_uri)) =
            (&details.oauth_refresh_token, &details.oauth_token_uri)
        {
            if !refresh_token.trim().is_empty() && !token_uri.trim().is_empty() {
                let client = state.client();
                match crate::gateway::oauth::refresh::refresh_provider_token_with_retry(
                    &client,
                    token_uri,
                    details.oauth_client_id.as_deref().unwrap_or(""),
                    details.oauth_client_secret.as_deref(),
                    refresh_token,
                )
                .await
                {
                    Ok(refreshed) => {
                        let new_token = refreshed.access_token.trim().to_string();
                        if !new_token.is_empty() {
                            match crate::providers::update_oauth_tokens_if_last_refreshed_matches(
                                &state.db,
                                provider.id,
                                "oauth",
                                oauth_adapter.provider_type(),
                                &new_token,
                                refreshed.refresh_token.as_deref().or(Some(refresh_token)),
                                refreshed
                                    .id_token
                                    .as_deref()
                                    .or(details.oauth_id_token.as_deref()),
                                token_uri,
                                details.oauth_client_id.as_deref().unwrap_or(""),
                                details.oauth_client_secret.as_deref(),
                                refreshed.expires_at.or(details.oauth_expires_at),
                                details.oauth_email.as_deref(),
                                details.oauth_last_refreshed_at,
                            ) {
                                Ok(true) => {}
                                Ok(false) => {
                                    tracing::info!(
                                        cli_key = %cli_key,
                                        provider_id = provider.id,
                                        "OAuth inline refresh CAS conflict: skipped stale token write"
                                    );
                                }
                                Err(persist_err) => {
                                    tracing::warn!(
                                        cli_key = %cli_key,
                                        provider_id = provider.id,
                                        "OAuth token refresh persisted failed: {}",
                                        persist_err
                                    );
                                }
                            }
                            tracing::info!(
                                cli_key = %cli_key,
                                provider_id = provider.id,
                                "OAuth token refreshed inline successfully"
                            );
                            return Ok(new_token);
                        }
                    }
                    Err(err) => {
                        let still_valid = details
                            .oauth_expires_at
                            .map(|exp| exp > now_unix)
                            .unwrap_or(false);
                        if still_valid {
                            tracing::warn!(
                                provider_id = provider.id,
                                cli_key = %cli_key,
                                "oauth inline refresh failed; fallback to existing token: {}",
                                err
                            );
                            return Ok(token);
                        }
                        return Err(format!("OAUTH_REFRESH_FAILED: {err}").into());
                    }
                }
            }
        }
    }

    Ok(token)
}

/// After a 401 response, attempt to refresh OAuth token and return the new credential.
pub(super) async fn refresh_oauth_credential_after_401<R: tauri::Runtime>(
    state: &crate::gateway::runtime::GatewayAppState<R>,
    cli_key: &str,
    provider: &crate::providers::ProviderForGateway,
) -> crate::shared::error::AppResult<String> {
    if provider.auth_mode != "oauth" {
        return Err("SEC_INVALID_INPUT: provider is not oauth mode"
            .to_string()
            .into());
    }

    let details = crate::providers::get_oauth_details(&state.db, provider.id)?;
    if details.cli_key != cli_key {
        return Err(format!(
            "SEC_INVALID_STATE: oauth details cli_key mismatch for provider_id={} (expected={cli_key}, actual={})",
            provider.id, details.cli_key
        )
        .into());
    }
    let oauth_adapter = resolve_oauth_adapter_for_provider(
        cli_key,
        provider.id,
        Some(details.oauth_provider_type.as_str()),
    )?;

    let (refresh_token, token_uri) = match (&details.oauth_refresh_token, &details.oauth_token_uri)
    {
        (Some(rt), Some(tu)) if !rt.trim().is_empty() && !tu.trim().is_empty() => (rt, tu),
        _ => {
            tracing::warn!(
                provider_id = provider.id,
                has_refresh_token = details
                    .oauth_refresh_token
                    .as_ref()
                    .map(|t| !t.trim().is_empty())
                    .unwrap_or(false),
                has_token_uri = details
                    .oauth_token_uri
                    .as_ref()
                    .map(|t| !t.trim().is_empty())
                    .unwrap_or(false),
                "oauth 401 refresh aborted: missing refresh_token or token_uri"
            );
            return Err(
                "OAUTH_REFRESH_FAILED: no refresh_token or token_uri available"
                    .to_string()
                    .into(),
            );
        }
    };

    let client = state.client();
    let refreshed = crate::gateway::oauth::refresh::refresh_provider_token_with_retry(
        &client,
        token_uri,
        details.oauth_client_id.as_deref().unwrap_or(""),
        details.oauth_client_secret.as_deref(),
        refresh_token,
    )
    .await
    .map_err(|e| format!("OAUTH_REFRESH_FAILED: {e}"))?;

    let new_token = refreshed.access_token.trim().to_string();
    if new_token.is_empty() {
        return Err("OAUTH_REFRESH_FAILED: refreshed access_token is empty"
            .to_string()
            .into());
    }

    match crate::providers::update_oauth_tokens_if_last_refreshed_matches(
        &state.db,
        provider.id,
        "oauth",
        oauth_adapter.provider_type(),
        &new_token,
        refreshed.refresh_token.as_deref().or(Some(refresh_token)),
        refreshed
            .id_token
            .as_deref()
            .or(details.oauth_id_token.as_deref()),
        token_uri,
        details.oauth_client_id.as_deref().unwrap_or(""),
        details.oauth_client_secret.as_deref(),
        refreshed.expires_at.or(details.oauth_expires_at),
        details.oauth_email.as_deref(),
        details.oauth_last_refreshed_at,
    ) {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                provider_id = provider.id,
                "oauth 401 refresh: CAS conflict, skipped stale token write"
            );
        }
        Err(err) => {
            tracing::warn!(
                provider_id = provider.id,
                "oauth 401 refresh: token persisted failed (will retry next request): {err}"
            );
        }
    }

    Ok(new_token)
}
