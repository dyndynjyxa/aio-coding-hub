//! Usage: Claude model validation facade (public APIs + shared constants).

use crate::db;
use std::time::Duration;

mod execute;
mod masking;
mod padding;
mod provider;
mod request;
mod response;
mod types;
mod workflow;

pub use types::ClaudeModelValidationResult;

// Keep these internal types visible at `super::*` for sibling modules that reference them.
use types::{
    CacheRoundtripConfig, ParsedRequest, ProviderForValidation, RoundtripConfig,
    SignatureRoundtripConfig,
};

const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_EXCERPT_BYTES: usize = 16 * 1024;
const MAX_TEXT_PREVIEW_CHARS: usize = 4000;
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn validate_provider_model(
    db: db::Db,
    provider_id: i64,
    base_url: &str,
    request_json: &str,
) -> Result<ClaudeModelValidationResult, String> {
    workflow::validate_provider_model(db, provider_id, base_url, request_json).await
}

pub async fn get_provider_api_key_plaintext(
    db: db::Db,
    provider_id: i64,
) -> Result<String, String> {
    let provider = provider::load_provider(db, provider_id).await?;
    if provider.cli_key != "claude" {
        return Err("SEC_INVALID_INPUT: only cli_key=claude is supported".to_string());
    }
    Ok(provider.api_key_plaintext)
}
