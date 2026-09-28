//! Usage: SQLite migration v37->v38 - Add the unified provider model policy.

use rusqlite::Connection;

pub(super) fn migrate_v37_to_v38(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v37->v38: {error}"))?;
    let has_table = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'providers')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("failed to inspect v37 providers table: {error}"))?;

    if has_table {
        let has_column = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('providers') WHERE name = 'model_policy_json')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("failed to inspect v37 providers schema: {error}"))?;

        if !has_column {
            tx.execute_batch(
                "ALTER TABLE providers ADD COLUMN model_policy_json TEXT NULL DEFAULT '{\"version\":1,\"mode\":\"all\",\"modelPatterns\":[],\"mappings\":[]}';",
            )
            .map_err(|error| format!("failed to add provider model policy: {error}"))?;
        }
        tx.execute(
            r#"
UPDATE providers
SET model_policy_json = CASE
  WHEN cli_key = 'claude' THEN NULL
  ELSE COALESCE(model_policy_json, '{"version":1,"mode":"all","modelPatterns":[],"mappings":[]}')
END
"#,
            [],
        )
        .map_err(|error| format!("failed to backfill provider model policy: {error}"))?;
    }

    super::set_user_version(&tx, 38)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v37->v38: {error}"))
}
