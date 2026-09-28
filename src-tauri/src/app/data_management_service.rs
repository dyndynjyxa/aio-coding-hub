//! Usage: Restore CLI proxy configurations before deleting application data.

use crate::app_state::{prepare_db_reset, DbInitState};
use crate::shared::error::AppResult;
use crate::{blocking, data_management};

pub(crate) async fn reset_app_data<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_state: &DbInitState,
) -> AppResult<bool> {
    // Keep starts and background writers out through restoration and deletion.
    let _gateway_lifecycle = super::gateway_lifecycle_lock::lock().await;
    super::cleanup::stop_gateway_best_effort_unlocked(&app).await;
    blocking::run("app_data_reset_restore_cli_proxy", {
        let app = app.clone();
        move || {
            crate::cli_proxy::restore_enabled_keep_state(&app)
                .and_then(super::cli_proxy_service::require_success)
        }
    })
    .await?;
    let _db_reset_guard = prepare_db_reset(db_state).await;
    blocking::run("app_data_reset", move || {
        data_management::app_data_reset(&app)
    })
    .await
}
