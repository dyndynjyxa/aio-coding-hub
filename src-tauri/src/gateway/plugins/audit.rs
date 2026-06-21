//! Usage: Best-effort persistence for gateway plugin hook audit events.

use super::permissions::GatewayPluginError;
use super::pipeline::GatewayPluginAuditEvent;
use crate::infra::plugins::repository::{
    self, AppendPluginAuditLogInput, RecordPluginRuntimeFailureInput,
};

pub(crate) fn persist_gateway_plugin_error_audit_events(
    db: &crate::db::Db,
    trace_id: &str,
    err: &mut GatewayPluginError,
) {
    let events = err.take_audit_events();
    if events.is_empty() {
        return;
    }
    persist_gateway_plugin_audit_events(db, trace_id, events);
}

pub(crate) fn persist_gateway_plugin_audit_events(
    db: &crate::db::Db,
    trace_id: &str,
    events: Vec<GatewayPluginAuditEvent>,
) {
    for event in events {
        if let Err(err) = repository::append_audit_log(
            db,
            AppendPluginAuditLogInput {
                plugin_id: Some(event.plugin_id.clone()),
                trace_id: Some(trace_id.to_string()),
                event_type: event.event_type.clone(),
                risk_level: event.risk_level,
                message: event.message.clone(),
                details: event.details.clone(),
            },
        ) {
            tracing::warn!(
                plugin_id = %event.plugin_id,
                hook_name = %event.hook_name,
                error = %err,
                "failed to persist gateway plugin audit event"
            );
        }

        if event.event_type == "plugin.hook.failed" {
            let failure_kind = event
                .details
                .get("failureKind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("hook_error")
                .to_string();
            if let Err(err) = repository::record_runtime_failure(
                db,
                RecordPluginRuntimeFailureInput {
                    plugin_id: event.plugin_id.clone(),
                    hook_name: Some(event.hook_name.clone()),
                    failure_kind,
                    message: event.message,
                    trace_id: Some(trace_id.to_string()),
                },
            ) {
                tracing::warn!(
                    plugin_id = %event.plugin_id,
                    hook_name = %event.hook_name,
                    error = %err,
                    "failed to persist gateway plugin runtime failure"
                );
            }
        }
    }
}
