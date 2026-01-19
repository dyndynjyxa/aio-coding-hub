use crate::{blocking, db};
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;

fn base_urls_from_row(base_url_fallback: &str, base_urls_json: &str) -> Vec<String> {
    let mut parsed: Vec<String> = serde_json::from_str::<Vec<String>>(base_urls_json)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    let mut seen: HashSet<String> = HashSet::with_capacity(parsed.len());
    parsed.retain(|v| seen.insert(v.clone()));

    if parsed.is_empty() {
        let fallback = base_url_fallback.trim();
        if fallback.is_empty() {
            return vec![String::new()];
        }
        return vec![fallback.to_string()];
    }

    parsed
}

pub(super) async fn load_provider(
    app: tauri::AppHandle,
    provider_id: i64,
) -> Result<super::ProviderForValidation, String> {
    blocking::run("claude_provider_validate_model_load_provider", move || {
        if provider_id <= 0 {
            return Err(format!(
                "SEC_INVALID_INPUT: invalid provider_id={provider_id}"
            ));
        }

        let conn = db::open_connection(&app)?;
        let row: Option<(i64, String, String, String, String, String)> = conn
            .query_row(
                r#"
SELECT
  id,
  cli_key,
  name,
  base_url,
  base_urls_json,
  api_key_plaintext
FROM providers
WHERE id = ?1
"#,
                params![provider_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("DB_ERROR: failed to query provider: {e}"))?;

        let Some((id, cli_key, name, base_url_fallback, base_urls_json, api_key_plaintext)) = row
        else {
            return Err("DB_NOT_FOUND: provider not found".to_string());
        };

        let base_urls = base_urls_from_row(&base_url_fallback, &base_urls_json);

        Ok(super::ProviderForValidation {
            id,
            cli_key,
            name,
            base_urls,
            api_key_plaintext,
        })
    })
    .await
}
