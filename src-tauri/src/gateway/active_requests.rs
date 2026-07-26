use crate::gateway::events::GatewayAttemptEvent;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveRequestStart {
    pub(crate) trace_id: String,
    pub(crate) cli_key: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) requested_model: Option<String>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub(crate) struct ActiveRequestSnapshotItem {
    pub trace_id: String,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub session_id: Option<String>,
    pub requested_model: Option<String>,
    pub created_at_ms: i64,
    pub last_activity_ms: i64,
    pub current_attempt: Option<GatewayAttemptEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveRequestFinishReason {
    Completed,
    Failed,
    ClientAborted,
    GatewayStopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveRequestEntry {
    start: ActiveRequestStart,
    last_activity_ms: i64,
    current_attempt: Option<GatewayAttemptEvent>,
}

#[derive(Debug, Default)]
pub(crate) struct ActiveRequestRegistry {
    entries: RwLock<HashMap<String, ActiveRequestEntry>>,
}

impl ActiveRequestRegistry {
    pub(crate) fn register(&self, start: ActiveRequestStart) {
        let last_activity_ms = start.created_at_ms.max(0);
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(
                start.trace_id.clone(),
                ActiveRequestEntry {
                    start,
                    last_activity_ms,
                    current_attempt: None,
                },
            );
        }
    }

    pub(crate) fn touch(&self, trace_id: &str, last_activity_ms: i64) {
        if let Ok(mut entries) = self.entries.write() {
            if let Some(entry) = entries.get_mut(trace_id) {
                entry.last_activity_ms = entry.last_activity_ms.max(last_activity_ms.max(0));
            }
        }
    }

    pub(crate) fn record_attempt_start(&self, attempt: GatewayAttemptEvent, last_activity_ms: i64) {
        if let Ok(mut entries) = self.entries.write() {
            let Some(entry) = entries.get_mut(&attempt.trace_id) else {
                return;
            };
            if entry
                .current_attempt
                .as_ref()
                .is_some_and(|current| current.attempt_index > attempt.attempt_index)
            {
                return;
            }
            entry.last_activity_ms = entry.last_activity_ms.max(last_activity_ms.max(0));
            entry.current_attempt = Some(attempt);
        }
    }

    pub(crate) fn finish(
        &self,
        trace_id: &str,
        _reason: ActiveRequestFinishReason,
    ) -> Option<ActiveRequestSnapshotItem> {
        self.entries
            .write()
            .ok()
            .and_then(|mut entries| entries.remove(trace_id))
            .map(ActiveRequestEntry::into_snapshot)
    }

    pub(crate) fn finish_all(
        &self,
        _reason: ActiveRequestFinishReason,
    ) -> Vec<ActiveRequestSnapshotItem> {
        let Ok(mut entries) = self.entries.write() else {
            return Vec::new();
        };
        let mut rows: Vec<_> = entries
            .drain()
            .map(|(_, entry)| entry.into_snapshot())
            .collect();
        sort_snapshot_items(&mut rows);
        rows
    }

    pub(crate) fn snapshot(&self) -> Vec<ActiveRequestSnapshotItem> {
        let Ok(entries) = self.entries.read() else {
            return Vec::new();
        };
        let mut rows: Vec<_> = entries
            .values()
            .cloned()
            .map(ActiveRequestEntry::into_snapshot)
            .collect();
        sort_snapshot_items(&mut rows);
        rows
    }
}

impl ActiveRequestEntry {
    fn into_snapshot(self) -> ActiveRequestSnapshotItem {
        ActiveRequestSnapshotItem {
            trace_id: self.start.trace_id,
            cli_key: self.start.cli_key,
            method: self.start.method,
            path: self.start.path,
            query: self.start.query,
            session_id: self.start.session_id,
            requested_model: self.start.requested_model,
            created_at_ms: self.start.created_at_ms,
            last_activity_ms: self.last_activity_ms,
            current_attempt: self.current_attempt,
        }
    }
}

fn sort_snapshot_items(rows: &mut [ActiveRequestSnapshotItem]) {
    rows.sort_by(|a, b| {
        b.created_at_ms
            .cmp(&a.created_at_ms)
            .then_with(|| b.trace_id.cmp(&a.trace_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::events::GatewayAttemptEvent;

    fn active_request_start(trace_id: &str) -> ActiveRequestStart {
        ActiveRequestStart {
            trace_id: trace_id.to_string(),
            cli_key: "claude".to_string(),
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            query: None,
            session_id: None,
            requested_model: Some("claude-sonnet-4".to_string()),
            created_at_ms: 1_000,
        }
    }

    fn gateway_attempt(
        trace_id: &str,
        attempt_index: u32,
        provider_name: &str,
    ) -> GatewayAttemptEvent {
        GatewayAttemptEvent {
            trace_id: trace_id.to_string(),
            cli_key: "claude".to_string(),
            session_id: None,
            method: "POST".to_string(),
            path: "/v1/messages".to_string(),
            query: None,
            requested_model: Some("claude-sonnet-4".to_string()),
            attempt_index,
            provider_id: i64::from(attempt_index),
            session_reuse: None,
            provider_name: provider_name.to_string(),
            base_url: "https://provider.example".to_string(),
            outcome: "started".to_string(),
            status: None,
            attempt_started_ms: u128::from(attempt_index) * 100,
            attempt_duration_ms: 0,
            circuit_state_before: Some("CLOSED"),
            circuit_state_after: None,
            circuit_failure_count: Some(0),
            circuit_failure_threshold: Some(3),
            claude_model_mapping: None,
        }
    }

    #[test]
    fn registry_register_touch_finish_lifecycle() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-active"));

        assert_eq!(registry.snapshot().len(), 1);
        registry.touch("trace-active", 2_000);
        assert_eq!(registry.snapshot()[0].last_activity_ms, 2_000);

        assert!(registry
            .finish("trace-active", ActiveRequestFinishReason::Completed)
            .is_some());
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn registry_finish_is_idempotent() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-once"));

        assert!(registry
            .finish("trace-once", ActiveRequestFinishReason::Completed)
            .is_some());
        assert!(registry
            .finish("trace-once", ActiveRequestFinishReason::Completed)
            .is_none());
    }

    #[test]
    fn registry_finish_all_clears_entries() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-a"));
        registry.register(active_request_start("trace-b"));
        registry.record_attempt_start(gateway_attempt("trace-a", 1, "Provider A"), 1_100);

        let removed = registry.finish_all(ActiveRequestFinishReason::GatewayStopped);

        assert_eq!(removed.len(), 2);
        assert!(removed.iter().any(|row| row.current_attempt.is_some()));
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn registry_snapshot_replays_latest_attempt() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-progress"));
        let attempt = gateway_attempt("trace-progress", 1, "Provider A");

        registry.record_attempt_start(attempt.clone(), 1_100);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].current_attempt, Some(attempt));
        assert_eq!(snapshot[0].last_activity_ms, 1_100);
    }

    #[test]
    fn registry_older_attempt_does_not_replace_newer_progress() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-monotonic"));
        let newer = gateway_attempt("trace-monotonic", 2, "Provider B");

        registry.record_attempt_start(newer.clone(), 1_200);
        registry.record_attempt_start(gateway_attempt("trace-monotonic", 1, "Provider A"), 1_300);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].current_attempt, Some(newer));
        assert_eq!(snapshot[0].last_activity_ms, 1_200);
    }

    #[test]
    fn registry_same_attempt_index_refreshes_progress() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-refresh"));
        registry.record_attempt_start(
            gateway_attempt("trace-refresh", 1, "Provider Before"),
            1_100,
        );
        let refreshed = gateway_attempt("trace-refresh", 1, "Provider After");

        registry.record_attempt_start(refreshed.clone(), 1_200);

        let snapshot = registry.snapshot();
        assert_eq!(snapshot[0].current_attempt, Some(refreshed));
        assert_eq!(snapshot[0].last_activity_ms, 1_200);
    }

    #[test]
    fn registry_finish_clears_attempt_progress() {
        let registry = ActiveRequestRegistry::default();
        registry.register(active_request_start("trace-finished"));
        let attempt = gateway_attempt("trace-finished", 1, "Provider A");
        registry.record_attempt_start(attempt.clone(), 1_100);

        let finished = registry
            .finish("trace-finished", ActiveRequestFinishReason::Completed)
            .expect("active request should be removed");

        assert_eq!(finished.current_attempt, Some(attempt));
        assert!(registry.snapshot().is_empty());
    }
}
