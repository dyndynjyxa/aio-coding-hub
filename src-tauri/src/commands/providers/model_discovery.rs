//! Usage: Thin IPC wrapper for Provider model discovery.

use crate::app::provider_model_discovery;
use crate::app_state::DbInitState;

pub(crate) use provider_model_discovery::{
    ProviderModelDiscoveryInput, ProviderModelDiscoveryResult,
};

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_models_discover(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: ProviderModelDiscoveryInput,
) -> Result<ProviderModelDiscoveryResult, String> {
    provider_model_discovery::provider_models_discover(app, db_state.inner(), input).await
}
