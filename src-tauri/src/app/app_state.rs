//! Usage: Shared DB initialization gate used by `commands/*`.

use crate::shared::error::AppResult;
use crate::{blocking, db};
use tokio::sync::{Mutex as AsyncMutex, MutexGuard};

#[derive(Default)]
pub(crate) struct DbInitState(pub(crate) AsyncMutex<Option<AppResult<db::Db>>>);

pub(crate) async fn ensure_db_ready<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: &DbInitState,
) -> AppResult<db::Db> {
    let mut guard = state.0.lock().await;
    if let Some(db) = guard.as_ref() {
        return db.clone();
    }

    let db = blocking::run("db_init", move || db::init(&app)).await;
    *guard = Some(db.clone());
    db
}

pub(crate) async fn prepare_db_reset<'a>(
    state: &'a DbInitState,
) -> MutexGuard<'a, Option<AppResult<db::Db>>> {
    let mut guard = state.0.lock().await;
    // Hold the cache lock through file deletion so no concurrent command can
    // recreate the pool midway through a destructive reset.
    let _ = guard.take();
    guard
}
