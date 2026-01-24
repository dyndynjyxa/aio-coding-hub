//! Usage: SQLite migration v11->v12.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v11_to_v12(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 12;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v11->v12: {e}"))?;

    let applied_at = now_unix_seconds();
    for (git_url, branch) in [
        ("https://github.com/anthropics/skills", "auto"),
        (
            "https://github.com/ComposioHQ/awesome-claude-skills",
            "auto",
        ),
        (
            "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill",
            "auto",
        ),
    ] {
        tx.execute(
            r#"
INSERT OR IGNORE INTO skill_repos(git_url, branch, enabled, created_at, updated_at)
VALUES (?1, ?2, 1, ?3, ?3)
"#,
            (git_url, branch, applied_at),
        )
        .map_err(|e| format!("failed to seed skill repo {git_url}#{branch}: {e}"))?;
    }

    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        (VERSION, applied_at),
    )
    .map_err(|e| format!("failed to record migration: {e}"))?;

    super::set_user_version(&tx, VERSION)?;

    tx.commit()
        .map_err(|e| format!("failed to commit migration: {e}"))?;

    Ok(())
}
