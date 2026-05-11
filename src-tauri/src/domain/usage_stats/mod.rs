//! Usage: Usage analytics queries and aggregation helpers backed by sqlite.

mod bounds;
mod cache_rate_trend_v1;
mod day_detail;
mod filters;
mod folder_options;
mod folders;
mod hourly;
mod input;
mod leaderboard_range;
mod leaderboard_v2;
mod summary;
mod tokens;
mod types;

pub use cache_rate_trend_v1::provider_cache_rate_trend_v1;
pub use day_detail::day_detail_v1;
pub use folder_options::folder_options_v1;
pub use folders::{UsageResolvedFolder, UsageSessionLookupKey};
pub use hourly::hourly_series;
pub use input::{UsageDayDetailParams, UsageQueryParams};
pub use leaderboard_range::{leaderboard_day, leaderboard_provider};
pub use leaderboard_v2::leaderboard_v2;
pub use summary::{summary, summary_v2};
pub use types::{
    UsageDayDetailV1, UsageDayFolderRow, UsageDayHourRow, UsageDayRow, UsageFolderOptionV1,
    UsageHourlyRow, UsageLeaderboardRow, UsageProviderCacheRateTrendRowV1, UsageProviderRow,
    UsageSummary,
};

use bounds::{compute_bounds_v2, compute_start_ts, compute_start_ts_last_n_days};
use input::{
    normalize_cli_filter, normalize_folder_keys, parse_range, parse_scope_v2, resolve_query_params,
    UsagePeriodV2, UsageRange, UsageScopeV2,
};
use leaderboard_range::{extract_final_provider, has_valid_provider_key, ProviderAgg, ProviderKey};
use tokens::{
    sql_effective_input_tokens_expr_with_alias, sql_effective_total_tokens_expr,
    sql_effective_total_tokens_expr_with_alias, token_total, SQL_EFFECTIVE_INPUT_TOKENS_EXPR,
};

#[cfg(test)]
mod tests;
