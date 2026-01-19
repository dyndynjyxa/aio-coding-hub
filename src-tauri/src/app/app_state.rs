//! Usage: Shared Tauri state types and DB initialization gate used by `commands/*`.

use crate::{blocking, db, gateway};
use std::sync::Mutex;
use tokio::sync::OnceCell;

#[derive(Default)]
pub(crate) struct GatewayState(pub(crate) Mutex<gateway::GatewayManager>);

#[derive(Default)]
pub(crate) struct DbInitState(pub(crate) OnceCell<Result<(), String>>);

pub(crate) async fn ensure_db_ready(
    app: tauri::AppHandle,
    state: &DbInitState,
) -> Result<(), String> {
    state
        .0
        .get_or_init(|| async move { blocking::run("db_init", move || db::init(&app)).await })
        .await
        .clone()
}
