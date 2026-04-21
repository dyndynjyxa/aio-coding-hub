//! Usage: Provider limit usage related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, provider_limit_usage};

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_limit_usage_v1(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: Option<String>,
) -> Result<Vec<provider_limit_usage::ProviderLimitUsageRow>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("provider_limit_usage_v1", move || {
        provider_limit_usage::list_v1(&db, cli_key.as_deref())
    })
    .await
    .map_err(Into::into)
}
