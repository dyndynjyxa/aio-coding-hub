//! Usage: Detect frontend/WebView hangs (white screen) with a heartbeat + pong watchdog and
//! attempt best-effort self-healing via reload.
//!
//! Contract:
//! - Backend emits `app:heartbeat` every 15s.
//! - Frontend listens to `app:heartbeat` and invokes `app_heartbeat_pong` (fire-and-forget).
//! - If backend sees no pong for 30s and the main window is visible (and not minimized),
//!   it triggers recovery with exponential backoff + circuit breaker.
//!
//! Recovery escalation:
//! 1. Page-level reload (for normal white screen).
//! 2. If error is unrecoverable (e.g. HRESULT 0x8007139F): mark webview broken,
//!    attempt to destroy + rebuild the main window.
//! 3. If rebuild fails or rebuild attempts exhausted: fallback to full app restart
//!    with restart-storm protection via a marker file.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::shared::error::AppError;

const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) const HEARTBEAT_EVENT_NAME: &str = "app:heartbeat";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const PONG_TIMEOUT: Duration = Duration::from_secs(30);

const RECOVERY_BACKOFF_BASE: Duration = Duration::from_secs(30);
const RECOVERY_BACKOFF_MAX: Duration = Duration::from_secs(5 * 60);

const RECOVERY_CIRCUIT_THRESHOLD: u32 = 5;
const RECOVERY_CIRCUIT_DURATION: Duration = Duration::from_secs(10 * 60);

/// Maximum number of window rebuild attempts within `REBUILD_COOLDOWN` before
/// escalating to a full app restart.
const REBUILD_MAX_ATTEMPTS: u32 = 3;
const REBUILD_COOLDOWN: Duration = Duration::from_secs(120);

/// If a restart marker file is younger than this duration at startup, we consider
/// the app to be in a restart storm and refuse to auto-recover.
pub(crate) const RESTART_STORM_WINDOW: Duration = Duration::from_secs(30);
const RESTART_MARKER_FILENAME: &str = "restart_marker";

#[derive(Debug, Clone, Copy, Serialize)]
struct HeartbeatPayload {
    ts_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryGate {
    Allowed,
    CircuitOpen { open_until_unix_ms: u64 },
    Backoff { next_allowed_unix_ms: u64 },
}

#[derive(Debug, Clone, Copy)]
struct WatchdogSnapshot {
    last_pong_unix_ms: u64,
    next_recovery_allowed_unix_ms: u64,
    circuit_open_until_unix_ms: u64,
    last_timeout_logged_unix_ms: u64,
}

#[derive(Debug)]
struct WatchdogInner {
    last_pong_unix_ms: u64,
    recovery_streak: u32,
    next_recovery_allowed_unix_ms: u64,
    circuit_open_until_unix_ms: u64,
    last_timeout_logged_unix_ms: u64,
    /// Whether the WebView has been classified as unrecoverably broken
    /// (e.g. HRESULT 0x8007139F).
    webview_broken: bool,
    /// Number of window rebuild attempts within the current cooldown window.
    rebuild_count: u32,
    /// Timestamp (unix ms) of the first rebuild attempt in the current window.
    first_rebuild_unix_ms: u64,
}

impl Default for WatchdogInner {
    fn default() -> Self {
        let now = now_unix_millis();
        Self {
            last_pong_unix_ms: now,
            recovery_streak: 0,
            next_recovery_allowed_unix_ms: 0,
            circuit_open_until_unix_ms: 0,
            last_timeout_logged_unix_ms: 0,
            webview_broken: false,
            rebuild_count: 0,
            first_rebuild_unix_ms: 0,
        }
    }
}

pub(crate) struct HeartbeatWatchdogState {
    inner: Mutex<WatchdogInner>,
    /// `false` when the WebView is confirmed unresponsive (reload failed).
    /// Checked by event emitters to skip sending to a dead WebView.
    webview_alive: AtomicBool,
    /// Prevents concurrent recovery attempts (rebuild / restart).
    recovery_in_flight: AtomicBool,
}

impl Default for HeartbeatWatchdogState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(WatchdogInner::default()),
            webview_alive: AtomicBool::new(true),
            recovery_in_flight: AtomicBool::new(false),
        }
    }
}

impl HeartbeatWatchdogState {
    /// Returns `true` when the WebView is believed to be responsive.
    /// Event emitters should skip `app.emit()` when this returns `false`.
    pub(crate) fn is_webview_alive(&self) -> bool {
        self.webview_alive.load(Ordering::Relaxed)
    }

    fn set_webview_alive(&self, alive: bool) {
        self.webview_alive.store(alive, Ordering::Relaxed);
    }

    pub(crate) fn record_pong(&self) {
        let now = now_unix_millis();
        // A pong proves the WebView is alive.
        self.set_webview_alive(true);
        self.recovery_in_flight.store(false, Ordering::Relaxed);

        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        inner.last_pong_unix_ms = now;
        inner.recovery_streak = 0;
        inner.next_recovery_allowed_unix_ms = 0;
        inner.circuit_open_until_unix_ms = 0;
        inner.webview_broken = false;
        inner.rebuild_count = 0;
        inner.first_rebuild_unix_ms = 0;
    }

    fn snapshot(&self) -> WatchdogSnapshot {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        WatchdogSnapshot {
            last_pong_unix_ms: inner.last_pong_unix_ms,
            next_recovery_allowed_unix_ms: inner.next_recovery_allowed_unix_ms,
            circuit_open_until_unix_ms: inner.circuit_open_until_unix_ms,
            last_timeout_logged_unix_ms: inner.last_timeout_logged_unix_ms,
        }
    }

    fn is_webview_broken(&self) -> bool {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.webview_broken
    }

    fn mark_webview_broken(&self) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.webview_broken = true;
    }

    /// Returns `true` if we can still attempt a window rebuild, `false` if
    /// max attempts within cooldown have been exhausted.
    fn try_bump_rebuild_count(&self) -> bool {
        let now = now_unix_millis();
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let cooldown_ms = REBUILD_COOLDOWN.as_millis() as u64;
        if now.saturating_sub(inner.first_rebuild_unix_ms) > cooldown_ms {
            // Reset the window.
            inner.rebuild_count = 1;
            inner.first_rebuild_unix_ms = now;
            return true;
        }

        inner.rebuild_count = inner.rebuild_count.saturating_add(1);
        inner.rebuild_count <= REBUILD_MAX_ATTEMPTS
    }

    fn set_last_timeout_logged_unix_ms(&self, ts_unix_ms: u64) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.last_timeout_logged_unix_ms = ts_unix_ms;
    }

    fn schedule_next_recovery(&self, streak: u32, now_unix_ms: u64) -> Duration {
        let delay = recovery_backoff_delay(streak);
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.next_recovery_allowed_unix_ms = now_unix_ms.saturating_add(delay.as_millis() as u64);
        delay
    }

    fn trip_circuit(&self, now_unix_ms: u64) {
        let until = now_unix_ms.saturating_add(RECOVERY_CIRCUIT_DURATION.as_millis() as u64);
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.circuit_open_until_unix_ms = until;
        inner.recovery_streak = 0;
        inner.next_recovery_allowed_unix_ms = until;
    }

    fn bump_recovery_streak(&self) -> u32 {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.recovery_streak = inner.recovery_streak.saturating_add(1);
        inner.recovery_streak
    }
}

/// Emit an event only when the WebView is believed to be alive.
/// Use this from any module that sends events to the frontend.
pub(crate) fn gated_emit<S: serde::Serialize + Clone>(
    app: &tauri::AppHandle,
    event: &str,
    payload: S,
) {
    let alive = app
        .try_state::<HeartbeatWatchdogState>()
        .map(|s| s.is_webview_alive())
        .unwrap_or(true);
    if !alive {
        tracing::debug!(event, "gated_emit: skipped (WebView marked dead)");
        return;
    }
    let _ = app.emit(event, payload);
}

fn app_is_terminating(app: &tauri::AppHandle) -> bool {
    app.try_state::<crate::resident::ResidentState>()
        .map(|state| state.is_terminating())
        .unwrap_or(false)
}

pub(crate) fn install(app: &tauri::AppHandle) {
    tracing::info!(
        interval_s = HEARTBEAT_INTERVAL.as_secs(),
        timeout_s = PONG_TIMEOUT.as_secs(),
        "WebView 心跳监控已启动"
    );

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        // First tick is immediate; skip it to avoid double fire at startup.
        interval.tick().await;
        // Counter used to probe the WebView at reduced frequency when it is marked dead.
        // Every PROBE_DIVISOR ticks (~60 s) we still emit a heartbeat so that a recovered
        // WebView can answer with a pong and flip the flag back to alive.
        let mut tick_counter: u32 = 0;
        const PROBE_DIVISOR: u32 = 4; // 4 * 15 s = 60 s
        loop {
            interval.tick().await;
            tick_counter = tick_counter.wrapping_add(1);

            let state = app.state::<HeartbeatWatchdogState>();
            let alive = state.is_webview_alive();

            // When the WebView is alive: emit every tick.
            // When dead: only emit once every PROBE_DIVISOR ticks as a recovery probe.
            let should_emit = alive || tick_counter.is_multiple_of(PROBE_DIVISOR);

            if should_emit {
                let now = now_unix_millis();
                let payload = HeartbeatPayload { ts_unix_ms: now };
                if let Err(err) = app.emit(HEARTBEAT_EVENT_NAME, payload) {
                    tracing::debug!("emit heartbeat failed: {}", err);
                }
            }

            check_and_recover_if_needed(&app).await;
        }
    });
}

async fn check_and_recover_if_needed(app: &tauri::AppHandle) {
    if app_is_terminating(app) {
        return;
    }

    let now = now_unix_millis();
    let state = app.state::<HeartbeatWatchdogState>();
    let snapshot = state.snapshot();

    let since_last_pong_ms = now.saturating_sub(snapshot.last_pong_unix_ms);
    if since_last_pong_ms <= PONG_TIMEOUT.as_millis() as u64 {
        return;
    }

    if now.saturating_sub(snapshot.last_timeout_logged_unix_ms) > 60_000 {
        state.set_last_timeout_logged_unix_ms(now);
        tracing::warn!(
            since_last_pong_ms,
            "frontend heartbeat timeout detected (possible blank screen / freeze)"
        );
    }

    // If recovery is already in flight (e.g. rebuild/restart), don't pile on.
    if state.recovery_in_flight.load(Ordering::Relaxed) {
        tracing::debug!("recovery already in flight, skipping");
        return;
    }

    // If the WebView has been classified as broken (unrecoverable), skip page-level
    // recovery and go straight to the rebuild/restart path.
    if state.is_webview_broken() {
        attempt_escalated_recovery(app).await;
        return;
    }

    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        tracing::debug!("heartbeat watchdog: main window not found");
        // Window gone — treat as broken and try to rebuild.
        state.mark_webview_broken();
        attempt_escalated_recovery(app).await;
        return;
    };

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(true);
    if !is_visible || is_minimized {
        return;
    }

    match recovery_gate(now, snapshot) {
        RecoveryGate::Allowed => {}
        RecoveryGate::CircuitOpen { open_until_unix_ms } => {
            tracing::info!(
                open_until_unix_ms,
                "blank screen recovery circuit open, skipping recovery attempt"
            );
            return;
        }
        RecoveryGate::Backoff {
            next_allowed_unix_ms,
        } => {
            tracing::info!(
                next_allowed_unix_ms,
                "blank screen recovery in backoff period, skipping recovery attempt"
            );
            return;
        }
    }

    let streak = state.bump_recovery_streak();

    if should_trip_circuit(streak) {
        state.trip_circuit(now);
        tracing::warn!(
            streak,
            open_for_s = RECOVERY_CIRCUIT_DURATION.as_secs(),
            "blank screen recovery circuit tripped, pausing auto-recovery"
        );
        return;
    }

    tracing::warn!(streak, since_last_pong_ms, "attempting page reload");

    let attempt = attempt_reload(&window).await;
    match attempt {
        Ok(()) => {
            let delay = state.schedule_next_recovery(streak, now);
            tracing::info!(
                streak,
                next_delay_s = delay.as_secs(),
                "已发起恢复指令，等待 pong；若仍无响应将按退避再次尝试"
            );
        }
        Err(err) => {
            let err_str = err.to_string();
            if is_unrecoverable_webview_error(&err_str) {
                tracing::error!(
                    error = %err_str,
                    "WebView entered unrecoverable state, escalating to window rebuild"
                );
                state.mark_webview_broken();
                state.set_webview_alive(false);
                attempt_escalated_recovery(app).await;
            } else {
                // WebView is confirmed unresponsive — gate all event emissions.
                state.set_webview_alive(false);
                let delay = state.schedule_next_recovery(streak, now);
                tracing::warn!(
                    streak,
                    next_delay_s = delay.as_secs(),
                    "恢复指令下发失败（可能 WebView 已崩溃），已暂停事件发送：{}",
                    err
                );
            }
        }
    }
}

/// Escalated recovery: try to rebuild the main window first; if that fails or
/// attempts are exhausted, fall back to a full app restart.
async fn attempt_escalated_recovery(app: &tauri::AppHandle) {
    if app_is_terminating(app) {
        tracing::debug!("explicit exit/restart in progress, skipping escalated recovery");
        return;
    }

    let state = app.state::<HeartbeatWatchdogState>();

    // Prevent concurrent recovery.
    if state
        .recovery_in_flight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        tracing::debug!("escalated recovery already in flight");
        return;
    }

    // Check rebuild budget.
    if state.try_bump_rebuild_count() {
        tracing::warn!("attempting main window rebuild");
        match rebuild_main_window(app) {
            Ok(()) => {
                tracing::info!(
                    "main window rebuilt successfully, waiting for frontend pong to confirm recovery"
                );
                // Keep recovery_in_flight=true until a pong arrives (record_pong clears it).
                return;
            }
            Err(err) => {
                tracing::error!(error = %err, "main window rebuild failed");
                // Fall through to app restart.
            }
        }
    } else {
        tracing::warn!(
            max = REBUILD_MAX_ATTEMPTS,
            cooldown_s = REBUILD_COOLDOWN.as_secs(),
            "window rebuild attempts exhausted within cooldown, escalating to app restart"
        );
    }

    // Final fallback: full app restart with storm protection.
    escalate_to_app_restart(app).await;
}

/// Destroy the current main window and recreate it with the same configuration.
fn rebuild_main_window(app: &tauri::AppHandle) -> Result<(), AppError> {
    // Destroy old window if it still exists.
    if let Some(old_window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        tracing::info!("destroying old main window");
        if let Err(err) = old_window.destroy() {
            tracing::warn!(error = %err, "failed to destroy old main window, continuing with rebuild");
        }
    }

    // Small delay to allow the old window resources to be released.
    std::thread::sleep(Duration::from_millis(100));

    // Rebuild with the same settings as tauri.conf.json.
    let url = tauri::WebviewUrl::App("index.html".into());
    let new_window = tauri::webview::WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, url)
        .title("AIO Coding Hub")
        .inner_size(1500.0, 900.0)
        .build()
        .map_err(|e| {
            AppError::new(
                "WINDOW_REBUILD_FAILED",
                format!("failed to build window: {e}"),
            )
        })?;

    // Make the window visible and focused.
    let _ = new_window.show();
    let _ = new_window.unminimize();
    let _ = new_window.set_focus();

    Ok(())
}

/// Write a restart marker, then request a full app restart.
async fn escalate_to_app_restart(app: &tauri::AppHandle) {
    if app_is_terminating(app) {
        tracing::debug!("explicit exit/restart in progress, skipping watchdog restart");
        return;
    }

    // Check for restart storm before proceeding.
    if is_restart_storm(app) {
        tracing::error!(
            "restart storm detected: previous restart was less than {}s ago, refusing to auto-restart. \
             The user will need to restart the app manually.",
            RESTART_STORM_WINDOW.as_secs()
        );
        show_restart_storm_dialog(app);
        return;
    }

    write_restart_marker(app);

    tracing::warn!("escalating to full app restart");

    let app = app.clone();
    // Run cleanup + restart in a background thread to avoid blocking the watchdog loop.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        tauri::async_runtime::block_on(crate::app::cleanup::cleanup_before_exit(&app));
        app.request_restart();
    });
}

// ── Unrecoverable error classification ──────────────────────────────────────

/// Returns `true` if the error string indicates a WebView state that cannot be
/// recovered by page-level reload/navigate.
///
/// Currently covers:
/// - HRESULT 0x8007139F: WebView2 controller entered invalid state.
///
/// This function is designed to be extended with more error codes as they are
/// discovered.
fn is_unrecoverable_webview_error(err: &str) -> bool {
    // Case-insensitive match for the HRESULT hex code.
    let err_lower = err.to_ascii_lowercase();
    err_lower.contains("0x8007139f")
}

// ── Restart storm protection ────────────────────────────────────────────────

fn restart_marker_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    crate::infra::app_paths::app_data_dir(app)
        .ok()
        .map(|dir| dir.join(RESTART_MARKER_FILENAME))
}

fn write_restart_marker(app: &tauri::AppHandle) {
    let Some(path) = restart_marker_path(app) else {
        return;
    };
    let now = now_unix_millis().to_string();
    if let Err(err) = std::fs::write(&path, now.as_bytes()) {
        tracing::warn!(path = %path.display(), "failed to write restart marker: {err}");
    }
}

fn is_restart_storm(app: &tauri::AppHandle) -> bool {
    read_restart_marker_age_ms(app)
        .map(|age_ms| age_ms < RESTART_STORM_WINDOW.as_millis() as u64)
        .unwrap_or(false)
}

fn read_restart_marker_age_ms(app: &tauri::AppHandle) -> Option<u64> {
    let path = restart_marker_path(app)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let marker_ts: u64 = content.trim().parse().ok()?;
    let now = now_unix_millis();
    Some(now.saturating_sub(marker_ts))
}

/// Called at startup to check and clear the restart marker.
/// Returns `true` if a restart storm is detected (marker exists and is recent).
pub(crate) fn check_and_clear_restart_marker(app: &tauri::AppHandle) -> bool {
    let storm = is_restart_storm(app);
    if storm {
        tracing::error!(
            "restart storm detected at startup: previous restart was less than {}s ago",
            RESTART_STORM_WINDOW.as_secs()
        );
    }
    // Always clear the marker after reading.
    if let Some(path) = restart_marker_path(app) {
        let _ = std::fs::remove_file(&path);
    }
    storm
}

fn show_restart_storm_dialog(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        app.dialog()
            .message(
                "AIO Coding Hub 检测到 WebView 反复崩溃，已停止自动恢复。\n\n\
                 请手动重启应用。如果问题持续出现，请检查系统 WebView2 运行时是否正常。",
            )
            .title("WebView 恢复失败")
            .blocking_show();
    });
}

// ── Page-level reload (unchanged logic) ─────────────────────────────────────

async fn attempt_reload(window: &tauri::WebviewWindow) -> Result<(), AppError> {
    let mut errors: Vec<(&'static str, String)> = Vec::new();

    if let Err(err) = window.reload() {
        errors.push(("webview.reload", err.to_string()));
    } else {
        return Ok(());
    }

    let url_string = match window.url() {
        Ok(url) => {
            let url_string = url.to_string();
            if let Err(err) = window.navigate(url) {
                errors.push(("webview.navigate", err.to_string()));
            } else {
                return Ok(());
            }
            Some(url_string)
        }
        Err(err) => {
            errors.push(("webview.url", err.to_string()));
            None
        }
    };

    if let Err(err) = window.eval("window.location.reload()") {
        errors.push(("eval.reload", err.to_string()));
    } else {
        return Ok(());
    }

    if let Some(url) = url_string {
        let url_literal = serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".to_string());
        let js = format!("window.location.href = {url_literal};");
        if let Err(err) = window.eval(js) {
            errors.push(("eval.href", err.to_string()));
        } else {
            return Ok(());
        }
    }

    let details = errors
        .into_iter()
        .map(|(label, err)| format!("{label}: {err}"))
        .collect::<Vec<_>>()
        .join(" | ");
    Err(AppError::new("WEBVIEW_RECOVERY_FAILED", details))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn recovery_gate(now_unix_ms: u64, snapshot: WatchdogSnapshot) -> RecoveryGate {
    if snapshot.circuit_open_until_unix_ms > now_unix_ms {
        return RecoveryGate::CircuitOpen {
            open_until_unix_ms: snapshot.circuit_open_until_unix_ms,
        };
    }
    if snapshot.next_recovery_allowed_unix_ms > now_unix_ms {
        return RecoveryGate::Backoff {
            next_allowed_unix_ms: snapshot.next_recovery_allowed_unix_ms,
        };
    }
    RecoveryGate::Allowed
}

fn should_trip_circuit(streak: u32) -> bool {
    streak >= RECOVERY_CIRCUIT_THRESHOLD
}

fn recovery_backoff_delay(streak: u32) -> Duration {
    let streak = streak.max(1);
    let max_exponent = 20u32;
    let exponent = (streak - 1).min(max_exponent);
    let base_ms = RECOVERY_BACKOFF_BASE.as_millis() as u64;
    let factor = 1u64.checked_shl(exponent).unwrap_or(u64::MAX);
    let ms = base_ms.saturating_mul(factor);
    let capped_ms = (RECOVERY_BACKOFF_MAX.as_millis() as u64).min(ms);
    Duration::from_millis(capped_ms)
}

fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_backoff_delay_matches_spec_and_caps() {
        assert_eq!(recovery_backoff_delay(1), Duration::from_secs(30));
        assert_eq!(recovery_backoff_delay(2), Duration::from_secs(60));
        assert_eq!(recovery_backoff_delay(3), Duration::from_secs(120));
        assert_eq!(recovery_backoff_delay(4), Duration::from_secs(240));
        // 30 * 2^(5-1) = 480s, but capped at 300s.
        assert_eq!(recovery_backoff_delay(5), Duration::from_secs(300));
        assert_eq!(recovery_backoff_delay(100), Duration::from_secs(300));
    }

    #[test]
    fn should_trip_circuit_triggers_at_threshold() {
        assert!(!should_trip_circuit(RECOVERY_CIRCUIT_THRESHOLD - 1));
        assert!(should_trip_circuit(RECOVERY_CIRCUIT_THRESHOLD));
        assert!(should_trip_circuit(RECOVERY_CIRCUIT_THRESHOLD + 1));
    }

    #[test]
    fn webview_alive_lifecycle() {
        let state = HeartbeatWatchdogState::default();

        // Initially alive.
        assert!(state.is_webview_alive());

        // Mark dead.
        state.set_webview_alive(false);
        assert!(!state.is_webview_alive());

        // Pong restores alive + resets recovery counters.
        state.set_webview_alive(false);
        state.record_pong();
        assert!(state.is_webview_alive());
        let snap = state.snapshot();
        assert_eq!(snap.next_recovery_allowed_unix_ms, 0);
        assert_eq!(snap.circuit_open_until_unix_ms, 0);
    }

    #[test]
    fn recovery_gate_blocks_when_circuit_open_or_backoff() {
        let now = 1_000u64;
        let base = WatchdogSnapshot {
            last_pong_unix_ms: 0,
            next_recovery_allowed_unix_ms: 0,
            circuit_open_until_unix_ms: 0,
            last_timeout_logged_unix_ms: 0,
        };

        assert_eq!(recovery_gate(now, base), RecoveryGate::Allowed);

        let backoff = WatchdogSnapshot {
            next_recovery_allowed_unix_ms: now + 1,
            ..base
        };
        assert_eq!(
            recovery_gate(now, backoff),
            RecoveryGate::Backoff {
                next_allowed_unix_ms: now + 1
            }
        );

        let circuit = WatchdogSnapshot {
            circuit_open_until_unix_ms: now + 2,
            ..base
        };
        assert_eq!(
            recovery_gate(now, circuit),
            RecoveryGate::CircuitOpen {
                open_until_unix_ms: now + 2
            }
        );
    }

    #[test]
    fn is_unrecoverable_webview_error_detects_known_hresult() {
        assert!(is_unrecoverable_webview_error(
            "webview.reload: HRESULT(0x8007139F)"
        ));
        assert!(is_unrecoverable_webview_error(
            "some prefix 0x8007139f something"
        ));
        assert!(!is_unrecoverable_webview_error("some other error"));
        assert!(!is_unrecoverable_webview_error(""));
    }

    #[test]
    fn webview_broken_state_lifecycle() {
        let state = HeartbeatWatchdogState::default();

        assert!(!state.is_webview_broken());

        state.mark_webview_broken();
        assert!(state.is_webview_broken());

        // Pong should clear the broken state.
        state.record_pong();
        assert!(!state.is_webview_broken());
    }

    #[test]
    fn rebuild_count_budget() {
        let state = HeartbeatWatchdogState::default();

        // First REBUILD_MAX_ATTEMPTS should succeed.
        for _ in 0..REBUILD_MAX_ATTEMPTS {
            assert!(state.try_bump_rebuild_count());
        }
        // Next one should fail (budget exhausted).
        assert!(!state.try_bump_rebuild_count());
    }

    #[test]
    fn recovery_in_flight_prevents_concurrent_recovery() {
        let state = HeartbeatWatchdogState::default();
        assert!(!state.recovery_in_flight.load(Ordering::Relaxed));

        state.recovery_in_flight.store(true, Ordering::Relaxed);
        assert!(state.recovery_in_flight.load(Ordering::Relaxed));

        // Pong clears it.
        state.record_pong();
        assert!(!state.recovery_in_flight.load(Ordering::Relaxed));
    }
}
