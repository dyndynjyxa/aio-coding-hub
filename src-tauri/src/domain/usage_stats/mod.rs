//! Usage: Usage analytics queries and aggregation helpers backed by sqlite.

mod bounds;
mod hourly;
mod input;
mod leaderboard_range;
mod leaderboard_v2;
mod summary;
mod tokens;
mod types;

pub use hourly::hourly_series;
pub use leaderboard_range::{leaderboard_day, leaderboard_provider};
pub use leaderboard_v2::leaderboard_v2;
pub use summary::{summary, summary_v2};
pub use types::{UsageDayRow, UsageHourlyRow, UsageLeaderboardRow, UsageProviderRow, UsageSummary};

use bounds::{compute_bounds_v2, compute_start_ts, compute_start_ts_last_n_days};
use input::{
    normalize_cli_filter, parse_period_v2, parse_range, parse_scope_v2, UsagePeriodV2, UsageRange,
    UsageScopeV2,
};
use leaderboard_range::{
    extract_final_provider, has_valid_provider_key, is_success, ProviderAgg, ProviderKey,
};
use tokens::{
    effective_input_tokens, effective_total_tokens, sql_effective_total_tokens_expr, token_total,
    SQL_EFFECTIVE_INPUT_TOKENS_EXPR,
};

#[cfg(test)]
use leaderboard_v2::leaderboard_v2_with_conn;
#[cfg(test)]
use summary::summary_query;

#[cfg(test)]
mod tests;
