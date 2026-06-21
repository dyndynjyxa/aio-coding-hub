//! Usage: SQLite migration v32->v33 - Split provider pool display order from default route order.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v32_to_v33(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 33;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;
    let now = now_unix_seconds();

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS provider_pool_order (
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, provider_id),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provider_pool_order_cli_sort_order
  ON provider_pool_order(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_provider_pool_order_provider_id
  ON provider_pool_order(provider_id);

CREATE TABLE IF NOT EXISTS default_route_providers (
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, provider_id),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_default_route_providers_cli_sort_order
  ON default_route_providers(cli_key, sort_order);
CREATE INDEX IF NOT EXISTS idx_default_route_providers_provider_id
  ON default_route_providers(provider_id);
"#,
    )
    .map_err(|e| format!("failed to create provider route order tables: {e}"))?;

    let providers_table_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| format!("failed to inspect providers table: {e}"))?
        != 0;

    if providers_table_exists {
        tx.execute(
            r#"
INSERT OR IGNORE INTO provider_pool_order(
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
)
SELECT
  cli_key,
  id,
  sort_order,
  ?1,
  ?1
FROM providers
ORDER BY cli_key ASC, sort_order ASC, id DESC
"#,
            [now],
        )
        .map_err(|e| format!("failed to backfill provider_pool_order: {e}"))?;

        tx.execute(
            r#"
INSERT OR IGNORE INTO default_route_providers(
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
)
SELECT
  cli_key,
  id,
  sort_order,
  ?1,
  ?1
FROM providers
WHERE enabled = 1
ORDER BY cli_key ASC, sort_order ASC, id DESC
"#,
            [now],
        )
        .map_err(|e| format!("failed to backfill default_route_providers: {e}"))?;
    }

    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
    )
    .map_err(|e| format!("failed to create schema_migrations table: {e}"))?;
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, ?)",
        [VERSION, now],
    )
    .map_err(|e| format!("failed to insert schema_migrations row for v{VERSION}: {e}"))?;

    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;

    Ok(())
}
