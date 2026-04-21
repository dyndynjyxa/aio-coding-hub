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
}

pub(super) fn parse_scope_v2(input: &str) -> crate::shared::error::AppResult<UsageScopeV2> {
    match input {
        "cli" => Ok(UsageScopeV2::Cli),
        "provider" => Ok(UsageScopeV2::Provider),
        "model" => Ok(UsageScopeV2::Model),
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

/// Validated and resolved query parameters ready for SQL execution.
pub(super) struct ResolvedQueryParams<'a> {
    pub period: UsagePeriodV2,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
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
    let (start_ts, end_ts) =
        super::compute_bounds_v2(conn, period, params.start_ts, params.end_ts)?;
    let cli_key = normalize_cli_filter(params.cli_key.as_deref())?;
    let provider_id = normalize_provider_id_filter(params.provider_id)?;
    Ok(ResolvedQueryParams {
        period,
        start_ts,
        end_ts,
        cli_key,
        provider_id,
    })
}
