use super::types::*;
use super::*;
use types::MAX_FAILURE_TIMESTAMPS;

fn breaker() -> CircuitBreaker {
    CircuitBreaker::new(CircuitBreakerConfig::default(), HashMap::new(), None)
}

#[test]
fn closed_to_open_after_threshold() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        let change = cb.record_failure(pid, now + i as i64, None);
        if i < DEFAULT_FAILURE_THRESHOLD {
            assert_eq!(change.after.state, CircuitState::Closed);
        }
    }

    let snap = cb.snapshot(pid, now + 100);
    assert_eq!(snap.state, CircuitState::Open);
    assert!(snap.open_until.is_some());
}

#[test]
fn open_expires_to_half_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let snap = cb.snapshot(pid, now + 10);
    assert_eq!(snap.state, CircuitState::Open);
    let open_until = snap.open_until.expect("open_until");

    let check = cb.should_allow(pid, open_until);
    assert!(check.allow);
    assert_eq!(check.after.state, CircuitState::HalfOpen);
    assert!(check.transition.is_some());
    let t = check.transition.unwrap();
    assert_eq!(t.prev_state, CircuitState::Open);
    assert_eq!(t.next_state, CircuitState::HalfOpen);
    assert_eq!(t.reason, "OPEN_EXPIRED");
}

#[test]
fn half_open_one_success_stays_half_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    let change = cb.record_success(pid, open_until + 1);
    assert_eq!(change.after.state, CircuitState::HalfOpen);
    assert!(change.transition.is_none());
}

#[test]
fn half_open_two_successes_stays_half_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    cb.record_success(pid, open_until + 1);
    let change = cb.record_success(pid, open_until + 2);
    assert_eq!(change.after.state, CircuitState::HalfOpen);
    assert!(change.transition.is_none());
}

#[test]
fn half_open_three_successes_transitions_to_closed() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    cb.record_success(pid, open_until + 1);
    cb.record_success(pid, open_until + 2);
    let change = cb.record_success(pid, open_until + 3);
    assert_eq!(change.after.state, CircuitState::Closed);
    assert_eq!(change.after.failure_count, 0);
    assert!(change.transition.is_some());
    let t = change.transition.unwrap();
    assert_eq!(t.prev_state, CircuitState::HalfOpen);
    assert_eq!(t.next_state, CircuitState::Closed);
    assert_eq!(t.reason, "HALF_OPEN_SUCCESS");
}

#[test]
fn half_open_two_successes_then_failure_resets_to_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    cb.record_success(pid, open_until + 1);
    cb.record_success(pid, open_until + 2);
    let change = cb.record_failure(pid, open_until + 3, None);
    assert_eq!(change.after.state, CircuitState::Open);
    assert!(change.after.open_until.is_some());
    assert!(change.transition.is_some());
    let t = change.transition.unwrap();
    assert_eq!(t.prev_state, CircuitState::HalfOpen);
    assert_eq!(t.next_state, CircuitState::Open);
    assert_eq!(t.reason, "HALF_OPEN_FAILURE");

    // After re-opening and expiring, half_open_success_count should be reset
    let new_open_until = cb
        .snapshot(pid, open_until + 4)
        .open_until
        .expect("open_until");
    cb.should_allow(pid, new_open_until); // transitions to HalfOpen again

    // Need 3 fresh successes, not 1
    let change = cb.record_success(pid, new_open_until + 1);
    assert_eq!(change.after.state, CircuitState::HalfOpen);
}

#[test]
fn half_open_failure_transitions_back_to_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    let change = cb.record_failure(pid, open_until + 1, None);
    assert_eq!(change.after.state, CircuitState::Open);
    assert!(change.after.open_until.is_some());
    assert!(change.transition.is_some());
    let t = change.transition.unwrap();
    assert_eq!(t.prev_state, CircuitState::HalfOpen);
    assert_eq!(t.next_state, CircuitState::Open);
    assert_eq!(t.reason, "HALF_OPEN_FAILURE");
}

#[test]
fn success_clears_failure_timestamps() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    cb.record_failure(pid, now, None);
    let before = cb.snapshot(pid, now + 1);
    assert_eq!(before.failure_count, 1);

    cb.record_success(pid, now + 2);
    let after = cb.snapshot(pid, now + 3);
    assert_eq!(after.failure_count, 0);
    assert_eq!(after.state, CircuitState::Closed);
}

#[test]
fn failures_within_window_counted_correctly() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        None,
    );
    let pid = 1;
    let now = 1_000;

    // Record 2 failures within the window
    cb.record_failure(pid, now, None);
    cb.record_failure(pid, now + 10, None);

    let snap = cb.snapshot(pid, now + 20);
    assert_eq!(snap.state, CircuitState::Closed);
    assert_eq!(snap.failure_count, 2);

    // Third failure within window trips the breaker
    let change = cb.record_failure(pid, now + 20, None);
    assert_eq!(change.after.state, CircuitState::Open);
}

#[test]
fn failures_older_than_window_not_counted() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        None,
    );
    let pid = 1;
    let now: i64 = 1_000;

    // Record 2 failures
    cb.record_failure(pid, now, None);
    cb.record_failure(pid, now + 1, None);

    // Jump forward past the window (300s)
    let later = now + (FAILURE_WINDOW_SECS as i64) + 10;

    // Old failures should have decayed
    let snap = cb.snapshot(pid, later);
    assert_eq!(snap.failure_count, 0);

    // Need 3 fresh failures to trip, not 1
    cb.record_failure(pid, later, None);
    let snap = cb.snapshot(pid, later + 1);
    assert_eq!(snap.state, CircuitState::Closed);
    assert_eq!(snap.failure_count, 1);

    cb.record_failure(pid, later + 2, None);
    let snap = cb.snapshot(pid, later + 3);
    assert_eq!(snap.state, CircuitState::Closed);
    assert_eq!(snap.failure_count, 2);

    let change = cb.record_failure(pid, later + 3, None);
    assert_eq!(change.after.state, CircuitState::Open);
}

#[test]
fn should_allow_prunes_expired_closed_failures_and_removes_inert_entry() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        Some(tx),
    );
    let provider_id = 1;
    let now = 1_000;

    cb.record_failure(provider_id, now, None);
    let _ = rx.try_recv().expect("failure state persisted");
    assert_eq!(cb.health.lock().expect("health lock").len(), 1);

    let later = now + (FAILURE_WINDOW_SECS as i64) + 1;
    let check = cb.should_allow(provider_id, later);

    assert!(check.allow);
    assert_eq!(check.after.state, CircuitState::Closed);
    assert_eq!(check.after.failure_count, 0);
    assert_eq!(cb.health.lock().expect("health lock").len(), 0);

    let persisted = rx.try_recv().expect("pruned state persisted");
    assert_eq!(persisted.provider_id, provider_id);
    assert_eq!(persisted.state, CircuitState::Closed);
    assert!(persisted.failure_timestamps.is_empty());
}

#[test]
fn persist_queue_full_keeps_latest_state_in_bounded_backlog_and_flushes_later() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        Some(tx),
    );
    let now = 1_000;

    cb.record_failure(1, now, None);
    cb.record_failure(2, now, None);

    {
        let backlog = cb.persist_backlog.lock().expect("backlog lock");
        assert!(backlog.contains_key(&2));
    }

    let first = rx.try_recv().expect("first state queued");
    assert_eq!(first.provider_id, 1);

    cb.record_failure(3, now, None);

    let flushed = rx.try_recv().expect("backlog state flushed first");
    assert_eq!(flushed.provider_id, 2);

    let backlog = cb.persist_backlog.lock().expect("backlog lock");
    assert!(!backlog.contains_key(&2));
    assert!(backlog.contains_key(&3));
}

#[tokio::test]
async fn persist_backlog_flushes_in_background_without_future_state_changes() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        Some(tx),
    );
    let now = 1_000;

    cb.record_failure(1, now, None);
    cb.record_failure(2, now, None);

    {
        let backlog = cb.persist_backlog.lock().expect("backlog lock");
        assert!(backlog.contains_key(&2));
    }

    let first = rx.recv().await.expect("first state queued");
    assert_eq!(first.provider_id, 1);

    let flushed = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("background backlog flush should not require another circuit update")
        .expect("backlog state queued");
    assert_eq!(flushed.provider_id, 2);

    let backlog = cb.persist_backlog.lock().expect("backlog lock");
    assert!(backlog.is_empty());
}

#[tokio::test]
async fn persist_backlog_background_flush_sends_latest_state_after_waiting_for_capacity() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        Some(tx),
    );
    let now = 1_000;

    cb.record_failure(1, now, None);
    cb.record_failure(2, now, None);
    cb.record_failure(2, now + 1, None);

    let first = rx.recv().await.expect("first state queued");
    assert_eq!(first.provider_id, 1);

    let flushed = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("background backlog flush should resume when capacity is available")
        .expect("backlog state queued");
    assert_eq!(flushed.provider_id, 2);
    assert_eq!(flushed.updated_at, now + 1);
    assert_eq!(
        flushed.failure_timestamps,
        vec![now as u64, (now + 1) as u64]
    );
}

fn persisted_state(provider_id: i64, updated_at: i64) -> CircuitPersistedState {
    CircuitPersistedState {
        provider_id,
        state: CircuitState::Closed,
        failure_timestamps: vec![updated_at as u64],
        half_open_success_count: 0,
        open_until: None,
        updated_at,
    }
}

#[test]
fn persist_backlog_flushes_oldest_updated_state_first() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration_secs: 60,
        },
        HashMap::new(),
        Some(tx),
    );

    cb.record_failure(1, 1_000, None);
    {
        let mut backlog = cb.persist_backlog.lock().expect("backlog lock");
        backlog.insert(3, persisted_state(3, 3_000));
        backlog.insert(2, persisted_state(2, 2_000));
    }

    let first = rx.try_recv().expect("first state queued");
    assert_eq!(first.provider_id, 1);

    cb.record_failure(4, 4_000, None);

    let flushed = rx.try_recv().expect("oldest backlog state flushed first");
    assert_eq!(flushed.provider_id, 2);

    let backlog = cb.persist_backlog.lock().expect("backlog lock");
    assert!(backlog.contains_key(&3));
    assert!(backlog.contains_key(&4));
}

#[test]
fn persist_backlog_evicts_oldest_updated_state_at_capacity() {
    let cb = breaker();

    {
        let mut backlog = cb.persist_backlog.lock().expect("backlog lock");
        for index in 0..MAX_PERSIST_BACKLOG {
            let provider_id = (index + 1) as i64;
            backlog.insert(
                provider_id,
                persisted_state(provider_id, 1_000 + index as i64),
            );
        }
    }

    cb.enqueue_persist_backlog(persisted_state(9_999, 9_999));

    let backlog = cb.persist_backlog.lock().expect("backlog lock");
    assert_eq!(backlog.len(), MAX_PERSIST_BACKLOG);
    assert!(!backlog.contains_key(&1));
    assert!(backlog.contains_key(&2));
    assert!(backlog.contains_key(&9_999));
}

#[test]
fn reset_clears_open_and_cooldown() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open = cb.snapshot(pid, now + 10);
    assert_eq!(open.state, CircuitState::Open);

    let reset = cb.reset(pid, now + 20);
    assert_eq!(reset.state, CircuitState::Closed);
    assert_eq!(reset.failure_count, 0);
    assert!(reset.open_until.is_none());
    assert!(reset.cooldown_until.is_none());

    let allow = cb.should_allow(pid, now + 21);
    assert!(allow.allow);
}

#[test]
fn reset_clears_half_open() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let open_until = cb.snapshot(pid, now + 10).open_until.expect("open_until");
    cb.should_allow(pid, open_until); // transitions to HalfOpen

    let snap = cb.snapshot(pid, open_until);
    assert_eq!(snap.state, CircuitState::HalfOpen);

    let reset = cb.reset(pid, open_until + 1);
    assert_eq!(reset.state, CircuitState::Closed);
    assert_eq!(reset.failure_count, 0);
}

#[test]
fn update_config_recalculates_open_until() {
    let cb = breaker(); // default: 30min open duration
    let pid = 1;
    let now = 1_000;

    // Trip the circuit breaker
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, None);
    }

    let snap = cb.snapshot(pid, now + 10);
    assert_eq!(snap.state, CircuitState::Open);
    let original_open_until = snap.open_until.expect("open_until");
    // Default: open_until = updated_at + 30*60
    assert_eq!(
        original_open_until,
        (now + DEFAULT_FAILURE_THRESHOLD as i64) + DEFAULT_OPEN_DURATION_SECS
    );

    // Hot-reload config: reduce to 60 seconds
    cb.update_config(CircuitBreakerConfig {
        failure_threshold: DEFAULT_FAILURE_THRESHOLD,
        open_duration_secs: 60,
    });

    let snap_after = cb.snapshot(pid, now + 10);
    assert_eq!(snap_after.state, CircuitState::Open);
    let new_open_until = snap_after.open_until.expect("open_until");
    // New: open_until = updated_at + 60
    assert_eq!(
        new_open_until,
        (now + DEFAULT_FAILURE_THRESHOLD as i64) + 60
    );
    assert!(new_open_until < original_open_until);

    // Verify circuit expires at the new time
    let check = cb.should_allow(pid, new_open_until);
    assert!(check.allow);
    assert_eq!(check.after.state, CircuitState::HalfOpen);
}

#[test]
fn failure_timestamps_capped_at_max() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: (MAX_FAILURE_TIMESTAMPS as u32) + 100,
            open_duration_secs: 60,
        },
        HashMap::new(),
        None,
    );
    let pid = 1;
    let now: i64 = 10_000;

    // Record more failures than the hard cap, all within the window
    for i in 0..(MAX_FAILURE_TIMESTAMPS + 50) {
        cb.record_failure(pid, now + i as i64, None);
    }

    let snap = cb.snapshot(pid, now + (MAX_FAILURE_TIMESTAMPS + 50) as i64);
    // failure_count should be capped at MAX_FAILURE_TIMESTAMPS
    assert!(
        snap.failure_count <= MAX_FAILURE_TIMESTAMPS as u32,
        "failure_count {} exceeded hard cap {}",
        snap.failure_count,
        MAX_FAILURE_TIMESTAMPS,
    );
    // Circuit should still be Closed because threshold is set very high
    assert_eq!(snap.state, CircuitState::Closed);
}

#[test]
fn healthy_read_success_and_missing_reset_do_not_create_closed_entries() {
    let cb = breaker();
    let now = 1_000;

    let check = cb.should_allow(10, now);
    assert!(check.allow);
    assert_eq!(check.after.state, CircuitState::Closed);

    let snap = cb.snapshot(11, now);
    assert_eq!(snap.state, CircuitState::Closed);

    let success = cb.record_success(12, now);
    assert_eq!(success.before.state, CircuitState::Closed);
    assert_eq!(success.after.state, CircuitState::Closed);
    assert!(success.transition.is_none());

    let reset = cb.reset(13, now);
    assert_eq!(reset.state, CircuitState::Closed);

    assert_eq!(cb.health.lock().expect("health lock").len(), 0);
}

#[test]
fn reset_removes_runtime_health_entry_after_failure() {
    let cb = breaker();
    let provider_id = 1;
    let now = 1_000;

    cb.record_failure(provider_id, now, None);
    assert_eq!(cb.health.lock().expect("health lock").len(), 1);

    let reset = cb.reset(provider_id, now + 1);
    assert_eq!(reset.state, CircuitState::Closed);
    assert_eq!(reset.failure_count, 0);
    assert_eq!(cb.health.lock().expect("health lock").len(), 0);
}

#[test]
fn initial_inert_closed_state_is_not_loaded_into_runtime_health() {
    let mut initial = HashMap::new();
    initial.insert(
        1,
        CircuitPersistedState {
            provider_id: 1,
            state: CircuitState::Closed,
            failure_timestamps: Vec::new(),
            half_open_success_count: 0,
            open_until: None,
            updated_at: 1_000,
        },
    );
    initial.insert(
        2,
        CircuitPersistedState {
            provider_id: 2,
            state: CircuitState::Closed,
            failure_timestamps: vec![1_000],
            half_open_success_count: 0,
            open_until: None,
            updated_at: 1_000,
        },
    );

    let cb = CircuitBreaker::new(CircuitBreakerConfig::default(), initial, None);
    let guard = cb.health.lock().expect("health lock");

    assert!(!guard.contains_key(&1));
    assert!(guard.contains_key(&2));
}

#[test]
fn update_config_new_failures_use_new_duration() {
    let cb = CircuitBreaker::new(
        CircuitBreakerConfig {
            failure_threshold: 2,
            open_duration_secs: 600,
        },
        HashMap::new(),
        None,
    );
    let pid = 1;
    let now = 1_000;

    // Hot-reload to shorter duration BEFORE tripping
    cb.update_config(CircuitBreakerConfig {
        failure_threshold: 2,
        open_duration_secs: 30,
    });

    // Trip the circuit
    cb.record_failure(pid, now, None);
    cb.record_failure(pid, now + 1, None);

    let snap = cb.snapshot(pid, now + 2);
    assert_eq!(snap.state, CircuitState::Open);
    // open_until should use the new 30s duration, not the original 600s
    let open_until = snap.open_until.expect("open_until");
    assert_eq!(open_until, (now + 1) + 30);
}

#[test]
fn record_failure_remembers_trigger_error_code_until_closed() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;

    // Attributed failures trip the circuit and remember the trigger.
    for i in 1..=DEFAULT_FAILURE_THRESHOLD {
        cb.record_failure(pid, now + i as i64, Some("GW_UPSTREAM_TIMEOUT"));
    }
    let snap = cb.snapshot(pid, now + 10);
    assert_eq!(snap.state, CircuitState::Open);
    assert_eq!(snap.last_trigger_error_code, Some("GW_UPSTREAM_TIMEOUT"));

    // Recover: Open -> HalfOpen keeps the trigger for attribution.
    let open_until = snap.open_until.expect("open_until");
    let check = cb.should_allow(pid, open_until);
    assert_eq!(check.after.state, CircuitState::HalfOpen);
    assert_eq!(
        check.after.last_trigger_error_code,
        Some("GW_UPSTREAM_TIMEOUT")
    );

    // HalfOpen -> Closed clears the trigger (no longer meaningful).
    cb.record_success(pid, open_until + 1);
    cb.record_success(pid, open_until + 2);
    let change = cb.record_success(pid, open_until + 3);
    assert_eq!(change.after.state, CircuitState::Closed);
    assert_eq!(change.after.last_trigger_error_code, None);
}

#[test]
fn unattributed_failure_keeps_known_trigger_error_code() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;

    cb.record_failure(pid, now, Some("GW_UPSTREAM_5XX"));
    let change = cb.record_failure(pid, now + 1, None);
    assert_eq!(
        change.after.last_trigger_error_code,
        Some("GW_UPSTREAM_5XX")
    );
}

#[test]
fn closed_success_clears_trigger_error_code() {
    let cb = breaker();
    let pid = 1;
    let now = 1_000;

    cb.record_failure(pid, now, Some("GW_UPSTREAM_5XX"));
    let change = cb.record_success(pid, now + 1);
    assert_eq!(change.after.state, CircuitState::Closed);
    assert_eq!(change.after.last_trigger_error_code, None);
}
