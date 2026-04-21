//! Usage: Claude provider model validation related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, claude_model_validation, claude_model_validation_history};

#[tauri::command]
#[specta::specta]
pub(crate) async fn claude_provider_validate_model(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    base_url: String,
    request_json: String,
) -> Result<claude_model_validation::ClaudeModelValidationResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    claude_model_validation::validate_provider_model(db, provider_id, &base_url, &request_json)
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn claude_validation_history_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    limit: Option<u32>,
) -> Result<Vec<claude_model_validation_history::ClaudeModelValidationRunRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = limit.unwrap_or(50).clamp(1, 500) as usize;
    blocking::run("claude_validation_history_list", move || {
        claude_model_validation_history::list_runs(&db, provider_id, Some(limit))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn claude_validation_history_clear_provider(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<bool, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("claude_validation_history_clear_provider", move || {
        claude_model_validation_history::clear_provider(&db, provider_id)
    })
    .await
    .map_err(Into::into)
}
