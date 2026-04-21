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
