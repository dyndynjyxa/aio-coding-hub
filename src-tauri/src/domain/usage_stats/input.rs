#[derive(Debug, Clone, Copy)]
pub(super) enum UsageRange {
    Today,
    Last7,
    Last30,
    Month,
    All,
}

pub(super) fn parse_range(input: &str) -> Result<UsageRange, String> {
    match input {
        "today" => Ok(UsageRange::Today),
        "last7" => Ok(UsageRange::Last7),
        "last30" => Ok(UsageRange::Last30),
        "month" => Ok(UsageRange::Month),
        "all" => Ok(UsageRange::All),
        _ => Err(format!("SEC_INVALID_INPUT: unknown range={input}")),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsageScopeV2 {
    Cli,
    Provider,
    Model,
}

pub(super) fn parse_scope_v2(input: &str) -> Result<UsageScopeV2, String> {
    match input {
        "cli" => Ok(UsageScopeV2::Cli),
        "provider" => Ok(UsageScopeV2::Provider),
        "model" => Ok(UsageScopeV2::Model),
        _ => Err(format!("SEC_INVALID_INPUT: unknown scope={input}")),
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

pub(super) fn parse_period_v2(input: &str) -> Result<UsagePeriodV2, String> {
    match input {
        "daily" => Ok(UsagePeriodV2::Daily),
        "weekly" => Ok(UsagePeriodV2::Weekly),
        "monthly" => Ok(UsagePeriodV2::Monthly),
        "allTime" | "all_time" | "all" => Ok(UsagePeriodV2::AllTime),
        "custom" => Ok(UsagePeriodV2::Custom),
        _ => Err(format!("SEC_INVALID_INPUT: unknown period={input}")),
    }
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn normalize_cli_filter(cli_key: Option<&str>) -> Result<Option<&str>, String> {
    if let Some(k) = cli_key {
        validate_cli_key(k)?;
        return Ok(Some(k));
    }
    Ok(None)
}
