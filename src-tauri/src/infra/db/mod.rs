//! Usage: SQLite connection setup, schema migrations, and common DB helpers.

mod migrations;

use crate::app_paths;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Duration;

const DB_FILE_NAME: &str = "aio-coding-hub.db";
const BUSY_TIMEOUT: Duration = Duration::from_millis(2000);

#[derive(Clone)]
pub(crate) struct Db {
    pool: Pool<SqliteConnectionManager>,
}

impl Db {
    pub(crate) fn open_connection(
        &self,
    ) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("DB_ERROR: failed to get connection from pool: {e}"))
    }
}

pub(crate) fn sql_placeholders(count: usize) -> String {
    if count == 0 {
        return String::new();
    }

    let mut out = String::with_capacity(count.saturating_mul(2).saturating_sub(1));
    for idx in 0..count {
        if idx > 0 {
            out.push(',');
        }
        out.push('?');
    }
    out
}

pub fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_paths::app_data_dir(app)?.join(DB_FILE_NAME))
}

pub fn init(app: &tauri::AppHandle) -> Result<Db, String> {
    let path = db_path(app)?;
    let path_hint = path.to_string_lossy();

    let manager = SqliteConnectionManager::file(&path).with_init(|conn| {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        configure_connection(conn)
    });

    let pool = Pool::new(manager).map_err(|e| format!("failed to create db pool: {e}"))?;
    let mut conn = pool
        .get()
        .map_err(|e| format!("failed to get startup connection: {e}"))?;

    migrations::apply_migrations(&mut conn)
        .map_err(|e| format!("sqlite migration failed at {path_hint}: {e}"))?;

    Ok(Db { pool })
}

fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
"#,
    )?;

    Ok(())
}
