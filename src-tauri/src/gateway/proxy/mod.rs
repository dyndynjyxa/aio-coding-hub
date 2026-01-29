//! Usage: Gateway proxy module facade (exports the proxy handler + shared types).

mod abort_guard;
mod caches;
mod cli_proxy_guard;
mod errors;
mod failover;
mod forwarder;
mod handler;
mod http_util;
mod logging;
mod model_rewrite;
pub(in crate::gateway) mod provider_router;
mod request_context;
mod request_end;
mod types;
mod upstream_client_error_rules;

pub(super) use caches::{ProviderBaseUrlPingCache, RecentErrorCache};
pub(in crate::gateway) use logging::spawn_enqueue_request_log_with_backpressure;
pub(super) use types::ErrorCategory;

pub(super) use handler::proxy_impl;

pub(super) struct RequestLogEnqueueArgs {
    pub(super) trace_id: String,
    pub(super) cli_key: String,
    pub(super) session_id: Option<String>,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) query: Option<String>,
    pub(super) excluded_from_stats: bool,
    pub(super) special_settings_json: Option<String>,
    pub(super) status: Option<u16>,
    pub(super) error_code: Option<&'static str>,
    pub(super) duration_ms: u128,
    pub(super) ttfb_ms: Option<u128>,
    pub(super) attempts_json: String,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) created_at: i64,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
}
