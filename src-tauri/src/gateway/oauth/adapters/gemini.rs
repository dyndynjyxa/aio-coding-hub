//! Usage: Gemini (Google) OAuth adapter.

use crate::gateway::oauth::provider_trait::*;
use crate::shared::http_body::read_text_with_limit;
use axum::http::{HeaderMap, HeaderValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub(crate) struct GeminiOAuthProvider {
    endpoints: OAuthEndpoints,
}

const GEMINI_DEFAULT_CLIENT_ID: &str = concat!(
    "681255809395",
    "-oo8ft2oprdrnp9e3aqf6av3hmdib135j",
    ".apps.googleusercontent.com"
);
const GEMINI_DEFAULT_CLIENT_SECRET: &str = concat!("GOCSPX-", "4uHgMPm-1o7Sk-geV6Cu5clXFsxl");
pub(crate) const GEMINI_CODE_ASSIST_BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
pub(crate) const GEMINI_CODE_ASSIST_API_VERSION: &str = "v1internal";
pub(crate) const GEMINI_CLI_USER_AGENT: &str = "GeminiCLI/1.0";
const GEMINI_SHORT_LIMIT_LABEL: &str = "短窗";
const GEMINI_FREE_TIER_ID: &str = "free-tier";
const GEMINI_FALLBACK_TIER_ID: &str = "legacy-tier";
const GEMINI_PROJECT_CACHE_TTL: Duration = Duration::from_secs(300);
const GEMINI_PROJECT_CACHE_MAX_ENTRIES: usize = 256;
const GEMINI_JSON_RESPONSE_BODY_LIMIT: usize = 2 * 1024 * 1024;
const GEMINI_ERROR_BODY_PREVIEW_CHARS: usize = 500;

#[derive(Debug, Clone)]
struct GeminiProjectCacheEntry {
    project_id: String,
    cached_at: Instant,
}

static GEMINI_PROJECT_CACHE: OnceLock<Mutex<BTreeMap<String, GeminiProjectCacheEntry>>> =
    OnceLock::new();

impl GeminiOAuthProvider {
    pub(crate) fn new() -> Self {
        let client_id = std::env::var("AIO_GEMINI_OAUTH_CLIENT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| GEMINI_DEFAULT_CLIENT_ID.to_string());
        let client_secret = Some(
            std::env::var("AIO_GEMINI_OAUTH_CLIENT_SECRET")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| GEMINI_DEFAULT_CLIENT_SECRET.to_string()),
        );

        Self {
            endpoints: OAuthEndpoints {
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
                token_url: "https://oauth2.googleapis.com/token",
                client_id,
                client_secret,
                scopes: vec![
                    "https://www.googleapis.com/auth/cloud-platform",
                    "https://www.googleapis.com/auth/userinfo.email",
                    "https://www.googleapis.com/auth/userinfo.profile",
                ],
                redirect_host: "127.0.0.1",
                callback_path: "/oauth2callback",
                default_callback_port: 8085,
            },
        }
    }
}

impl OAuthProvider for GeminiOAuthProvider {
    fn cli_key(&self) -> &'static str {
        "gemini"
    }

    fn provider_type(&self) -> &'static str {
        "gemini_oauth"
    }

    fn endpoints(&self) -> &OAuthEndpoints {
        &self.endpoints
    }

    fn default_base_url(&self) -> &'static str {
        GEMINI_CODE_ASSIST_BASE_URL
    }

    fn extra_authorize_params(&self) -> Vec<(&'static str, &'static str)> {
        vec![("access_type", "offline"), ("prompt", "consent")]
    }

    fn resolve_effective_token(
        &self,
        token_set: &OAuthTokenSet,
        _stored_id_token: Option<&str>,
    ) -> (String, Option<String>) {
        let token = &token_set.access_token;
        if !token.starts_with("ya29.") {
            tracing::warn!(
                "gemini oauth: access_token does not start with 'ya29.' prefix, may be invalid"
            );
        }
        (token.clone(), token_set.id_token.clone())
    }

    fn inject_upstream_headers(
        &self,
        headers: &mut HeaderMap,
        access_token: &str,
    ) -> Result<(), String> {
        insert_bearer_auth(headers, access_token, "gemini oauth")?;
        if !headers.contains_key("x-goog-api-client") {
            headers.insert(
                "x-goog-api-client",
                HeaderValue::from_static(GEMINI_CLI_USER_AGENT),
            );
        }
        Ok(())
    }

    fn fetch_limits(
        &self,
        client: &reqwest::Client,
        access_token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthLimitsResult, String>> + Send + '_>> {
        let token = access_token.to_string();
        let client = client.clone();
        Box::pin(async move {
            let project_id = resolve_project_id_for_access_token(&client, &token).await?;
            let quota_response = gemini_post_json(
                &client,
                &token,
                "retrieveUserQuota",
                serde_json::json!({
                    "project": project_id,
                }),
            )
            .await?;
            let (limit_5h_text, limit_weekly_text) = gemini_parse_quota_texts(&quota_response);

            Ok(OAuthLimitsResult {
                limit_short_label: Some(GEMINI_SHORT_LIMIT_LABEL.to_string()),
                limit_5h_text,
                limit_weekly_text,
                raw_json: Some(quota_response),
            })
        })
    }
}

pub(crate) async fn resolve_project_id_for_access_token(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<String, String> {
    let project_override = gemini_project_override();
    let cache_key = gemini_project_cache_key(access_token, project_override.as_deref());

    if let Some(project_id) = gemini_project_cache_get(&cache_key) {
        return Ok(project_id);
    }

    let load_response =
        gemini_load_code_assist(client, access_token, project_override.as_deref()).await?;
    let project_id = gemini_resolve_project_id(
        client,
        access_token,
        &load_response,
        project_override.as_deref(),
    )
    .await?;
    gemini_project_cache_put(cache_key, project_id.clone());
    Ok(project_id)
}

async fn gemini_load_code_assist(
    client: &reqwest::Client,
    access_token: &str,
    project_override: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut payload = serde_json::Map::new();
    payload.insert("metadata".into(), gemini_client_metadata(project_override));
    if let Some(project_id) = project_override.filter(|value| !value.trim().is_empty()) {
        payload.insert(
            "cloudaicompanionProject".into(),
            serde_json::Value::String(project_id.to_string()),
        );
    }
    gemini_post_json(
        client,
        access_token,
        "loadCodeAssist",
        serde_json::Value::Object(payload),
    )
    .await
}

async fn gemini_resolve_project_id(
    client: &reqwest::Client,
    access_token: &str,
    load_response: &serde_json::Value,
    project_override: Option<&str>,
) -> Result<String, String> {
    if let Some(project_id) = gemini_load_response_project_id(load_response) {
        return Ok(project_id);
    }

    if load_response.get("currentTier").is_some() {
        if let Some(project_id) = project_override.filter(|value| !value.trim().is_empty()) {
            return Ok(project_id.to_string());
        }
        return Err(
            "gemini limits fetch requires GOOGLE_CLOUD_PROJECT / GOOGLE_CLOUD_PROJECT_ID for this account tier"
                .to_string(),
        );
    }

    let tier_id = gemini_default_onboard_tier_id(load_response);
    let mut onboard_payload = serde_json::Map::new();
    onboard_payload.insert("tierId".into(), serde_json::Value::String(tier_id.clone()));

    if tier_id == GEMINI_FREE_TIER_ID {
        onboard_payload.insert("metadata".into(), gemini_client_metadata(None));
    } else {
        let Some(project_id) = project_override.filter(|value| !value.trim().is_empty()) else {
            return Err(
                "gemini limits fetch requires GOOGLE_CLOUD_PROJECT / GOOGLE_CLOUD_PROJECT_ID for this account tier"
                    .to_string(),
            );
        };
        onboard_payload.insert(
            "cloudaicompanionProject".into(),
            serde_json::Value::String(project_id.to_string()),
        );
        onboard_payload.insert("metadata".into(), gemini_client_metadata(Some(project_id)));
    }

    let mut operation = gemini_post_json(
        client,
        access_token,
        "onboardUser",
        serde_json::Value::Object(onboard_payload),
    )
    .await?;

    if let Some(project_id) = gemini_operation_project_id(&operation) {
        return Ok(project_id);
    }

    let Some(operation_name) = json_string_field(operation.get("name")).map(str::to_string) else {
        return Err("gemini limits fetch could not resolve onboarding operation".to_string());
    };

    for _ in 0..12 {
        if operation.get("done").and_then(serde_json::Value::as_bool) == Some(true) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        operation = gemini_get_json(client, access_token, &operation_name).await?;
        if let Some(project_id) = gemini_operation_project_id(&operation) {
            return Ok(project_id);
        }
    }

    gemini_operation_project_id(&operation)
        .ok_or_else(|| "gemini limits fetch could not resolve a quota project".to_string())
}

fn gemini_load_response_project_id(load_response: &serde_json::Value) -> Option<String> {
    json_string_field(load_response.get("cloudaicompanionProject")).map(str::to_string)
}

fn gemini_operation_project_id(operation: &serde_json::Value) -> Option<String> {
    operation
        .get("response")
        .and_then(|value| value.get("cloudaicompanionProject"))
        .and_then(|value| value.get("id"))
        .and_then(json_string_value)
        .map(str::to_string)
}

fn gemini_default_onboard_tier_id(load_response: &serde_json::Value) -> String {
    load_response
        .get("allowedTiers")
        .and_then(serde_json::Value::as_array)
        .and_then(|tiers| {
            tiers.iter().find_map(|tier| {
                if tier.get("isDefault").and_then(serde_json::Value::as_bool) != Some(true) {
                    return None;
                }
                tier.get("id")
                    .and_then(json_string_value)
                    .map(str::to_string)
            })
        })
        .unwrap_or_else(|| GEMINI_FALLBACK_TIER_ID.to_string())
}

fn gemini_project_override() -> Option<String> {
    [
        "AIO_GEMINI_CLOUD_PROJECT",
        "GOOGLE_CLOUD_PROJECT",
        "GOOGLE_CLOUD_PROJECT_ID",
    ]
    .into_iter()
    .find_map(|key| std::env::var(key).ok())
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn gemini_client_metadata(project_id: Option<&str>) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "ideType".into(),
        serde_json::Value::String("IDE_UNSPECIFIED".to_string()),
    );
    metadata.insert(
        "platform".into(),
        serde_json::Value::String("PLATFORM_UNSPECIFIED".to_string()),
    );
    metadata.insert(
        "pluginType".into(),
        serde_json::Value::String("GEMINI".to_string()),
    );
    if let Some(project_id) = project_id.filter(|value| !value.trim().is_empty()) {
        metadata.insert(
            "duetProject".into(),
            serde_json::Value::String(project_id.to_string()),
        );
    }
    serde_json::Value::Object(metadata)
}

async fn gemini_post_json(
    client: &reqwest::Client,
    access_token: &str,
    method: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = format!("{GEMINI_CODE_ASSIST_BASE_URL}/{GEMINI_CODE_ASSIST_API_VERSION}:{method}");
    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("x-goog-api-client", GEMINI_CLI_USER_AGENT)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("gemini {method} request failed: {e}"))?;

    gemini_decode_response(response, &format!("gemini {method}")).await
}

async fn gemini_get_json(
    client: &reqwest::Client,
    access_token: &str,
    operation_name: &str,
) -> Result<serde_json::Value, String> {
    let url = format!(
        "{GEMINI_CODE_ASSIST_BASE_URL}/{}",
        operation_name.trim_start_matches('/')
    );
    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("x-goog-api-client", GEMINI_CLI_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("gemini operation request failed: {e}"))?;

    gemini_decode_response(response, "gemini operation").await
}

fn gemini_project_cache() -> &'static Mutex<BTreeMap<String, GeminiProjectCacheEntry>> {
    GEMINI_PROJECT_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn gemini_project_cache_key(access_token: &str, project_override: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(access_token.as_bytes());
    if let Some(project_id) = project_override {
        hasher.update(b"|");
        hasher.update(project_id.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn gemini_project_cache_get(cache_key: &str) -> Option<String> {
    let mut cache = gemini_project_cache().lock().ok()?;
    let now = Instant::now();
    gemini_project_cache_prune_locked(&mut cache, now);
    let entry = cache.get(cache_key)?;
    Some(entry.project_id.clone())
}

fn gemini_project_cache_put(cache_key: String, project_id: String) {
    if let Ok(mut cache) = gemini_project_cache().lock() {
        let now = Instant::now();
        gemini_project_cache_prune_locked(&mut cache, now);
        cache.insert(
            cache_key,
            GeminiProjectCacheEntry {
                project_id,
                cached_at: now,
            },
        );
        gemini_project_cache_prune_locked(&mut cache, now);
    }
}

fn gemini_project_cache_prune_locked(
    cache: &mut BTreeMap<String, GeminiProjectCacheEntry>,
    now: Instant,
) {
    cache.retain(|_, entry| {
        now.saturating_duration_since(entry.cached_at) <= GEMINI_PROJECT_CACHE_TTL
    });

    if cache.len() <= GEMINI_PROJECT_CACHE_MAX_ENTRIES {
        return;
    }

    let mut entries: Vec<(String, Instant)> = cache
        .iter()
        .map(|(key, entry)| (key.clone(), entry.cached_at))
        .collect();
    entries.sort_by_key(|(_, cached_at)| *cached_at);
    let evict_count = cache.len() - GEMINI_PROJECT_CACHE_MAX_ENTRIES;
    for (key, _) in entries.into_iter().take(evict_count) {
        cache.remove(&key);
    }
}

fn gemini_body_preview(body: &str) -> String {
    let body = body.trim();
    let mut preview: String = body.chars().take(GEMINI_ERROR_BODY_PREVIEW_CHARS).collect();
    if preview.len() < body.len() {
        preview.push_str(" [truncated]");
    }
    preview
}

async fn gemini_decode_response(
    response: reqwest::Response,
    context: &str,
) -> Result<serde_json::Value, String> {
    let status = response.status();
    let text = read_text_with_limit(response, GEMINI_JSON_RESPONSE_BODY_LIMIT, context)
        .await
        .map_err(|e| {
            if status.is_success() {
                e
            } else {
                format!("{context} status: {status}; {e}")
            }
        })?;

    if !status.is_success() {
        let body = gemini_body_preview(&text);
        if body.is_empty() {
            return Err(format!("{context} status: {status}"));
        }
        return Err(format!("{context} status: {status}; body: {body}"));
    }

    serde_json::from_str(&text).map_err(|e| {
        format!(
            "{context} parse failed: {e}; body: {}",
            gemini_body_preview(&text)
        )
    })
}

fn gemini_parse_quota_texts(body: &serde_json::Value) -> (Option<String>, Option<String>) {
    let Some(buckets) = body.get("buckets").and_then(serde_json::Value::as_array) else {
        return (None, None);
    };

    let mut groups: BTreeMap<String, Vec<&serde_json::Value>> = BTreeMap::new();
    let mut fallback_group = Vec::new();

    for bucket in buckets.iter().filter(|bucket| bucket.is_object()) {
        if let Some(reset_time) = bucket.get("resetTime").and_then(json_string_value) {
            groups
                .entry(reset_time.to_string())
                .or_default()
                .push(bucket);
        } else {
            fallback_group.push(bucket);
        }
    }

    let limit_5h_text = if let Some((_, group)) = groups.iter().next() {
        gemini_bucket_group_text(group)
    } else {
        gemini_bucket_group_text(&fallback_group)
    };

    let limit_weekly_text = match (groups.iter().next(), groups.iter().next_back()) {
        (Some((first_key, _)), Some((last_key, group))) if first_key != last_key => {
            gemini_bucket_group_text(group)
        }
        _ => None,
    };

    (limit_5h_text, limit_weekly_text)
}

fn gemini_bucket_group_text(group: &[&serde_json::Value]) -> Option<String> {
    if group.is_empty() {
        return None;
    }

    if let Some(remaining) = group
        .iter()
        .filter_map(|bucket| gemini_bucket_amount_string(bucket))
        .filter_map(|amount| amount.parse::<f64>().ok())
        .reduce(f64::min)
    {
        return Some(gemini_format_amount(remaining));
    }

    if let Some(remaining) = group
        .iter()
        .filter_map(|bucket| gemini_bucket_amount_string(bucket))
        .next()
    {
        return Some(remaining.to_string());
    }

    group
        .iter()
        .filter_map(|bucket| gemini_bucket_remaining_fraction(bucket))
        .reduce(f64::min)
        .map(|fraction| gemini_format_percent(fraction * 100.0))
}

fn gemini_bucket_amount_string(bucket: &serde_json::Value) -> Option<&str> {
    bucket.get("remainingAmount").and_then(json_string_value)
}

fn gemini_bucket_remaining_fraction(bucket: &serde_json::Value) -> Option<f64> {
    bucket
        .get("remainingFraction")
        .and_then(serde_json::Value::as_f64)
        .map(|value| if value > 1.0 { value / 100.0 } else { value })
        .map(|value| value.clamp(0.0, 1.0))
}

fn gemini_format_amount(value: f64) -> String {
    let rounded = if value.abs() < 0.005 { 0.0 } else { value };
    let text = format!("{rounded:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn gemini_format_percent(value: f64) -> String {
    format!("{:.0}%", value.clamp(0.0, 100.0))
}

fn json_string_value(value: &serde_json::Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn json_string_field(value: Option<&serde_json::Value>) -> Option<&str> {
    value.and_then(json_string_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_parse_quota_texts_picks_shortest_and_longest_reset_windows() {
        let body = serde_json::json!({
            "buckets": [
                {
                    "remainingAmount": "11",
                    "resetTime": "2026-03-09T11:00:00Z"
                },
                {
                    "remainingAmount": "7",
                    "resetTime": "2026-03-16T00:00:00Z"
                }
            ]
        });

        let (limit_5h_text, limit_weekly_text) = gemini_parse_quota_texts(&body);
        assert_eq!(limit_5h_text.as_deref(), Some("11"));
        assert_eq!(limit_weekly_text.as_deref(), Some("7"));
    }

    #[test]
    fn gemini_parse_quota_texts_uses_minimum_remaining_fraction_per_window() {
        let body = serde_json::json!({
            "buckets": [
                {
                    "remainingFraction": 0.70,
                    "resetTime": "2026-03-09T11:00:00Z"
                },
                {
                    "remainingFraction": 0.40,
                    "resetTime": "2026-03-09T11:00:00Z"
                },
                {
                    "remainingFraction": 0.20,
                    "resetTime": "2026-03-16T00:00:00Z"
                }
            ]
        });

        let (limit_5h_text, limit_weekly_text) = gemini_parse_quota_texts(&body);
        assert_eq!(limit_5h_text.as_deref(), Some("40%"));
        assert_eq!(limit_weekly_text.as_deref(), Some("20%"));
    }

    #[test]
    fn gemini_project_cache_prune_removes_expired_entries() {
        let cached_at = Instant::now();
        let now = cached_at + GEMINI_PROJECT_CACHE_TTL + Duration::from_secs(1);
        let mut cache = BTreeMap::new();
        cache.insert(
            "expired".to_string(),
            GeminiProjectCacheEntry {
                project_id: "old-project".to_string(),
                cached_at,
            },
        );
        cache.insert(
            "fresh".to_string(),
            GeminiProjectCacheEntry {
                project_id: "fresh-project".to_string(),
                cached_at: now,
            },
        );

        gemini_project_cache_prune_locked(&mut cache, now);

        assert!(!cache.contains_key("expired"));
        assert_eq!(
            cache.get("fresh").map(|e| e.project_id.as_str()),
            Some("fresh-project")
        );
    }

    #[test]
    fn gemini_project_cache_prune_evicts_oldest_entries_over_capacity() {
        let base = Instant::now();
        let now = base + Duration::from_secs((GEMINI_PROJECT_CACHE_MAX_ENTRIES + 2) as u64);
        let mut cache = BTreeMap::new();
        for index in 0..(GEMINI_PROJECT_CACHE_MAX_ENTRIES + 2) {
            cache.insert(
                format!("key-{index:03}"),
                GeminiProjectCacheEntry {
                    project_id: format!("project-{index}"),
                    cached_at: base + Duration::from_secs(index as u64),
                },
            );
        }

        gemini_project_cache_prune_locked(&mut cache, now);

        assert_eq!(cache.len(), GEMINI_PROJECT_CACHE_MAX_ENTRIES);
        assert!(!cache.contains_key("key-000"));
        assert!(!cache.contains_key("key-001"));
        assert!(cache.contains_key("key-002"));
    }
}
