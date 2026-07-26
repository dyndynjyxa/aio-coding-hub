//! Usage: Best-effort persistence for gateway plugin hook audit events.

use super::permissions::GatewayPluginError;
use super::pipeline::{GatewayPluginAuditEvent, GatewayPluginHookExecutionReport};
use crate::infra::plugins::repository::{
    self, AppendPluginAuditLogInput, RecordPluginRuntimeFailureInput,
};
use crate::infra::plugins::runtime_reports::{self, RecordPluginHookExecutionReportInput};

pub(crate) fn persist_gateway_plugin_error_audit_events(
    db: &crate::db::Db,
    trace_id: &str,
    err: &mut GatewayPluginError,
) {
    let events = err.take_audit_events();
    let reports = err.take_execution_reports();
    persist_gateway_plugin_diagnostics(db, trace_id, events, reports);
}

pub(crate) fn persist_gateway_plugin_diagnostics(
    db: &crate::db::Db,
    trace_id: &str,
    events: Vec<GatewayPluginAuditEvent>,
    reports: Vec<GatewayPluginHookExecutionReport>,
) {
    if events.is_empty() && reports.is_empty() {
        return;
    }

    // Callers sit on the request/stream hot path; run the rusqlite writes on the
    // bounded blocking pool instead of the async worker. Best-effort: failures are
    // logged inside the blocking body, join errors by blocking::run itself.
    let db = db.clone();
    let trace_id = trace_id.to_string();
    tauri::async_runtime::spawn(async move {
        let _ = crate::blocking::run("gateway_plugin_audit_persist", move || {
            persist_gateway_plugin_diagnostics_blocking(&db, &trace_id, events, reports);
            Ok::<_, crate::shared::error::AppError>(())
        })
        .await;
    });
}

fn persist_gateway_plugin_diagnostics_blocking(
    db: &crate::db::Db,
    trace_id: &str,
    events: Vec<GatewayPluginAuditEvent>,
    reports: Vec<GatewayPluginHookExecutionReport>,
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

    for report in reports {
        if let Err(err) = runtime_reports::record_hook_execution_report(
            db,
            RecordPluginHookExecutionReportInput {
                plugin_id: report.plugin_id.clone(),
                trace_id: Some(trace_id.to_string()),
                hook_name: report.hook_name.clone(),
                runtime_kind: report.runtime_kind,
                status: report.status,
                started_at_ms: report.started_at_ms,
                duration_ms: report.duration_ms,
                failure_kind: report.failure_kind,
                error_code: report.error_code,
                failure_policy: report.failure_policy,
                circuit_state: report.circuit_state,
                context_budget_json: report.context_budget,
                output_budget_json: report.output_budget,
                mutation_summary_json: report.mutation_summary,
                replayable: report.replayable,
                replay_export_reason: report.replay_export_reason,
            },
        ) {
            tracing::warn!(
                plugin_id = %report.plugin_id,
                hook_name = %report.hook_name,
                error = %err,
                "failed to persist gateway plugin hook execution report"
            );
        }
    }
}
