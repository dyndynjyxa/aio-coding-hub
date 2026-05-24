//! Usage: Provider availability test Tauri command.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::domain::provider_availability;

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_test_availability(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<provider_availability::ProviderAvailabilityResult, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    provider_availability::test_provider_availability(db, provider_id)
        .await
        .map_err(Into::into)
}
