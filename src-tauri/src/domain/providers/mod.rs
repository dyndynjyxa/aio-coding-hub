//! Usage: Provider configuration persistence and gateway selection helpers.

mod queries;
mod types;
mod validation;

pub use types::{
    ClaudeModels, DailyResetMode, ProviderAuthMode, ProviderBaseUrlMode,
    ProviderExtensionValuesInput, ProviderSummary, ProviderUpsertParams,
};

#[allow(unused_imports)]
pub(crate) use types::{
    is_cx2cc_bridge, ClaudeTerminalLaunchContext, GatewayProvidersSelection, ProviderForGateway,
    ProviderOAuthDetails, ProviderRouteRow,
};

pub use queries::{
    default_route_list, default_route_set_order, delete, duplicate, get_api_key_plaintext,
    list_by_cli, names_by_id, reorder, upsert,
};

pub(crate) use queries::{
    active_sort_mode_id_for_gateway, claude_terminal_launch_context, clear_oauth, cli_key_by_id,
    get_by_id, get_oauth_details, get_source_provider_for_gateway,
    list_enabled_for_gateway_in_mode, list_enabled_for_gateway_using_active_mode,
    list_oauth_providers_needing_refresh, set_enabled, set_oauth_last_error, update_oauth_tokens,
    update_oauth_tokens_if_last_refreshed_matches,
};

#[cfg(test)]
use types::{claude_models_from_json, normalize_model_slot, MAX_MODEL_NAME_LEN};
#[cfg(test)]
use validation::{
    base_urls_from_row, normalize_base_urls, normalize_reset_time_hms_lossy,
    normalize_reset_time_hms_strict, parse_reset_time_hms, validate_limit_usd, MAX_LIMIT_USD,
    MAX_PROVIDER_BASE_URLS, MAX_PROVIDER_BASE_URL_CHARS, MAX_PROVIDER_NOTE_CHARS,
    MAX_PROVIDER_ORDER_IDS,
};

#[cfg(test)]
mod tests;
