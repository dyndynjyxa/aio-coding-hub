//! Usage: Detect frontend/WebView hangs (white screen) with a heartbeat + pong watchdog and
//! attempt best-effort self-healing via reload.
//!
//! Contract:
//! - Backend emits `app:heartbeat` every 15s.
//! - Frontend listens to `app:heartbeat` and invokes `app_heartbeat_pong` (fire-and-forget).
//! - If backend sees no pong for 30s (60s while the window is hidden/minimized, to
//!   tolerate OS throttling), it triggers recovery with exponential backoff.
//!
//! Recovery escalation (visibility tunes behavior, it never blocks detection):
//! 1. Page-level reload — runs even while the window is hidden in the tray, so a
//!    WebView that dies in the background is repaired before the user opens it.
//! 2. If error is unrecoverable (e.g. HRESULT 0x8007139F) or reloads are exhausted:
//!    mark webview broken, destroy + rebuild the main window from the tauri.conf.json
//!    window config, preserving its previous visibility (a hidden window is rebuilt
//!    hidden, without stealing focus). At most `REBUILD_MAX_ATTEMPTS` consecutive
//!    unconfirmed rebuilds per broken episode (a pong resets the budget).
//! 3. If rebuild fails or the rebuild budget is exhausted: full app restart with
//!    restart-storm protection via a marker file. Restart is deferred while the
//!    window is hidden so ongoing gateway traffic is not killed behind the user's
//!    back; a missing window is always restartable (there is nothing left to show).
//!
//! Every "wait for the frontend to confirm recovery" state carries a deadline
//! (`recovery_confirm_deadline`): if no pong arrives in time the attempt counts as
//! failed and escalation continues, so recovery can never deadlock waiting forever.
//! Deferring never holds the confirm slot — only an actually-issued rebuild does.
//! `on_main_window_shown` runs an immediate check when the window is shown or
//! focused, so a user opening the window never stares at a white screen until the
//! next tick; freshly shown windows get one heartbeat round-trip of grace before
//! silence accrued under OS throttling is judged by the strict visible threshold.
//! Checks are serialized by `check_in_progress` — the tick loop and show/focus
//! hooks can never run two recovery passes concurrently.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::shared::error::AppError;
use crate::shared::fs::read_file_with_max_len;

const MAIN_WINDOW_LABEL: &str = "main";

pub(crate) const HEARTBEAT_EVENT_NAME: &str = "app:heartbeat";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const PONG_TIMEOUT: Duration = Duration::from_secs(30);
/// Hidden/minimized windows may have their timers throttled by the OS, so give
/// them a longer grace period before treating pong silence as a dead WebView.
const PONG_TIMEOUT_HIDDEN: Duration = Duration::from_secs(60);
/// Until the very first pong of this process arrives, a hidden window (e.g.
/// `start_minimized` on a slow cold start) gets an even longer allowance so a
/// still-loading frontend is not reloaded mid-boot.
const STARTUP_PONG_TIMEOUT_HIDDEN: Duration = Duration::from_secs(180);
/// After the window becomes visible/focused, silence accrued while hidden is
/// still judged by the lenient hidden threshold for this long — the frontend
/// needs at least one heartbeat round-trip to prove itself after unthrottling.
const RECENTLY_SHOWN_GRACE: Duration = Duration::from_secs(20);
/// How long an escalated recovery (window rebuild) may wait for a confirming
/// pong before the attempt is considered failed and escalation continues.
const RECOVERY_CONFIRM_TIMEOUT: Duration = Duration::from_secs(90);

const RECOVERY_BACKOFF_BASE: Duration = Duration::from_secs(30);
const RECOVERY_BACKOFF_MAX: Duration = Duration::from_secs(5 * 60);

const RECOVERY_CIRCUIT_THRESHOLD: u32 = 5;

/// Maximum number of consecutive unconfirmed window rebuild attempts before
/// escalating to a full app restart. Reset by a pong.
const REBUILD_MAX_ATTEMPTS: u32 = 3;

/// If a restart marker file is younger than this duration at startup, we consider
/// the app to be in a restart storm and refuse to auto-recover.
pub(crate) const RESTART_STORM_WINDOW: Duration = Duration::from_secs(30);
const RESTART_MARKER_FILENAME: &str = "restart_marker";
const RESTART_MARKER_MAX_BYTES: usize = 64;

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
    /// `true` once any pong has been received in this process — before that,
    /// silence may just be a slow frontend boot, not a dead WebView.
    has_received_pong: bool,
    recovery_streak: u32,
    next_recovery_allowed_unix_ms: u64,
    circuit_open_until_unix_ms: u64,
    last_timeout_logged_unix_ms: u64,
    /// Throttles the recurring "restart deferred while hidden" warning.
    last_deferred_restart_logged_unix_ms: u64,
    /// Timestamp (unix ms) of the last show/focus of the main window.
    last_shown_unix_ms: u64,
    /// Whether the WebView has been classified as unrecoverably broken
    /// (e.g. HRESULT 0x8007139F).
    webview_broken: bool,
    /// Number of consecutive unconfirmed rebuild attempts. Reset by a pong.
    rebuild_count: u32,
}

impl Default for WatchdogInner {
    fn default() -> Self {
        let now = now_unix_millis();
        Self {
            last_pong_unix_ms: now,
            has_received_pong: false,
            recovery_streak: 0,
            next_recovery_allowed_unix_ms: 0,
            circuit_open_until_unix_ms: 0,
            last_timeout_logged_unix_ms: 0,
            last_deferred_restart_logged_unix_ms: 0,
            last_shown_unix_ms: 0,
            webview_broken: false,
            rebuild_count: 0,
        }
    }
}

pub(crate) struct HeartbeatWatchdogState {
    inner: Mutex<WatchdogInner>,
    /// `false` when the WebView is confirmed unresponsive (reload failed).
    /// Checked by event emitters to skip sending to a dead WebView.
    webview_alive: AtomicBool,
    /// Unix ms until which an escalated recovery (rebuild) is awaiting a
    /// confirming pong. `0` means no recovery is awaiting confirmation.
    /// Once the deadline passes without a pong the attempt counts as failed
    /// and escalation continues — this can never deadlock recovery.
    recovery_confirm_deadline_unix_ms: AtomicU64,
    /// Set when a restart storm is detected: auto-recovery stays off for the
    /// rest of the session so the storm dialog is shown exactly once.
    auto_recovery_disabled: AtomicBool,
    /// Serializes `check_and_recover_if_needed` — the tick loop and the
    /// show/focus hooks must never run two recovery passes concurrently.
    check_in_progress: AtomicBool,
}

impl Default for HeartbeatWatchdogState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(WatchdogInner::default()),
            webview_alive: AtomicBool::new(true),
            recovery_confirm_deadline_unix_ms: AtomicU64::new(0),
            auto_recovery_disabled: AtomicBool::new(false),
            check_in_progress: AtomicBool::new(false),
        }
    }
}

/// Clears `check_in_progress` on every exit path of a recovery check.
struct CheckInProgressGuard<'a>(&'a AtomicBool);

impl Drop for CheckInProgressGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
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
        self.recovery_confirm_deadline_unix_ms
            .store(0, Ordering::Release);

        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        inner.last_pong_unix_ms = now;
        inner.has_received_pong = true;
        inner.recovery_streak = 0;
        inner.next_recovery_allowed_unix_ms = 0;
        inner.circuit_open_until_unix_ms = 0;
        inner.webview_broken = false;
        inner.rebuild_count = 0;
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

    /// Returns `true` if we can still attempt a window rebuild, `false` once
    /// `REBUILD_MAX_ATTEMPTS` consecutive rebuilds went unconfirmed. Only a
    /// pong (proof of a live WebView) resets the budget — a wall-clock window
    /// would silently re-arm an endless rebuild loop.
    fn try_bump_rebuild_count(&self) -> bool {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if inner.rebuild_count >= REBUILD_MAX_ATTEMPTS {
            return false;
        }
        inner.rebuild_count += 1;
        true
    }

    fn note_window_shown(&self, now_unix_ms: u64) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.last_shown_unix_ms = now_unix_ms;
    }

    fn recently_shown(&self, now_unix_ms: u64) -> bool {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        now_unix_ms.saturating_sub(inner.last_shown_unix_ms)
            < RECENTLY_SHOWN_GRACE.as_millis() as u64
    }

    fn has_received_pong(&self) -> bool {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.has_received_pong
    }

    /// Rate-limits the recurring "restart deferred while hidden" warning.
    fn should_log_deferred_restart(&self, now_unix_ms: u64) -> bool {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if now_unix_ms.saturating_sub(inner.last_deferred_restart_logged_unix_ms) <= 60_000 {
            return false;
        }
        inner.last_deferred_restart_logged_unix_ms = now_unix_ms;
        true
    }

    fn recovery_confirm_deadline_unix_ms(&self) -> u64 {
        self.recovery_confirm_deadline_unix_ms
            .load(Ordering::Acquire)
    }

    fn clear_recovery_confirm_deadline(&self) {
        self.recovery_confirm_deadline_unix_ms
            .store(0, Ordering::Release);
    }

    /// Claim the escalated-recovery slot until `deadline_unix_ms`. Fails when a
    /// previous attempt is still awaiting confirmation (deadline not yet passed).
    fn try_claim_recovery_confirm(&self, now_unix_ms: u64, deadline_unix_ms: u64) -> bool {
        let current = self
            .recovery_confirm_deadline_unix_ms
            .load(Ordering::Acquire);
        if current != 0 && now_unix_ms < current {
            return false;
        }
        self.recovery_confirm_deadline_unix_ms
            .compare_exchange(
                current,
                deadline_unix_ms,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn is_auto_recovery_disabled(&self) -> bool {
        self.auto_recovery_disabled.load(Ordering::Relaxed)
    }

    fn disable_auto_recovery(&self) {
        self.auto_recovery_disabled.store(true, Ordering::Relaxed);
    }

    /// Forget any scheduled backoff so the next check may act immediately.
    /// The recovery streak is preserved: repeated failures still escalate.
    fn clear_recovery_backoff(&self) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.next_recovery_allowed_unix_ms = 0;
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
pub(crate) fn gated_emit<R: tauri::Runtime, S: serde::Serialize + Clone>(
    app: &tauri::AppHandle<R>,
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
        let mut interval = heartbeat_interval();
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

/// Called whenever the main window is shown or focused (tray click, dock icon,
/// second instance, startup settings, OS-level unminimize/restore). If the
/// WebView already looks dead, run a recovery check right away — the user
/// should never stare at a white screen waiting for the next heartbeat tick or
/// a stale backoff window.
pub(crate) fn on_main_window_shown(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<HeartbeatWatchdogState>() else {
        return;
    };

    let now = now_unix_millis();
    state.note_window_shown(now);

    // The silence judged here accrued while the window was hidden/throttled,
    // so use the lenient hidden allowance (and the startup allowance before
    // the first pong) — a healthy-but-throttled frontend must get a chance to
    // answer the next heartbeat instead of being reloaded on show.
    let stale_after = if state.has_received_pong() {
        PONG_TIMEOUT_HIDDEN
    } else {
        STARTUP_PONG_TIMEOUT_HIDDEN
    };
    let since_last_pong_ms = now.saturating_sub(state.snapshot().last_pong_unix_ms);
    if since_last_pong_ms <= stale_after.as_millis() as u64 {
        return;
    }

    state.clear_recovery_backoff();
    tracing::warn!(
        since_last_pong_ms,
        "main window shown with a stale heartbeat, running immediate recovery check"
    );

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        check_and_recover_if_needed(&app).await;
    });
}

fn heartbeat_interval() -> tokio::time::Interval {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}

/// Returns how long pong silence is tolerated for the current window state.
///
/// - Visible, settled window: strict 30s — a user is (potentially) looking at
///   a white screen, recover fast.
/// - Freshly shown/focused window: silence accrued under OS throttling while
///   hidden must not be judged strictly; give one heartbeat round-trip.
/// - Hidden window: lenient 60s (OS timer throttling), and until the very
///   first pong of the process an even longer startup allowance so a slow
///   `start_minimized` boot is not reloaded mid-load.
fn pong_timeout_for(
    window_visible: bool,
    recently_shown: bool,
    has_received_pong: bool,
) -> Duration {
    if window_visible && !recently_shown {
        return PONG_TIMEOUT;
    }
    if has_received_pong {
        PONG_TIMEOUT_HIDDEN
    } else {
        STARTUP_PONG_TIMEOUT_HIDDEN
    }
}

async fn check_and_recover_if_needed(app: &tauri::AppHandle) {
    if app_is_terminating(app) {
        return;
    }

    let now = now_unix_millis();
    let state = app.state::<HeartbeatWatchdogState>();

    // Serialize recovery passes: the tick loop and the show/focus hooks may
    // call in concurrently; a second pass would double reloads and streaks.
    if state
        .check_in_progress
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    let _check_guard = CheckInProgressGuard(&state.check_in_progress);

    let snapshot = state.snapshot();
    let since_last_pong_ms = now.saturating_sub(snapshot.last_pong_unix_ms);

    // Cheap early-return on the global minimum threshold BEFORE any window
    // query — window getters are blocking round-trips to the main thread and
    // this path runs every 15s for the app's whole lifetime.
    if since_last_pong_ms <= PONG_TIMEOUT.as_millis() as u64 {
        return;
    }

    // Visibility tunes the detection threshold and rebuild/restart behavior;
    // it never blocks recovery — a WebView that dies while the window is
    // hidden in the tray must be repaired before the user opens it again.
    // Errors while querying window state default to "treat as visible" so a
    // misreporting platform can never silently disable recovery. It is
    // sampled ONCE here and threaded through the whole pass so reload,
    // rebuild, and restart decisions can never disagree mid-recovery.
    let window = app.get_webview_window(MAIN_WINDOW_LABEL);
    let window_exists = window.is_some();
    let window_visible = window
        .as_ref()
        .map(|w| w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false))
        .unwrap_or(false);

    let pong_timeout = pong_timeout_for(
        window_visible,
        state.recently_shown(now),
        state.has_received_pong(),
    );
    if since_last_pong_ms <= pong_timeout.as_millis() as u64 {
        return;
    }

    if now.saturating_sub(snapshot.last_timeout_logged_unix_ms) > 60_000 {
        state.set_last_timeout_logged_unix_ms(now);
        tracing::warn!(
            since_last_pong_ms,
            window_visible,
            "frontend heartbeat timeout detected (possible blank screen / freeze)"
        );
    }

    if state.is_auto_recovery_disabled() {
        // Restart storm was detected earlier in this session; the dialog has
        // already told the user to restart manually.
        return;
    }

    // An escalated recovery is awaiting pong confirmation. Wait until its
    // deadline; once it passes, the attempt counts as failed and we continue.
    let confirm_deadline = state.recovery_confirm_deadline_unix_ms();
    if confirm_deadline != 0 {
        if now < confirm_deadline {
            return;
        }
        state.clear_recovery_confirm_deadline();
        tracing::warn!(
            confirm_deadline,
            "recovery attempt was not confirmed by a pong within the deadline, continuing escalation"
        );
    }

    // If the WebView has been classified as broken (unrecoverable), skip page-level
    // recovery and go straight to the rebuild/restart path.
    if state.is_webview_broken() {
        attempt_escalated_recovery(app, window_visible, window_exists).await;
        return;
    }

    let Some(window) = window else {
        tracing::warn!("heartbeat watchdog: main window not found, escalating to rebuild");
        // Window gone — treat as broken and try to rebuild.
        state.mark_webview_broken();
        attempt_escalated_recovery(app, window_visible, window_exists).await;
        return;
    };

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
        // Page-level reload has been attempted RECOVERY_CIRCUIT_THRESHOLD times
        // without receiving a pong. This strongly suggests the WebView is in an
        // unrecoverable state (e.g. reload() returns Ok but the operation fails
        // asynchronously in the wry event loop with HRESULT 0x8007139F).
        // Escalate to window rebuild instead of waiting passively.
        tracing::warn!(
            streak,
            "page reload exhausted without pong, escalating to window rebuild"
        );
        state.mark_webview_broken();
        state.set_webview_alive(false);
        attempt_escalated_recovery(app, window_visible, window_exists).await;
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
                attempt_escalated_recovery(app, window_visible, window_exists).await;
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
/// the rebuild budget is exhausted, fall back to a full app restart.
///
/// `window_visible`/`window_exists` are the values sampled once by the calling
/// check — re-querying here would let the reload/rebuild/restart decisions of
/// one recovery pass disagree with each other.
async fn attempt_escalated_recovery(
    app: &tauri::AppHandle,
    window_visible: bool,
    window_exists: bool,
) {
    if app_is_terminating(app) {
        tracing::debug!("explicit exit/restart in progress, skipping escalated recovery");
        return;
    }

    let state = app.state::<HeartbeatWatchdogState>();

    // Claim the recovery slot until the confirm deadline. A previous attempt
    // that is still within its deadline blocks new attempts; an expired one
    // does not — recovery can always make progress.
    let now = now_unix_millis();
    let deadline = now.saturating_add(RECOVERY_CONFIRM_TIMEOUT.as_millis() as u64);
    if !state.try_claim_recovery_confirm(now, deadline) {
        tracing::debug!("escalated recovery already awaiting confirmation");
        return;
    }

    // A pong may have arrived between the caller's staleness snapshot and this
    // claim; never destroy a WebView that has just proven itself alive.
    if now.saturating_sub(state.snapshot().last_pong_unix_ms) <= PONG_TIMEOUT.as_millis() as u64 {
        state.clear_recovery_confirm_deadline();
        tracing::info!("pong arrived during escalation, aborting recovery");
        return;
    }

    // Check rebuild budget.
    let mut rebuild_exhausted = false;
    if state.try_bump_rebuild_count() {
        tracing::warn!(window_visible, "attempting main window rebuild");
        match rebuild_main_window(app, window_visible) {
            Ok(()) => {
                tracing::info!(
                    confirm_timeout_s = RECOVERY_CONFIRM_TIMEOUT.as_secs(),
                    "main window rebuilt, waiting for frontend pong to confirm recovery"
                );
                // The claimed confirm deadline stays set: a pong clears it,
                // otherwise the next check after the deadline escalates again.
                return;
            }
            Err(err) => {
                tracing::error!(error = %err, "main window rebuild failed");
                // Fall through to app restart.
            }
        }
    } else {
        rebuild_exhausted = true;
    }

    // A full restart tears down the gateway and kills in-flight upstream
    // requests. Never do that behind the user's back: while a window exists
    // but is hidden, wait for it to be shown (`on_main_window_shown` runs an
    // immediate check then). A MISSING window is always restartable — it can
    // never be "shown", so deferring would leave a permanent headless zombie.
    if window_exists && !window_visible {
        // Nothing is awaiting pong confirmation — release the slot so the
        // show-triggered check can escalate immediately instead of stalling
        // on a deadline that guards no action.
        state.clear_recovery_confirm_deadline();
        if state.should_log_deferred_restart(now) {
            tracing::warn!(
                rebuild_exhausted,
                "webview broken while window is hidden; deferring app restart until the window is shown"
            );
        }
        return;
    }

    if rebuild_exhausted {
        tracing::warn!(
            max = REBUILD_MAX_ATTEMPTS,
            "consecutive window rebuilds went unconfirmed, escalating to app restart"
        );
    }

    // Final fallback: full app restart with storm protection.
    escalate_to_app_restart(app).await;
}

/// Destroy the current main window and recreate it from the tauri.conf.json
/// window config (so titleBarStyle/hiddenTitle/etc. survive recovery), with
/// only visibility overridden: a window hidden in the tray is rebuilt hidden,
/// so background recovery never pops a window or steals focus.
fn rebuild_main_window(app: &tauri::AppHandle, show: bool) -> Result<(), AppError> {
    // Destroy old window if it still exists.
    if let Some(old_window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        tracing::info!(show, "destroying old main window");
        if let Err(err) = old_window.destroy() {
            tracing::warn!(error = %err, "failed to destroy old main window, continuing with rebuild");
        }
    }

    // Small delay to allow the old window resources to be released.
    std::thread::sleep(Duration::from_millis(100));

    let map_build_err = |e: tauri::Error| {
        AppError::new(
            "WINDOW_REBUILD_FAILED",
            format!("failed to build window: {e}"),
        )
    };

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW_LABEL)
        .cloned();

    let new_window = match window_config {
        Some(mut config) => {
            config.visible = show;
            tauri::webview::WebviewWindowBuilder::from_config(app, &config)
                .map_err(map_build_err)?
                .build()
                .map_err(map_build_err)?
        }
        None => {
            // No config entry for the main window (should not happen) —
            // fall back to a minimal window rather than giving up.
            let url = tauri::WebviewUrl::App("index.html".into());
            tauri::webview::WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, url)
                .title("AIO Coding Hub")
                .inner_size(1500.0, 900.0)
                .visible(show)
                .build()
                .map_err(map_build_err)?
        }
    };
    crate::app::window_chrome::apply_main_window_chrome(&new_window);

    if show {
        let _ = new_window.show();
        let _ = new_window.unminimize();
        let _ = new_window.set_focus();
    }

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
        app.state::<HeartbeatWatchdogState>()
            .disable_auto_recovery();
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
    let marker_ts = read_restart_marker_timestamp(&path)?;
    let now = now_unix_millis();
    Some(now.saturating_sub(marker_ts))
}

fn read_restart_marker_timestamp(path: &std::path::Path) -> Option<u64> {
    let bytes = read_file_with_max_len(path, RESTART_MARKER_MAX_BYTES).ok()?;
    let content = std::str::from_utf8(&bytes).ok()?;
    content.trim().parse().ok()
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

    #[tokio::test]
    async fn heartbeat_interval_delays_missed_ticks() {
        assert_eq!(
            heartbeat_interval().missed_tick_behavior(),
            tokio::time::MissedTickBehavior::Delay
        );
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
        // Budget stays exhausted no matter how much time passes — only a pong
        // (proof of a live WebView) re-arms it, never the wall clock.
        assert!(!state.try_bump_rebuild_count());
        assert!(!state.try_bump_rebuild_count());

        state.record_pong();
        assert!(state.try_bump_rebuild_count());
    }

    #[test]
    fn recovery_confirm_deadline_prevents_concurrent_recovery_until_expiry() {
        let state = HeartbeatWatchdogState::default();
        let now = 10_000u64;
        let deadline = now + RECOVERY_CONFIRM_TIMEOUT.as_millis() as u64;

        // First claim succeeds.
        assert!(state.try_claim_recovery_confirm(now, deadline));
        assert_eq!(state.recovery_confirm_deadline_unix_ms(), deadline);

        // A concurrent claim before the deadline is rejected.
        assert!(!state.try_claim_recovery_confirm(now + 1_000, deadline + 1_000));

        // Once the deadline passes, the slot can be reclaimed — the previous
        // unconfirmed attempt can never deadlock recovery.
        let later = deadline + 1;
        assert!(state.try_claim_recovery_confirm(later, later + 90_000));
        assert_eq!(state.recovery_confirm_deadline_unix_ms(), later + 90_000);
    }

    #[test]
    fn pong_clears_recovery_confirm_deadline() {
        let state = HeartbeatWatchdogState::default();
        assert!(state.try_claim_recovery_confirm(1_000, 91_000));

        state.record_pong();
        assert_eq!(state.recovery_confirm_deadline_unix_ms(), 0);
    }

    #[test]
    fn pong_timeout_matrix_covers_visibility_grace_and_startup() {
        // Settled visible window: strict threshold.
        assert_eq!(pong_timeout_for(true, false, true), PONG_TIMEOUT);
        // Hidden window: lenient threshold once the frontend has ponged.
        assert_eq!(pong_timeout_for(false, false, true), PONG_TIMEOUT_HIDDEN);
        // Freshly shown window: silence accrued while hidden gets the lenient
        // threshold for one heartbeat round-trip.
        assert_eq!(pong_timeout_for(true, true, true), PONG_TIMEOUT_HIDDEN);
        // Before the first pong (slow start_minimized boot): startup allowance.
        assert_eq!(
            pong_timeout_for(false, false, false),
            STARTUP_PONG_TIMEOUT_HIDDEN
        );
        assert_eq!(
            pong_timeout_for(true, true, false),
            STARTUP_PONG_TIMEOUT_HIDDEN
        );
        // Visible settled window recovers fast even before the first pong
        // (startup white screens must still be repaired quickly).
        assert_eq!(pong_timeout_for(true, false, false), PONG_TIMEOUT);
        assert!(PONG_TIMEOUT_HIDDEN > PONG_TIMEOUT);
        assert!(STARTUP_PONG_TIMEOUT_HIDDEN > PONG_TIMEOUT_HIDDEN);
    }

    #[test]
    fn recently_shown_grace_window() {
        let state = HeartbeatWatchdogState::default();
        assert!(!state.recently_shown(now_unix_millis()));

        let now = 100_000u64;
        state.note_window_shown(now);
        assert!(state.recently_shown(now + RECENTLY_SHOWN_GRACE.as_millis() as u64 - 1));
        assert!(!state.recently_shown(now + RECENTLY_SHOWN_GRACE.as_millis() as u64));
    }

    #[test]
    fn first_pong_flips_has_received_pong() {
        let state = HeartbeatWatchdogState::default();
        assert!(!state.has_received_pong());

        state.record_pong();
        assert!(state.has_received_pong());
    }

    #[test]
    fn deferred_restart_log_is_throttled() {
        let state = HeartbeatWatchdogState::default();
        assert!(state.should_log_deferred_restart(100_000));
        assert!(!state.should_log_deferred_restart(100_000 + 60_000));
        assert!(state.should_log_deferred_restart(100_000 + 60_001));
    }

    #[test]
    fn clear_recovery_backoff_resets_schedule_but_keeps_streak() {
        let state = HeartbeatWatchdogState::default();
        let streak = state.bump_recovery_streak();
        state.schedule_next_recovery(streak, 1_000);
        assert!(state.snapshot().next_recovery_allowed_unix_ms > 0);

        state.clear_recovery_backoff();
        assert_eq!(state.snapshot().next_recovery_allowed_unix_ms, 0);
        // Streak is preserved so repeated failures still escalate.
        assert_eq!(state.bump_recovery_streak(), streak + 1);
    }

    #[test]
    fn auto_recovery_disabled_lifecycle() {
        let state = HeartbeatWatchdogState::default();
        assert!(!state.is_auto_recovery_disabled());

        state.disable_auto_recovery();
        assert!(state.is_auto_recovery_disabled());
    }

    #[test]
    fn restart_marker_timestamp_rejects_oversized_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("restart_marker");
        std::fs::write(&path, vec![b'1'; RESTART_MARKER_MAX_BYTES + 1]).expect("write marker");

        assert_eq!(read_restart_marker_timestamp(&path), None);
    }
}
