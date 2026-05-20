//! Usage: SQLite connection setup, schema migrations, and common DB helpers.

mod migrations;

use crate::app_paths;
use crate::shared::error::db_err;
use crate::shared::error::AppResult;
use crate::shared::fs::read_file_with_max_len;
use crate::shared::time::now_unix_seconds;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

const DB_FILE_NAME: &str = "aio-coding-hub.db";
const BUSY_TIMEOUT_DEFAULT: Duration = Duration::from_millis(2000);
const POOL_MAX_SIZE_DEFAULT: u32 = 8;
const POOL_MIN_IDLE_DEFAULT: u32 = 1;
const POOL_CONNECTION_TIMEOUT_DEFAULT: Duration = Duration::from_secs(5);
const PRAGMA_SYNCHRONOUS_DEFAULT: &str = "NORMAL";
const PRAGMA_MMAP_SIZE_DEFAULT: i64 = 268_435_456;
const DB_OPTIMIZE_STAMP_FILE_NAME: &str = "db_optimize.stamp";
const DB_OPTIMIZE_MIN_INTERVAL_SECS: i64 = 24 * 60 * 60;
const DB_OPTIMIZE_STAMP_MAX_BYTES: usize = 64;

#[derive(Debug, Clone)]
struct DbRuntimeConfig {
    busy_timeout: Duration,
    pool_max_size: u32,
    pool_min_idle: u32,
    pool_connection_timeout: Duration,
    pragma_synchronous: String,
    pragma_mmap_size: i64,
    pragma_cache_size: Option<i64>,
    pragma_wal_autocheckpoint: Option<i64>,
    pragma_journal_size_limit: Option<i64>,
}

impl DbRuntimeConfig {
    fn from_env() -> Self {
        Self::from_env_get(|key| env::var(key).ok())
    }

    fn from_env_get(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let busy_timeout = get("AIO_DB_BUSY_TIMEOUT_MS")
            .as_deref()
            .and_then(parse_u64_trimmed)
            .filter(|v| *v > 0)
            .map(Duration::from_millis)
            .unwrap_or(BUSY_TIMEOUT_DEFAULT);

        let pool_max_size = get("AIO_DB_POOL_MAX_SIZE")
            .as_deref()
            .and_then(parse_u32_trimmed)
            .filter(|v| *v > 0)
            .unwrap_or(POOL_MAX_SIZE_DEFAULT);

        let pool_min_idle_raw = get("AIO_DB_POOL_MIN_IDLE")
            .as_deref()
            .and_then(parse_u32_trimmed)
            .unwrap_or(POOL_MIN_IDLE_DEFAULT);
        let pool_min_idle = pool_min_idle_raw.min(pool_max_size);

        let pool_connection_timeout = get("AIO_DB_POOL_CONNECTION_TIMEOUT_MS")
            .as_deref()
            .and_then(parse_u64_trimmed)
            .filter(|v| *v > 0)
            .map(Duration::from_millis)
            .unwrap_or(POOL_CONNECTION_TIMEOUT_DEFAULT);

        let pragma_synchronous = get("AIO_DB_PRAGMA_SYNCHRONOUS")
            .as_deref()
            .and_then(parse_pragma_synchronous)
            .unwrap_or_else(|| PRAGMA_SYNCHRONOUS_DEFAULT.to_string());

        let pragma_mmap_size = get("AIO_DB_PRAGMA_MMAP_SIZE")
            .as_deref()
            .and_then(parse_i64_trimmed)
            .filter(|v| *v >= 0)
            .unwrap_or(PRAGMA_MMAP_SIZE_DEFAULT);

        let pragma_cache_size = get("AIO_DB_PRAGMA_CACHE_SIZE")
            .as_deref()
            .and_then(parse_i64_trimmed)
            .filter(|v| *v != 0);

        let pragma_wal_autocheckpoint = get("AIO_DB_PRAGMA_WAL_AUTOCHECKPOINT")
            .as_deref()
            .and_then(parse_i64_trimmed)
            .filter(|v| *v > 0);

        let pragma_journal_size_limit = get("AIO_DB_PRAGMA_JOURNAL_SIZE_LIMIT")
            .as_deref()
            .and_then(parse_i64_trimmed)
            .filter(|v| *v >= 0);

        Self {
            busy_timeout,
            pool_max_size,
            pool_min_idle,
            pool_connection_timeout,
            pragma_synchronous,
            pragma_mmap_size,
            pragma_cache_size,
            pragma_wal_autocheckpoint,
            pragma_journal_size_limit,
        }
    }
}

fn parse_u32_trimmed(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u32>().ok()
}

fn parse_u64_trimmed(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

fn parse_i64_trimmed(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

fn parse_pragma_synchronous(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "OFF" | "NORMAL" | "FULL" | "EXTRA" => Some(normalized),
        _ => None,
    }
}

#[derive(Clone)]
pub(crate) struct Db {
    pool: Pool<SqliteConnectionManager>,
}

impl Db {
    pub(crate) fn open_connection(
        &self,
    ) -> AppResult<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| db_err!("failed to get connection from pool: {e}"))
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

pub fn db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join(DB_FILE_NAME))
}

pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<Db> {
    let path = db_path(app)?;
    let path_hint = path.to_string_lossy();

    let config = DbRuntimeConfig::from_env();
    if config.pool_min_idle < POOL_MIN_IDLE_DEFAULT {
        tracing::warn!(
            pool_min_idle = config.pool_min_idle,
            pool_min_idle_default = POOL_MIN_IDLE_DEFAULT,
            "sqlite pool min idle lowered from default"
        );
    }
    tracing::info!(
        busy_timeout_ms = config.busy_timeout.as_millis(),
        pool_max_size = config.pool_max_size,
        pool_min_idle = config.pool_min_idle,
        pool_connection_timeout_ms = config.pool_connection_timeout.as_millis(),
        pragma_synchronous = %config.pragma_synchronous,
        pragma_mmap_size = config.pragma_mmap_size,
        pragma_cache_size = config.pragma_cache_size,
        pragma_wal_autocheckpoint = config.pragma_wal_autocheckpoint,
        pragma_journal_size_limit = config.pragma_journal_size_limit,
        db_optimize_enabled = db_optimize_enabled(),
        "sqlite runtime config"
    );

    let manager = SqliteConnectionManager::file(&path).with_init({
        let config = config.clone();
        move |conn| {
            conn.busy_timeout(config.busy_timeout)?;
            configure_connection(conn, &config)
        }
    });

    let pool = Pool::builder()
        .max_size(config.pool_max_size)
        .min_idle(Some(config.pool_min_idle))
        .connection_timeout(config.pool_connection_timeout)
        .build(manager)
        .map_err(|e| db_err!("failed to create db pool: {e}"))?;
    let mut conn = pool
        .get()
        .map_err(|e| db_err!("failed to get startup connection: {e}"))?;

    migrations::apply_migrations(&mut conn)
        .map_err(|e| format!("sqlite migration failed at {path_hint}: {e}"))?;

    maybe_run_db_optimize(app, &conn);

    Ok(Db { pool })
}

#[cfg(test)]
pub(crate) fn init_for_tests(path: &std::path::Path) -> AppResult<Db> {
    let config = DbRuntimeConfig::from_env();
    let manager = SqliteConnectionManager::file(path).with_init({
        let config = config.clone();
        move |conn| {
            conn.busy_timeout(config.busy_timeout)?;
            configure_connection(conn, &config)
        }
    });

    let pool = Pool::builder()
        .max_size(1)
        .min_idle(Some(1))
        .connection_timeout(config.pool_connection_timeout)
        .build(manager)
        .map_err(|e| db_err!("failed to create test db pool: {e}"))?;
    let mut conn = pool
        .get()
        .map_err(|e| db_err!("failed to get startup connection: {e}"))?;

    migrations::apply_migrations(&mut conn).map_err(|e| format!("sqlite migration failed: {e}"))?;

    Ok(Db { pool })
}

fn db_optimize_enabled() -> bool {
    env::var("AIO_DB_ENABLE_OPTIMIZE")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .is_some_and(|v| v == "1" || v == "true" || v == "yes")
}

fn maybe_run_db_optimize<R: tauri::Runtime>(app: &tauri::AppHandle<R>, conn: &Connection) {
    if !db_optimize_enabled() {
        return;
    }

    let now = now_unix_seconds();
    let stamp_path = match app_paths::app_data_dir(app) {
        Ok(dir) => dir.join(DB_OPTIMIZE_STAMP_FILE_NAME),
        Err(err) => {
            tracing::warn!("sqlite optimize skipped: failed to resolve app_data_dir: {err}");
            return;
        }
    };

    let last_run = read_db_optimize_stamp(&stamp_path);

    if last_run > 0 && now.saturating_sub(last_run) < DB_OPTIMIZE_MIN_INTERVAL_SECS {
        tracing::debug!(
            last_run = last_run,
            now = now,
            "sqlite optimize skipped (recently ran)"
        );
        return;
    }

    if let Err(err) = conn.execute_batch("PRAGMA optimize;") {
        tracing::warn!("sqlite optimize failed: {err}");
        return;
    }

    if let Err(err) = std::fs::write(&stamp_path, format!("{now}\n")) {
        tracing::warn!(
            path = %stamp_path.display(),
            "sqlite optimize ran but failed to write stamp file: {err}"
        );
        return;
    }

    tracing::info!("sqlite optimize completed");
}

fn read_db_optimize_stamp(path: &std::path::Path) -> i64 {
    read_file_with_max_len(path, DB_OPTIMIZE_STAMP_MAX_BYTES)
        .ok()
        .and_then(|bytes| {
            std::str::from_utf8(&bytes)
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok())
        })
        .unwrap_or(0)
}

fn configure_connection(conn: &Connection, config: &DbRuntimeConfig) -> rusqlite::Result<()> {
    let mut sql = format!(
        r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = {synchronous};
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = {mmap_size};
"#,
        synchronous = config.pragma_synchronous.as_str(),
        mmap_size = config.pragma_mmap_size
    );

    if let Some(cache_size) = config.pragma_cache_size {
        sql.push_str(&format!("PRAGMA cache_size = {cache_size};\n"));
    }
    if let Some(wal_autocheckpoint) = config.pragma_wal_autocheckpoint {
        sql.push_str(&format!(
            "PRAGMA wal_autocheckpoint = {wal_autocheckpoint};\n"
        ));
    }
    if let Some(journal_size_limit) = config.pragma_journal_size_limit {
        sql.push_str(&format!(
            "PRAGMA journal_size_limit = {journal_size_limit};\n"
        ));
    }

    conn.execute_batch(&sql)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn sql_placeholders_zero_returns_empty() {
        assert_eq!(sql_placeholders(0), "");
    }

    #[test]
    fn sql_placeholders_one_returns_single_question_mark() {
        assert_eq!(sql_placeholders(1), "?");
    }

    #[test]
    fn sql_placeholders_three_returns_comma_separated() {
        assert_eq!(sql_placeholders(3), "?,?,?");
    }

    #[test]
    fn sql_placeholders_large_count() {
        let result = sql_placeholders(5);
        assert_eq!(result, "?,?,?,?,?");
        // Verify no trailing comma
        assert!(!result.ends_with(','));
        assert!(!result.starts_with(','));
    }

    #[test]
    fn db_runtime_config_defaults_match_constants() {
        let cfg = DbRuntimeConfig::from_env_get(|_| None);
        assert_eq!(cfg.busy_timeout, BUSY_TIMEOUT_DEFAULT);
        assert_eq!(cfg.pool_max_size, POOL_MAX_SIZE_DEFAULT);
        assert_eq!(cfg.pool_min_idle, POOL_MIN_IDLE_DEFAULT);
        assert_eq!(cfg.pool_connection_timeout, POOL_CONNECTION_TIMEOUT_DEFAULT);
        assert_eq!(cfg.pragma_synchronous, PRAGMA_SYNCHRONOUS_DEFAULT);
        assert_eq!(cfg.pragma_mmap_size, PRAGMA_MMAP_SIZE_DEFAULT);
        assert_eq!(cfg.pragma_cache_size, None);
        assert_eq!(cfg.pragma_wal_autocheckpoint, None);
        assert_eq!(cfg.pragma_journal_size_limit, None);
    }

    #[test]
    fn db_runtime_config_parses_env_values() {
        let vars: HashMap<&str, &str> = HashMap::from([
            ("AIO_DB_BUSY_TIMEOUT_MS", "1500"),
            ("AIO_DB_POOL_MAX_SIZE", "12"),
            ("AIO_DB_POOL_MIN_IDLE", "10"),
            ("AIO_DB_POOL_CONNECTION_TIMEOUT_MS", "2500"),
            ("AIO_DB_PRAGMA_SYNCHRONOUS", "full"),
            ("AIO_DB_PRAGMA_MMAP_SIZE", "123"),
            ("AIO_DB_PRAGMA_CACHE_SIZE", "-64000"),
            ("AIO_DB_PRAGMA_WAL_AUTOCHECKPOINT", "2000"),
            ("AIO_DB_PRAGMA_JOURNAL_SIZE_LIMIT", "1048576"),
        ]);
        let cfg = DbRuntimeConfig::from_env_get(|key| vars.get(key).map(|v| (*v).to_string()));
        assert_eq!(cfg.busy_timeout, Duration::from_millis(1500));
        assert_eq!(cfg.pool_max_size, 12);
        assert_eq!(cfg.pool_min_idle, 10);
        assert_eq!(cfg.pool_connection_timeout, Duration::from_millis(2500));
        assert_eq!(cfg.pragma_synchronous, "FULL");
        assert_eq!(cfg.pragma_mmap_size, 123);
        assert_eq!(cfg.pragma_cache_size, Some(-64000));
        assert_eq!(cfg.pragma_wal_autocheckpoint, Some(2000));
        assert_eq!(cfg.pragma_journal_size_limit, Some(1048576));
    }

    #[test]
    fn db_runtime_config_clamps_min_idle_to_max_size() {
        let vars: HashMap<&str, &str> = HashMap::from([
            ("AIO_DB_POOL_MAX_SIZE", "4"),
            ("AIO_DB_POOL_MIN_IDLE", "10"),
        ]);
        let cfg = DbRuntimeConfig::from_env_get(|key| vars.get(key).map(|v| (*v).to_string()));
        assert_eq!(cfg.pool_max_size, 4);
        assert_eq!(cfg.pool_min_idle, 4);
    }

    #[test]
    fn db_runtime_config_ignores_invalid_values() {
        let vars: HashMap<&str, &str> = HashMap::from([
            ("AIO_DB_BUSY_TIMEOUT_MS", "0"),
            ("AIO_DB_POOL_MAX_SIZE", "0"),
            ("AIO_DB_POOL_CONNECTION_TIMEOUT_MS", "nope"),
            ("AIO_DB_PRAGMA_SYNCHRONOUS", "invalid"),
            ("AIO_DB_PRAGMA_MMAP_SIZE", "-1"),
            ("AIO_DB_PRAGMA_WAL_AUTOCHECKPOINT", "0"),
            ("AIO_DB_PRAGMA_CACHE_SIZE", "0"),
        ]);
        let cfg = DbRuntimeConfig::from_env_get(|key| vars.get(key).map(|v| (*v).to_string()));
        assert_eq!(cfg.busy_timeout, BUSY_TIMEOUT_DEFAULT);
        assert_eq!(cfg.pool_max_size, POOL_MAX_SIZE_DEFAULT);
        assert_eq!(cfg.pool_connection_timeout, POOL_CONNECTION_TIMEOUT_DEFAULT);
        assert_eq!(cfg.pragma_synchronous, PRAGMA_SYNCHRONOUS_DEFAULT);
        assert_eq!(cfg.pragma_mmap_size, PRAGMA_MMAP_SIZE_DEFAULT);
        assert_eq!(cfg.pragma_wal_autocheckpoint, None);
        assert_eq!(cfg.pragma_cache_size, None);
    }

    #[test]
    fn db_optimize_stamp_rejects_oversized_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(DB_OPTIMIZE_STAMP_FILE_NAME);
        std::fs::write(&path, vec![b'1'; DB_OPTIMIZE_STAMP_MAX_BYTES + 1]).expect("write stamp");

        assert_eq!(read_db_optimize_stamp(&path), 0);
    }
}
