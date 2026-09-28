//! Usage: image_gen_configs table queries (get/upsert; the key plaintext never leaves Rust).

use crate::db;
use crate::shared::error::{db_err, AppResult};
use crate::shared::time::now_unix_seconds;
use rusqlite::OptionalExtension;

const MAX_ADAPTER_ID_CHARS: usize = 64;
const MAX_BASE_URL_CHARS: usize = 2048;
const MAX_MODEL_CHARS: usize = 256;
const MAX_API_KEY_CHARS: usize = 4096;

/// IPC-facing view: intentionally has no api_key field, only a configured flag.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenConfigView {
    pub adapter_id: String,
    pub base_url: String,
    pub model: String,
    pub api_key_configured: bool,
}

fn normalize_adapter_id(adapter_id: &str) -> Result<String, String> {
    let adapter_id = adapter_id.trim();
    if adapter_id.is_empty() {
        return Err("SEC_INVALID_INPUT: adapter_id is required".to_string());
    }
    if adapter_id.len() > MAX_ADAPTER_ID_CHARS {
        return Err("SEC_INVALID_INPUT: adapter_id is too long".to_string());
    }
    Ok(adapter_id.to_string())
}

fn read_row(db: &db::Db, adapter_id: &str) -> AppResult<Option<(String, String, String)>> {
    let conn = db.open_connection()?;
    conn.query_row(
        "SELECT base_url, model, api_key_plaintext FROM image_gen_configs WHERE adapter_id = ?1",
        rusqlite::params![adapter_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(|e| db_err!("failed to query image gen config: {e}"))
}

/// Missing row is not an error: returns an empty, unconfigured view.
pub(crate) fn config_get(db: &db::Db, adapter_id: &str) -> AppResult<ImageGenConfigView> {
    let adapter_id = normalize_adapter_id(adapter_id)?;
    let (base_url, model, api_key) = read_row(db, &adapter_id)?.unwrap_or_default();

    Ok(ImageGenConfigView {
        adapter_id,
        base_url,
        model,
        api_key_configured: !api_key.trim().is_empty(),
    })
}

/// `api_key` semantics: `None` = preserve current value, `Some("")` = clear,
/// `Some(value)` = replace.
pub(crate) fn config_set(
    db: &db::Db,
    adapter_id: &str,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
) -> AppResult<ImageGenConfigView> {
    let adapter_id = normalize_adapter_id(adapter_id)?;
    let base_url = base_url.trim();
    let model = model.trim();
    if base_url.len() > MAX_BASE_URL_CHARS {
        return Err("SEC_INVALID_INPUT: base_url is too long".to_string().into());
    }
    if model.len() > MAX_MODEL_CHARS {
        return Err("SEC_INVALID_INPUT: model is too long".to_string().into());
    }

    let now = now_unix_seconds();
    let conn = db.open_connection()?;
    match api_key {
        None => conn
            .execute(
                r#"
INSERT INTO image_gen_configs(adapter_id, base_url, model, api_key_plaintext, created_at, updated_at)
VALUES (?1, ?2, ?3, '', ?4, ?4)
ON CONFLICT(adapter_id) DO UPDATE SET
  base_url = excluded.base_url,
  model = excluded.model,
  updated_at = excluded.updated_at
"#,
                rusqlite::params![adapter_id, base_url, model, now],
            )
            .map_err(|e| db_err!("failed to upsert image gen config: {e}"))?,
        Some(key) => {
            let key = key.trim();
            if key.len() > MAX_API_KEY_CHARS {
                return Err("SEC_INVALID_INPUT: api_key is too long".to_string().into());
            }
            conn.execute(
                r#"
INSERT INTO image_gen_configs(adapter_id, base_url, model, api_key_plaintext, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?5)
ON CONFLICT(adapter_id) DO UPDATE SET
  base_url = excluded.base_url,
  model = excluded.model,
  api_key_plaintext = excluded.api_key_plaintext,
  updated_at = excluded.updated_at
"#,
                rusqlite::params![adapter_id, base_url, model, key, now],
            )
            .map_err(|e| db_err!("failed to upsert image gen config: {e}"))?
        }
    };
    // Release the pooled connection before config_get acquires its own.
    drop(conn);

    config_get(db, &adapter_id)
}

/// Connection material `(base_url, api_key)` for outbound requests only.
/// Never expose this through IPC or logs.
pub(crate) fn config_connection(db: &db::Db, adapter_id: &str) -> AppResult<(String, String)> {
    let adapter_id = normalize_adapter_id(adapter_id)?;
    let (base_url, _model, api_key) = read_row(db, &adapter_id)?.ok_or_else(|| {
        String::from("SEC_INVALID_INPUT: image gen config is not set for this adapter")
    })?;
    Ok((base_url, api_key))
}
