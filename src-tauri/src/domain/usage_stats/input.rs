use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageQueryParams {
    pub period: String,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<String>,
    pub provider_id: Option<i64>,
    pub folder_keys: Option<Vec<String>>,
    pub day_start_hour: Option<i64>,
    pub full_idle_gap_minutes: Option<i64>,
    pub session_break_gap_minutes: Option<i64>,
    #[serde(
        rename = "excludeCx2CcGatewayBridge",
        alias = "excludeCx2ccGatewayBridge"
    )]
    #[specta(rename = "excludeCx2CcGatewayBridge")]
    pub exclude_cx2cc_gateway_bridge: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageDayDetailParams {
    pub day: String,
    pub cli_key: Option<String>,
    pub provider_id: Option<i64>,
    pub folder_limit: Option<u32>,
    pub folder_keys: Option<Vec<String>>,
    pub day_start_hour: Option<i64>,
    #[serde(
        rename = "excludeCx2CcGatewayBridge",
        alias = "excludeCx2ccGatewayBridge"
    )]
    #[specta(rename = "excludeCx2CcGatewayBridge")]
    pub exclude_cx2cc_gateway_bridge: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsageRange {
    Today,
    Last7,
    Last30,
    Month,
    All,
}

pub(super) fn parse_range(input: &str) -> crate::shared::error::AppResult<UsageRange> {
    match input {
        "today" => Ok(UsageRange::Today),
        "last7" => Ok(UsageRange::Last7),
        "last30" => Ok(UsageRange::Last30),
        "month" => Ok(UsageRange::Month),
        "all" => Ok(UsageRange::All),
        _ => Err(format!("SEC_INVALID_INPUT: unknown range={input}").into()),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsageScopeV2 {
    Cli,
    Provider,
    Model,
    Folder,
    Day,
}

pub(super) fn parse_scope_v2(input: &str) -> crate::shared::error::AppResult<UsageScopeV2> {
    match input {
        "cli" => Ok(UsageScopeV2::Cli),
        "provider" => Ok(UsageScopeV2::Provider),
        "model" => Ok(UsageScopeV2::Model),
        "folder" => Ok(UsageScopeV2::Folder),
        "day" => Ok(UsageScopeV2::Day),
        _ => Err(format!("SEC_INVALID_INPUT: unknown scope={input}").into()),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsagePeriodV2 {
    Daily,
    Weekly,
    Monthly,
    AllTime,
    Custom,
}

pub(super) fn parse_period_v2(input: &str) -> crate::shared::error::AppResult<UsagePeriodV2> {
    match input {
        "daily" => Ok(UsagePeriodV2::Daily),
        "weekly" => Ok(UsagePeriodV2::Weekly),
        "monthly" => Ok(UsagePeriodV2::Monthly),
        "allTime" | "all_time" | "all" => Ok(UsagePeriodV2::AllTime),
        "custom" => Ok(UsagePeriodV2::Custom),
        _ => Err(format!("SEC_INVALID_INPUT: unknown period={input}").into()),
    }
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn normalize_cli_filter(
    cli_key: Option<&str>,
) -> crate::shared::error::AppResult<Option<&str>> {
    if let Some(k) = cli_key {
        validate_cli_key(k)?;
        return Ok(Some(k));
    }
    Ok(None)
}

pub(super) fn normalize_provider_id_filter(
    provider_id: Option<i64>,
) -> crate::shared::error::AppResult<Option<i64>> {
    if let Some(id) = provider_id {
        if id <= 0 {
            return Err("SEC_INVALID_INPUT: provider_id must be > 0".into());
        }
        return Ok(Some(id));
    }
    Ok(None)
}

pub(super) fn normalize_day(input: &str) -> crate::shared::error::AppResult<String> {
    let trimmed = input.trim();
    let day = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| format!("SEC_INVALID_INPUT: invalid day={trimmed}"))?;
    Ok(day.format("%Y-%m-%d").to_string())
}

pub(super) fn normalize_folder_keys(
    folder_keys: Option<&[String]>,
) -> crate::shared::error::AppResult<Option<Vec<String>>> {
    let Some(folder_keys) = folder_keys else {
        return Ok(None);
    };

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw in folder_keys {
        let key = raw.trim();
        if key.is_empty() {
            continue;
        }
        if seen.insert(key.to_string()) {
            out.push(key.to_string());
        }
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

pub(super) fn normalize_day_start_hour(value: Option<i64>) -> crate::shared::error::AppResult<i64> {
    let Some(hour) = value else {
        return Ok(0);
    };
    if !(0..=9).contains(&hour) {
        return Err("SEC_INVALID_INPUT: day_start_hour must be between 0 and 9".into());
    }
    Ok(hour)
}

const DEFAULT_FULL_IDLE_GAP_MINUTES: i64 = 15;
const DEFAULT_SESSION_BREAK_GAP_MINUTES: i64 = 30;
const MIN_FULL_IDLE_GAP_MINUTES: i64 = 1;
const MAX_FULL_IDLE_GAP_MINUTES: i64 = 30;
const MIN_SESSION_BREAK_GAP_MINUTES: i64 = 15;
const MAX_SESSION_BREAK_GAP_MINUTES: i64 = 60;
const MILLIS_PER_MINUTE: i64 = 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DevelopmentTimeGapThresholds {
    pub full_idle_gap_ms: i64,
    pub session_break_gap_ms: i64,
}

impl Default for DevelopmentTimeGapThresholds {
    fn default() -> Self {
        Self {
            full_idle_gap_ms: DEFAULT_FULL_IDLE_GAP_MINUTES * MILLIS_PER_MINUTE,
            session_break_gap_ms: DEFAULT_SESSION_BREAK_GAP_MINUTES * MILLIS_PER_MINUTE,
        }
    }
}

pub(super) fn normalize_development_time_gap_thresholds(
    full_idle_gap_minutes: Option<i64>,
    session_break_gap_minutes: Option<i64>,
) -> crate::shared::error::AppResult<DevelopmentTimeGapThresholds> {
    let full_idle_gap_minutes = full_idle_gap_minutes.unwrap_or(DEFAULT_FULL_IDLE_GAP_MINUTES);
    if !(MIN_FULL_IDLE_GAP_MINUTES..=MAX_FULL_IDLE_GAP_MINUTES).contains(&full_idle_gap_minutes) {
        return Err("SEC_INVALID_INPUT: full_idle_gap_minutes must be between 1 and 30".into());
    }

    let session_break_gap_minutes =
        session_break_gap_minutes.unwrap_or(DEFAULT_SESSION_BREAK_GAP_MINUTES);
    if !(MIN_SESSION_BREAK_GAP_MINUTES..=MAX_SESSION_BREAK_GAP_MINUTES)
        .contains(&session_break_gap_minutes)
    {
        return Err(
            "SEC_INVALID_INPUT: session_break_gap_minutes must be between 15 and 60".into(),
        );
    }
    if full_idle_gap_minutes >= session_break_gap_minutes {
        return Err(
            "SEC_INVALID_INPUT: full_idle_gap_minutes must be less than session_break_gap_minutes"
                .into(),
        );
    }

    Ok(DevelopmentTimeGapThresholds {
        full_idle_gap_ms: full_idle_gap_minutes * MILLIS_PER_MINUTE,
        session_break_gap_ms: session_break_gap_minutes * MILLIS_PER_MINUTE,
    })
}

/// Validated and resolved query parameters ready for SQL execution.
pub(super) struct ResolvedQueryParams<'a> {
    pub period: UsagePeriodV2,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub folder_keys: Option<Vec<String>>,
    pub day_start_hour: i64,
    pub development_time_gap_thresholds: DevelopmentTimeGapThresholds,
    pub exclude_cx2cc_gateway_bridge: bool,
}

/// Parse, validate, and compute bounds from raw [`UsageQueryParams`].
///
/// Consolidates the 4-step resolution sequence (parse period, compute bounds,
/// normalize cli_key, normalize provider_id) that was previously duplicated
/// across `summary_v2`, `leaderboard_v2`, and `provider_cache_rate_trend_v1`.
pub(super) fn resolve_query_params<'a>(
    conn: &Connection,
    params: &'a UsageQueryParams,
) -> crate::shared::error::AppResult<ResolvedQueryParams<'a>> {
    let period = parse_period_v2(&params.period)?;
    let day_start_hour = normalize_day_start_hour(params.day_start_hour)?;
    let (start_ts, end_ts) =
        super::compute_bounds_v2(conn, period, params.start_ts, params.end_ts, day_start_hour)?;
    let cli_key = normalize_cli_filter(params.cli_key.as_deref())?;
    let provider_id = normalize_provider_id_filter(params.provider_id)?;
    let folder_keys = normalize_folder_keys(params.folder_keys.as_deref())?;
    let development_time_gap_thresholds = normalize_development_time_gap_thresholds(
        params.full_idle_gap_minutes,
        params.session_break_gap_minutes,
    )?;
    let exclude_cx2cc_gateway_bridge = params.exclude_cx2cc_gateway_bridge.unwrap_or(false);
    Ok(ResolvedQueryParams {
        period,
        start_ts,
        end_ts,
        cli_key,
        provider_id,
        folder_keys,
        day_start_hour,
        development_time_gap_thresholds,
        exclude_cx2cc_gateway_bridge,
    })
}
