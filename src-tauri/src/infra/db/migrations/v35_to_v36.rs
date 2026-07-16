//! Usage: SQLite migration v35->v36 - Add image generation adapter configs.

use rusqlite::Connection;

pub(super) fn migrate_v35_to_v36(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start v35->v36: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS image_gen_configs (
  adapter_id TEXT PRIMARY KEY,
  base_url TEXT NOT NULL DEFAULT '',
  api_key_plaintext TEXT NOT NULL DEFAULT '',
  model TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v35->v36: {e}"))?;

    super::set_user_version(&tx, 36)?;

    tx.commit()
        .map_err(|e| format!("failed to commit v35->v36: {e}"))?;

    Ok(())
}
