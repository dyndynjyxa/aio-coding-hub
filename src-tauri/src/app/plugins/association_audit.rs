//! Usage: Native passive association audit plugin engine.

use crate::domain::plugins::PluginDetail;
use crate::gateway::plugins::context::{GatewayHookResult, GatewayVisibleHookContext};
use crate::gateway::plugins::model_invoke::{invoke_model, ModelInvokeInput};
use crate::gateway::plugins::permissions::GatewayPluginError;
use crate::infra::plugins::repository::{self, AppendPluginAuditLogInput};
use crate::{blocking, db, providers};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const AUDIT_RESPONSE_COLLECTOR_MAX_BYTES: usize = 128 * 1024;
const AUDIT_PROVIDER_RESPONSE_MAX_BYTES: usize = 64 * 1024;
const AUDIT_MAX_OUTPUT_TOKENS: u32 = 700;
const AUDIT_MAX_EVENTS: usize = 80;
const AUDIT_MAX_SIGNALS: usize = 12;
const AUDIT_MAX_REASON_CHARS: usize = 900;
const AUDIT_MAX_EVIDENCE_CHARS: usize = 700;
const DEFAULT_SAMPLE_RATE: u8 = 10;
const DEFAULT_TIMEOUT_SECONDS: u32 = 8;
const DEFAULT_MAX_INPUT_CHARS: u32 = 6_000;
const DEFAULT_MAX_OUTPUT_CHARS: u32 = 12_000;
const MAX_TIMEOUT_SECONDS: u32 = 60;
const MIN_CAPTURE_CHARS: u32 = 256;
const MAX_CAPTURE_CHARS: u32 = 50_000;
const CAPTURE_TTL_MS: i64 = 10 * 60 * 1000;

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
  "notes": "optional short note"
}"#;

#[derive(Debug, Default)]
pub(crate) struct AssociationAuditGatewayPluginExecutor {
    captures: Mutex<HashMap<String, AssociationAuditCapture>>,
}

impl AssociationAuditGatewayPluginExecutor {
    pub(crate) fn execute_plugin(
        &self,
        plugin: &PluginDetail,
        context: GatewayVisibleHookContext,
        db: Option<&db::Db>,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let config = AssociationAuditConfig::from_plugin_config(&plugin.config);
        if config.mode == AssociationAuditMode::Off {
            return Ok(GatewayHookResult::continue_unchanged());
        }

        match context.hook_name.as_str() {
            "gateway.request.afterBodyRead" => self.capture_request(&context),
            "gateway.response.chunk" => self.capture_stream_chunk(&context),
            "gateway.response.after" | "gateway.error" => {
                self.finalize_response(plugin, &config, &context, db)
            }
            "log.beforePersist" => self.finalize_logged_stream(plugin, &config, &context, db),
            _ => Ok(GatewayHookResult::continue_unchanged()),
        }
    }

    fn capture_request(
        &self,
        context: &GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        self.prune_expired_captures();
        let Some(body) = context.request.body.clone() else {
            return Ok(GatewayHookResult::continue_unchanged());
        };
        let capture = AssociationAuditCapture {
            trace_id: context.trace_id.clone(),
            cli_key: context.request.cli_key.clone().unwrap_or_default(),
            method: context.request.method.clone().unwrap_or_default(),
            path: context.request.path.clone().unwrap_or_default(),
            requested_model: context.request.requested_model.clone(),
            request_body: body,
            response_body: String::new(),
            response_truncated: false,
            status: None,
            updated_at_ms: now_ms(),
        };
        let mut captures = self.lock_captures()?;
        captures.insert(context.trace_id.clone(), capture);
        Ok(GatewayHookResult::continue_unchanged())
    }

    fn capture_stream_chunk(
        &self,
        context: &GatewayVisibleHookContext,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let Some(chunk) = context.stream.chunk.as_deref() else {
            return Ok(GatewayHookResult::continue_unchanged());
        };
        let mut captures = self.lock_captures()?;
        let capture = captures
            .entry(context.trace_id.clone())
            .or_insert_with(|| AssociationAuditCapture::from_trace(context.trace_id.clone()));
        capture.append_response_chunk(chunk);
        Ok(GatewayHookResult::continue_unchanged())
    }

    fn finalize_response(
        &self,
        plugin: &PluginDetail,
        config: &AssociationAuditConfig,
        context: &GatewayVisibleHookContext,
        db: Option<&db::Db>,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let mut capture = {
            let mut captures = self.lock_captures()?;
            captures
                .remove(&context.trace_id)
                .unwrap_or_else(|| AssociationAuditCapture::from_trace(context.trace_id.clone()))
        };
        if let Some(body) = context.response.body.clone() {
            capture.response_body = body;
        }
        capture.status = context.response.status;
        capture.updated_at_ms = now_ms();
        self.spawn_background_audit(plugin, config, capture, db)?;
        Ok(GatewayHookResult::continue_unchanged())
    }

    fn finalize_logged_stream(
        &self,
        plugin: &PluginDetail,
        config: &AssociationAuditConfig,
        context: &GatewayVisibleHookContext,
        db: Option<&db::Db>,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let capture = {
            let mut captures = self.lock_captures()?;
            captures.remove(&context.trace_id)
        };
        if let Some(capture) = capture {
            self.spawn_background_audit(plugin, config, capture, db)?;
        }
        Ok(GatewayHookResult::continue_unchanged())
    }

    fn spawn_background_audit(
        &self,
        plugin: &PluginDetail,
        config: &AssociationAuditConfig,
        capture: AssociationAuditCapture,
        db: Option<&db::Db>,
    ) -> Result<(), GatewayPluginError> {
        if capture.response_body.trim().is_empty() {
            return Ok(());
        }
        let Some(db) = db.cloned() else {
            return Err(GatewayPluginError::new(
                "PLUGIN_ASSOCIATION_AUDIT_UNAVAILABLE",
                "association audit native plugin requires database access",
            ));
        };
        let plugin_id = plugin.summary.plugin_id.clone();
        let config = config.clone();
        tauri::async_runtime::spawn(async move {
            run_association_audit(db, plugin_id, config, capture).await;
        });
        Ok(())
    }

    fn prune_expired_captures(&self) {
        let Ok(mut captures) = self.captures.lock() else {
            return;
        };
        let cutoff = now_ms().saturating_sub(CAPTURE_TTL_MS);
        captures.retain(|_, capture| capture.updated_at_ms >= cutoff);
    }

    fn lock_captures(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, HashMap<String, AssociationAuditCapture>>,
        GatewayPluginError,
    > {
        self.captures.lock().map_err(|_| {
            GatewayPluginError::new(
                "PLUGIN_ASSOCIATION_AUDIT_STATE_POISONED",
                "association audit capture state is unavailable",
            )
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssociationAuditConfig {
    #[serde(default)]
    provider_id: Option<i64>,
    #[serde(default)]
    model: String,
    #[serde(default)]
    mode: AssociationAuditMode,
    #[serde(default = "default_sample_rate")]
    sample_rate: u8,
    #[serde(default = "default_timeout_seconds")]
    timeout_seconds: u32,
    #[serde(default = "default_max_input_chars")]
    max_input_chars: u32,
    #[serde(default = "default_max_output_chars")]
    max_output_chars: u32,
}

impl AssociationAuditConfig {
    fn from_plugin_config(value: &Value) -> Self {
        let mut config = serde_json::from_value::<Self>(value.clone()).unwrap_or_default();
        config.sample_rate = config.sample_rate.clamp(1, 100);
        config.timeout_seconds = config.timeout_seconds.clamp(1, MAX_TIMEOUT_SECONDS);
        config.max_input_chars = config
            .max_input_chars
            .clamp(MIN_CAPTURE_CHARS, MAX_CAPTURE_CHARS);
        config.max_output_chars = config
            .max_output_chars
            .clamp(MIN_CAPTURE_CHARS, MAX_CAPTURE_CHARS);
        config.model = config.model.trim().to_string();
        config
    }
}

impl Default for AssociationAuditConfig {
    fn default() -> Self {
        Self {
            provider_id: None,
            model: String::new(),
            mode: AssociationAuditMode::default(),
            sample_rate: DEFAULT_SAMPLE_RATE,
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            max_input_chars: DEFAULT_MAX_INPUT_CHARS,
            max_output_chars: DEFAULT_MAX_OUTPUT_CHARS,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum AssociationAuditMode {
    Off,
    Sampled,
    Prefiltered,
    All,
}

impl AssociationAuditMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Sampled => "sampled",
            Self::Prefiltered => "prefiltered",
            Self::All => "all",
        }
    }
}

impl Default for AssociationAuditMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone)]
struct AssociationAuditCapture {
    trace_id: String,
    cli_key: String,
    method: String,
    path: String,
    requested_model: Option<String>,
    request_body: String,
    response_body: String,
    response_truncated: bool,
    status: Option<u16>,
    updated_at_ms: i64,
}

impl AssociationAuditCapture {
    fn from_trace(trace_id: String) -> Self {
        Self {
            trace_id,
            cli_key: String::new(),
            method: String::new(),
            path: String::new(),
            requested_model: None,
            request_body: String::new(),
            response_body: String::new(),
            response_truncated: false,
            status: None,
            updated_at_ms: now_ms(),
        }
    }

    fn append_response_chunk(&mut self, chunk: &str) {
        self.updated_at_ms = now_ms();
        if self.response_truncated {
            return;
        }
        for ch in chunk.chars() {
            if self.response_body.len() + ch.len_utf8() > AUDIT_RESPONSE_COLLECTOR_MAX_BYTES {
                self.response_body.push_str("\n[TRUNCATED]");
                self.response_truncated = true;
                break;
            }
            self.response_body.push(ch);
        }
    }
}

struct AuditPackageBuild {
    package: Value,
    input_chars: usize,
    output_chars: usize,
    package_truncated: bool,
}

async fn run_association_audit(
    db: db::Db,
    plugin_id: String,
    config: AssociationAuditConfig,
    capture: AssociationAuditCapture,
) {
    let started_at_ms = now_ms();
    let build = build_audit_package(&capture, &config);
    let trace_id = capture.trace_id.clone();

    if config.mode == AssociationAuditMode::Sampled
        && !sample_selected(&trace_id, config.sample_rate)
    {
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.skipped",
            "low",
            "Association audit skipped",
            status_details(
                "skipped",
                Some("sampled_out"),
                &capture,
                &config,
                Some(&build),
                None,
                started_at_ms,
            ),
        )
        .await;
        return;
    }

    if config.mode == AssociationAuditMode::Prefiltered && !prefilter_should_call_llm(&build) {
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.skipped",
            "low",
            "Association audit skipped",
            status_details(
                "skipped",
                Some("prefilter_no_signal"),
                &capture,
                &config,
                Some(&build),
                None,
                started_at_ms,
            ),
        )
        .await;
        return;
    }

    let Some(provider_id) = config.provider_id.filter(|id| *id > 0) else {
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.not_configured",
            "low",
            "Association audit provider is not configured",
            status_details(
                "not_configured",
                Some("provider_not_selected"),
                &capture,
                &config,
                Some(&build),
                None,
                started_at_ms,
            ),
        )
        .await;
        return;
    };
    if config.model.is_empty() {
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.not_configured",
            "low",
            "Association audit model is not configured",
            status_details(
                "not_configured",
                Some("model_not_selected"),
                &capture,
                &config,
                Some(&build),
                None,
                started_at_ms,
            ),
        )
        .await;
        return;
    }

    let provider = match load_provider_summary(&db, provider_id).await {
        Ok(provider) => provider,
        Err(err) => {
            append_status_log(
                db,
                plugin_id,
                trace_id.clone(),
                "association.audit.failed",
                "medium",
                "Association audit provider lookup failed",
                failed_details(
                    "provider_lookup_failed",
                    &err,
                    &capture,
                    &config,
                    Some(&build),
                    None,
                    started_at_ms,
                ),
            )
            .await;
            return;
        }
    };

    if let Some(reason) = provider_unavailable_reason(&provider) {
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.not_configured",
            "low",
            "Association audit provider is unavailable",
            status_details(
                "not_configured",
                Some(reason),
                &capture,
                &config,
                Some(&build),
                Some(&provider),
                started_at_ms,
            ),
        )
        .await;
        return;
    }

    let body = audit_model_body(&provider.cli_key, &build.package);
    let output = invoke_model(
        &db,
        crate::gateway::http_client::get(),
        ModelInvokeInput {
            provider_id,
            model: config.model.clone(),
            body,
            timeout_ms: Some(u64::from(config.timeout_seconds) * 1_000),
            max_response_bytes: Some(AUDIT_PROVIDER_RESPONSE_MAX_BYTES),
            trace_id: Some(trace_id.clone()),
        },
    )
    .await;

    if !output.ok {
        let error_code = output
            .error
            .as_ref()
            .map(|error| error.code.as_str())
            .unwrap_or("MODEL_INVOKE_FAILED");
        let error_message = output
            .error
            .as_ref()
            .map(|error| error.message.as_str())
            .unwrap_or("model invocation failed");
        append_status_log(
            db,
            plugin_id,
            trace_id.clone(),
            "association.audit.failed",
            "medium",
            "Association audit model call failed",
            failed_details(
                error_code,
                error_message,
                &capture,
                &config,
                Some(&build),
                Some(&provider),
                started_at_ms,
            ),
        )
        .await;
        return;
    }

    let response_body = output.response_body.unwrap_or_default();
    let response_json = match serde_json::from_str::<Value>(&response_body) {
        Ok(value) => value,
        Err(err) => {
            append_status_log(
                db,
                plugin_id,
                trace_id.clone(),
                "association.audit.invalid_response",
                "medium",
                "Association audit response was not JSON",
                failed_details(
                    "invalid_response_json",
                    &err.to_string(),
                    &capture,
                    &config,
                    Some(&build),
                    Some(&provider),
                    started_at_ms,
                ),
            )
            .await;
            return;
        }
    };
    let model_text = match extract_provider_response_text(&provider.cli_key, &response_json) {
        Some(text) => text,
        None => {
            append_status_log(
                db,
                plugin_id,
                trace_id.clone(),
                "association.audit.invalid_response",
                "medium",
                "Association audit response did not contain text",
                failed_details(
                    "missing_model_text",
                    "provider response did not contain an audit text payload",
                    &capture,
                    &config,
                    Some(&build),
                    Some(&provider),
                    started_at_ms,
                ),
            )
            .await;
            return;
        }
    };

    let normalized = match normalize_audit_model_output(&model_text) {
        Ok(value) => value,
        Err(err) => {
            append_status_log(
                db,
                plugin_id,
                trace_id.clone(),
                "association.audit.invalid_response",
                "medium",
                "Association audit model output was invalid",
                failed_details(
                    "invalid_audit_output",
                    &err,
                    &capture,
                    &config,
                    Some(&build),
                    Some(&provider),
                    started_at_ms,
                ),
            )
            .await;
            return;
        }
    };

    let risk_level = audit_log_risk_level(&normalized);
    let overall_risk = normalized
        .get("overall_risk")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    append_status_log(
        db,
        plugin_id,
        trace_id,
        "association.audit.completed",
        risk_level,
        format!("Association audit completed ({overall_risk})"),
        completed_details(
            normalized,
            &capture,
            &config,
            &build,
            &provider,
            started_at_ms,
        ),
    )
    .await;
}

async fn load_provider_summary(
    db: &db::Db,
    provider_id: i64,
) -> Result<providers::ProviderSummary, String> {
    let db = db.clone();
    blocking::run("association_audit_load_provider", move || {
        let conn = db.open_connection()?;
        providers::get_by_id(&conn, provider_id)
    })
    .await
    .map_err(|err| err.to_string())
}

async fn append_status_log(
    db: db::Db,
    plugin_id: String,
    trace_id: String,
    event_type: &'static str,
    risk_level: &'static str,
    message: impl Into<String>,
    details: Value,
) {
    let message = message.into();
    let plugin_id_for_log = plugin_id.clone();
    let trace_id_for_log = trace_id.clone();
    let input = AppendPluginAuditLogInput {
        plugin_id: Some(plugin_id),
        trace_id: Some(trace_id),
        event_type: event_type.to_string(),
        risk_level: risk_level.to_string(),
        message,
        details,
    };
    if let Err(err) = blocking::run("association_audit_append_log", move || {
        repository::append_audit_log(&db, input)
    })
    .await
    {
        tracing::warn!(
            plugin_id = %plugin_id_for_log,
            trace_id = %trace_id_for_log,
            event_type,
            error = %err,
            "failed to persist association audit plugin log"
        );
    }
}

fn provider_unavailable_reason(provider: &providers::ProviderSummary) -> Option<&'static str> {
    if !provider.enabled {
        return Some("provider_disabled");
    }
    if provider.source_provider_id.is_some() || provider.bridge_type.is_some() {
        return Some("bridged_provider_unsupported");
    }
    if provider.auth_mode != providers::ProviderAuthMode::ApiKey.as_str() {
        return Some("api_key_provider_required");
    }
    if !matches!(provider.cli_key.as_str(), "claude" | "codex") {
        return Some("provider_cli_unsupported");
    }
    if !provider.api_key_configured {
        return Some("provider_api_key_missing");
    }
    if provider.base_urls.iter().all(|url| url.trim().is_empty()) {
        return Some("provider_base_url_missing");
    }
    None
}

fn audit_model_body(cli_key: &str, package: &Value) -> Value {
    let payload = serde_json::to_string_pretty(package).unwrap_or_else(|_| package.to_string());
    if cli_key == "claude" {
        json!({
            "max_tokens": AUDIT_MAX_OUTPUT_TOKENS,
            "system": AUDIT_SYSTEM_PROMPT,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": payload }
                    ]
                }
            ]
        })
    } else {
        json!({
            "max_output_tokens": AUDIT_MAX_OUTPUT_TOKENS,
            "input": [
                { "role": "system", "content": AUDIT_SYSTEM_PROMPT },
                { "role": "user", "content": payload }
            ]
        })
    }
}

fn build_audit_package(
    capture: &AssociationAuditCapture,
    config: &AssociationAuditConfig,
) -> AuditPackageBuild {
    let (request_text, request_truncated) = truncate_chars_with_flag(
        &redact_text(&capture.request_body),
        config.max_input_chars as usize,
    );
    let (response_text, output_truncated) = truncate_chars_with_flag(
        &redact_text(&capture.response_body),
        config.max_output_chars as usize,
    );
    let input_chars = request_text.chars().count();
    let output_chars = response_text.chars().count();
    let events = payload_events(&capture.response_body, config.max_output_chars as usize);
    let package_truncated = request_truncated || output_truncated || capture.response_truncated;

    AuditPackageBuild {
        package: json!({
            "schemaVersion": 1,
            "traceId": capture.trace_id,
            "mode": config.mode.as_str(),
            "request": {
                "cliKey": capture.cli_key,
                "method": capture.method,
                "path": capture.path,
                "requestedModel": capture.requested_model,
                "body": request_text,
                "truncated": request_truncated,
            },
            "response": {
                "status": capture.status,
                "body": response_text,
                "truncated": output_truncated || capture.response_truncated,
                "events": events,
            },
            "task": {
                "kind": "association_audit",
                "instructions": "Return only the requested JSON object. Quote only redacted evidence from response events.",
            }
        }),
        input_chars,
        output_chars,
        package_truncated,
    }
}

fn payload_events(body: &str, max_chars: usize) -> Vec<Value> {
    if body.trim_start().starts_with("data:") || body.contains("\ndata:") {
        return sse_events(body, max_chars)
            .into_iter()
            .take(AUDIT_MAX_EVENTS)
            .collect();
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        let mut events = Vec::new();
        collect_json_events(&value, &mut events, max_chars);
        if !events.is_empty() {
            return events.into_iter().take(AUDIT_MAX_EVENTS).collect();
        }
    }
    vec![json!({
        "index": 0,
        "kind": "text",
        "text": truncate_chars(&redact_text(body), max_chars),
    })]
}

fn collect_json_events(value: &Value, events: &mut Vec<Value>, max_chars: usize) {
    if events.len() >= AUDIT_MAX_EVENTS {
        return;
    }
    match value {
        Value::Object(object) => {
            if let Some(output) = object.get("output").and_then(Value::as_array) {
                for item in output {
                    collect_json_events(item, events, max_chars);
                }
                return;
            }
            if let Some(content) = object.get("content").and_then(Value::as_array) {
                for item in content {
                    collect_json_events(item, events, max_chars);
                }
                return;
            }
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                events.push(json!({
                    "index": events.len(),
                    "kind": object.get("type").and_then(Value::as_str).unwrap_or("text"),
                    "text": truncate_chars(&redact_text(text), max_chars),
                }));
                return;
            }
            if object.get("type").and_then(Value::as_str) == Some("tool_use") {
                events.push(json!({
                    "index": events.len(),
                    "kind": "tool_call",
                    "name": object.get("name").and_then(Value::as_str),
                    "input": truncate_chars(&redact_text(&Value::Object(object.clone()).to_string()), max_chars),
                }));
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_json_events(item, events, max_chars);
                if events.len() >= AUDIT_MAX_EVENTS {
                    break;
                }
            }
        }
        Value::String(text) => {
            events.push(json!({
                "index": events.len(),
                "kind": "text",
                "text": truncate_chars(&redact_text(text), max_chars),
            }));
        }
        _ => {}
    }
}

fn sse_events(body: &str, max_chars: usize) -> Vec<Value> {
    let mut events = Vec::new();
    for block in body.split("\n\n") {
        let mut event_kind = "message".to_string();
        let mut data_lines = Vec::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("event:") {
                event_kind = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim());
            }
        }
        if data_lines.is_empty() {
            continue;
        }
        let data = data_lines.join("\n");
        if data == "[DONE]" {
            continue;
        }
        let text = if let Ok(value) = serde_json::from_str::<Value>(&data) {
            extract_event_text(&value).unwrap_or_else(|| value.to_string())
        } else {
            data
        };
        events.push(json!({
            "index": events.len(),
            "kind": event_kind,
            "text": truncate_chars(&redact_text(&text), max_chars),
        }));
        if events.len() >= AUDIT_MAX_EVENTS {
            break;
        }
    }
    events
}

fn extract_event_text(value: &Value) -> Option<String> {
    if let Some(text) = value.pointer("/delta/text").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    if let Some(text) = value.pointer("/content_block/text").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    if let Some(text) = value
        .pointer("/message/content/0/text")
        .and_then(Value::as_str)
    {
        return Some(text.to_string());
    }
    value
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn prefilter_should_call_llm(build: &AuditPackageBuild) -> bool {
    if build.package_truncated {
        return true;
    }
    let request = build
        .package
        .pointer("/request/body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let response = build
        .package
        .pointer("/response/body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if response.len() < 120 {
        return false;
    }
    let suspicious_markers = [
        "curl ",
        "powershell",
        "cmd.exe",
        "bash -c",
        "chmod ",
        "rm -rf",
        "http://",
        "https://",
        "api_key",
        "token=",
        "bearer ",
        "callback",
        "webhook",
        "cron",
        "systemctl",
        "ssh-rsa",
        "BEGIN PRIVATE KEY",
    ];
    let response_lower = response.to_ascii_lowercase();
    if suspicious_markers
        .iter()
        .any(|marker| response_lower.contains(marker))
    {
        return true;
    }
    let overlap = rough_token_overlap(request, response);
    overlap < 0.08 && response.chars().count() > 400
}

fn rough_token_overlap(left: &str, right: &str) -> f64 {
    let left_tokens = token_set(left);
    let right_tokens = token_set(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 1.0;
    }
    let shared = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count();
    shared as f64 / left_tokens.len().min(right_tokens.len()) as f64
}

fn token_set(input: &str) -> std::collections::HashSet<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|part| {
            let part = part.trim().to_ascii_lowercase();
            (part.len() >= 4).then_some(part)
        })
        .take(300)
        .collect()
}

fn sample_selected(trace_id: &str, sample_rate: u8) -> bool {
    let rate = sample_rate.clamp(1, 100);
    if rate >= 100 {
        return true;
    }
    let mut hasher = DefaultHasher::new();
    trace_id.hash(&mut hasher);
    (hasher.finish() % 100) < u64::from(rate)
}

fn extract_provider_response_text(cli_key: &str, value: &Value) -> Option<String> {
    let mut texts = Vec::new();
    if cli_key == "claude" {
        collect_claude_text(value, &mut texts);
    } else {
        collect_openai_text(value, &mut texts);
    }
    let joined = texts
        .into_iter()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!joined.is_empty()).then_some(joined)
}

fn collect_claude_text(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                out.push(text.to_string());
            }
            if let Some(content) = object.get("content").and_then(Value::as_array) {
                for item in content {
                    collect_claude_text(item, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_claude_text(item, out);
            }
        }
        _ => {}
    }
}

fn collect_openai_text(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if let Some(text) = object.get("output_text").and_then(Value::as_str) {
                out.push(text.to_string());
            }
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                out.push(text.to_string());
            }
            if let Some(content) = Value::Object(object.clone())
                .pointer("/message/content")
                .and_then(Value::as_str)
            {
                out.push(content.to_string());
            }
            if let Some(choices) = object.get("choices").and_then(Value::as_array) {
                for choice in choices {
                    collect_openai_text(choice, out);
                }
            }
            if let Some(output) = object.get("output").and_then(Value::as_array) {
                for item in output {
                    collect_openai_text(item, out);
                }
            }
            if let Some(content) = object.get("content").and_then(Value::as_array) {
                for item in content {
                    collect_openai_text(item, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_openai_text(item, out);
            }
        }
        _ => {}
    }
}

fn normalize_audit_model_output(raw: &str) -> Result<Value, String> {
    let value = parse_json_object(raw)
        .ok_or_else(|| "audit output did not contain a JSON object".to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "audit output JSON root must be an object".to_string())?;
    let association_score = clamp_f64(
        object
            .get("association_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        0.0,
        1.0,
    );
    let overall_risk = normalize_risk(
        object
            .get("overall_risk")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
    );
    let signals = object
        .get("signals")
        .and_then(Value::as_array)
        .map(|signals| normalize_signals(signals))
        .unwrap_or_default();
    let insufficient_context = object
        .get("insufficient_context")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let notes = object
        .get("notes")
        .and_then(Value::as_str)
        .map(|notes| truncate_chars(&redact_text(notes), AUDIT_MAX_REASON_CHARS));

    Ok(json!({
        "association_score": association_score,
        "overall_risk": overall_risk,
        "signals": signals,
        "insufficient_context": insufficient_context,
        "notes": notes,
    }))
}

fn parse_json_object(raw: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return value.as_object().is_some().then_some(value);
    }
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<Value>(&raw[start..=end]).ok()
}

fn normalize_signals(signals: &[Value]) -> Vec<Value> {
    signals
        .iter()
        .take(AUDIT_MAX_SIGNALS)
        .filter_map(|signal| signal.as_object())
        .map(|signal| {
            json!({
                "code": normalize_code(signal.get("code").and_then(Value::as_str).unwrap_or("signal")),
                "severity": normalize_signal_severity(signal.get("severity").and_then(Value::as_str).unwrap_or("low")),
                "confidence": clamp_f64(signal.get("confidence").and_then(Value::as_f64).unwrap_or(0.0), 0.0, 1.0),
                "event_index": signal.get("event_index").and_then(Value::as_i64).unwrap_or(0).max(0),
                "event_kind": normalize_code(signal.get("event_kind").and_then(Value::as_str).unwrap_or("other")),
                "reason": truncate_chars(&redact_text(signal.get("reason").and_then(Value::as_str).unwrap_or_default()), AUDIT_MAX_REASON_CHARS),
                "evidence": truncate_chars(&redact_text(signal.get("evidence").and_then(Value::as_str).unwrap_or_default()), AUDIT_MAX_EVIDENCE_CHARS),
            })
        })
        .collect()
}

fn normalize_risk(raw: &str) -> &'static str {
    match normalize_code(raw).as_str() {
        "none" => "none",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "critical" => "critical",
        _ => "unknown",
    }
}

fn normalize_signal_severity(raw: &str) -> &'static str {
    match normalize_code(raw).as_str() {
        "medium" => "medium",
        "high" => "high",
        "critical" => "critical",
        _ => "low",
    }
}

fn normalize_code(raw: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_sep = false;
        } else if !last_was_sep && !out.is_empty() {
            out.push('_');
            last_was_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn audit_log_risk_level(normalized: &Value) -> &'static str {
    match normalized
        .get("overall_risk")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
    {
        "critical" => "critical",
        "high" => "high",
        "medium" => "medium",
        _ => "low",
    }
}

fn completed_details(
    normalized: Value,
    capture: &AssociationAuditCapture,
    config: &AssociationAuditConfig,
    build: &AuditPackageBuild,
    provider: &providers::ProviderSummary,
    started_at_ms: i64,
) -> Value {
    json!({
        "status": "completed",
        "durationMs": now_ms().saturating_sub(started_at_ms),
        "mode": config.mode.as_str(),
        "provider": provider_details(Some(provider), config),
        "request": capture_details(capture),
        "package": package_details(build),
        "result": normalized,
    })
}

fn status_details(
    status: &str,
    reason: Option<&str>,
    capture: &AssociationAuditCapture,
    config: &AssociationAuditConfig,
    build: Option<&AuditPackageBuild>,
    provider: Option<&providers::ProviderSummary>,
    started_at_ms: i64,
) -> Value {
    json!({
        "status": status,
        "reason": reason,
        "durationMs": now_ms().saturating_sub(started_at_ms),
        "mode": config.mode.as_str(),
        "provider": provider_details(provider, config),
        "request": capture_details(capture),
        "package": build.map(package_details),
    })
}

fn failed_details(
    reason: &str,
    error: &str,
    capture: &AssociationAuditCapture,
    config: &AssociationAuditConfig,
    build: Option<&AuditPackageBuild>,
    provider: Option<&providers::ProviderSummary>,
    started_at_ms: i64,
) -> Value {
    let mut details = status_details(
        "failed",
        Some(reason),
        capture,
        config,
        build,
        provider,
        started_at_ms,
    );
    if let Value::Object(map) = &mut details {
        map.insert("error".to_string(), Value::String(redact_text(error)));
    }
    details
}

fn provider_details(
    provider: Option<&providers::ProviderSummary>,
    config: &AssociationAuditConfig,
) -> Value {
    json!({
        "providerId": provider.map(|provider| provider.id).or(config.provider_id),
        "providerName": provider.map(|provider| provider.name.as_str()),
        "cliKey": provider.map(|provider| provider.cli_key.as_str()),
        "model": config.model,
    })
}

fn capture_details(capture: &AssociationAuditCapture) -> Value {
    json!({
        "traceId": capture.trace_id,
        "cliKey": capture.cli_key,
        "method": capture.method,
        "path": capture.path,
        "requestedModel": capture.requested_model,
        "status": capture.status,
        "responseTruncated": capture.response_truncated,
    })
}

fn package_details(build: &AuditPackageBuild) -> Value {
    json!({
        "inputChars": build.input_chars,
        "outputChars": build.output_chars,
        "truncated": build.package_truncated,
    })
}

fn redact_text(input: &str) -> String {
    let mut out = input.to_string();
    out = private_key_regex()
        .replace_all(&out, "[REDACTED_PRIVATE_KEY]")
        .into_owned();
    out = bearer_regex()
        .replace_all(&out, "$1[REDACTED_SECRET]")
        .into_owned();
    out = assignment_secret_regex()
        .replace_all(&out, "$1[REDACTED_SECRET]")
        .into_owned();
    out = url_credentials_regex()
        .replace_all(&out, "$1[REDACTED_CREDENTIALS]@")
        .into_owned();
    long_token_regex()
        .replace_all(&out, "[REDACTED_TOKEN]")
        .into_owned()
}

fn private_key_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?is)-----BEGIN [^-]*PRIVATE KEY-----.*?-----END [^-]*PRIVATE KEY-----")
            .expect("private key regex")
    })
}

fn bearer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(authorization\s*[:=]\s*bearer\s+)[A-Za-z0-9._~+/=-]{12,}")
            .expect("bearer regex")
    })
}

fn assignment_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b((?:api[_-]?key|token|secret|password)\s*[:=]\s*)[^\s,;\"']{8,}"#)
            .expect("assignment secret regex")
    })
}

fn url_credentials_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(https?://)[^\s/@:]+:[^\s/@]+@").expect("url credentials regex")
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

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn default_sample_rate() -> u8 {
    DEFAULT_SAMPLE_RATE
}

fn default_timeout_seconds() -> u32 {
    DEFAULT_TIMEOUT_SECONDS
}

fn default_max_input_chars() -> u32 {
    DEFAULT_MAX_INPUT_CHARS
}

fn default_max_output_chars() -> u32 {
    DEFAULT_MAX_OUTPUT_CHARS
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
        let bearer_value = "abcdefghijklmnopqrstuvwxyz123456";
        let api_key_value = format!("{}{}", "sk-", "1234567890abcdefghijklmnop");
        let private_key_marker = format!(
            "-----BEGIN {}-----\nsecret\n-----END {}-----",
            "PRIVATE KEY", "PRIVATE KEY"
        );
        let raw = format!(
            "Authorization: Bearer {bearer_value}\napi_key={api_key_value}\nhttps://user:pass@example.com/path\n{private_key_marker}"
        );

        let redacted = redact_text(&raw);

        assert!(!redacted.contains(bearer_value));
        assert!(!redacted.contains(&api_key_value));
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
    fn build_audit_package_redacts_and_truncates_payloads() {
        let capture = AssociationAuditCapture {
            trace_id: "trace-audit".to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            requested_model: Some("gpt-5".to_string()),
            request_body: "api_key=example-secret-value\nPlease summarize this.".to_string(),
            response_body: "x".repeat(1_000),
            response_truncated: false,
            status: Some(200),
            updated_at_ms: now_ms(),
        };
        let config = AssociationAuditConfig {
            max_input_chars: 256,
            max_output_chars: 256,
            ..AssociationAuditConfig::default()
        };

        let build = build_audit_package(&capture, &config);

        assert!(build.package_truncated);
        assert!(build
            .package
            .pointer("/request/body")
            .and_then(Value::as_str)
            .unwrap()
            .contains("[REDACTED_SECRET]"));
    }
}
