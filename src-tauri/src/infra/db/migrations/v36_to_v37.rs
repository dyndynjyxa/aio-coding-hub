//! Usage: SQLite migration v36->v37 - Add image generation task history.

use rusqlite::Connection;

pub(super) fn migrate_v36_to_v37(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start v36->v37: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS image_gen_tasks (
  id TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL DEFAULT 'gpt-image',
  prompt TEXT NOT NULL DEFAULT '',
  request_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL DEFAULT 'done',
  error TEXT,
  usage_json TEXT,
  images_json TEXT NOT NULL DEFAULT '[]',
  ref_images_json TEXT NOT NULL DEFAULT '[]',
  dir TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  elapsed_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_image_gen_tasks_created ON image_gen_tasks(created_at DESC);
"#,
    )
    .map_err(|e| format!("failed to migrate v36->v37: {e}"))?;

    super::set_user_version(&tx, 37)?;

    tx.commit()
        .map_err(|e| format!("failed to commit v36->v37: {e}"))?;

    Ok(())
}
