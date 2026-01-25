use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub requests_total: i64,
    pub requests_with_usage: i64,
    pub requests_success: i64,
    pub requests_failed: i64,
    pub avg_duration_ms: Option<i64>,
    pub avg_ttfb_ms: Option<i64>,
    pub avg_output_tokens_per_second: Option<f64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub io_total_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_creation_5m_input_tokens: i64,
    pub cache_creation_1h_input_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageProviderRow {
    pub cli_key: String,
    pub provider_id: i64,
    pub provider_name: String,
    pub requests_total: i64,
    pub requests_success: i64,
    pub requests_failed: i64,
    pub avg_duration_ms: Option<i64>,
    pub avg_ttfb_ms: Option<i64>,
    pub avg_output_tokens_per_second: Option<f64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_creation_5m_input_tokens: i64,
    pub cache_creation_1h_input_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageDayRow {
    pub day: String,
    pub requests_total: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_creation_5m_input_tokens: i64,
    pub cache_creation_1h_input_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageHourlyRow {
    pub day: String,
    pub hour: i64,
    pub requests_total: i64,
    pub requests_with_usage: i64,
    pub requests_success: i64,
    pub requests_failed: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageLeaderboardRow {
    pub key: String,
    pub name: String,
    pub requests_total: i64,
    pub requests_success: i64,
    pub requests_failed: i64,
    pub total_tokens: i64,
    pub io_total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub avg_duration_ms: Option<i64>,
    pub avg_ttfb_ms: Option<i64>,
    pub avg_output_tokens_per_second: Option<f64>,
}
