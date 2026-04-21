//! Usage: Claude model mapping application for a provider attempt.

use super::context::{CommonCtx, ProviderCtx};
use crate::gateway::proxy::model_rewrite::{
    replace_model_in_body_json, replace_model_in_path, replace_model_in_query,
};
use crate::gateway::util::RequestedModelLocation;
use crate::providers;
use crate::shared::mutex_ext::MutexExt;
use axum::body::Bytes;

pub(super) struct UpstreamRequestMut<'a> {
    pub(super) forwarded_path: &'a mut String,
    pub(super) query: &'a mut Option<String>,
    pub(super) body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
}

pub(super) fn apply_if_needed(
    ctx: CommonCtx<'_>,
    provider: &providers::ProviderForGateway,
    provider_ctx: ProviderCtx<'_>,
    requested_model_location: Option<RequestedModelLocation>,
    introspection_json: Option<&serde_json::Value>,
    upstream: UpstreamRequestMut<'_>,
) {
    if ctx.cli_key != "claude" || !provider.claude_models.has_any() {
        return;
    }

    let Some(requested_model) = ctx.requested_model.as_deref() else {
        return;
    };

    let has_thinking = introspection_json
        .and_then(|v| v.get("thinking"))
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str())
        == Some("enabled");

    let effective_model = provider.get_effective_claude_model(requested_model, has_thinking);
    if effective_model == requested_model {
        return;
    }

    let UpstreamRequestMut {
        forwarded_path,
        query,
        body_bytes,
        strip_request_content_encoding,
    } = upstream;

    let location = requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
    let mut applied = false;
    match location {
        RequestedModelLocation::BodyJson => {
            if let Some(root) = introspection_json {
                let mut next = root.clone();
                let replaced = replace_model_in_body_json(&mut next, &effective_model);
                if replaced {
                    if let Ok(bytes) = serde_json::to_vec(&next) {
                        *body_bytes = Bytes::from(bytes);
                        *strip_request_content_encoding = true;
                        applied = true;
                    }
                }
            }
        }
        RequestedModelLocation::Query => {
            if let Some(q) = query.as_deref() {
                let next = replace_model_in_query(q, &effective_model);
                applied = next != q;
                *query = Some(next);
            }
        }
        RequestedModelLocation::Path => {
            if let Some(next_path) = replace_model_in_path(forwarded_path, &effective_model) {
                applied = next_path != *forwarded_path;
                *forwarded_path = next_path;
            }
        }
    }

    let model_lower = requested_model.to_ascii_lowercase();
    let kind = if has_thinking
        && provider
            .claude_models
            .reasoning_model
            .as_deref()
            .is_some_and(|v| v == effective_model.as_str())
    {
        "reasoning"
    } else if model_lower.contains("haiku")
        && provider
            .claude_models
            .haiku_model
            .as_deref()
            .is_some_and(|v| v == effective_model.as_str())
    {
        "haiku"
    } else if model_lower.contains("sonnet")
        && provider
            .claude_models
            .sonnet_model
            .as_deref()
            .is_some_and(|v| v == effective_model.as_str())
    {
        "sonnet"
    } else if model_lower.contains("opus")
        && provider
            .claude_models
            .opus_model
            .as_deref()
            .is_some_and(|v| v == effective_model.as_str())
    {
        "opus"
    } else {
        "main"
    };

    let ProviderCtx {
        provider_id,
        provider_name_base,
        ..
    } = provider_ctx;

    let mut settings = ctx.special_settings.lock_or_recover();
    settings.push(serde_json::json!({
        "type": "claude_model_mapping",
        "scope": "attempt",
        "hit": true,
        "applied": applied,
        "providerId": provider_id,
        "providerName": provider_name_base.clone(),
        "requestedModel": requested_model,
        "effectiveModel": effective_model,
        "mappingKind": kind,
        "hasThinking": has_thinking,
        "location": match location {
            RequestedModelLocation::BodyJson => "body",
            RequestedModelLocation::Query => "query",
            RequestedModelLocation::Path => "path",
        },
    }));
}
