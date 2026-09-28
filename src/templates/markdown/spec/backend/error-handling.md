# Error Handling

> How errors are handled in this project.

---

## Overview

Backend errors should fail early, keep context, and avoid turning external
integration failures into silent hangs.

---

## Error Types

- Domain/infra code should use the shared `AppError` / `AppResult` path.
- Tauri commands adapt those errors into `Result<T, String>` for the desktop
  boundary.
- Gateway-facing failures should preserve the stable error-code contract.

---

## Error Handling Patterns

- If a flow depends on an external side effect, do not discard that error and
  keep waiting on the next step.
  Example: browser open failure in OAuth should return immediately.
- If a settings write changes the address or endpoint used by a running
  subsystem, fail closed on rebind failure. Do not publish, sync, or announce a
  new listener value until the runtime path actually serves it.
- Cleanup-sensitive flows should fail closed.
  If temp files are written for a launcher, define rollback on partial failure
  and explicit cleanup before any `exec` handoff.
- Prefer keeping blocking IO inside `blocking::run` or other clearly marked
  boundaries so failures surface in one place.
- Desktop bridge commands should deny unknown URL schemes, unknown filesystem
  roots, or unsupported actions in Rust and return an actionable error to the
  frontend.
- If the frontend only needs success/failure, metadata, or a masked preview, do
  not send plaintext credentials or other sensitive secrets back across the IPC
  boundary.

---

## API Error Responses

- Tauri command errors should be actionable and include enough context for the
  frontend to stop or recover cleanly.
- Gateway errors should preserve the canonical `GW_*` code when applicable.

---

## Common Mistakes

- Ignoring `open_url` / OS integration failures and then waiting for a callback
  that can never happen.
- Relying on shell lifecycle assumptions for secret cleanup instead of explicit
  removal.
- Returning generic internal errors after discarding the real boundary failure.
- Returning or logging sensitive credentials to the renderer because it is
  convenient for a settings form or copy action.
- Saving a new listener address to disk, then letting WSL/CLI sync use it while
  the running gateway is still bound to the old socket.

---

## Provider-Health Neutral Failures

Not every gateway failure should mutate provider health.

- Internal helper requests such as Claude `/v1/messages/count_tokens` should be
  treated as **provider-health neutral** by default.
- Provider-health neutral failures must not increment circuit failure counts or
  trigger provider cooldown just because the helper request failed.
- When a route is special-cased, keep timeout, connect-error, upstream-status,
  and post-response read-error branches aligned. Do not fix only one failure
  branch and leave the others still mutating provider state.
- If product requirements ever decide that a helper route should affect
  provider health, document that rule explicitly in the gateway contract
  instead of relying on shared fallback behavior.

## macOS Notification Audio Isolation

### 1. Scope / Trigger
- Notification playback uses `src-tauri/src/app/notification_sound.rs`.
- cpal 0.16.0 CoreAudio enumeration writes through shared references to native
  output parameters. The 0.60.18 release binary passed `0x4` as the device-list
  output buffer and crashed with SIGSEGV. Rust error/panic handlers cannot
  contain this native process failure.

### 2. Signatures
- IPC: `desktop_notification_play_sound() -> Result<bool, String>`.
- Worker entry: `play_notification_sound() -> Result<(), String>`.
- macOS player: `play_embedded_sound_blocking() -> AppResult<()>`.

### 3. Contracts
- macOS invokes the absolute system executable `/usr/bin/afplay` directly on
  the bundled MP3; no shell, renderer-provided path, or in-process rodio fallback.
- IPC success means the worker was spawned. Later playback errors are warnings.
- Keep the unique temporary MP3 alive until the player exits, then remove it.
- Poll for completion; after 10 seconds, kill and reap the player. Also terminate
  and reap on wait errors. Null child stdio avoids inherited input/output pipes.
- Windows uses WASAPI and Linux uses ALSA through rodio; neither compiles the
  faulty CoreAudio module. Their device-enumeration output arguments were checked
  for this shared-reference issue, not exhaustively audited for all native faults.
- Keep rodio restricted to non-macOS targets. Do not restore WebView audio:
  native playback also avoids the media-key capture regression from PR #251.

### 4. Validation & Error Matrix
| Condition | Behavior |
| --- | --- |
| Temporary file creation/write fails | `NOTIFICATION_SOUND_TEMPFILE_FAILED` / `NOTIFICATION_SOUND_WRITE_FAILED` |
| Player cannot spawn | `NOTIFICATION_SOUND_PLAYER_SPAWN_FAILED`; remove temporary file |
| Nonzero or signaled exit | `NOTIFICATION_SOUND_PLAYER_FAILED`; remove temporary file |
| Player exceeds deadline | `NOTIFICATION_SOUND_PLAYER_TIMEOUT`; kill, reap, remove file |
| Waiting fails | `NOTIFICATION_SOUND_PLAYER_WAIT_FAILED`; kill, reap, remove file |
| Cleanup fails | Structured warning with error; preserve playback result |

### 5. Good/Base/Bad Cases
- Base: play the unchanged bundled MP3 asynchronously.
- Good: a failed player leaves the application alive and later playback succeeds.
- Bad: catch a Rust panic around an in-process native audio call and claim that
  this protects the application from SIGSEGV.

### 6. Tests Required
- Assert exact asset handoff, paths with spaces, and file cleanup after success,
  spawn failure, nonzero/signaled exit, and timeout.
- Assert a timed-out child terminates and is reaped.
- Keep physical-device playback in an explicit opt-in macOS smoke test.
- Inspect per-target Cargo graphs; distinguish source review from real
  Windows/Linux hardware testing in the validation report.

### 7. Wrong vs Correct
- Wrong: macOS notification worker calls
  `rodio::OutputStreamBuilder::open_default_stream()` inside the app.
- Correct: macOS worker delegates to the bounded system-player process; compile
  the rodio implementation only under `cfg(not(target_os = "macos"))`.
