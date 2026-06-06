//! Passive input-output association audit sidecar.

use crate::domain::providers::ProviderSummary;
use crate::{blocking, db, providers, request_logs, settings};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

const AUDIT_RESPONSE_COLLECTOR_MAX_BYTES: usize = 128 * 1024;
const AUDIT_PROVIDER_RESPONSE_MAX_BYTES: usize = 64 * 1024;
const AUDIT_MAX_OUTPUT_TOKENS: u32 = 700;
const AUDIT_MAX_EVENTS: usize = 80;
const AUDIT_MAX_SIGNALS: usize = 12;
const AUDIT_MAX_REASON_CHARS: usize = 900;
const AUDIT_MAX_EVIDENCE_CHARS: usize = 700;
const AUDIT_PENDING_STATUS: &str = "pending";
const INTERNAL_FORWARD_HEADER: &str = "x-aio-gateway-forwarded";
const INTERNAL_FORWARD_VALUE: &str = "aio-coding-hub";

const AUDIT_SYSTEM_PROMPT: &str = r#"You are a passive association audit sidecar for a local AI gateway.
Judge whether the returned content is grounded in the user's request.
Focus on output-side risk signals: low association, ungrounded executable/script/callback/credential/network/persistence content, role or policy drift, and unrelated promotional/vendor content.
Do not decide blocking, routing, retrying, rewriting, or user intent. Emit advisory signals only.
Normal diagnostic tool calls are not risky by themselves when they are reasonably related to the request.
Return one JSON object only:
{
  "association_score": 0.0,
  "overall_risk": "none|low|medium|high|critical|unknown",
  "signals": [
    {
      "code": "short_reason_code",
      "severity": "low|medium|high|critical",
      "confidence": 0.0,
      "event_index": 0,
      "event_kind": "assistant_text|tool_call|tool_result|other",
      "reason": "short explanation",
      "evidence": "short redacted quote"
    }
  ],
  "insufficient_context": false,
  "notes": "short optional diagnostic"
}"#;

#[derive(Debug, Clone)]
pub(in crate::gateway) struct AssociationAuditInput {
    pub(in crate::gateway) trace_id: String,
    pub(in crate::gateway) cli_key: String,
    pub(in crate::gateway) method: String,
    pub(in crate::gateway) path: String,
    pub(in crate::gateway) requested_model: Option<String>,
    pub(in crate::gateway) request_body: Vec<u8>,
    pub(in crate::gateway) response_body: Vec<u8>,
    pub(in crate::gateway) response_truncated: bool,
    pub(in crate::gateway) status: Option<u16>,
}

#[derive(Debug, Clone)]
pub(in crate::gateway) struct AuditResponseCollector {
    bytes: Vec<u8>,
    truncated: bool,
}

impl AuditResponseCollector {
    pub(in crate::gateway) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            truncated: false,
        }
    }

    pub(in crate::gateway) fn ingest(&mut self, chunk: &[u8]) {
        if self.truncated || chunk.is_empty() {
            return;
        }
        let remaining = AUDIT_RESPONSE_COLLECTOR_MAX_BYTES.saturating_sub(self.bytes.len());
        if remaining == 0 {
            self.truncated = true;
            return;
        }
        let keep = chunk.len().min(remaining);
        self.bytes.extend_from_slice(&chunk[..keep]);
        if keep < chunk.len() {
            self.truncated = true;
        }
    }

    pub(in crate::gateway) fn into_parts(self) -> (Vec<u8>, bool) {
        (self.bytes, self.truncated)
    }
}

#[derive(Debug, Clone)]
struct LoadedAuditConfig {
    provider: Option<ProviderSummary>,
    api_key: Option<String>,
    settings: settings::AppSettings,
}

#[derive(Debug, Clone)]
struct AuditPackageBuild {
    package: Value,
    package_truncated: bool,
    input_chars: usize,
    output_chars: usize,
}

#[derive(Debug, Clone)]
struct AuditProviderText {
    text: String,
}

static AUDIT_LIMITER: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn audit_limiter() -> Arc<Semaphore> {
    AUDIT_LIMITER
        .get_or_init(|| Arc::new(Semaphore::new(2)))
        .clone()
}

pub(in crate::gateway) fn maybe_spawn_association_audit<R>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    input: AssociationAuditInput,
) where
    R: tauri::Runtime + 'static,
{
    if input.trace_id.trim().is_empty() || input.response_body.is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        run_association_audit(app, db, input).await;
    });
}

async fn run_association_audit<R>(
    app: tauri::AppHandle<R>,
    db: db::Db,
    input: AssociationAuditInput,
) where
    R: tauri::Runtime + 'static,
{
    let started_at_ms = now_ms();
    let settings = match blocking::run("association_audit_read_settings", {
        let app = app.clone();
        move || settings::read(&app)
    })
    .await
    {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!(
                trace_id = %input.trace_id,
                error = %err,
                "association audit settings read failed"
            );
            return;
        }
    };

    if !settings.enable_association_audit
        || matches!(
            settings.association_audit_mode,
            settings::AssociationAuditMode::Off
        )
    {
        return;
    }

    let build = build_audit_package(&input, &settings);
    if matches!(
        settings.association_audit_mode,
        settings::AssociationAuditMode::Sampled
    ) && !sample_selected(&input.trace_id, settings.association_audit_sample_rate)
    {
        persist_status(
            db,
            input.trace_id,
            status_json(
                "skipped",
                started_at_ms,
                Some("sampled_out"),
                None,
                None,
                &build,
            ),
        )
        .await;
        return;
    }

    if matches!(
        settings.association_audit_mode,
        settings::AssociationAuditMode::Prefiltered
    ) && !prefilter_should_call_llm(&build)
    {
        persist_status(
            db,
            input.trace_id,
            status_json(
                "skipped",
                started_at_ms,
                Some("prefilter_no_signal"),
                None,
                None,
                &build,
            ),
        )
        .await;
        return;
    }

    let limiter = audit_limiter();
    let Ok(_permit) = limiter.try_acquire_owned() else {
        persist_status(
            db,
            input.trace_id,
            status_json(
                "skipped",
                started_at_ms,
                Some("audit_queue_saturated"),
                None,
                None,
                &build,
            ),
        )
        .await;
        return;
    };

    persist_status(
        db.clone(),
        input.trace_id.clone(),
        status_json(
            AUDIT_PENDING_STATUS,
            started_at_ms,
            None,
            None,
            None,
            &build,
        ),
    )
    .await;

    let loaded = match load_audit_config(db.clone(), settings).await {
        Ok(loaded) => loaded,
        Err(err) => {
            persist_status(
                db,
                input.trace_id,
                status_json("failed", started_at_ms, None, Some(err), None, &build),
            )
            .await;
            return;
        }
    };

    let Some(provider) = loaded.provider.as_ref() else {
        persist_status(
            db,
            input.trace_id,
            status_json(
                "not_configured",
                started_at_ms,
                Some("provider_not_selected"),
                None,
                None,
                &build,
            ),
        )
        .await;
        return;
    };

    if let Some(reason) = audit_provider_unavailable_reason(provider, loaded.api_key.as_deref()) {
        persist_status(
            db,
            input.trace_id,
            status_json(
                if reason == "unsupported_provider" || reason == "unsupported_auth_mode" {
                    "unsupported"
                } else {
                    "not_configured"
                },
                started_at_ms,
                Some(reason),
                None,
                Some(provider),
                &build,
            ),
        )
        .await;
        return;
    }

    let model = loaded.settings.association_audit_model.trim().to_string();
    if model.is_empty() {
        persist_status(
            db,
            input.trace_id,
            status_json(
                "not_configured",
                started_at_ms,
                Some("model_not_configured"),
                None,
                Some(provider),
                &build,
            ),
        )
        .await;
        return;
    }

    let timeout = Duration::from_secs(loaded.settings.association_audit_timeout_seconds as u64);
    let provider_text = match call_audit_provider(
        provider,
        loaded.api_key.as_deref().unwrap_or_default(),
        &model,
        &build.package,
        timeout,
    )
    .await
    {
        Ok(value) => value,
        Err(AuditCallError::Timeout(reason)) => {
            persist_status(
                db,
                input.trace_id,
                status_json(
                    "timeout",
                    started_at_ms,
                    Some(reason),
                    None,
                    Some(provider),
                    &build,
                ),
            )
            .await;
            return;
        }
        Err(AuditCallError::Failed(reason)) => {
            persist_status(
                db,
                input.trace_id,
                status_json(
                    "failed",
                    started_at_ms,
                    Some(reason.as_str()),
                    None,
                    Some(provider),
                    &build,
                ),
            )
            .await;
            return;
        }
    };

    let completed = match normalize_audit_model_output(&provider_text.text) {
        Ok(mut value) => {
            merge_status_meta(
                &mut value,
                "completed",
                started_at_ms,
                None,
                Some(provider),
                &model,
                &build,
            );
            value
        }
        Err(reason) => status_json(
            "invalid_response",
            started_at_ms,
            Some(reason.as_str()),
            None,
            Some(provider),
            &build,
        ),
    };

    persist_status(db, input.trace_id, completed).await;
}

async fn load_audit_config(
    db: db::Db,
    settings: settings::AppSettings,
) -> Result<LoadedAuditConfig, String> {
    let Some(provider_id) = settings.association_audit_provider_id else {
        return Ok(LoadedAuditConfig {
            provider: None,
            api_key: None,
            settings,
        });
    };

    blocking::run("association_audit_load_provider", move || {
        let conn = db.open_connection()?;
        let provider = providers::get_by_id(&conn, provider_id)?;
        let api_key = providers::get_api_key_plaintext(&db, provider_id).ok();
        Ok::<_, crate::shared::error::AppError>(LoadedAuditConfig {
            provider: Some(provider),
            api_key,
            settings,
        })
    })
    .await
    .map_err(|err| err.to_string())
}

fn audit_provider_unavailable_reason(
    provider: &ProviderSummary,
    api_key: Option<&str>,
) -> Option<&'static str> {
    if !provider.enabled {
        return Some("provider_disabled");
    }
    if provider.source_provider_id.is_some() || provider.bridge_type.is_some() {
        return Some("unsupported_provider");
    }
    if provider.auth_mode.trim() != providers::ProviderAuthMode::ApiKey.as_str() {
        return Some("unsupported_auth_mode");
    }
    if !matches!(provider.cli_key.as_str(), "claude" | "codex") {
        return Some("unsupported_provider");
    }
    if api_key.map(str::trim).unwrap_or_default().is_empty() {
        return Some("api_key_not_configured");
    }
    if first_base_url(provider).is_none() {
        return Some("base_url_not_configured");
    }
    None
}

#[derive(Debug)]
enum AuditCallError {
    Timeout(&'static str),
    Failed(String),
}

async fn call_audit_provider(
    provider: &ProviderSummary,
    api_key: &str,
    model: &str,
    package: &Value,
    timeout: Duration,
) -> Result<AuditProviderText, AuditCallError> {
    let Some(base_url) = first_base_url(provider) else {
        return Err(AuditCallError::Failed(
            "base_url_not_configured".to_string(),
        ));
    };
    let package_text = serde_json::to_string(package)
        .map_err(|err| AuditCallError::Failed(format!("package_serialize_failed: {err}")))?;

    let request = match provider.cli_key.as_str() {
        "claude" => build_claude_audit_request(&base_url, api_key, model, &package_text)?,
        "codex" => build_codex_audit_request(&base_url, api_key, model, &package_text)?,
        _ => return Err(AuditCallError::Failed("unsupported_provider".to_string())),
    };

    let response = match tokio::time::timeout(timeout, request.timeout(timeout).send()).await {
        Ok(Ok(response)) => response,
        Ok(Err(err)) if err.is_timeout() => return Err(AuditCallError::Timeout("request_timeout")),
        Ok(Err(err)) => return Err(AuditCallError::Failed(format!("request_failed: {err}"))),
        Err(_) => return Err(AuditCallError::Timeout("request_timeout")),
    };

    let status = response.status();
    let text = crate::shared::http_body::read_text_with_limit(
        response,
        AUDIT_PROVIDER_RESPONSE_MAX_BYTES,
        "association audit provider response",
    )
    .await
    .map_err(AuditCallError::Failed)?;

    if !status.is_success() {
        let snippet = truncate_chars(&redact_text(&text), 500);
        return Err(AuditCallError::Failed(format!(
            "provider_http_status: {} body={}",
            status.as_u16(),
            snippet
        )));
    }

    let value: Value = serde_json::from_str(&text)
        .map_err(|err| AuditCallError::Failed(format!("provider_json_parse_failed: {err}")))?;
    let Some(text) = extract_provider_response_text(provider.cli_key.as_str(), &value) else {
        return Err(AuditCallError::Failed(
            "provider_response_text_missing".to_string(),
        ));
    };

    Ok(AuditProviderText { text })
}

fn build_claude_audit_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    package_text: &str,
) -> Result<reqwest::RequestBuilder, AuditCallError> {
    let url = join_base_url(base_url, "/v1/messages")?;
    Ok(crate::gateway::http_client::get()
        .post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header(INTERNAL_FORWARD_HEADER, INTERNAL_FORWARD_VALUE)
        .json(&json!({
            "model": model,
            "max_tokens": AUDIT_MAX_OUTPUT_TOKENS,
            "system": AUDIT_SYSTEM_PROMPT,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": package_text }
                    ]
                }
            ]
        })))
}

fn build_codex_audit_request(
    base_url: &str,
    api_key: &str,
    model: &str,
    package_text: &str,
) -> Result<reqwest::RequestBuilder, AuditCallError> {
    let url = join_base_url(base_url, "/v1/responses")?;
    Ok(crate::gateway::http_client::get()
        .post(url)
        .bearer_auth(api_key)
        .header(INTERNAL_FORWARD_HEADER, INTERNAL_FORWARD_VALUE)
        .json(&json!({
            "model": model,
            "max_output_tokens": AUDIT_MAX_OUTPUT_TOKENS,
            "input": [
                { "role": "system", "content": AUDIT_SYSTEM_PROMPT },
                { "role": "user", "content": package_text }
            ]
        })))
}

fn join_base_url(base_url: &str, path: &str) -> Result<String, AuditCallError> {
    let trimmed_base = base_url.trim();
    if trimmed_base.is_empty() {
        return Err(AuditCallError::Failed(
            "base_url_not_configured".to_string(),
        ));
    }
    let mut url = reqwest::Url::parse(trimmed_base)
        .map_err(|err| AuditCallError::Failed(format!("invalid_base_url: {err}")))?;
    let mut segments = url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut incoming = path
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.last().is_some_and(|last| last == "v1")
        && incoming.first().is_some_and(|first| first == "v1")
    {
        incoming.remove(0);
    }
    segments.extend(incoming);
    url.set_path(&segments.join("/"));
    Ok(url.to_string())
}

fn first_base_url(provider: &ProviderSummary) -> Option<String> {
    provider
        .base_urls
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn extract_provider_response_text(cli_key: &str, value: &Value) -> Option<String> {
    match cli_key {
        "claude" => value
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| {
                let parts = items
                    .iter()
                    .filter_map(|item| item.get("text").and_then(Value::as_str))
                    .filter(|text| !text.trim().is_empty())
                    .collect::<Vec<_>>();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("\n"))
                }
            }),
        "codex" => value
            .get("output_text")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                let mut parts = Vec::new();
                collect_response_texts(value, &mut parts);
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("\n"))
                }
            }),
        _ => None,
    }
}

fn collect_response_texts<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for key in ["text", "content", "message"] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    if !text.trim().is_empty() {
                        out.push(text);
                    }
                }
            }
            for child in map.values() {
                collect_response_texts(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_response_texts(item, out);
            }
        }
        _ => {}
    }
}

fn build_audit_package(
    input: &AssociationAuditInput,
    settings: &settings::AppSettings,
) -> AuditPackageBuild {
    let request = build_payload_snapshot(
        "request",
        &input.cli_key,
        &input.request_body,
        settings.association_audit_max_input_chars as usize,
        false,
    );
    let response = build_payload_snapshot(
        "response",
        &input.cli_key,
        &input.response_body,
        settings.association_audit_max_output_chars as usize,
        input.response_truncated,
    );
    let input_chars = request
        .get("captured_chars")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let output_chars = response
        .get("captured_chars")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let package_truncated = request
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || response
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    AuditPackageBuild {
        package: json!({
            "version": 1,
            "trace_id": input.trace_id,
            "metadata": {
                "cli_key": input.cli_key,
                "method": input.method,
                "path": input.path,
                "requested_model": input.requested_model,
                "status": input.status,
            },
            "request": request,
            "response": response,
            "audit_task": {
                "goal": "Judge whether response content is grounded in the user request.",
                "passive_only": true,
                "do_not_flag_normal_related_diagnostic_tools": true
            }
        }),
        package_truncated,
        input_chars,
        output_chars,
    }
}

fn build_payload_snapshot(
    direction: &str,
    cli_key: &str,
    bytes: &[u8],
    max_chars: usize,
    already_truncated: bool,
) -> Value {
    let raw = String::from_utf8_lossy(bytes).to_string();
    let redacted = redact_text(&raw);
    let (preview, preview_truncated) = truncate_chars_with_flag(&redacted, max_chars);
    let events = payload_events(cli_key, &redacted, max_chars);

    json!({
        "direction": direction,
        "kind": payload_kind(&redacted),
        "captured_chars": preview.chars().count(),
        "truncated": already_truncated || preview_truncated,
        "preview": preview,
        "events": events,
    })
}

fn payload_kind(text: &str) -> &'static str {
    if looks_like_sse(text) {
        "sse"
    } else if serde_json::from_str::<Value>(text).is_ok() {
        "json"
    } else {
        "text"
    }
}

fn payload_events(cli_key: &str, text: &str, max_chars: usize) -> Vec<Value> {
    if looks_like_sse(text) {
        return sse_events(text, max_chars);
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        let mut events = Vec::new();
        collect_json_events(cli_key, &value, &mut events, max_chars);
        if !events.is_empty() {
            return events;
        }
    }
    vec![json!({
        "index": 0,
        "kind": "text",
        "text": truncate_chars(text, max_chars.min(2_000)),
    })]
}

fn looks_like_sse(text: &str) -> bool {
    text.trim_start().starts_with("event:")
        || text.trim_start().starts_with("data:")
        || text.contains("\ndata:")
        || text.contains("\nevent:")
}

fn sse_events(text: &str, max_chars: usize) -> Vec<Value> {
    let normalized = text.replace("\r\n", "\n");
    let mut events = Vec::new();
    let mut used_chars = 0usize;
    for frame in normalized.split("\n\n") {
        if events.len() >= AUDIT_MAX_EVENTS || used_chars >= max_chars {
            break;
        }
        let mut event_name: Option<String> = None;
        let mut data_lines = Vec::new();
        for line in frame.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event_name = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim_start());
            }
        }
        if event_name.is_none() && data_lines.is_empty() {
            continue;
        }
        let data = data_lines.join("\n");
        let event_kind = event_name
            .or_else(|| event_json_type(&data))
            .unwrap_or_else(|| "data".to_string());
        let text = if data.trim() == "[DONE]" {
            "[DONE]".to_string()
        } else if let Ok(json_value) = serde_json::from_str::<Value>(&data) {
            summarize_json_for_event(&json_value)
        } else {
            data
        };
        let remaining = max_chars.saturating_sub(used_chars);
        let bounded = truncate_chars(&text, remaining.min(1_500));
        used_chars = used_chars.saturating_add(bounded.chars().count());
        events.push(json!({
            "index": events.len(),
            "kind": event_kind,
            "text": bounded,
        }));
    }
    events
}

fn event_json_type(data: &str) -> Option<String> {
    serde_json::from_str::<Value>(data).ok().and_then(|value| {
        value
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn collect_json_events(cli_key: &str, value: &Value, out: &mut Vec<Value>, max_chars: usize) {
    let mut used = out
        .iter()
        .filter_map(|event| event.get("text").and_then(Value::as_str))
        .map(|text| text.chars().count())
        .sum::<usize>();
    collect_json_events_inner(cli_key, value, out, max_chars, &mut used);
}

fn collect_json_events_inner(
    cli_key: &str,
    value: &Value,
    out: &mut Vec<Value>,
    max_chars: usize,
    used: &mut usize,
) {
    if out.len() >= AUDIT_MAX_EVENTS || *used >= max_chars {
        return;
    }
    match value {
        Value::Object(map) => {
            let type_name = map.get("type").and_then(Value::as_str);
            let role = map.get("role").and_then(Value::as_str);
            let name = map
                .get("name")
                .or_else(|| map.get("tool_name"))
                .or_else(|| map.get("function"))
                .and_then(Value::as_str);

            let direct_text = map
                .get("text")
                .or_else(|| map.get("input"))
                .or_else(|| map.get("content"))
                .and_then(Value::as_str);

            let should_emit = direct_text.is_some()
                || role.is_some()
                || type_name.is_some_and(|kind| {
                    kind.contains("tool")
                        || kind.contains("function")
                        || kind.contains("message")
                        || kind.contains("text")
                });

            if should_emit {
                let kind = classify_json_event_kind(cli_key, type_name, role, name);
                let summary = direct_text
                    .map(str::to_string)
                    .unwrap_or_else(|| summarize_json_for_event(value));
                let remaining = max_chars.saturating_sub(*used);
                let bounded = truncate_chars(&summary, remaining.min(1_500));
                *used = (*used).saturating_add(bounded.chars().count());
                out.push(json!({
                    "index": out.len(),
                    "kind": kind,
                    "role": role,
                    "name": name,
                    "text": bounded,
                }));
            }

            for child in map.values() {
                collect_json_events_inner(cli_key, child, out, max_chars, used);
                if out.len() >= AUDIT_MAX_EVENTS || *used >= max_chars {
                    break;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_json_events_inner(cli_key, item, out, max_chars, used);
                if out.len() >= AUDIT_MAX_EVENTS || *used >= max_chars {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn classify_json_event_kind(
    cli_key: &str,
    type_name: Option<&str>,
    role: Option<&str>,
    name: Option<&str>,
) -> &'static str {
    if name.is_some() {
        return "tool_call";
    }
    if let Some(kind) = type_name {
        if kind.contains("tool") || kind.contains("function") {
            return "tool_call";
        }
        if kind.contains("text") {
            return "assistant_text";
        }
        if kind.contains("message") {
            return "message";
        }
    }
    match (cli_key, role) {
        (_, Some("assistant")) => "assistant_text",
        (_, Some("user")) => "user_message",
        _ => "json_fragment",
    }
}

fn summarize_json_for_event(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "<json serialization failed>".to_string())
        .chars()
        .take(2_000)
        .collect()
}

fn prefilter_should_call_llm(build: &AuditPackageBuild) -> bool {
    let text = build
        .package
        .get("response")
        .map(Value::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let request_text = build
        .package
        .get("request")
        .map(Value::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();

    let risky_markers = [
        "curl",
        "wget",
        "powershell",
        "bash",
        "chmod",
        "ssh ",
        "scp ",
        "eval(",
        "document.cookie",
        "<script",
        "api_key",
        "access_token",
        "refresh_token",
        "password",
        "secret",
        "ignore previous",
        "system prompt",
        "role:",
        "bypass",
        "callback",
        "webhook",
        "广告",
        "服务器线路",
        "优惠",
        "购买",
        "加群",
        "推广",
    ];
    if risky_markers.iter().any(|marker| text.contains(marker)) {
        return true;
    }

    build.output_chars >= 1_000 && rough_token_overlap(&request_text, &text) < 0.08
}

fn rough_token_overlap(left: &str, right: &str) -> f64 {
    let left_tokens = rough_tokens(left);
    if left_tokens.is_empty() {
        return 1.0;
    }
    let right_tokens = rough_tokens(right);
    if right_tokens.is_empty() {
        return 0.0;
    }
    let matched = left_tokens
        .iter()
        .filter(|token| right_tokens.iter().any(|other| other == *token))
        .count();
    matched as f64 / left_tokens.len() as f64
}

fn rough_tokens(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|token| token.chars().count() >= 4)
        .take(200)
        .map(str::to_string)
        .collect()
}

fn sample_selected(trace_id: &str, sample_rate: u8) -> bool {
    if sample_rate >= 100 {
        return true;
    }
    if sample_rate == 0 {
        return false;
    }
    let mut hasher = DefaultHasher::new();
    trace_id.hash(&mut hasher);
    (hasher.finish() % 100) < sample_rate as u64
}

fn normalize_audit_model_output(raw: &str) -> Result<Value, String> {
    let json_text = extract_json_object_text(raw)
        .ok_or_else(|| "audit_model_json_object_missing".to_string())?;
    let value: Value = serde_json::from_str(json_text)
        .map_err(|err| format!("audit_model_json_parse_failed: {err}"))?;
    if !value.is_object() {
        return Err("audit_model_json_root_not_object".to_string());
    }

    let association_score = value
        .get("association_score")
        .and_then(Value::as_f64)
        .map(|score| score.clamp(0.0, 1.0));
    let overall_risk = normalize_risk(value.get("overall_risk").and_then(Value::as_str));
    let insufficient_context = value
        .get("insufficient_context")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let notes = value
        .get("notes")
        .and_then(Value::as_str)
        .map(|text| truncate_chars(&redact_text(text), AUDIT_MAX_REASON_CHARS));

    let mut signals = Vec::new();
    if let Some(items) = value.get("signals").and_then(Value::as_array) {
        for item in items.iter().take(AUDIT_MAX_SIGNALS) {
            if let Some(signal) = normalize_signal(item) {
                signals.push(signal);
            }
        }
    }

    Ok(json!({
        "association_score": association_score,
        "overall_risk": overall_risk,
        "signals": signals,
        "insufficient_context": insufficient_context,
        "notes": notes,
    }))
}

fn extract_json_object_text(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if start < end {
        Some(&trimmed[start..=end])
    } else {
        None
    }
}

fn normalize_signal(value: &Value) -> Option<Value> {
    let object = value.as_object()?;
    let code = object
        .get("code")
        .and_then(Value::as_str)
        .map(|text| normalize_code(text, "unknown_signal"))
        .unwrap_or_else(|| "unknown_signal".to_string());
    let severity = normalize_severity(object.get("severity").and_then(Value::as_str));
    let confidence = object
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let event_index = object
        .get("event_index")
        .and_then(Value::as_i64)
        .filter(|value| *value >= 0);
    let event_kind = object
        .get("event_kind")
        .and_then(Value::as_str)
        .map(|text| normalize_code(text, "other"))
        .unwrap_or_else(|| "other".to_string());
    let reason = object
        .get("reason")
        .and_then(Value::as_str)
        .map(|text| truncate_chars(&redact_text(text), AUDIT_MAX_REASON_CHARS))
        .unwrap_or_default();
    let evidence = object
        .get("evidence")
        .and_then(Value::as_str)
        .map(|text| truncate_chars(&redact_text(text), AUDIT_MAX_EVIDENCE_CHARS))
        .unwrap_or_default();

    Some(json!({
        "code": code,
        "severity": severity,
        "confidence": confidence,
        "event_index": event_index,
        "event_kind": event_kind,
        "reason": reason,
        "evidence": evidence,
    }))
}

fn normalize_risk(value: Option<&str>) -> &'static str {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "none" => "none",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "critical" => "critical",
        _ => "unknown",
    }
}

fn normalize_severity(value: Option<&str>) -> &'static str {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "critical" => "critical",
        _ => "low",
    }
}

fn normalize_code(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars().take(64) {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() {
            out.push('_');
        }
    }
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

fn status_json(
    status: &str,
    started_at_ms: i64,
    reason: Option<&str>,
    error: Option<String>,
    provider: Option<&ProviderSummary>,
    build: &AuditPackageBuild,
) -> Value {
    let mut value = json!({
        "status": status,
        "started_at_ms": started_at_ms,
        "finished_at_ms": now_ms(),
        "duration_ms": now_ms().saturating_sub(started_at_ms),
        "input_chars": build.input_chars,
        "output_chars": build.output_chars,
        "package_truncated": build.package_truncated,
    });
    if let Some(reason) = reason {
        value["reason"] = Value::String(reason.to_string());
    }
    if let Some(error) = error {
        value["error"] = Value::String(truncate_chars(&redact_text(&error), 700));
    }
    if let Some(provider) = provider {
        value["provider_id"] = Value::from(provider.id);
        value["provider_name"] = Value::String(provider.name.clone());
        value["provider_cli_key"] = Value::String(provider.cli_key.clone());
    }
    value
}

fn merge_status_meta(
    value: &mut Value,
    status: &str,
    started_at_ms: i64,
    reason: Option<&str>,
    provider: Option<&ProviderSummary>,
    model: &str,
    build: &AuditPackageBuild,
) {
    let finished_at_ms = now_ms();
    value["status"] = Value::String(status.to_string());
    value["started_at_ms"] = Value::from(started_at_ms);
    value["finished_at_ms"] = Value::from(finished_at_ms);
    value["duration_ms"] = Value::from(finished_at_ms.saturating_sub(started_at_ms));
    value["input_chars"] = Value::from(build.input_chars);
    value["output_chars"] = Value::from(build.output_chars);
    value["package_truncated"] = Value::Bool(build.package_truncated);
    value["model"] = Value::String(model.to_string());
    if let Some(reason) = reason {
        value["reason"] = Value::String(reason.to_string());
    }
    if let Some(provider) = provider {
        value["provider_id"] = Value::from(provider.id);
        value["provider_name"] = Value::String(provider.name.clone());
        value["provider_cli_key"] = Value::String(provider.cli_key.clone());
    }
}

async fn persist_status(db: db::Db, trace_id: String, value: Value) {
    let json = match serde_json::to_string(&value) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(
                trace_id = %trace_id,
                error = %err,
                "association audit status serialization failed"
            );
            return;
        }
    };

    for delay_ms in [0_u64, 50, 100, 250, 500, 1_000, 2_000] {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        let update_result = blocking::run("association_audit_persist_status", {
            let db = db.clone();
            let trace_id = trace_id.clone();
            let json = json.clone();
            move || request_logs::update_association_audit_json_by_trace_id(&db, &trace_id, json)
        })
        .await;
        match update_result {
            Ok(true) => return,
            Ok(false) => continue,
            Err(err) => {
                tracing::warn!(
                    trace_id = %trace_id,
                    error = %err,
                    "association audit status update failed"
                );
                return;
            }
        }
    }

    tracing::warn!(
        trace_id = %trace_id,
        "association audit status update skipped because request log row was not found"
    );
}

fn redact_text(input: &str) -> String {
    let mut out = input.to_string();
    for (regex, replacement) in [
        (private_key_regex(), "[REDACTED_PRIVATE_KEY]"),
        (bearer_regex(), "Bearer [REDACTED_TOKEN]"),
        (sk_key_regex(), "[REDACTED_API_KEY]"),
        (url_credential_regex(), "://[REDACTED_CREDENTIALS]@"),
        (secret_assignment_regex(), "$1=[REDACTED_SECRET]"),
        (long_token_regex(), "[REDACTED_TOKEN]"),
    ] {
        out = regex.replace_all(&out, replacement).into_owned();
    }
    out
}

fn private_key_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----")
            .expect("private key regex")
    })
}

fn bearer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{12,}").expect("bearer regex")
    })
}

fn sk_key_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\bsk-[A-Za-z0-9_-]{16,}\b").expect("api key regex"))
}

fn url_credential_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"://[^/\s:@]+:[^/\s@]+@").expect("url credential regex"))
}

fn secret_assignment_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(api[_-]?key|access[_-]?token|refresh[_-]?token|id[_-]?token|password|passwd|secret|authorization)\b\s*[:=]\s*"?[^"\s,}\]]+"?"#,
        )
        .expect("secret assignment regex")
    })
}

fn long_token_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\b[A-Za-z0-9_-]{48,}\b").expect("long token regex"))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    truncate_chars_with_flag(value, max_chars).0
}

fn truncate_chars_with_flag(value: &str, max_chars: usize) -> (String, bool) {
    if value.chars().nth(max_chars).is_none() {
        return (value.to_string(), false);
    }
    let mut out = value.chars().take(max_chars).collect::<String>();
    out.push_str("\n[TRUNCATED]");
    (out, true)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_shapes() {
        let raw = concat!(
            "Authorization: Bearer abcdefghijklmnopqrstuvwxyz123456\n",
            "api_key=sk-1234567890abcdefghijklmnop\n",
            "https://user:pass@example.com/path\n",
            "-----BEGIN PRIVATE KEY-----\nsecret\n-----END PRIVATE KEY-----"
        );

        let redacted = redact_text(raw);

        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!redacted.contains("sk-1234567890abcdefghijklmnop"));
        assert!(!redacted.contains("user:pass"));
        assert!(!redacted.contains("BEGIN PRIVATE KEY"));
        assert!(redacted.contains("[REDACTED"));
    }

    #[test]
    fn sse_events_preserve_order_and_event_kind() {
        let raw = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"analysis\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"<script>alert(1)</script>\"}}\n\n"
        );

        let events = sse_events(raw, 10_000);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["index"], 0);
        assert_eq!(events[1]["kind"], "content_block_delta");
        assert!(events[2]["text"].as_str().unwrap().contains("<script>"));
    }

    #[test]
    fn normalizes_model_output_and_redacts_evidence() {
        let raw = r#"{
          "association_score": 1.4,
          "overall_risk": "HIGH",
          "signals": [
            {
              "code": "Ungrounded Script",
              "severity": "critical",
              "confidence": 2.0,
              "event_index": 3,
              "event_kind": "tool call",
              "reason": "Writes script with api_key=secret-value",
              "evidence": "Bearer abcdefghijklmnopqrstuvwxyz123456"
            }
          ],
          "insufficient_context": false,
          "notes": "ok"
        }"#;

        let normalized = normalize_audit_model_output(raw).expect("normalize");

        assert_eq!(normalized["association_score"], 1.0);
        assert_eq!(normalized["overall_risk"], "high");
        assert_eq!(normalized["signals"][0]["code"], "ungrounded_script");
        assert_eq!(normalized["signals"][0]["confidence"], 1.0);
        assert!(normalized["signals"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("[REDACTED_SECRET]"));
        assert!(normalized["signals"][0]["evidence"]
            .as_str()
            .unwrap()
            .contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn join_base_url_avoids_duplicate_v1() {
        assert_eq!(
            join_base_url("https://api.example.com/v1", "/v1/messages").unwrap(),
            "https://api.example.com/v1/messages"
        );
        assert_eq!(
            join_base_url("https://api.example.com/root", "/v1/responses").unwrap(),
            "https://api.example.com/root/v1/responses"
        );
    }
}
