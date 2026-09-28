//! Usage: Apply a ready Provider model policy to one upstream attempt.

use super::context::{CommonCtx, ProviderCtx};
use crate::gateway::events::ModelRedirect;
use crate::gateway::proxy::model_rewrite::rewrite_model_in_request;
use crate::gateway::util::RequestedModelLocation;
use crate::providers;
use axum::body::Bytes;

pub(super) struct UpstreamRequestMut<'a> {
    pub(super) forwarded_path: &'a mut String,
    pub(super) query: &'a mut Option<String>,
    pub(super) body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
}

pub(super) fn resolve_target_model(
    provider: &providers::ProviderForGateway,
    requested_model: Option<&str>,
) -> Option<String> {
    if provider.model_policy_status != providers::ProviderModelPolicyStatus::Ready {
        return None;
    }
    let requested_model = requested_model?;
    let effective_model = provider
        .model_policy
        .as_ref()?
        .resolve_mapping(requested_model);
    (effective_model != requested_model).then_some(effective_model)
}

pub(super) fn apply_if_needed<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    provider_ctx: ProviderCtx<'_>,
    requested_model_location: Option<RequestedModelLocation>,
    effective_model: Option<&str>,
    model_already_applied: bool,
    upstream: UpstreamRequestMut<'_>,
) -> Option<ModelRedirect> {
    let requested_model = ctx.requested_model.as_deref()?;
    let effective_model = effective_model?;

    let UpstreamRequestMut {
        forwarded_path,
        query,
        body_bytes,
        strip_request_content_encoding,
    } = upstream;
    if !model_already_applied {
        let location = requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
        if !rewrite_model_in_request(
            location,
            effective_model,
            forwarded_path,
            query,
            body_bytes,
            strip_request_content_encoding,
        ) {
            return None;
        }
    }

    // cx2cc bridges never reach this point (their policy_target_model is None),
    // so the stage is always "provider" here.
    let stage = "provider";
    let redirect = ModelRedirect {
        stage: stage.to_string(),
        provider_id: provider_ctx.provider_id,
        provider_name: provider_ctx.provider_name_base.clone(),
        source_model: requested_model.to_string(),
        target_model: effective_model.to_string(),
    };
    crate::gateway::response_fixer::push_special_setting(
        ctx.special_settings,
        serde_json::json!({
            "type": "model_redirect",
            "scope": "attempt",
            "hit": true,
            "stage": stage,
            "providerId": provider_ctx.provider_id,
            "providerName": provider_ctx.provider_name_base,
            "sourceModel": requested_model,
            "targetModel": effective_model,
        }),
    );
    Some(redirect)
}
