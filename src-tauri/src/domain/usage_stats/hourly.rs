use crate::db;
use rusqlite::params;

use super::{compute_start_ts_last_n_days, UsageHourlyRow};

pub fn hourly_series(db: &db::Db, days: u32) -> Result<Vec<UsageHourlyRow>, String> {
    let conn = db.open_connection()?;
    let days = days.clamp(1, 60);
    let start_ts = compute_start_ts_last_n_days(&conn, days)?;

    let mut stmt = conn
        .prepare(
            r#"
	SELECT
	  strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') AS day,
	  CAST(strftime('%H', created_at, 'unixepoch', 'localtime') AS INTEGER) AS hour,
	  COUNT(*) AS requests_total,
	  SUM(
	    CASE WHEN (
	      total_tokens IS NOT NULL OR
	      input_tokens IS NOT NULL OR
	      output_tokens IS NOT NULL OR
	      cache_read_input_tokens IS NOT NULL OR
	      cache_creation_input_tokens IS NOT NULL OR
	      cache_creation_5m_input_tokens IS NOT NULL OR
	      cache_creation_1h_input_tokens IS NOT NULL OR
	      usage_json IS NOT NULL
	    ) THEN 1 ELSE 0 END
	  ) AS requests_with_usage,
	  SUM(CASE WHEN status >= 200 AND status < 300 AND error_code IS NULL THEN 1 ELSE 0 END) AS requests_success,
	  SUM(
	    CASE WHEN (
	      status IS NULL OR
	      status < 200 OR
	      status >= 300 OR
	      error_code IS NOT NULL
	    ) THEN 1 ELSE 0 END
	  ) AS requests_failed,
	  SUM(COALESCE(total_tokens, COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0))) AS total_tokens
	FROM request_logs
	WHERE excluded_from_stats = 0
	AND created_at >= ?1
	GROUP BY day, hour
	ORDER BY day ASC, hour ASC
	"#,
        )
        .map_err(|e| format!("DB_ERROR: failed to prepare hourly series query: {e}"))?;

    let rows = stmt
        .query_map(params![start_ts], |row| {
            Ok(UsageHourlyRow {
                day: row.get("day")?,
                hour: row.get("hour")?,
                requests_total: row.get("requests_total")?,
                requests_with_usage: row
                    .get::<_, Option<i64>>("requests_with_usage")?
                    .unwrap_or(0),
                requests_success: row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0),
                requests_failed: row.get::<_, Option<i64>>("requests_failed")?.unwrap_or(0),
                total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
            })
        })
        .map_err(|e| format!("DB_ERROR: failed to run hourly series query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("DB_ERROR: failed to read hourly row: {e}"))?);
    }
    Ok(out)
}
