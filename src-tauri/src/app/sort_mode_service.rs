//! Usage: Sort mode mutations that coordinate persistence and route runtime state.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_control::app_gateway_clear_cli_route_runtime_state;
use crate::{blocking, sort_modes};

pub(crate) async fn sort_mode_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let (affected_cli_keys, mapping_sources_changed) =
        blocking::run("sort_mode_delete", move || {
            crate::app::provider_service::with_codex_mapping_tracking(&db, true, || {
                sort_modes::delete_mode_with_affected_cli_keys(&db, mode_id)
            })
        })
        .await?;

    for cli_key in affected_cli_keys {
        app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    }
    crate::app::provider_service::refresh_codex_catalog_after_routing_change(
        &app,
        refresh_db,
        mapping_sources_changed,
    );
    Ok(true)
}

pub(crate) async fn sort_mode_active_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    mode_id: Option<i64>,
) -> Result<sort_modes::SortModeActiveRow, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let cli_key_for_db = cli_key.clone();
    let row = blocking::run("sort_mode_active_set", move || {
        sort_modes::set_active(&db, &cli_key_for_db, mode_id)
    })
    .await?;
    app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    Ok(row)
}

pub(crate) async fn sort_mode_providers_set_order(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<sort_modes::SortModeProviderRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let cli_key_for_db = cli_key.clone();
    let (rows, mapping_sources_changed) =
        blocking::run("sort_mode_providers_set_order", move || {
            crate::app::provider_service::with_codex_mapping_tracking(
                &db,
                cli_key_for_db == "codex",
                || {
                    sort_modes::set_mode_providers_order(
                        &db,
                        mode_id,
                        &cli_key_for_db,
                        ordered_provider_ids,
                    )
                },
            )
        })
        .await?;

    app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    crate::app::provider_service::refresh_codex_catalog_after_routing_change(
        &app,
        refresh_db,
        mapping_sources_changed,
    );
    Ok(rows)
}

pub(crate) async fn sort_mode_provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mode_id: i64,
    cli_key: String,
    provider_id: i64,
    enabled: bool,
) -> Result<sort_modes::SortModeProviderRow, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let refresh_db = db.clone();
    let cli_key_for_db = cli_key.clone();
    let (row, mapping_sources_changed) =
        blocking::run("sort_mode_provider_set_enabled", move || {
            crate::app::provider_service::with_codex_mapping_tracking(
                &db,
                cli_key_for_db == "codex",
                || {
                    sort_modes::set_mode_provider_enabled(
                        &db,
                        mode_id,
                        &cli_key_for_db,
                        provider_id,
                        enabled,
                    )
                },
            )
        })
        .await?;

    app_gateway_clear_cli_route_runtime_state(&app, &cli_key);
    crate::app::provider_service::refresh_codex_catalog_after_routing_change(
        &app,
        refresh_db,
        mapping_sources_changed,
    );
    Ok(row)
}
