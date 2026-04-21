//! Usage: Types for circuit breaker state machine.

use tokio::sync::mpsc;

/// Failures older than this window (in seconds) are not counted toward the threshold.
pub(super) const FAILURE_WINDOW_SECS: u64 = 300;

/// Hard cap on stored failure timestamps to prevent unbounded memory growth.
pub(super) const MAX_FAILURE_TIMESTAMPS: usize = 256;

/// In HalfOpen state, this many consecutive successes are required to close the circuit.
pub(super) const HALF_OPEN_SUCCESS_REQUIRED: u32 = 3;

pub(super) const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
pub(super) const DEFAULT_OPEN_DURATION_SECS: i64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "CLOSED",
            Self::Open => "OPEN",
            Self::HalfOpen => "HALF_OPEN",
        }
    }

    pub fn from_str(raw: &str) -> Self {
        match raw {
            "OPEN" => Self::Open,
            "HALF_OPEN" => Self::HalfOpen,
            _ => Self::Closed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub open_duration_secs: i64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            open_duration_secs: DEFAULT_OPEN_DURATION_SECS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitSnapshot {
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub open_until: Option<i64>,
    pub cooldown_until: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CircuitTransition {
    pub prev_state: CircuitState,
    pub next_state: CircuitState,
    pub reason: &'static str,
    pub snapshot: CircuitSnapshot,
}

#[derive(Debug, Clone)]
pub struct CircuitChange {
    pub before: CircuitSnapshot,
    pub after: CircuitSnapshot,
    pub transition: Option<CircuitTransition>,
}

#[derive(Debug, Clone)]
pub struct CircuitCheck {
    pub allow: bool,
    pub after: CircuitSnapshot,
    pub transition: Option<CircuitTransition>,
}

#[derive(Debug, Clone)]
pub struct CircuitPersistedState {
    pub provider_id: i64,
    pub state: CircuitState,
    pub failure_timestamps: Vec<u64>,
    pub half_open_success_count: u32,
    pub open_until: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub(super) struct ProviderHealth {
    pub(super) state: CircuitState,
    pub(super) failure_timestamps: Vec<u64>,
    pub(super) half_open_success_count: u32,
    pub(super) open_until: Option<i64>,
    pub(super) cooldown_until: Option<i64>,
    pub(super) updated_at: i64,
}

impl ProviderHealth {
    pub(super) fn closed(provider_id: i64, now_unix: i64) -> (i64, Self) {
        (
            provider_id,
            Self {
                state: CircuitState::Closed,
                failure_timestamps: Vec::new(),
                half_open_success_count: 0,
                open_until: None,
                cooldown_until: None,
                updated_at: now_unix,
            },
        )
    }

    /// Count failures within the sliding window.
    pub(super) fn effective_failure_count(&self, now: u64) -> u32 {
        let cutoff = now.saturating_sub(FAILURE_WINDOW_SECS);
        let count = self
            .failure_timestamps
            .iter()
            .filter(|&&ts| ts > cutoff)
            .count();
        count.min(u32::MAX as usize) as u32
    }

    /// Remove failure timestamps that have fallen outside the window,
    /// and enforce a hard cap to prevent unbounded growth.
    pub(super) fn prune_old_failures(&mut self, now: u64) {
        let cutoff = now.saturating_sub(FAILURE_WINDOW_SECS);
        self.failure_timestamps.retain(|&ts| ts > cutoff);
        if self.failure_timestamps.len() > MAX_FAILURE_TIMESTAMPS {
            let excess = self.failure_timestamps.len() - MAX_FAILURE_TIMESTAMPS;
            self.failure_timestamps.drain(..excess);
        }
    }
}

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct CircuitBreaker {
    pub(super) config: Mutex<CircuitBreakerConfig>,
    pub(super) health: Mutex<HashMap<i64, ProviderHealth>>,
    pub(super) persist_tx: Option<mpsc::Sender<CircuitPersistedState>>,
}
