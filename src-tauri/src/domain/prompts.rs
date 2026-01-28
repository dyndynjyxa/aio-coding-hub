//! Usage: Prompt templates persistence and CLI sync orchestration.

use crate::db;
use crate::prompt_sync;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PromptSummary {
    pub id: i64,
    pub cli_key: String,
    pub name: String,
    pub content: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefaultPromptSyncItem {
    pub cli_key: String,
    pub action: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefaultPromptSyncReport {
    pub items: Vec<DefaultPromptSyncItem>,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn enabled_to_int(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<PromptSummary, rusqlite::Error> {
    Ok(PromptSummary {
        id: row.get("id")?,
        cli_key: row.get("cli_key")?,
        name: row.get("name")?,
        content: row.get("content")?,
        enabled: row.get::<_, i64>("enabled")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_default_lookup(row: &rusqlite::Row<'_>) -> Result<(i64, bool, String), rusqlite::Error> {
    Ok((
        row.get::<_, i64>("id")?,
        row.get::<_, i64>("enabled")? != 0,
        row.get::<_, String>("content")?,
    ))
}

fn get_by_id(conn: &Connection, prompt_id: i64) -> Result<PromptSummary, String> {
    conn.query_row(
        r#"
SELECT
  id,
  cli_key,
  name,
  content,
  enabled,
  created_at,
  updated_at
FROM prompts
WHERE id = ?1
"#,
        params![prompt_id],
        row_to_summary,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query prompt: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: prompt not found".to_string())
}

pub fn list_by_cli(db: &db::Db, cli_key: &str) -> Result<Vec<PromptSummary>, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let conn = db.open_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
SELECT
  id,
  cli_key,
  name,
  content,
  enabled,
  created_at,
  updated_at
FROM prompts
WHERE cli_key = ?1
ORDER BY id DESC
"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| format!("DB_ERROR: failed to list prompts: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("DB_ERROR: failed to read prompt row: {e}"))?);
    }

    Ok(items)
}

fn list_cli_keys() -> [&'static str; 3] {
    crate::shared::cli_key::SUPPORTED_CLI_KEYS
}

fn read_prompt_file_utf8(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<String>, String> {
    let Some(bytes) = prompt_sync::read_target_bytes(app, cli_key)? else {
        return Ok(None);
    };

    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("PROMPT_SYNC_INVALID_UTF8: cli_key={cli_key}"))
}

fn lookup_default_prompt(
    conn: &Connection,
    cli_key: &str,
) -> Result<Option<(i64, bool, String)>, String> {
    conn.query_row(
        r#"
SELECT
  id,
  enabled,
  content
FROM prompts
WHERE cli_key = ?1 AND name = 'default'
LIMIT 1
"#,
        params![cli_key],
        row_default_lookup,
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query default prompt: {e}"))
}

fn count_prompts_by_cli(conn: &Connection, cli_key: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(1) FROM prompts WHERE cli_key = ?1",
        params![cli_key],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| format!("DB_ERROR: failed to count prompts: {e}"))
}

pub fn default_sync_from_files(
    app: &tauri::AppHandle,
    db: &db::Db,
) -> Result<DefaultPromptSyncReport, String> {
    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    let mut items: Vec<DefaultPromptSyncItem> = Vec::new();

    for cli_key in list_cli_keys() {
        validate_cli_key(cli_key)?;

        let default_row = lookup_default_prompt(&conn, cli_key)?;
        match default_row {
            Some((id, enabled, existing_content)) => {
                if !enabled {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("default_disabled".to_string()),
                    });
                    continue;
                }

                let file_content = match read_prompt_file_utf8(app, cli_key) {
                    Ok(v) => v,
                    Err(err) => {
                        items.push(DefaultPromptSyncItem {
                            cli_key: cli_key.to_string(),
                            action: "error".to_string(),
                            message: Some(err),
                        });
                        continue;
                    }
                };
                let Some(file_content) = file_content else {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("file_missing".to_string()),
                    });
                    continue;
                };

                if file_content.trim().is_empty() {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("file_empty".to_string()),
                    });
                    continue;
                }

                if file_content == existing_content {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "unchanged".to_string(),
                        message: None,
                    });
                    continue;
                }

                conn.execute(
                    "UPDATE prompts SET content = ?1, updated_at = ?2 WHERE id = ?3",
                    params![file_content, now, id],
                )
                .map_err(|e| format!("DB_ERROR: failed to update default prompt: {e}"))?;

                items.push(DefaultPromptSyncItem {
                    cli_key: cli_key.to_string(),
                    action: "updated".to_string(),
                    message: None,
                });
            }
            None => {
                let prompt_count = count_prompts_by_cli(&conn, cli_key)?;
                if prompt_count != 0 {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("default_missing".to_string()),
                    });
                    continue;
                }

                let file_content = match read_prompt_file_utf8(app, cli_key) {
                    Ok(v) => v,
                    Err(err) => {
                        items.push(DefaultPromptSyncItem {
                            cli_key: cli_key.to_string(),
                            action: "error".to_string(),
                            message: Some(err),
                        });
                        continue;
                    }
                };
                let Some(file_content) = file_content else {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("file_missing".to_string()),
                    });
                    continue;
                };

                if file_content.trim().is_empty() {
                    items.push(DefaultPromptSyncItem {
                        cli_key: cli_key.to_string(),
                        action: "skipped".to_string(),
                        message: Some("file_empty".to_string()),
                    });
                    continue;
                }

                conn.execute(
                    r#"
INSERT INTO prompts(
  cli_key,
  name,
  content,
  enabled,
  created_at,
  updated_at
) VALUES (?1, 'default', ?2, 1, ?3, ?3)
"#,
                    params![cli_key, file_content, now],
                )
                .map_err(|e| format!("DB_ERROR: failed to insert default prompt: {e}"))?;

                items.push(DefaultPromptSyncItem {
                    cli_key: cli_key.to_string(),
                    action: "created".to_string(),
                    message: None,
                });
            }
        }
    }

    Ok(DefaultPromptSyncReport { items })
}

fn clear_enabled_for_cli(tx: &Connection, cli_key: &str) -> Result<(), String> {
    tx.execute(
        "UPDATE prompts SET enabled = 0 WHERE cli_key = ?1 AND enabled = 1",
        params![cli_key],
    )
    .map_err(|e| format!("DB_ERROR: failed to clear enabled prompts: {e}"))?;
    Ok(())
}

pub fn upsert(
    app: &tauri::AppHandle,
    db: &db::Db,
    prompt_id: Option<i64>,
    cli_key: &str,
    name: &str,
    content: &str,
    enabled: bool,
) -> Result<PromptSummary, String> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: prompt name is required".to_string());
    }

    let content = content.trim();
    if content.is_empty() {
        return Err("SEC_INVALID_INPUT: prompt content is required".to_string());
    }

    let mut conn = db.open_connection()?;
    let now = now_unix_seconds();

    match prompt_id {
        None => {
            let tx = conn
                .transaction()
                .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

            let touched_files = enabled;
            let mut prev_target_bytes: Option<Vec<u8>> = None;
            let mut prev_manifest_bytes: Option<Vec<u8>> = None;

            if enabled {
                clear_enabled_for_cli(&tx, cli_key)?;
                prev_target_bytes = prompt_sync::read_target_bytes(app, cli_key)?;
                prev_manifest_bytes = prompt_sync::read_manifest_bytes(app, cli_key)?;
            }

            tx.execute(
                r#"
INSERT INTO prompts(
  cli_key,
  name,
  content,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
"#,
                params![cli_key, name, content, enabled_to_int(enabled), now, now],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    format!(
                        "DB_CONSTRAINT: prompt already exists for cli_key={cli_key}, name={name}"
                    )
                }
                other => format!("DB_ERROR: failed to insert prompt: {other}"),
            })?;

            let id = tx.last_insert_rowid();

            if enabled {
                if let Err(err) = prompt_sync::apply_enabled_prompt(app, cli_key, id, content) {
                    let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
                    let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
                    return Err(err);
                }
            }

            if let Err(err) = tx.commit() {
                if touched_files {
                    let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
                    let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
                }
                return Err(format!("DB_ERROR: failed to commit: {err}"));
            }

            get_by_id(&conn, id)
        }
        Some(id) => {
            let before = get_by_id(&conn, id)?;

            let tx = conn
                .transaction()
                .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

            let needs_file_apply = enabled;
            let needs_file_restore = before.enabled && !enabled;
            let touched_files = needs_file_apply || needs_file_restore;
            let mut prev_target_bytes: Option<Vec<u8>> = None;
            let mut prev_manifest_bytes: Option<Vec<u8>> = None;
            if touched_files {
                prev_target_bytes = prompt_sync::read_target_bytes(app, cli_key)?;
                prev_manifest_bytes = prompt_sync::read_manifest_bytes(app, cli_key)?;
            }

            let existing_cli_key: Option<String> = tx
                .query_row(
                    "SELECT cli_key FROM prompts WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| format!("DB_ERROR: failed to query prompt: {e}"))?;

            let Some(existing_cli_key) = existing_cli_key else {
                return Err("DB_NOT_FOUND: prompt not found".to_string());
            };

            if existing_cli_key != cli_key {
                return Err("SEC_INVALID_INPUT: cli_key mismatch".to_string());
            }

            if enabled {
                clear_enabled_for_cli(&tx, cli_key)?;
            }

            tx.execute(
                r#"
UPDATE prompts
SET
  name = ?1,
  content = ?2,
  enabled = ?3,
  updated_at = ?4
WHERE id = ?5
"#,
                params![name, content, enabled_to_int(enabled), now, id],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                    format!("DB_CONSTRAINT: prompt name already exists for cli_key={cli_key}, name={name}")
                }
                other => format!("DB_ERROR: failed to update prompt: {other}"),
            })?;

            if touched_files {
                let file_result = if needs_file_restore {
                    prompt_sync::restore_disabled_prompt(app, cli_key)
                } else {
                    Ok(())
                }
                .and_then(|_| {
                    if needs_file_apply {
                        prompt_sync::apply_enabled_prompt(app, cli_key, id, content)
                    } else {
                        Ok(())
                    }
                });

                if let Err(err) = file_result {
                    let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
                    let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
                    return Err(err);
                }
            }

            if let Err(err) = tx.commit() {
                if touched_files {
                    let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
                    let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
                }
                return Err(format!("DB_ERROR: failed to commit: {err}"));
            }

            get_by_id(&conn, id)
        }
    }
}

pub fn set_enabled(
    app: &tauri::AppHandle,
    db: &db::Db,
    prompt_id: i64,
    enabled: bool,
) -> Result<PromptSummary, String> {
    let mut conn = db.open_connection()?;
    let before = get_by_id(&conn, prompt_id)?;
    let cli_key = before.cli_key.as_str();

    let now = now_unix_seconds();

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let needs_file_apply = enabled;
    let needs_file_restore = before.enabled && !enabled;
    let touched_files = needs_file_apply || needs_file_restore;
    let mut prev_target_bytes: Option<Vec<u8>> = None;
    let mut prev_manifest_bytes: Option<Vec<u8>> = None;
    if touched_files {
        prev_target_bytes = prompt_sync::read_target_bytes(app, cli_key)?;
        prev_manifest_bytes = prompt_sync::read_manifest_bytes(app, cli_key)?;
    }

    if enabled {
        clear_enabled_for_cli(&tx, cli_key)?;
        let changed = tx
            .execute(
                "UPDATE prompts SET enabled = 1, updated_at = ?1 WHERE id = ?2",
                params![now, prompt_id],
            )
            .map_err(|e| format!("DB_ERROR: failed to enable prompt: {e}"))?;

        if changed == 0 {
            return Err("DB_NOT_FOUND: prompt not found".to_string());
        }
    } else {
        let changed = tx
            .execute(
                "UPDATE prompts SET enabled = 0, updated_at = ?1 WHERE id = ?2",
                params![now, prompt_id],
            )
            .map_err(|e| format!("DB_ERROR: failed to disable prompt: {e}"))?;

        if changed == 0 {
            return Err("DB_NOT_FOUND: prompt not found".to_string());
        }
    }

    if touched_files {
        let file_result = if needs_file_restore {
            prompt_sync::restore_disabled_prompt(app, cli_key)
        } else {
            Ok(())
        }
        .and_then(|_| {
            if needs_file_apply {
                prompt_sync::apply_enabled_prompt(app, cli_key, prompt_id, &before.content)
            } else {
                Ok(())
            }
        });

        if let Err(err) = file_result {
            let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
            let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
            return Err(err);
        }
    }

    if let Err(err) = tx.commit() {
        if touched_files {
            let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
            let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
        }
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    get_by_id(&conn, prompt_id)
}

pub fn delete(app: &tauri::AppHandle, db: &db::Db, prompt_id: i64) -> Result<(), String> {
    let mut conn = db.open_connection()?;
    let before = get_by_id(&conn, prompt_id)?;

    let cli_key = before.cli_key.as_str();
    let needs_file_restore = before.enabled;

    let tx = conn
        .transaction()
        .map_err(|e| format!("DB_ERROR: failed to start transaction: {e}"))?;

    let mut prev_target_bytes: Option<Vec<u8>> = None;
    let mut prev_manifest_bytes: Option<Vec<u8>> = None;

    if needs_file_restore {
        prev_target_bytes = prompt_sync::read_target_bytes(app, cli_key)?;
        prev_manifest_bytes = prompt_sync::read_manifest_bytes(app, cli_key)?;

        if let Err(err) = prompt_sync::restore_disabled_prompt(app, cli_key) {
            let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
            let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
            return Err(err);
        }
    }

    let changed = tx
        .execute("DELETE FROM prompts WHERE id = ?1", params![prompt_id])
        .map_err(|e| format!("DB_ERROR: failed to delete prompt: {e}"))?;

    if changed == 0 {
        return Err("DB_NOT_FOUND: prompt not found".to_string());
    }

    if let Err(err) = tx.commit() {
        if needs_file_restore {
            let _ = prompt_sync::restore_target_bytes(app, cli_key, prev_target_bytes);
            let _ = prompt_sync::restore_manifest_bytes(app, cli_key, prev_manifest_bytes);
        }
        return Err(format!("DB_ERROR: failed to commit: {err}"));
    }

    Ok(())
}
