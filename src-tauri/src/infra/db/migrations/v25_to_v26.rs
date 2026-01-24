//! Usage: SQLite migration v25->v26.

use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;

pub(super) fn migrate_v25_to_v26(conn: &mut Connection) -> Result<(), String> {
    const VERSION: i64 = 26;
    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;

    // Keep schema_migrations available for troubleshooting/diagnostics.
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);
"#,
    )
    .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;

    let mut has_claude_models_json = false;
    {
        let mut stmt = tx
            .prepare("PRAGMA table_info(providers)")
            .map_err(|e| format!("failed to prepare providers table_info query: {e}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| format!("failed to query providers table_info: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers table_info row: {e}"))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| format!("failed to read providers column name: {e}"))?;
            if name == "claude_models_json" {
                has_claude_models_json = true;
                break;
            }
        }
    }

    if !has_claude_models_json {
        tx.execute_batch(
            r#"
ALTER TABLE providers
ADD COLUMN claude_models_json TEXT NOT NULL DEFAULT '{}';
"#,
        )
        .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;
    }

    // Backfill invalid/empty values to keep JSON parsing stable even if DB gets partially corrupted.
    tx.execute_batch(
        r#"
UPDATE providers
SET claude_models_json = '{}'
WHERE claude_models_json IS NULL OR TRIM(claude_models_json) = '';
"#,
    )
    .map_err(|e| format!("failed to migrate v25->v26: {e}"))?;

    fn normalize_slot(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.len() > 200 {
            return Some(trimmed[..200].to_string());
        }
        Some(trimmed.to_string())
    }

    fn main_candidate_score(key_lower: &str) -> i64 {
        if key_lower == "*" {
            return 100;
        }
        if key_lower.contains('*') && key_lower.contains("claude") {
            return 90;
        }
        if key_lower.contains('*') {
            return 80;
        }
        if key_lower.contains("default") || key_lower.contains("main") {
            return 75;
        }
        if key_lower.contains("claude") {
            return 70;
        }
        50
    }

    {
        let mut select_stmt = tx
            .prepare(
                r#"
	SELECT id, model_mapping_json, claude_models_json
	FROM providers
	WHERE cli_key = 'claude'
	"#,
            )
            .map_err(|e| format!("failed to prepare providers migration query: {e}"))?;

        let mut update_stmt = tx
            .prepare(
                r#"
	UPDATE providers
	SET claude_models_json = ?1
	WHERE id = ?2
	"#,
            )
            .map_err(|e| format!("failed to prepare providers migration update: {e}"))?;

        let mut rows = select_stmt
            .query([])
            .map_err(|e| format!("failed to run providers migration query: {e}"))?;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("failed to read providers migration row: {e}"))?
        {
            let id: i64 = row
                .get("id")
                .map_err(|e| format!("failed to read providers.id during migration: {e}"))?;
            let model_mapping_json: String = row.get("model_mapping_json").unwrap_or_default();
            let claude_models_json: String = row.get("claude_models_json").unwrap_or_default();

            // Do not override if user already has a non-empty Claude models config (defensive).
            let claude_models_trimmed = claude_models_json.trim();
            if !claude_models_trimmed.is_empty() && claude_models_trimmed != "{}" {
                continue;
            }

            let mapping: std::collections::HashMap<String, String> =
                serde_json::from_str(&model_mapping_json).unwrap_or_default();

            let mut main_model: Option<String> = None;
            let mut reasoning_model: Option<String> = None;
            let mut haiku_model: Option<String> = None;
            let mut sonnet_model: Option<String> = None;
            let mut opus_model: Option<String> = None;

            let mut main_candidates: Vec<(i64, String, String)> = Vec::new();

            for (raw_key, raw_value) in mapping {
                let key = raw_key.trim();
                let Some(value) = normalize_slot(&raw_value) else {
                    continue;
                };

                let key_lower = key.to_ascii_lowercase();

                if reasoning_model.is_none()
                    && (key_lower.contains("thinking")
                        || key_lower.contains("reasoning")
                        || key_lower.contains("extended"))
                {
                    reasoning_model = Some(value.clone());
                }
                if haiku_model.is_none() && key_lower.contains("haiku") {
                    haiku_model = Some(value.clone());
                }
                if sonnet_model.is_none() && key_lower.contains("sonnet") {
                    sonnet_model = Some(value.clone());
                }
                if opus_model.is_none() && key_lower.contains("opus") {
                    opus_model = Some(value.clone());
                }

                let is_specialized = key_lower.contains("thinking")
                    || key_lower.contains("reasoning")
                    || key_lower.contains("extended")
                    || key_lower.contains("haiku")
                    || key_lower.contains("sonnet")
                    || key_lower.contains("opus");
                if !is_specialized {
                    let score = main_candidate_score(&key_lower);
                    main_candidates.push((score, key.to_string(), value));
                }
            }

            if main_model.is_none() && !main_candidates.is_empty() {
                // Deterministic selection: higher score first, then lexicographic key.
                main_candidates
                    .sort_by(|(sa, ka, _), (sb, kb, _)| sb.cmp(sa).then_with(|| ka.cmp(kb)));
                main_model = Some(main_candidates[0].2.clone());
            }

            let mut obj = serde_json::Map::new();
            if let Some(v) = main_model {
                obj.insert("main_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = reasoning_model {
                obj.insert("reasoning_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = haiku_model {
                obj.insert("haiku_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = sonnet_model {
                obj.insert("sonnet_model".to_string(), serde_json::Value::String(v));
            }
            if let Some(v) = opus_model {
                obj.insert("opus_model".to_string(), serde_json::Value::String(v));
            }

            let next_json = serde_json::to_string(&serde_json::Value::Object(obj))
                .unwrap_or_else(|_| "{}".to_string());

            update_stmt
                .execute(rusqlite::params![next_json, id])
                .map_err(|e| format!("failed to update providers.claude_models_json: {e}"))?;
        }
    }

    tx.execute_batch(
        r#"
UPDATE providers
SET supported_models_json = '{}',
    model_mapping_json = '{}'
WHERE cli_key = 'claude';
"#,
    )
    .map_err(|e| format!("failed to clear legacy provider model config: {e}"))?;

    let applied_at = now_unix_seconds();
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
