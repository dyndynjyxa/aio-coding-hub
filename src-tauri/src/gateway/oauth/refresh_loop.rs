//! Background OAuth token refresh loop.
//!
//! Periodically scans enabled OAuth providers whose tokens are approaching
//! expiry and proactively refreshes them in the background, so that requests
//! through the gateway don't hit expired-token errors.

use super::refresh::refresh_provider_token_with_retry;
use crate::providers;
use tokio::sync::watch;

/// How often the loop polls for providers needing refresh.
const POLL_INTERVAL_SECS: u64 = 180;

/// Spawns the background OAuth refresh loop.
///
/// The loop runs until `shutdown_rx` receives a signal (the gateway stop path
/// should send it). Returns a `JoinHandle` that can be used to await termination.
pub(crate) fn spawn(
    db: crate::db::Db,
    shutdown_rx: watch::Receiver<bool>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        run_loop(db, shutdown_rx).await;
    })
}

async fn run_loop(db: crate::db::Db, mut shutdown_rx: watch::Receiver<bool>) {
    let client = match super::build_default_oauth_http_client() {
        Ok(client) => client,
        Err(err) => {
            tracing::error!("oauth_refresh_loop: failed to build http client: {err}");
            return;
        }
    };

    tracing::info!("oauth_refresh_loop: started (poll_interval={POLL_INTERVAL_SECS}s)");

    loop {
        // Wait for the poll interval or shutdown signal.
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)) => {}
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("oauth_refresh_loop: shutdown signal received, exiting");
                    return;
                }
            }
        }

        // Query providers that need a token refresh.
        let providers_to_refresh = match crate::blocking::run("oauth_refresh_loop_list", {
            let db = db.clone();
            move || providers::list_oauth_providers_needing_refresh(&db)
        })
        .await
        {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("oauth_refresh_loop: failed to list providers: {e}");
                continue;
            }
        };

        if providers_to_refresh.is_empty() {
            continue;
        }

        tracing::debug!(
            "oauth_refresh_loop: {} provider(s) need token refresh",
            providers_to_refresh.len()
        );

        for details in providers_to_refresh {
            // Check for shutdown between each provider to avoid blocking exit.
            if *shutdown_rx.borrow() {
                tracing::info!("oauth_refresh_loop: shutdown during provider iteration, exiting");
                return;
            }

            let provider_id = details.id;
            let provider_type = details.oauth_provider_type.clone();
            let oauth_adapter = match super::registry::resolve_oauth_adapter_for_details(&details) {
                Ok(adapter) => adapter,
                Err(err) => {
                    tracing::warn!(
                        provider_id,
                        cli_key = %details.cli_key,
                        provider_type = %provider_type,
                        "oauth_refresh_loop: skipping — adapter resolution failed: {err}"
                    );
                    let db_for_error = db.clone();
                    let err_msg = err.clone();
                    let _ =
                        crate::blocking::run("oauth_refresh_loop_set_error_adapter", move || {
                            providers::set_oauth_last_error(&db_for_error, provider_id, &err_msg)
                        })
                        .await;
                    continue;
                }
            };
            let canonical_provider_type = oauth_adapter.provider_type().to_string();

            let Some(ref refresh_token) = details.oauth_refresh_token else {
                continue;
            };

            let Some(ref token_uri) = details.oauth_token_uri else {
                tracing::warn!(
                    provider_id,
                    provider_type = %canonical_provider_type,
                    "oauth_refresh_loop: skipping — missing token_uri"
                );
                continue;
            };

            let Some(ref client_id) = details.oauth_client_id else {
                tracing::warn!(
                    provider_id,
                    provider_type = %canonical_provider_type,
                    "oauth_refresh_loop: skipping — missing client_id"
                );
                continue;
            };

            tracing::info!(
                provider_id,
                provider_type = %canonical_provider_type,
                "oauth_refresh_loop: refreshing token"
            );

            match refresh_provider_token_with_retry(
                &client,
                token_uri,
                client_id,
                details.oauth_client_secret.as_deref(),
                refresh_token,
            )
            .await
            {
                Ok(token_set) => {
                    let db = db.clone();
                    let provider_type_owned = canonical_provider_type.clone();
                    let token_uri_owned = token_uri.clone();
                    let client_id_owned = client_id.clone();
                    let client_secret = details.oauth_client_secret.clone();
                    let email = details.oauth_email.clone();
                    let expected_last_refreshed_at = details.oauth_last_refreshed_at;

                    let new_refresh_token = token_set
                        .refresh_token
                        .as_deref()
                        .or(Some(refresh_token.as_str()));

                    let (effective_token, resolved_id_token) = oauth_adapter
                        .resolve_effective_token(&token_set, details.oauth_id_token.as_deref());

                    if effective_token.trim().is_empty() {
                        tracing::warn!(
                            provider_id,
                            provider_type = %canonical_provider_type,
                            "oauth_refresh_loop: skipping persist — effective token resolved empty"
                        );
                        let db_for_error = db.clone();
                        let _ = crate::blocking::run(
                            "oauth_refresh_loop_set_error_empty_effective_token",
                            move || {
                                providers::set_oauth_last_error(
                                    &db_for_error,
                                    provider_id,
                                    "SEC_INVALID_STATE: resolved effective token is empty",
                                )
                            },
                        )
                        .await;
                        continue;
                    }

                    match crate::blocking::run("oauth_refresh_loop_persist", {
                        let access_token = effective_token;
                        let new_refresh_token = new_refresh_token.map(str::to_string);
                        let new_id_token = resolved_id_token;
                        let expires_at = token_set.expires_at;
                        move || {
                            providers::update_oauth_tokens_if_last_refreshed_matches(
                                &db,
                                provider_id,
                                "oauth",
                                &provider_type_owned,
                                &access_token,
                                new_refresh_token.as_deref(),
                                new_id_token.as_deref(),
                                &token_uri_owned,
                                &client_id_owned,
                                client_secret.as_deref(),
                                expires_at,
                                email.as_deref(),
                                expected_last_refreshed_at,
                            )
                        }
                    })
                    .await
                    {
                        Err(e) => {
                            tracing::error!(
                                provider_id,
                                "oauth_refresh_loop: failed to persist refreshed tokens: {e}"
                            );
                        }
                        Ok(false) => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: skip persist due concurrent token update"
                            );
                        }
                        Ok(true) => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: token refreshed successfully"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        provider_id,
                        provider_type = %canonical_provider_type,
                        "oauth_refresh_loop: refresh failed: {e}"
                    );

                    // Persist the error for UI display.
                    let db = db.clone();
                    let err_msg = e.clone();
                    let _ = crate::blocking::run("oauth_refresh_loop_set_error", move || {
                        providers::set_oauth_last_error(&db, provider_id, &err_msg)
                    })
                    .await;
                }
            }
        }
    }
}
