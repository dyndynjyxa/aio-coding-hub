//! Usage: SQLite migration v38->v39 - Add vendor column to model_prices.

use rusqlite::Connection;

pub(super) fn migrate_v38_to_v39(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v38->v39: {error}"))?;
    let has_table = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'model_prices')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("failed to inspect v38 model_prices table: {error}"))?;

    if has_table {
        let has_column = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('model_prices') WHERE name = 'vendor')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| format!("failed to inspect v38 model_prices schema: {error}"))?;

        if !has_column {
            tx.execute_batch(
                "ALTER TABLE model_prices ADD COLUMN vendor TEXT NOT NULL DEFAULT '';",
            )
            .map_err(|error| format!("failed to add model_prices vendor: {error}"))?;
        }
        // Best-effort backfill for rows synced before vendor existed; the next
        // price sync overwrites with the real upstream vendor.
        tx.execute(
            r#"
UPDATE model_prices
SET vendor = CASE cli_key
  WHEN 'claude' THEN 'anthropic'
  WHEN 'codex' THEN 'openai'
  WHEN 'gemini' THEN 'google'
  WHEN 'grok' THEN 'xai'
  ELSE ''
END
WHERE vendor = ''
"#,
            [],
        )
        .map_err(|error| format!("failed to backfill model_prices vendor: {error}"))?;
    }

    super::set_user_version(&tx, 39)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v38->v39: {error}"))
}
