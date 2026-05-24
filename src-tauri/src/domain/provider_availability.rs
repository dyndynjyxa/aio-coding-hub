//! Usage: Lightweight provider availability probe.
//!
//! Sends a minimal API request to verify that a provider's base URL + credentials
//! are reachable and functional. Supports all CLI types (claude, codex, gemini).

use crate::shared::error::AppResult;
use crate::{blocking, db};
use reqwest::header::{HeaderMap, HeaderValue};
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const PROBE_RESPONSE_BODY_LIMIT: usize = 64 * 1024;
const PROBE_RESPONSE_PREVIEW_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderAvailabilityResult {
    pub ok: bool,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub status: Option<u16>,
    pub latency_ms: i64,
    pub error: Option<String>,
    pub response_preview: Option<String>,
}

struct LoadedProvider {
    id: i64,
    cli_key: String,
    name: String,
    base_urls: Vec<String>,
    api_key_plaintext: String,
    auth_mode: String,
    source_provider_id: Option<i64>,
    bridge_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeResponseBody {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

fn append_probe_response_chunk(bytes: &mut Vec<u8>, chunk: &[u8], limit: usize) -> bool {
    let remaining = limit.saturating_sub(bytes.len());
    if remaining == 0 {
        return !chunk.is_empty();
    }

    let keep = chunk.len().min(remaining);
    bytes.extend_from_slice(&chunk[..keep]);
    keep < chunk.len()
}

async fn read_probe_response_body_with_limit(
    mut resp: reqwest::Response,
    limit: usize,
) -> Result<ProbeResponseBody, String> {
    let content_length = resp.content_length();
    let mut truncated = content_length.is_some_and(|len| len > limit as u64);
    let capacity = content_length
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or_default()
        .min(limit);
    let mut bytes = Vec::with_capacity(capacity);

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("failed to read probe response: {e}"))?
    {
        if append_probe_response_chunk(&mut bytes, chunk.as_ref(), limit) {
            truncated = true;
            break;
        }
        if bytes.len() >= limit && content_length != Some(limit as u64) {
            truncated = true;
            break;
        }
    }

    Ok(ProbeResponseBody {
        bytes,
        truncated,
        limit,
    })
}

fn probe_response_preview(body: &ProbeResponseBody) -> String {
    let preview_len = body.bytes.len().min(PROBE_RESPONSE_PREVIEW_LIMIT);
    let mut preview = String::from_utf8_lossy(&body.bytes[..preview_len]).to_string();
    if body.truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str(&format!(
            "[probe response truncated after {} bytes]",
            body.limit
        ));
    }
    preview
}

async fn load_provider_for_test(db: db::Db, provider_id: i64) -> AppResult<LoadedProvider> {
    blocking::run("provider_availability_load", move || -> AppResult<LoadedProvider> {
        if provider_id <= 0 {
            return Err(format!("SEC_INVALID_INPUT: invalid provider_id={provider_id}").into());
        }

        let conn = db.open_connection()?;
        #[allow(clippy::type_complexity)]
        let row: Option<(i64, String, String, String, String, String, String, Option<i64>, Option<String>)> = conn
            .query_row(
                r#"
SELECT id, cli_key, name, base_url, base_urls_json, api_key_plaintext, auth_mode, source_provider_id, bridge_type
FROM providers
WHERE id = ?1
"#,
                rusqlite::params![provider_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("DB_ERROR: {e}"))?;

        let Some((id, cli_key, name, base_url_fallback, base_urls_json, api_key_plaintext, auth_mode, source_provider_id, bridge_type)) = row else {
            return Err("DB_NOT_FOUND: provider not found".into());
        };

        let mut base_urls: Vec<String> = serde_json::from_str::<Vec<String>>(&base_urls_json)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();

        if base_urls.is_empty() {
            let fallback = base_url_fallback.trim().to_string();
            if !fallback.is_empty() {
                base_urls.push(fallback);
            }
        }

        Ok(LoadedProvider {
            id,
            cli_key,
            name,
            base_urls,
            api_key_plaintext,
            auth_mode,
            source_provider_id,
            bridge_type,
        })
    })
    .await
}

fn build_probe_request(
    cli_key: &str,
    base_url: &str,
    api_key: &str,
) -> AppResult<(String, HeaderMap, serde_json::Value)> {
    let base = base_url.trim_end_matches('/');

    match cli_key {
        "claude" => {
            let url = format!("{base}/v1/messages");
            let mut headers = HeaderMap::new();
            if let Ok(v) = HeaderValue::from_str(api_key) {
                headers.insert("x-api-key", v);
            }
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "ping"}]
            });
            Ok((url, headers, body))
        }
        "codex" => {
            let url = format!("{base}/v1/chat/completions");
            let mut headers = HeaderMap::new();
            let bearer = format!("Bearer {api_key}");
            if let Ok(v) = HeaderValue::from_str(&bearer) {
                headers.insert("authorization", v);
            }
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "model": "gpt-4o-mini",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "ping"}]
            });
            Ok((url, headers, body))
        }
        "gemini" => {
            let url =
                format!("{base}/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}");
            let mut headers = HeaderMap::new();
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "contents": [{"parts": [{"text": "ping"}]}],
                "generationConfig": {"maxOutputTokens": 1}
            });
            Ok((url, headers, body))
        }
        _ => Err(format!("UNSUPPORTED_CLI_KEY: {cli_key}").into()),
    }
}

fn redact_key_param(msg: &str) -> String {
    regex::Regex::new(r"([?&])key=[^&\s]*")
        .map(|re| re.replace_all(msg, "${1}key=***").to_string())
        .unwrap_or_else(|_| msg.to_string())
}

fn looks_like_auth_failure(status: u16, response_text: &str) -> bool {
    if matches!(status, 401 | 403) {
        return true;
    }

    let lower = response_text.to_ascii_lowercase();
    [
        "api key not valid",
        "invalid api key",
        "invalid_api_key",
        "invalid x-api-key",
        "authentication",
        "unauthorized",
        "permission denied",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_probe_available_status(status: u16, response_text: &str) -> bool {
    status < 500 && !looks_like_auth_failure(status, response_text)
}

pub async fn test_provider_availability(
    db: db::Db,
    provider_id: i64,
) -> AppResult<ProviderAvailabilityResult> {
    let provider = load_provider_for_test(db, provider_id).await?;

    if provider.auth_mode == "oauth" {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url: provider.base_urls.first().cloned().unwrap_or_default(),
            status: None,
            latency_ms: 0,
            error: Some("OAuth 供应商暂不支持直接测试，请使用 OAuth 刷新功能检查状态".into()),
            response_preview: None,
        });
    }

    let is_cx2cc =
        provider.source_provider_id.is_some() || provider.bridge_type.as_deref() == Some("cx2cc");
    if is_cx2cc {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url: provider.base_urls.first().cloned().unwrap_or_default(),
            status: None,
            latency_ms: 0,
            error: Some("CX2CC 桥接供应商需通过其源供应商测试可用性".into()),
            response_preview: None,
        });
    }

    let base_url = provider.base_urls.first().cloned().unwrap_or_default();
    if base_url.is_empty() {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url,
            status: None,
            latency_ms: 0,
            error: Some("供应商未配置 Base URL".into()),
            response_preview: None,
        });
    }

    if provider.api_key_plaintext.trim().is_empty() {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url,
            status: None,
            latency_ms: 0,
            error: Some("供应商未配置 API Key".into()),
            response_preview: None,
        });
    }

    let (url, headers, body) =
        build_probe_request(&provider.cli_key, &base_url, &provider.api_key_plaintext)?;

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "aio-coding-hub-probe/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP_CLIENT_INIT: {e}"))?;

    let started = Instant::now();
    let result = client.post(&url).headers(headers).json(&body).send().await;

    let latency_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = read_probe_response_body_with_limit(resp, PROBE_RESPONSE_BODY_LIMIT)
                .await
                .unwrap_or_else(|_| ProbeResponseBody {
                    bytes: Vec::new(),
                    truncated: false,
                    limit: PROBE_RESPONSE_BODY_LIMIT,
                });
            let preview = probe_response_preview(&body);
            // Provider is "available" if the endpoint responds without an auth
            // failure or upstream 5xx. 400/404 model errors and 429 rate limits
            // still prove the configured base URL and credential reached the
            // provider, but Gemini invalid API keys are reported as 400 and must
            // not be treated as available.
            let ok = is_probe_available_status(status, &preview);

            let error = if ok {
                None
            } else {
                let msg = serde_json::from_slice::<serde_json::Value>(&body.bytes)
                    .ok()
                    .and_then(|v| {
                        v.get("error").and_then(|e| {
                            e.get("message")
                                .and_then(|m| m.as_str().map(String::from))
                                .or_else(|| e.as_str().map(String::from))
                        })
                    })
                    .unwrap_or_else(|| format!("HTTP {status}"));
                Some(msg)
            };

            Ok(ProviderAvailabilityResult {
                ok,
                provider_id: provider.id,
                provider_name: provider.name,
                base_url,
                status: Some(status),
                latency_ms,
                error,
                response_preview: if ok { None } else { Some(preview) },
            })
        }
        Err(err) => {
            let error_message = if err.is_timeout() {
                "请求超时（15秒）".to_string()
            } else if err.is_connect() {
                redact_key_param(&format!("连接失败: {err}"))
            } else {
                redact_key_param(&format!("请求失败: {err}"))
            };

            Ok(ProviderAvailabilityResult {
                ok: false,
                provider_id: provider.id,
                provider_name: provider.name,
                base_url,
                status: None,
                latency_ms,
                error: Some(error_message),
                response_preview: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_value(headers: &HeaderMap, key: &str) -> String {
        headers
            .get(key)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn build_probe_request_for_claude_uses_messages_endpoint_and_x_api_key() {
        let (url, headers, body) =
            build_probe_request("claude", "https://api.example.com/", "sk-claude")
                .expect("claude request");

        assert_eq!(url, "https://api.example.com/v1/messages");
        assert_eq!(header_value(&headers, "x-api-key"), "sk-claude");
        assert_eq!(header_value(&headers, "anthropic-version"), "2023-06-01");
        assert_eq!(body["messages"][0]["content"], "ping");
    }

    #[test]
    fn build_probe_request_for_codex_uses_chat_completions_and_bearer_auth() {
        let (url, headers, body) =
            build_probe_request("codex", "https://api.example.com", "sk-openai")
                .expect("codex request");

        assert_eq!(url, "https://api.example.com/v1/chat/completions");
        assert_eq!(header_value(&headers, "authorization"), "Bearer sk-openai");
        assert_eq!(body["messages"][0]["content"], "ping");
    }

    #[test]
    fn build_probe_request_for_gemini_uses_generate_content_key_param() {
        let (url, headers, body) = build_probe_request(
            "gemini",
            "https://generativelanguage.googleapis.com/",
            "sk-google",
        )
        .expect("gemini request");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=sk-google"
        );
        assert_eq!(header_value(&headers, "content-type"), "application/json");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "ping");
    }

    #[test]
    fn build_probe_request_rejects_unsupported_cli_key() {
        let err = build_probe_request("unknown", "https://api.example.com", "secret")
            .unwrap_err()
            .to_string();

        assert_eq!(err, "UNSUPPORTED_CLI_KEY: unknown");
    }

    #[test]
    fn redact_key_param_preserves_delimiters_and_hides_gemini_key() {
        let redacted =
            redact_key_param("连接失败: https://host/v1beta/models?alt=sse&key=sk-secret&other=1");

        assert_eq!(
            redacted,
            "连接失败: https://host/v1beta/models?alt=sse&key=***&other=1"
        );
        assert!(!redacted.contains("sk-secret"));
    }

    #[test]
    fn append_probe_response_chunk_keeps_bounded_prefix() {
        let mut bytes = b"abcd".to_vec();
        let truncated = append_probe_response_chunk(&mut bytes, b"efgh", 6);

        assert_eq!(bytes, b"abcdef");
        assert!(truncated);
    }

    #[test]
    fn probe_response_preview_marks_truncated_payloads() {
        let preview = probe_response_preview(&ProbeResponseBody {
            bytes: b"upstream error".to_vec(),
            truncated: true,
            limit: 12,
        });

        assert_eq!(
            preview,
            "upstream error\n[probe response truncated after 12 bytes]"
        );
    }

    #[test]
    fn probe_status_rejects_5xx_and_auth_errors_but_allows_model_or_rate_limit_errors() {
        assert!(is_probe_available_status(
            400,
            r#"{"error":{"message":"model not found"}}"#
        ));
        assert!(is_probe_available_status(404, "model not found"));
        assert!(is_probe_available_status(429, "rate limit exceeded"));

        assert!(!is_probe_available_status(500, "upstream error"));
        assert!(!is_probe_available_status(401, "unauthorized"));
        assert!(!is_probe_available_status(
            400,
            r#"{"error":{"message":"API key not valid. Please pass a valid API key."}}"#
        ));
    }
}
