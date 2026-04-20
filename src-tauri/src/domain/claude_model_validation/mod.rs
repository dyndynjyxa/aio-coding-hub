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
) -> crate::shared::error::AppResult<ClaudeModelValidationResult> {
    workflow::validate_provider_model(db, provider_id, base_url, request_json).await
}
