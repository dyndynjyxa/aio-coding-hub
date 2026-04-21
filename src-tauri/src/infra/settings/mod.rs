//! Usage: Persisted application settings (schema + read/write helpers).

mod defaults;
mod migration;
mod persistence;
mod types;

// Re-export public API (preserves identical surface for all consumers).
pub use defaults::{
    DEFAULT_CX2CC_FALLBACK_MODEL, DEFAULT_GATEWAY_PORT,
    DEFAULT_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS, DEFAULT_PROVIDER_COOLDOWN_SECONDS,
    DEFAULT_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS,
    DEFAULT_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS,
    DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS, MAX_GATEWAY_PORT,
    MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS, SCHEMA_VERSION,
};
pub use persistence::{clear_cache, log_retention_days_fail_open, read, write};
pub use types::{
    AppSettings, CodexHomeMode, GatewayListenMode, HomeUsagePeriod, WslHostAddressMode,
    WslTargetCli,
};
