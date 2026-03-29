use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::infra::config_migrate;

#[tauri::command]
#[specta::specta]
pub(crate) async fn config_export(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<config_migrate::ConfigBundle, String> {
    tracing::info!("config_export: starting");
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    tracing::info!("config_export: db ready");
    let result = blocking::run("config_export", move || {
        config_migrate::config_export(&app, &db)
    })
    .await
    .map_err(|e| {
        tracing::error!("config_export failed: {}", e);
        e.into()
    });
    tracing::info!("config_export: result={:?}", result.is_ok());
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn config_import(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    bundle: config_migrate::ConfigBundle,
) -> Result<config_migrate::ConfigImportResult, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("config_import", move || {
        config_migrate::config_import(&app, &db, bundle)
    })
    .await
    .map_err(|err| -> String { err.into() })?;

    #[cfg(windows)]
    super::wsl::wsl_sync_trigger::trigger(app.clone());

    Ok(result)
}
