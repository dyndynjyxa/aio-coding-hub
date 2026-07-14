//! Usage: Provider routing helpers (gate + record circuit outcomes) for gateway proxy.

use crate::circuit_breaker;
use crate::gateway::events::{emit_circuit_event, emit_circuit_transition, GatewayCircuitEvent};

pub(super) struct GateProviderArgs<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) app: Option<&'a tauri::AppHandle<R>>,
    pub(super) circuit: &'a circuit_breaker::CircuitBreaker,
    pub(super) trace_id: &'a str,
    pub(super) cli_key: &'a str,
    pub(super) provider_id: i64,
    pub(super) provider_name: &'a str,
    pub(super) provider_base_url_display: &'a str,
    pub(super) now_unix: i64,
    pub(super) earliest_available_unix: &'a mut Option<i64>,
    pub(super) skipped_open: &'a mut usize,
    pub(super) skipped_cooldown: &'a mut usize,
    /// Filled with the circuit snapshot when the gate denies, so callers can
    /// attach circuit attribution to the skipped attempt.
    pub(super) deny_snapshot: &'a mut Option<circuit_breaker::CircuitSnapshot>,
}

pub(super) fn gate_provider<R: tauri::Runtime>(
    args: GateProviderArgs<'_, R>,
) -> Option<circuit_breaker::CircuitSnapshot> {
    let GateProviderArgs {
        app,
        circuit,
        trace_id,
        cli_key,
        provider_id,
        provider_name,
        provider_base_url_display,
        now_unix,
        earliest_available_unix,
        skipped_open,
        skipped_cooldown,
        deny_snapshot,
    } = args;

    let allow = circuit.should_allow(provider_id, now_unix);
    if let (Some(app), Some(t)) = (app, allow.transition.as_ref()) {
        emit_circuit_transition(
            app,
            trace_id,
            cli_key,
            provider_id,
            provider_name,
            provider_base_url_display,
            t,
            now_unix,
            None,
            None,
        );
    }

    if allow.allow {
        return Some(allow.after.clone());
    }

    let snap = allow.after;
    *deny_snapshot = Some(snap.clone());
    let reason = match snap.state {
        circuit_breaker::CircuitState::Open => {
            *skipped_open = skipped_open.saturating_add(1);
            "SKIP_OPEN"
        }
        _ => {
            // Cooldown active (state is Closed or HalfOpen but cooldown_until is set)
            *skipped_cooldown = skipped_cooldown.saturating_add(1);
            "SKIP_COOLDOWN"
        }
    };

    if let Some(until) = snap.cooldown_until.or(snap.open_until) {
        if until > now_unix {
            *earliest_available_unix = Some(match *earliest_available_unix {
                Some(cur) => cur.min(until),
                None => until,
            });
        }
    }

    if let Some(app) = app {
        emit_circuit_event(
            app,
            GatewayCircuitEvent {
                trace_id: trace_id.to_string(),
                cli_key: cli_key.to_string(),
                provider_id,
                provider_name: provider_name.to_string(),
                base_url: provider_base_url_display.to_string(),
                prev_state: snap.state.as_str(),
                next_state: snap.state.as_str(),
                failure_count: snap.failure_count,
                failure_threshold: snap.failure_threshold,
                open_until: snap.open_until,
                cooldown_until: snap.cooldown_until,
                reason,
                ts: now_unix,
                // Non-transition skip event: no trigger-failure attribution.
                trigger_error_code: None,
                first_byte_timeout_secs: None,
            },
        );
    }

    None
}

pub(in crate::gateway) struct RecordCircuitArgs<'a, R: tauri::Runtime = tauri::Wry> {
    pub(in crate::gateway) app: Option<&'a tauri::AppHandle<R>>,
    pub(in crate::gateway) circuit: &'a circuit_breaker::CircuitBreaker,
    pub(in crate::gateway) trace_id: &'a str,
    pub(in crate::gateway) cli_key: &'a str,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) provider_name: &'a str,
    pub(in crate::gateway) provider_base_url: &'a str,
    pub(in crate::gateway) now_unix: i64,
    /// Error code of the failure being recorded; feeds the "触发失败" line of
    /// the circuit-open notice. `None` for call sites without attribution.
    pub(in crate::gateway) trigger_error_code: Option<&'static str>,
    /// Effective first-byte timeout (seconds); the notice builder only uses it
    /// when `trigger_error_code` is `GW_UPSTREAM_TIMEOUT`.
    pub(in crate::gateway) first_byte_timeout_secs: Option<u32>,
    /// Trusted background requests still route normally but must not mutate
    /// circuit success, failure, or cooldown state.
    pub(in crate::gateway) provider_health_neutral: bool,
}

impl<'a, R: tauri::Runtime> RecordCircuitArgs<'a, R> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gateway) fn new(
        app: Option<&'a tauri::AppHandle<R>>,
        circuit: &'a circuit_breaker::CircuitBreaker,
        trace_id: &'a str,
        cli_key: &'a str,
        provider_id: i64,
        provider_name: &'a str,
        provider_base_url: &'a str,
        now_unix: i64,
    ) -> Self {
        Self {
            app,
            circuit,
            trace_id,
            cli_key,
            provider_id,
            provider_name,
            provider_base_url,
            now_unix,
            trigger_error_code: None,
            first_byte_timeout_secs: None,
            provider_health_neutral: false,
        }
    }

    pub(in crate::gateway) fn with_trigger(
        mut self,
        trigger_error_code: Option<&'static str>,
        first_byte_timeout_secs: Option<u32>,
    ) -> Self {
        self.trigger_error_code = trigger_error_code;
        self.first_byte_timeout_secs = first_byte_timeout_secs;
        self
    }

    pub(in crate::gateway) fn with_provider_health_neutral(
        mut self,
        provider_health_neutral: bool,
    ) -> Self {
        self.provider_health_neutral = provider_health_neutral;
        self
    }
}

impl<'a, R: tauri::Runtime> RecordCircuitArgs<'a, R> {
    pub(in crate::gateway) fn from_state(
        state: &'a crate::gateway::runtime::GatewayAppState<R>,
        trace_id: &'a str,
        cli_key: &'a str,
        provider_id: i64,
        provider_name: &'a str,
        provider_base_url: &'a str,
        now_unix: i64,
    ) -> Self {
        Self::new(
            Some(&state.app),
            state.circuit.as_ref(),
            trace_id,
            cli_key,
            provider_id,
            provider_name,
            provider_base_url,
            now_unix,
        )
    }

    pub(in crate::gateway) fn from_stream_ctx(
        ctx: &'a crate::gateway::streams::StreamFinalizeCtx<R>,
        now_unix: i64,
    ) -> Self {
        Self::new(
            Some(&ctx.app),
            ctx.circuit.as_ref(),
            ctx.trace_id.as_str(),
            ctx.cli_key.as_str(),
            ctx.provider_id,
            ctx.provider_name.as_str(),
            ctx.base_url.as_str(),
            now_unix,
        )
        .with_provider_health_neutral(ctx.provider_health_neutral)
    }
}

fn unchanged_circuit_change(
    circuit: &circuit_breaker::CircuitBreaker,
    provider_id: i64,
    now_unix: i64,
) -> circuit_breaker::CircuitChange {
    let snapshot = circuit.snapshot(provider_id, now_unix);
    circuit_breaker::CircuitChange {
        before: snapshot.clone(),
        after: snapshot,
        transition: None,
    }
}

pub(in crate::gateway) fn record_success_and_emit_transition(
    args: RecordCircuitArgs<'_, impl tauri::Runtime>,
) -> circuit_breaker::CircuitChange {
    let RecordCircuitArgs {
        app,
        circuit,
        trace_id,
        cli_key,
        provider_id,
        provider_name,
        provider_base_url,
        now_unix,
        provider_health_neutral,
        ..
    } = args;

    let change = if provider_health_neutral {
        unchanged_circuit_change(circuit, provider_id, now_unix)
    } else {
        circuit.record_success(provider_id, now_unix)
    };
    if let (Some(app), Some(t)) = (app, change.transition.as_ref()) {
        emit_circuit_transition(
            app,
            trace_id,
            cli_key,
            provider_id,
            provider_name,
            provider_base_url,
            t,
            now_unix,
            None,
            None,
        );
    }
    change
}

pub(in crate::gateway) fn record_failure_and_emit_transition(
    args: RecordCircuitArgs<'_, impl tauri::Runtime>,
) -> circuit_breaker::CircuitChange {
    let RecordCircuitArgs {
        app,
        circuit,
        trace_id,
        cli_key,
        provider_id,
        provider_name,
        provider_base_url,
        now_unix,
        trigger_error_code,
        first_byte_timeout_secs,
        provider_health_neutral,
    } = args;

    let change = if provider_health_neutral {
        unchanged_circuit_change(circuit, provider_id, now_unix)
    } else {
        circuit.record_failure(provider_id, now_unix, trigger_error_code)
    };
    if let (Some(app), Some(t)) = (app, change.transition.as_ref()) {
        emit_circuit_transition(
            app,
            trace_id,
            cli_key,
            provider_id,
            provider_name,
            provider_base_url,
            t,
            now_unix,
            trigger_error_code,
            first_byte_timeout_secs,
        );
    }
    change
}

pub(in crate::gateway) fn trigger_cooldown(
    circuit: &circuit_breaker::CircuitBreaker,
    provider_id: i64,
    now_unix: i64,
    cooldown_secs: i64,
    provider_health_neutral: bool,
) -> circuit_breaker::CircuitSnapshot {
    if provider_health_neutral {
        circuit.snapshot(provider_id, now_unix)
    } else {
        circuit.trigger_cooldown(provider_id, now_unix, cooldown_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    type TestGateProviderArgs<'a> = GateProviderArgs<'a, tauri::Wry>;
    type TestRecordCircuitArgs<'a> = RecordCircuitArgs<'a, tauri::Wry>;

    fn breaker(config: circuit_breaker::CircuitBreakerConfig) -> circuit_breaker::CircuitBreaker {
        circuit_breaker::CircuitBreaker::new(config, HashMap::new(), None)
    }

    #[test]
    fn gate_provider_allows_closed() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;

        let mut earliest: Option<i64> = None;
        let mut skipped_open = 0usize;
        let mut skipped_cooldown = 0usize;
        let mut deny_snapshot = None;

        let snap = gate_provider(TestGateProviderArgs {
            app: None,
            circuit: &cb,
            trace_id: "t",
            cli_key: "claude",
            provider_id: pid,
            provider_name: "p1",
            provider_base_url_display: "https://example.invalid",
            now_unix: now,
            earliest_available_unix: &mut earliest,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
            deny_snapshot: &mut deny_snapshot,
        })
        .expect("should allow");

        assert_eq!(snap.state, circuit_breaker::CircuitState::Closed);
        assert_eq!(earliest, None);
        assert_eq!(skipped_open, 0);
        assert_eq!(skipped_cooldown, 0);
    }

    #[test]
    fn gate_provider_skips_open_and_updates_earliest() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;

        cb.record_failure(pid, now, None);
        let open = cb.snapshot(pid, now);
        assert_eq!(open.state, circuit_breaker::CircuitState::Open);
        let open_until = open.open_until.expect("open_until");

        let mut earliest: Option<i64> = None;
        let mut skipped_open = 0usize;
        let mut skipped_cooldown = 0usize;
        let mut deny_snapshot = None;

        let allowed = gate_provider(TestGateProviderArgs {
            app: None,
            circuit: &cb,
            trace_id: "t",
            cli_key: "claude",
            provider_id: pid,
            provider_name: "p1",
            provider_base_url_display: "https://example.invalid",
            now_unix: now,
            earliest_available_unix: &mut earliest,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
            deny_snapshot: &mut deny_snapshot,
        });

        assert!(allowed.is_none());
        assert_eq!(earliest, Some(open_until));
        assert_eq!(skipped_open, 1);
        assert_eq!(skipped_cooldown, 0);
    }

    #[test]
    fn gate_provider_skips_cooldown_and_updates_earliest() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;
        let cooldown_until = cb
            .trigger_cooldown(pid, now, 60)
            .cooldown_until
            .expect("cooldown");

        let mut earliest: Option<i64> = None;
        let mut skipped_open = 0usize;
        let mut skipped_cooldown = 0usize;
        let mut deny_snapshot = None;

        let allowed = gate_provider(TestGateProviderArgs {
            app: None,
            circuit: &cb,
            trace_id: "t",
            cli_key: "claude",
            provider_id: pid,
            provider_name: "p1",
            provider_base_url_display: "https://example.invalid",
            now_unix: now,
            earliest_available_unix: &mut earliest,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
            deny_snapshot: &mut deny_snapshot,
        });

        assert!(allowed.is_none());
        assert_eq!(earliest, Some(cooldown_until));
        assert_eq!(skipped_open, 0);
        assert_eq!(skipped_cooldown, 1);
    }

    #[test]
    fn gate_provider_allows_when_open_expires() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration_secs: 10,
        });
        let pid = 1;
        let now = 1_000;
        cb.record_failure(pid, now, None);
        let open_until = cb.snapshot(pid, now).open_until.expect("open_until");

        let mut earliest: Option<i64> = None;
        let mut skipped_open = 0usize;
        let mut skipped_cooldown = 0usize;
        let mut deny_snapshot = None;

        let snap = gate_provider(TestGateProviderArgs {
            app: None,
            circuit: &cb,
            trace_id: "t",
            cli_key: "claude",
            provider_id: pid,
            provider_name: "p1",
            provider_base_url_display: "https://example.invalid",
            now_unix: open_until,
            earliest_available_unix: &mut earliest,
            skipped_open: &mut skipped_open,
            skipped_cooldown: &mut skipped_cooldown,
            deny_snapshot: &mut deny_snapshot,
        })
        .expect("should allow after expiry");

        // After open expires, circuit transitions to HalfOpen (probe state)
        assert_eq!(snap.state, circuit_breaker::CircuitState::HalfOpen);
        assert_eq!(earliest, None);
        assert_eq!(skipped_open, 0);
        assert_eq!(skipped_cooldown, 0);
    }

    #[test]
    fn record_failure_reports_open_transition_when_threshold_reached() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;

        let change = record_failure_and_emit_transition(TestRecordCircuitArgs::new(
            None,
            &cb,
            "t",
            "claude",
            pid,
            "p1",
            "https://example.invalid",
            now,
        ));

        assert_eq!(change.after.state, circuit_breaker::CircuitState::Open);
        assert!(change.transition.is_some());
    }

    #[test]
    fn record_circuit_args_default_to_no_trigger_and_with_trigger_sets_fields() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            open_duration_secs: 60,
        });

        let args = TestRecordCircuitArgs::new(
            None,
            &cb,
            "t",
            "claude",
            1,
            "p1",
            "https://example.invalid",
            1_000,
        );
        assert_eq!(args.trigger_error_code, None);
        assert_eq!(args.first_byte_timeout_secs, None);
        assert!(!args.provider_health_neutral);

        let args = args.with_trigger(Some("GW_UPSTREAM_TIMEOUT"), Some(300));
        assert_eq!(args.trigger_error_code, Some("GW_UPSTREAM_TIMEOUT"));
        assert_eq!(args.first_byte_timeout_secs, Some(300));

        let args = args.with_provider_health_neutral(true);
        assert!(args.provider_health_neutral);
    }

    #[test]
    fn record_success_clears_failure_count() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 5,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;
        cb.record_failure(pid, now, None);
        assert!(cb.snapshot(pid, now).failure_count > 0);

        let change = record_success_and_emit_transition(TestRecordCircuitArgs::new(
            None,
            &cb,
            "t",
            "claude",
            pid,
            "p1",
            "https://example.invalid",
            now + 1,
        ));

        assert_eq!(change.after.failure_count, 0);
    }

    #[test]
    fn provider_health_neutral_outcomes_do_not_mutate_circuit() {
        let cb = breaker(circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 2,
            open_duration_secs: 60,
        });
        let pid = 1;
        let now = 1_000;
        cb.record_failure(pid, now, None);

        let neutral_failure = record_failure_and_emit_transition(
            TestRecordCircuitArgs::new(
                None,
                &cb,
                "system-request",
                "codex",
                pid,
                "p1",
                "https://example.invalid",
                now + 1,
            )
            .with_provider_health_neutral(true),
        );
        assert_eq!(
            neutral_failure.after.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(neutral_failure.after.failure_count, 1);
        assert!(neutral_failure.transition.is_none());

        let neutral_success = record_success_and_emit_transition(
            TestRecordCircuitArgs::new(
                None,
                &cb,
                "system-request",
                "codex",
                pid,
                "p1",
                "https://example.invalid",
                now + 2,
            )
            .with_provider_health_neutral(true),
        );
        assert_eq!(
            neutral_success.after.state,
            circuit_breaker::CircuitState::Closed
        );
        assert_eq!(neutral_success.after.failure_count, 1);
        assert!(neutral_success.transition.is_none());

        let neutral_cooldown = trigger_cooldown(&cb, pid, now + 3, 60, true);
        assert_eq!(neutral_cooldown.failure_count, 1);
        assert_eq!(neutral_cooldown.cooldown_until, None);
    }
}
