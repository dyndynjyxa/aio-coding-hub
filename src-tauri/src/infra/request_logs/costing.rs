//! Usage: Cost and pricing helpers for request logs.

use crate::cost;

use super::RequestLogInsert;

pub(super) fn cost_usd_from_femto(cost_usd_femto: Option<i64>) -> Option<f64> {
    cost_usd_femto
        .filter(|v| *v > 0)
        .map(|v| v as f64 / 1_000_000_000_000_000.0)
}

pub(super) fn is_success_status(status: Option<i64>, error_code: Option<&str>) -> bool {
    status.is_some_and(|v| (200..300).contains(&v)) && error_code.is_none()
}

pub(super) fn usage_for_cost(item: &RequestLogInsert) -> cost::CostUsage {
    cost::CostUsage {
        input_tokens: item.input_tokens.unwrap_or(0),
        output_tokens: item.output_tokens.unwrap_or(0),
        cache_read_input_tokens: item.cache_read_input_tokens.unwrap_or(0),
        cache_creation_input_tokens: item.cache_creation_input_tokens.unwrap_or(0),
        cache_creation_5m_input_tokens: item.cache_creation_5m_input_tokens.unwrap_or(0),
        cache_creation_1h_input_tokens: item.cache_creation_1h_input_tokens.unwrap_or(0),
    }
}

pub(super) fn has_any_cost_usage(usage: &cost::CostUsage) -> bool {
    usage.input_tokens > 0
        || usage.output_tokens > 0
        || usage.cache_read_input_tokens > 0
        || usage.cache_creation_input_tokens > 0
        || usage.cache_creation_5m_input_tokens > 0
        || usage.cache_creation_1h_input_tokens > 0
}
