//! Usage: Runtime cache and gateway gating for OAuth provider quota snapshots.

use crate::db;
use crate::shared::error::{db_err, AppError, AppResult};
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};

const TEXT_MAX_CHARS: usize = 96;
const SHORT_LABEL_MAX_CHARS: usize = 32;
const FALLBACK_COOLDOWN_SECS: i64 = 5 * 60;

#[derive(Debug, Clone)]
pub(crate) struct OAuthLimitSnapshotInput<'a> {
    pub provider_id: i64,
    pub limit_short_label: Option<&'a str>,
    pub limit_5h_text: Option<&'a str>,
    pub limit_weekly_text: Option<&'a str>,
    pub limit_5h_reset_at: Option<i64>,
    pub limit_weekly_reset_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OAuthLimitGate {
    Allow,
    Limited { reset_at: Option<i64> },
}

#[derive(Debug, Clone)]
struct OAuthLimitSnapshot {
    limit_5h_text: Option<String>,
    limit_weekly_text: Option<String>,
    limit_5h_reset_at: Option<i64>,
    limit_weekly_reset_at: Option<i64>,
    checked_at: i64,
}

fn validate_provider_id(provider_id: i64) -> AppResult<i64> {
    if provider_id <= 0 {
        return Err(AppError::from(format!(
            "SEC_INVALID_INPUT: invalid provider_id={provider_id}"
        )));
    }
    Ok(provider_id)
}

fn take_first_chars(value: &str, max_chars: usize) -> String {
    if value.chars().nth(max_chars).is_none() {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

fn normalize_text(input: Option<&str>, max_chars: usize) -> Option<String> {
    let value = input.map(str::trim).filter(|value| !value.is_empty())?;
    Some(take_first_chars(value, max_chars))
}

fn normalize_reset_at(input: Option<i64>) -> Option<i64> {
    input.filter(|value| *value > 0)
}

fn update_latest(latest: &mut Option<i64>, candidate: i64) {
    if candidate <= 0 {
        return;
    }
    match latest {
        Some(existing) if *existing >= candidate => {}
        _ => *latest = Some(candidate),
    }
}

fn parse_leading_number(text: &str) -> Option<(f64, &str)> {
    let mut end = 0usize;
    let mut seen_digit = false;
    let mut seen_dot = false;

    for (idx, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            seen_digit = true;
            end = idx + ch.len_utf8();
            continue;
        }
        if ch == '.' && !seen_dot {
            seen_dot = true;
            end = idx + ch.len_utf8();
            continue;
        }
        break;
    }

    if !seen_digit || end == 0 {
        return None;
    }

    let number = text[..end].parse::<f64>().ok()?;
    Some((number, &text[end..]))
}

fn is_exhausted_quota_text(input: Option<&str>) -> bool {
    let Some(text) = input.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let normalized = text.replace(',', "");
    let Some((value, rest)) = parse_leading_number(&normalized) else {
        return false;
    };

    if value.abs() > f64::EPSILON {
        return false;
    }

    let rest = rest.trim_start();
    let starts_with_unit = rest
        .chars()
        .next()
        .is_some_and(|ch| ch == '%' || ch == '/' || ch.is_alphabetic());
    rest.is_empty() || starts_with_unit
}

fn active_exhausted_window_reset_at(
    text: Option<&str>,
    reset_at: Option<i64>,
    checked_at: i64,
    now_unix: i64,
) -> Option<i64> {
    if !is_exhausted_quota_text(text) {
        return None;
    }

    if let Some(reset_at) = reset_at {
        return (reset_at > now_unix).then_some(reset_at);
    }

    let fallback_until = checked_at.saturating_add(FALLBACK_COOLDOWN_SECS);
    (fallback_until > now_unix).then_some(fallback_until)
}

pub(crate) fn save_snapshot(db: &db::Db, input: OAuthLimitSnapshotInput<'_>) -> AppResult<()> {
    let provider_id = validate_provider_id(input.provider_id)?;
    let conn = db.open_connection()?;
    let now = now_unix_seconds();
    let limit_short_label = normalize_text(input.limit_short_label, SHORT_LABEL_MAX_CHARS);
    let limit_5h_text = normalize_text(input.limit_5h_text, TEXT_MAX_CHARS);
    let limit_weekly_text = normalize_text(input.limit_weekly_text, TEXT_MAX_CHARS);
    let limit_5h_reset_at = normalize_reset_at(input.limit_5h_reset_at);
    let limit_weekly_reset_at = normalize_reset_at(input.limit_weekly_reset_at);

    conn.execute(
        r#"
INSERT INTO provider_oauth_limit_snapshots(
  provider_id,
  limit_short_label,
  limit_5h_text,
  limit_weekly_text,
  limit_5h_reset_at,
  limit_weekly_reset_at,
  checked_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
ON CONFLICT(provider_id) DO UPDATE SET
  limit_short_label = excluded.limit_short_label,
  limit_5h_text = excluded.limit_5h_text,
  limit_weekly_text = excluded.limit_weekly_text,
  limit_5h_reset_at = excluded.limit_5h_reset_at,
  limit_weekly_reset_at = excluded.limit_weekly_reset_at,
  checked_at = excluded.checked_at,
  updated_at = excluded.updated_at
"#,
        params![
            provider_id,
            limit_short_label,
            limit_5h_text,
            limit_weekly_text,
            limit_5h_reset_at,
            limit_weekly_reset_at,
            now
        ],
    )
    .map_err(|e| db_err!("failed to save OAuth limit snapshot: {e}"))?;

    Ok(())
}

pub(crate) fn save_exhausted_snapshot(
    db: &db::Db,
    provider_id: i64,
    reset_at: Option<i64>,
) -> AppResult<()> {
    let now = now_unix_seconds();
    let effective_reset_at = match reset_at {
        Some(reset_at) => Some(reset_at),
        None => {
            let conn = db.open_connection()?;
            read_snapshot(&conn, provider_id)?.and_then(|snapshot| {
                [snapshot.limit_5h_reset_at, snapshot.limit_weekly_reset_at]
                    .into_iter()
                    .flatten()
                    .filter(|candidate| *candidate > now)
                    .max()
            })
        }
    };

    save_snapshot(
        db,
        OAuthLimitSnapshotInput {
            provider_id,
            limit_short_label: None,
            limit_5h_text: Some("0"),
            limit_weekly_text: None,
            limit_5h_reset_at: effective_reset_at,
            limit_weekly_reset_at: None,
        },
    )
}

pub(crate) fn clear_snapshot(db: &db::Db, provider_id: i64) -> AppResult<()> {
    let provider_id = validate_provider_id(provider_id)?;
    let conn = db.open_connection()?;
    conn.execute(
        "DELETE FROM provider_oauth_limit_snapshots WHERE provider_id = ?1",
        params![provider_id],
    )
    .map_err(|e| db_err!("failed to clear OAuth limit snapshot: {e}"))?;
    Ok(())
}

fn read_snapshot(conn: &Connection, provider_id: i64) -> AppResult<Option<OAuthLimitSnapshot>> {
    validate_provider_id(provider_id)?;
    conn.query_row(
        r#"
SELECT
  limit_5h_text,
  limit_weekly_text,
  limit_5h_reset_at,
  limit_weekly_reset_at,
  checked_at
FROM provider_oauth_limit_snapshots
WHERE provider_id = ?1
"#,
        params![provider_id],
        |row| {
            Ok(OAuthLimitSnapshot {
                limit_5h_text: row.get(0)?,
                limit_weekly_text: row.get(1)?,
                limit_5h_reset_at: row.get(2)?,
                limit_weekly_reset_at: row.get(3)?,
                checked_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| db_err!("failed to read OAuth limit snapshot: {e}"))
}

pub(crate) fn gate_snapshot(
    conn: &Connection,
    provider_id: i64,
    now_unix: i64,
) -> AppResult<OAuthLimitGate> {
    let Some(snapshot) = read_snapshot(conn, provider_id)? else {
        return Ok(OAuthLimitGate::Allow);
    };

    let mut reset_at = None;
    if let Some(candidate) = active_exhausted_window_reset_at(
        snapshot.limit_5h_text.as_deref(),
        snapshot.limit_5h_reset_at,
        snapshot.checked_at,
        now_unix,
    ) {
        update_latest(&mut reset_at, candidate);
    }
    if let Some(candidate) = active_exhausted_window_reset_at(
        snapshot.limit_weekly_text.as_deref(),
        snapshot.limit_weekly_reset_at,
        snapshot.checked_at,
        now_unix,
    ) {
        update_latest(&mut reset_at, candidate);
    }

    match reset_at {
        Some(reset_at) => Ok(OAuthLimitGate::Limited {
            reset_at: Some(reset_at),
        }),
        None => Ok(OAuthLimitGate::Allow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn create_snapshot_table(conn: &Connection) {
        conn.execute_batch(
            r#"
CREATE TABLE provider_oauth_limit_snapshots (
  provider_id INTEGER PRIMARY KEY,
  limit_short_label TEXT,
  limit_5h_text TEXT,
  limit_weekly_text TEXT,
  limit_5h_reset_at INTEGER,
  limit_weekly_reset_at INTEGER,
  checked_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
"#,
        )
        .expect("create snapshot table");
    }

    fn insert_snapshot(
        conn: &Connection,
        provider_id: i64,
        limit_5h_text: Option<&str>,
        limit_weekly_text: Option<&str>,
        limit_5h_reset_at: Option<i64>,
        limit_weekly_reset_at: Option<i64>,
        checked_at: i64,
    ) {
        conn.execute(
            r#"
INSERT INTO provider_oauth_limit_snapshots(
  provider_id,
  limit_5h_text,
  limit_weekly_text,
  limit_5h_reset_at,
  limit_weekly_reset_at,
  checked_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
"#,
            params![
                provider_id,
                limit_5h_text,
                limit_weekly_text,
                limit_5h_reset_at,
                limit_weekly_reset_at,
                checked_at
            ],
        )
        .expect("insert snapshot");
    }

    fn insert_test_provider(db: &db::Db) -> i64 {
        crate::providers::upsert(
            db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: "OAuth limit snapshot test".to_string(),
                base_urls: vec!["https://example.test".to_string()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(crate::providers::ProviderAuthMode::ApiKey),
                api_key: Some("sk-test".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(0),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert provider")
        .id
    }

    #[test]
    fn exhausted_snapshot_limits_until_latest_reset() {
        let conn = Connection::open_in_memory().expect("open");
        create_snapshot_table(&conn);
        insert_snapshot(
            &conn,
            7,
            Some("0%"),
            Some("0"),
            Some(1_800),
            Some(3_600),
            1_000,
        );

        let gate = gate_snapshot(&conn, 7, 1_200).expect("gate");

        assert_eq!(
            gate,
            OAuthLimitGate::Limited {
                reset_at: Some(3_600)
            }
        );
    }

    #[test]
    fn expired_exhausted_snapshot_allows_provider() {
        let conn = Connection::open_in_memory().expect("open");
        create_snapshot_table(&conn);
        insert_snapshot(&conn, 7, Some("0%"), None, Some(1_800), None, 1_000);

        let gate = gate_snapshot(&conn, 7, 1_800).expect("gate");

        assert_eq!(gate, OAuthLimitGate::Allow);
    }

    #[test]
    fn exhausted_snapshot_without_reset_uses_short_fallback_window() {
        let conn = Connection::open_in_memory().expect("open");
        create_snapshot_table(&conn);
        insert_snapshot(&conn, 7, Some("0 requests"), None, None, None, 1_000);

        let gate = gate_snapshot(&conn, 7, 1_100).expect("gate");
        assert_eq!(
            gate,
            OAuthLimitGate::Limited {
                reset_at: Some(1_000 + FALLBACK_COOLDOWN_SECS)
            }
        );

        let expired = gate_snapshot(&conn, 7, 1_000 + FALLBACK_COOLDOWN_SECS).expect("gate");
        assert_eq!(expired, OAuthLimitGate::Allow);
    }

    #[test]
    fn non_zero_snapshot_allows_provider() {
        let conn = Connection::open_in_memory().expect("open");
        create_snapshot_table(&conn);
        insert_snapshot(
            &conn,
            7,
            Some("1%"),
            Some("2"),
            Some(1_800),
            Some(3_600),
            1_000,
        );

        let gate = gate_snapshot(&conn, 7, 1_200).expect("gate");

        assert_eq!(gate, OAuthLimitGate::Allow);
    }

    #[test]
    fn refreshed_available_snapshot_overwrites_exhausted_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("oauth-limits.db")).expect("init db");
        let now = now_unix_seconds();
        let provider_id = insert_test_provider(&db);

        save_exhausted_snapshot(&db, provider_id, Some(now + 3_600)).expect("save exhausted");
        {
            let conn = db.open_connection().expect("open");
            let gate = gate_snapshot(&conn, provider_id, now).expect("gate");
            assert_eq!(
                gate,
                OAuthLimitGate::Limited {
                    reset_at: Some(now + 3_600)
                }
            );
        }

        save_snapshot(
            &db,
            OAuthLimitSnapshotInput {
                provider_id,
                limit_short_label: Some("5h"),
                limit_5h_text: Some("25%"),
                limit_weekly_text: Some("80%"),
                limit_5h_reset_at: None,
                limit_weekly_reset_at: None,
            },
        )
        .expect("save refreshed snapshot");

        let conn = db.open_connection().expect("open");
        let gate = gate_snapshot(&conn, provider_id, now).expect("gate");
        assert_eq!(gate, OAuthLimitGate::Allow);
    }

    #[test]
    fn exhausted_snapshot_preserves_existing_future_reset_when_missing_new_reset() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("oauth-limits-preserve-reset.db"))
            .expect("init db");
        let now = now_unix_seconds();
        let provider_id = insert_test_provider(&db);

        save_snapshot(
            &db,
            OAuthLimitSnapshotInput {
                provider_id,
                limit_short_label: Some("5h"),
                limit_5h_text: Some("1%"),
                limit_weekly_text: Some("10%"),
                limit_5h_reset_at: Some(now + 1_800),
                limit_weekly_reset_at: Some(now + 86_400),
            },
        )
        .expect("save current snapshot");

        save_exhausted_snapshot(&db, provider_id, None).expect("save exhausted snapshot");

        let conn = db.open_connection().expect("open");
        let gate = gate_snapshot(&conn, provider_id, now).expect("gate");
        assert_eq!(
            gate,
            OAuthLimitGate::Limited {
                reset_at: Some(now + 86_400)
            }
        );
    }
}
