//! Usage: Request log DTOs and insertion payloads.

use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RequestLogInsert {
    pub trace_id: String,
    pub cli_key: String,
    pub session_id: Option<String>,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub excluded_from_stats: bool,
    pub special_settings_json: Option<String>,
    pub status: Option<i64>,
    pub error_code: Option<String>,
    pub duration_ms: i64,
    pub ttfb_ms: Option<i64>,
    pub attempts_json: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_creation_5m_input_tokens: Option<i64>,
    pub cache_creation_1h_input_tokens: Option<i64>,
    pub usage_json: Option<String>,
    pub requested_model: Option<String>,
    pub created_at_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestLogRouteHop {
    pub provider_id: i64,
    pub provider_name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestLogSummary {
    pub id: i64,
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub requested_model: Option<String>,
    pub status: Option<i64>,
    pub error_code: Option<String>,
    pub duration_ms: i64,
    pub ttfb_ms: Option<i64>,
    pub attempt_count: i64,
    pub has_failover: bool,
    pub start_provider_id: i64,
    pub start_provider_name: String,
    pub final_provider_id: i64,
    pub final_provider_name: String,
    pub route: Vec<RequestLogRouteHop>,
    pub session_reuse: bool,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_creation_5m_input_tokens: Option<i64>,
    pub cache_creation_1h_input_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub cost_multiplier: f64,
    pub created_at_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestLogDetail {
    pub id: i64,
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub excluded_from_stats: bool,
    pub special_settings_json: Option<String>,
    pub status: Option<i64>,
    pub error_code: Option<String>,
    pub duration_ms: i64,
    pub ttfb_ms: Option<i64>,
    pub attempts_json: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_creation_5m_input_tokens: Option<i64>,
    pub cache_creation_1h_input_tokens: Option<i64>,
    pub usage_json: Option<String>,
    pub requested_model: Option<String>,
    pub cost_usd: Option<f64>,
    pub cost_multiplier: f64,
    pub created_at_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionStatsAggregate {
    pub request_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd_femto: i64,
    pub total_duration_ms: i64,
}
