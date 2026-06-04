//! Background OAuth token refresh loop.
//!
//! Periodically scans enabled OAuth providers whose tokens are approaching
//! expiry and proactively refreshes them in the background, so that requests
//! through the gateway don't hit expired-token errors.

use super::refresh::refresh_provider_token_with_retry;
use crate::providers;
use std::future::Future;
use std::time::Duration;
use tokio::sync::watch;

/// How often the loop polls for providers needing refresh.
const POLL_INTERVAL_SECS: u64 = 180;

enum RefreshLoopStep<T> {
    Completed(T),
    Shutdown,
}

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
        match wait_for_next_poll_or_shutdown(
            &mut shutdown_rx,
            Duration::from_secs(POLL_INTERVAL_SECS),
        )
        .await
        {
            RefreshLoopStep::Completed(()) => {}
            RefreshLoopStep::Shutdown => {
                tracing::info!("oauth_refresh_loop: shutdown signal received, exiting");
                return;
            }
        }

        // Query providers that need a token refresh.
        let providers_to_refresh = match await_with_shutdown(
            &mut shutdown_rx,
            crate::blocking::run("oauth_refresh_loop_list", {
                let db = db.clone();
                move || providers::list_oauth_providers_needing_refresh(&db)
            }),
        )
        .await
        {
            RefreshLoopStep::Completed(Ok(list)) => list,
            RefreshLoopStep::Completed(Err(e)) => {
                tracing::warn!("oauth_refresh_loop: failed to list providers: {e}");
                continue;
            }
            RefreshLoopStep::Shutdown => {
                tracing::info!("oauth_refresh_loop: shutdown during provider listing, exiting");
                return;
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
                    match await_with_shutdown(
                        &mut shutdown_rx,
                        crate::blocking::run("oauth_refresh_loop_set_error_adapter", move || {
                            providers::set_oauth_last_error(&db_for_error, provider_id, &err_msg)
                        }),
                    )
                    .await
                    {
                        RefreshLoopStep::Completed(Ok(())) => {}
                        RefreshLoopStep::Completed(Err(err)) => {
                            tracing::warn!(
                                provider_id,
                                "oauth_refresh_loop: failed to persist adapter error: {err}"
                            );
                        }
                        RefreshLoopStep::Shutdown => {
                            tracing::info!(
                                provider_id,
                                provider_type = %provider_type,
                                "oauth_refresh_loop: shutdown while persisting adapter error, exiting"
                            );
                            return;
                        }
                    }
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

            let refresh_result = match await_with_shutdown(
                &mut shutdown_rx,
                refresh_provider_token_with_retry(
                    &client,
                    token_uri,
                    client_id,
                    details.oauth_client_secret.as_deref(),
                    refresh_token,
                ),
            )
            .await
            {
                RefreshLoopStep::Completed(result) => result,
                RefreshLoopStep::Shutdown => {
                    tracing::info!(
                        provider_id,
                        provider_type = %canonical_provider_type,
                        "oauth_refresh_loop: shutdown during token refresh, exiting"
                    );
                    return;
                }
            };

            match refresh_result {
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
                        match await_with_shutdown(
                            &mut shutdown_rx,
                            crate::blocking::run(
                                "oauth_refresh_loop_set_error_empty_effective_token",
                                move || {
                                    providers::set_oauth_last_error(
                                        &db_for_error,
                                        provider_id,
                                        "SEC_INVALID_STATE: resolved effective token is empty",
                                    )
                                },
                            ),
                        )
                        .await
                        {
                            RefreshLoopStep::Completed(Ok(())) => {}
                            RefreshLoopStep::Completed(Err(err)) => {
                                tracing::warn!(
                                    provider_id,
                                    "oauth_refresh_loop: failed to persist empty-token error: {err}"
                                );
                            }
                            RefreshLoopStep::Shutdown => {
                                tracing::info!(
                                    provider_id,
                                    provider_type = %canonical_provider_type,
                                    "oauth_refresh_loop: shutdown while persisting empty-token error, exiting"
                                );
                                return;
                            }
                        }
                        continue;
                    }

                    match await_with_shutdown(
                        &mut shutdown_rx,
                        crate::blocking::run("oauth_refresh_loop_persist", {
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
                        }),
                    )
                    .await
                    {
                        RefreshLoopStep::Completed(Err(e)) => {
                            tracing::error!(
                                provider_id,
                                "oauth_refresh_loop: failed to persist refreshed tokens: {e}"
                            );
                        }
                        RefreshLoopStep::Completed(Ok(false)) => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: skip persist due concurrent token update"
                            );
                        }
                        RefreshLoopStep::Completed(Ok(true)) => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: token refreshed successfully"
                            );
                        }
                        RefreshLoopStep::Shutdown => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: shutdown while persisting refreshed token, exiting"
                            );
                            return;
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
                    match await_with_shutdown(
                        &mut shutdown_rx,
                        crate::blocking::run("oauth_refresh_loop_set_error", move || {
                            providers::set_oauth_last_error(&db, provider_id, &err_msg)
                        }),
                    )
                    .await
                    {
                        RefreshLoopStep::Completed(Ok(())) => {}
                        RefreshLoopStep::Completed(Err(err)) => {
                            tracing::warn!(
                                provider_id,
                                "oauth_refresh_loop: failed to persist refresh error: {err}"
                            );
                        }
                        RefreshLoopStep::Shutdown => {
                            tracing::info!(
                                provider_id,
                                provider_type = %canonical_provider_type,
                                "oauth_refresh_loop: shutdown while persisting refresh error, exiting"
                            );
                            return;
                        }
                    }
                }
            }
        }
    }
}

async fn wait_for_next_poll_or_shutdown(
    shutdown_rx: &mut watch::Receiver<bool>,
    interval: Duration,
) -> RefreshLoopStep<()> {
    if *shutdown_rx.borrow() {
        return RefreshLoopStep::Shutdown;
    }

    tokio::select! {
        _ = tokio::time::sleep(interval) => RefreshLoopStep::Completed(()),
        changed = shutdown_rx.changed() => {
            match changed {
                Ok(()) if *shutdown_rx.borrow() => RefreshLoopStep::Shutdown,
                Ok(()) => RefreshLoopStep::Completed(()),
                Err(_) => RefreshLoopStep::Shutdown,
            }
        }
    }
}

async fn await_with_shutdown<T, F>(
    shutdown_rx: &mut watch::Receiver<bool>,
    future: F,
) -> RefreshLoopStep<T>
where
    F: Future<Output = T>,
{
    tokio::pin!(future);

    loop {
        if *shutdown_rx.borrow() {
            return RefreshLoopStep::Shutdown;
        }

        tokio::select! {
            result = &mut future => return RefreshLoopStep::Completed(result),
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => return RefreshLoopStep::Shutdown,
                    Ok(()) => continue,
                    Err(_) => return RefreshLoopStep::Shutdown,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_next_poll_or_shutdown_completes_after_interval() {
        let (_tx, mut rx) = watch::channel(false);

        let result = wait_for_next_poll_or_shutdown(&mut rx, Duration::from_millis(1)).await;

        match result {
            RefreshLoopStep::Completed(()) => {}
            RefreshLoopStep::Shutdown => panic!("poll interval should complete without shutdown"),
        }
    }

    #[tokio::test]
    async fn wait_for_next_poll_or_shutdown_exits_when_already_shutdown() {
        let (_tx, mut rx) = watch::channel(true);

        let result = wait_for_next_poll_or_shutdown(&mut rx, Duration::from_secs(60)).await;

        match result {
            RefreshLoopStep::Shutdown => {}
            RefreshLoopStep::Completed(()) => panic!("pre-set shutdown should exit immediately"),
        }
    }

    #[tokio::test]
    async fn wait_for_next_poll_or_shutdown_exits_when_sender_dropped() {
        let (tx, mut rx) = watch::channel(false);
        drop(tx);

        let result = tokio::time::timeout(
            Duration::from_millis(100),
            wait_for_next_poll_or_shutdown(&mut rx, Duration::from_secs(60)),
        )
        .await
        .expect("closed shutdown channel should finish promptly");

        match result {
            RefreshLoopStep::Shutdown => {}
            RefreshLoopStep::Completed(()) => panic!("closed shutdown channel should exit"),
        }
    }

    #[tokio::test]
    async fn await_with_shutdown_returns_completed_future_result() {
        let (_tx, mut rx) = watch::channel(false);

        let result = await_with_shutdown(&mut rx, async { 42 }).await;

        match result {
            RefreshLoopStep::Completed(value) => assert_eq!(value, 42),
            RefreshLoopStep::Shutdown => panic!("future should complete before shutdown"),
        }
    }

    #[tokio::test]
    async fn await_with_shutdown_cancels_pending_future_on_shutdown() {
        let (tx, mut rx) = watch::channel(false);
        let task = tokio::spawn(async move {
            await_with_shutdown(&mut rx, std::future::pending::<()>()).await
        });

        tx.send(true).expect("send shutdown");

        let result = tokio::time::timeout(Duration::from_millis(100), task)
            .await
            .expect("shutdown should finish promptly")
            .expect("task should not panic");

        match result {
            RefreshLoopStep::Shutdown => {}
            RefreshLoopStep::Completed(()) => panic!("pending future should not complete"),
        }
    }

    #[tokio::test]
    async fn await_with_shutdown_cancels_pending_result_future_on_shutdown() {
        let (tx, mut rx) = watch::channel(false);
        let task = tokio::spawn(async move {
            await_with_shutdown(&mut rx, std::future::pending::<Result<(), &'static str>>()).await
        });

        tx.send(true).expect("send shutdown");

        let result = tokio::time::timeout(Duration::from_millis(100), task)
            .await
            .expect("shutdown should finish promptly")
            .expect("task should not panic");

        match result {
            RefreshLoopStep::Shutdown => {}
            RefreshLoopStep::Completed(Ok(())) => panic!("pending future should not complete"),
            RefreshLoopStep::Completed(Err(err)) => panic!("unexpected error: {err}"),
        }
    }
}
