use rusqlite::{params, Connection};

use super::{UsagePeriodV2, UsageRange};

pub(super) fn compute_bounds_v2(
    conn: &Connection,
    period: UsagePeriodV2,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> Result<(Option<i64>, Option<i64>), String> {
    match period {
        UsagePeriodV2::Daily => Ok((compute_start_ts(conn, UsageRange::Today)?, None)),
        UsagePeriodV2::Weekly => Ok((compute_start_ts(conn, UsageRange::Last7)?, None)),
        UsagePeriodV2::Monthly => Ok((compute_start_ts(conn, UsageRange::Month)?, None)),
        UsagePeriodV2::AllTime => Ok((None, None)),
        UsagePeriodV2::Custom => {
            let start_ts = start_ts
                .ok_or_else(|| "SEC_INVALID_INPUT: custom period requires start_ts".to_string())?;
            let end_ts = end_ts
                .ok_or_else(|| "SEC_INVALID_INPUT: custom period requires end_ts".to_string())?;
            if start_ts >= end_ts {
                return Err(
                    "SEC_INVALID_INPUT: custom range requires start_ts < end_ts".to_string()
                );
            }
            Ok((Some(start_ts), Some(end_ts)))
        }
    }
}

pub(super) fn compute_start_ts(
    conn: &Connection,
    range: UsageRange,
) -> Result<Option<i64>, String> {
    let sql = match range {
        UsageRange::All => return Ok(None),
        UsageRange::Today => {
            "SELECT CAST(strftime('%s','now','localtime','start of day','utc') AS INTEGER)"
        }
        UsageRange::Last7 => {
            "SELECT CAST(strftime('%s','now','localtime','start of day','-6 days','utc') AS INTEGER)"
        }
        UsageRange::Last30 => {
            "SELECT CAST(strftime('%s','now','localtime','start of day','-29 days','utc') AS INTEGER)"
        }
        UsageRange::Month => {
            "SELECT CAST(strftime('%s','now','localtime','start of month','utc') AS INTEGER)"
        }
    };

    let ts = conn
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("DB_ERROR: failed to compute range start ts: {e}"))?;

    Ok(Some(ts))
}

pub(super) fn compute_start_ts_last_n_days(conn: &Connection, days: u32) -> Result<i64, String> {
    if days < 1 {
        return Err("SEC_INVALID_INPUT: days must be >= 1".to_string());
    }
    let offset_days = days.saturating_sub(1);
    let modifier = format!("-{offset_days} days");

    let ts = conn
        .query_row(
            "SELECT CAST(strftime('%s','now','localtime','start of day', ?1,'utc') AS INTEGER)",
            params![modifier],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| format!("DB_ERROR: failed to compute last-days start ts: {e}"))?;

    Ok(ts)
}
