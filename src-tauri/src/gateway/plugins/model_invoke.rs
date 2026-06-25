//! Usage: Host-mediated model invocation capability for gateway plugins.
#![allow(dead_code)]

use crate::gateway::util::{build_target_url, ensure_cli_required_headers, inject_provider_auth};
use crate::shared::time::now_unix_millis;
use axum::http::{header, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};

const MODEL_INVOKE_PATH: &str = "/v1/responses";
const MAX_MODEL_BYTES: usize = 256;
const MAX_REQUEST_BYTES: usize = 128 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const MAX_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelInvokeInput {
    pub(crate) provider_id: i64,
    pub(crate) model: String,
    pub(crate) body: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_response_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelInvokeOutput {
    pub(crate) ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<u16>,
    pub(crate) duration_ms: u64,
    pub(crate) provider_id: i64,
    pub(crate) provider_name: String,
    pub(crate) model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) response_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) usage: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<ModelInvokeError>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelInvokeError {
    pub(crate) code: String,
    pub(crate) message: String,
}

struct ModelInvokeProvider {
    id: i64,
    name: String,
    cli_key: String,
    base_url: String,
    api_key: String,
}

pub(crate) async fn invoke_model(
    db: &crate::db::Db,
    client: reqwest::Client,
    input: ModelInvokeInput,
) -> ModelInvokeOutput {
    let started = Instant::now();
    let provider = match load_provider(db, input.provider_id) {
        Ok(provider) => provider,
        Err(err) => return error_output(&input, started, err.0, err.1),
    };
    let body_bytes = match build_request_body(&input) {
        Ok(body_bytes) => body_bytes,
        Err(err) => return error_output_for_provider(&input, &provider, started, err.0, err.1),
    };
    let url = match build_target_url(&provider.base_url, MODEL_INVOKE_PATH, None) {
        Ok(url) => url,
        Err(message) => {
            return error_output_for_provider(
                &input,
                &provider,
                started,
                "MODEL_INVOKE_INVALID_PROVIDER_URL",
                message,
            )
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
    ensure_cli_required_headers(&provider.cli_key, &mut headers);
    inject_provider_auth(&provider.cli_key, &provider.api_key, &mut headers);

    let timeout = Duration::from_millis(normalize_timeout_ms(input.timeout_ms));
    let max_response_bytes = normalize_max_response_bytes(input.max_response_bytes);
    let sent_at_unix_ms = now_unix_millis();
    tracing::debug!(
        provider_id = provider.id,
        provider_name = %provider.name,
        trace_id = input.trace_id.as_deref().unwrap_or(""),
        sent_at_unix_ms,
        "plugin model.invoke dispatching host-mediated request"
    );

    let response = client
        .post(url)
        .headers(headers)
        .body(body_bytes)
        .timeout(timeout)
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            let code = if err.is_timeout() {
                "MODEL_INVOKE_TIMEOUT"
            } else {
                "MODEL_INVOKE_REQUEST_FAILED"
            };
            return error_output_for_provider(&input, &provider, started, code, err.to_string());
        }
    };

    let status = response.status();
    let status_u16 = status.as_u16();
    let (response_bytes, truncated) =
        match read_response_body_limited(response, max_response_bytes).await {
            Ok(body) => body,
            Err(err) => {
                return error_output_for_provider(
                    &input,
                    &provider,
                    started,
                    "MODEL_INVOKE_RESPONSE_READ_FAILED",
                    err.to_string(),
                )
            }
        };
    let response_body = String::from_utf8_lossy(&response_bytes).into_owned();
    let usage = response_usage(&response_body);
    let ok = status.is_success();
    ModelInvokeOutput {
        ok,
        status: Some(status_u16),
        duration_ms: elapsed_ms(started),
        provider_id: provider.id,
        provider_name: provider.name,
        model: normalized_model(&input.model),
        response_body: Some(response_body),
        usage,
        error: (!ok).then(|| ModelInvokeError {
            code: "MODEL_INVOKE_UPSTREAM_STATUS".to_string(),
            message: format!("upstream returned HTTP {status_u16}"),
        }),
        truncated,
    }
}

fn load_provider(
    db: &crate::db::Db,
    provider_id: i64,
) -> Result<ModelInvokeProvider, (&'static str, String)> {
    if provider_id <= 0 {
        return Err((
            "MODEL_INVOKE_INVALID_PROVIDER",
            "providerId must be positive".to_string(),
        ));
    }
    let conn = db.open_connection().map_err(|err| {
        (
            "MODEL_INVOKE_PROVIDER_LOOKUP_FAILED",
            format!("failed to open provider database connection: {err}"),
        )
    })?;
    let summary = crate::providers::get_by_id(&conn, provider_id).map_err(|err| {
        (
            "MODEL_INVOKE_PROVIDER_NOT_FOUND",
            format!("provider not found or unavailable: {err}"),
        )
    })?;
    drop(conn);

    if !summary.enabled {
        return Err((
            "MODEL_INVOKE_PROVIDER_DISABLED",
            format!("provider is disabled: {provider_id}"),
        ));
    }
    if summary.auth_mode != "api_key" {
        return Err((
            "MODEL_INVOKE_UNSUPPORTED_AUTH_MODE",
            format!(
                "model.invoke currently supports api_key providers, got {}",
                summary.auth_mode
            ),
        ));
    }
    let base_url = summary
        .base_urls
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .ok_or_else(|| {
            (
                "MODEL_INVOKE_PROVIDER_BASE_URL_MISSING",
                format!("provider has no base URL: {provider_id}"),
            )
        })?
        .to_string();
    let api_key = crate::providers::get_api_key_plaintext(db, provider_id).map_err(|err| {
        (
            "MODEL_INVOKE_PROVIDER_CREDENTIAL_FAILED",
            format!("failed to resolve provider credential: {err}"),
        )
    })?;
    if api_key.trim().is_empty() {
        return Err((
            "MODEL_INVOKE_PROVIDER_CREDENTIAL_MISSING",
            format!("provider credential is empty: {provider_id}"),
        ));
    }

    Ok(ModelInvokeProvider {
        id: provider_id,
        name: summary.name,
        cli_key: summary.cli_key,
        base_url,
        api_key,
    })
}

fn build_request_body(input: &ModelInvokeInput) -> Result<Vec<u8>, (&'static str, String)> {
    let model = normalized_model(&input.model);
    if model.is_empty() {
        return Err((
            "MODEL_INVOKE_INVALID_MODEL",
            "model must be non-empty".to_string(),
        ));
    }
    if model.len() > MAX_MODEL_BYTES {
        return Err((
            "MODEL_INVOKE_INVALID_MODEL",
            format!("model must be at most {MAX_MODEL_BYTES} bytes"),
        ));
    }
    let Some(mut body) = input.body.as_object().cloned() else {
        return Err((
            "MODEL_INVOKE_INVALID_BODY",
            "body must be a JSON object".to_string(),
        ));
    };
    body.insert("model".to_string(), Value::String(model));
    body.insert("stream".to_string(), Value::Bool(false));
    let bytes = serde_json::to_vec(&Value::Object(body)).map_err(|err| {
        (
            "MODEL_INVOKE_BODY_ENCODE_FAILED",
            format!("failed to encode model.invoke request body: {err}"),
        )
    })?;
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err((
            "MODEL_INVOKE_BODY_TOO_LARGE",
            format!("request body exceeded {MAX_REQUEST_BYTES} bytes"),
        ));
    }
    Ok(bytes)
}

async fn read_response_body_limited(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<(Vec<u8>, bool), reqwest::Error> {
    let mut out = Vec::new();
    let mut truncated = false;
    while let Some(chunk) = response.chunk().await? {
        let remaining = max_bytes.saturating_sub(out.len());
        if chunk.len() > remaining {
            out.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        out.extend_from_slice(&chunk);
    }
    Ok((out, truncated))
}

fn response_usage(response_body: &str) -> Option<Value> {
    serde_json::from_str::<Value>(response_body)
        .ok()
        .and_then(|value| value.get("usage").cloned())
}

fn normalize_timeout_ms(timeout_ms: Option<u64>) -> u64 {
    timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(1, MAX_TIMEOUT_MS)
}

fn normalize_max_response_bytes(max_response_bytes: Option<usize>) -> usize {
    max_response_bytes
        .unwrap_or(DEFAULT_MAX_RESPONSE_BYTES)
        .clamp(1, MAX_RESPONSE_BYTES)
}

fn normalized_model(model: &str) -> String {
    model.trim().to_string()
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn error_output(
    input: &ModelInvokeInput,
    started: Instant,
    code: &'static str,
    message: String,
) -> ModelInvokeOutput {
    ModelInvokeOutput {
        ok: false,
        status: None,
        duration_ms: elapsed_ms(started),
        provider_id: input.provider_id,
        provider_name: String::new(),
        model: normalized_model(&input.model),
        response_body: None,
        usage: None,
        error: Some(ModelInvokeError {
            code: code.to_string(),
            message,
        }),
        truncated: false,
    }
}

fn error_output_for_provider(
    input: &ModelInvokeInput,
    provider: &ModelInvokeProvider,
    started: Instant,
    code: &'static str,
    message: String,
) -> ModelInvokeOutput {
    ModelInvokeOutput {
        provider_id: provider.id,
        provider_name: provider.name.clone(),
        ..error_output(input, started, code, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{self, ProviderBaseUrlMode, ProviderUpsertParams};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::oneshot;

    #[tokio::test(flavor = "current_thread")]
    async fn model_invoke_posts_bounded_request_with_host_auth() {
        let (base_url, captured_rx, task) = spawn_json_upstream(
            r#"{"id":"resp-1","usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3}}"#,
        )
        .await;
        let (_dir, db) = init_test_db();
        let provider_id = insert_provider(&db, base_url);

        let output = invoke_model(
            &db,
            reqwest::Client::new(),
            ModelInvokeInput {
                provider_id,
                model: "gpt-test".to_string(),
                body: json!({
                    "input": "hello",
                    "stream": true
                }),
                timeout_ms: Some(1_000),
                max_response_bytes: Some(4 * 1024),
                trace_id: Some("trace-model-invoke".to_string()),
            },
        )
        .await;

        assert!(output.ok, "unexpected output: {output:?}");
        assert_eq!(output.status, Some(200));
        assert_eq!(output.provider_name, "Model Invoke Test");
        assert_eq!(
            output.usage,
            Some(json!({ "input_tokens": 1, "output_tokens": 2, "total_tokens": 3 }))
        );
        let captured = captured_rx.await.expect("captured request");
        assert!(
            captured.starts_with("POST /v1/responses HTTP/1.1"),
            "{captured}"
        );
        assert!(
            captured.contains("authorization: Bearer sk-model-invoke-test"),
            "{captured}"
        );
        assert!(captured.contains(r#""model":"gpt-test""#), "{captured}");
        assert!(captured.contains(r#""stream":false"#), "{captured}");

        task.await.expect("upstream task");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn model_invoke_truncates_large_response_body() {
        let (base_url, _captured_rx, task) = spawn_json_upstream(r#"{"value":"abcdef"}"#).await;
        let (_dir, db) = init_test_db();
        let provider_id = insert_provider(&db, base_url);

        let output = invoke_model(
            &db,
            reqwest::Client::new(),
            ModelInvokeInput {
                provider_id,
                model: "gpt-test".to_string(),
                body: json!({ "input": "hello" }),
                timeout_ms: Some(1_000),
                max_response_bytes: Some(8),
                trace_id: None,
            },
        )
        .await;

        assert!(output.ok, "unexpected output: {output:?}");
        assert!(output.truncated);
        assert_eq!(output.response_body.as_deref(), Some("{\"value\""));

        task.await.expect("upstream task");
    }

    fn init_test_db() -> (tempfile::TempDir, crate::db::Db) {
        let dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&dir.path().join("model-invoke.sqlite"))
            .expect("init test db");
        (dir, db)
    }

    fn insert_provider(db: &crate::db::Db, base_url: String) -> i64 {
        providers::upsert(
            db,
            ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: "Model Invoke Test".to_string(),
                base_urls: vec![base_url],
                base_url_mode: ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some("sk-model-invoke-test".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(0),
                claude_models: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
            },
        )
        .expect("insert provider")
        .id
    }

    async fn spawn_json_upstream(
        body: &'static str,
    ) -> (
        String,
        oneshot::Receiver<String>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind model invoke upstream");
        let addr = listener.local_addr().expect("upstream addr");
        let (captured_tx, captured_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0_u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let captured = String::from_utf8_lossy(&buf[..n]).into_owned();
                let _ = captured_tx.send(captured);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(), body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        (format!("http://{addr}"), captured_rx, task)
    }
}
