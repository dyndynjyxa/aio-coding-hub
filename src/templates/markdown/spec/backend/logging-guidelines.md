# Logging Guidelines

> How logging is done in this project.

---

## Overview

Logging should make startup, gateway, and integration failures diagnosable
without leaking secrets.

---

## Log Levels

- `debug`: high-frequency flow details and internal decisions
- `info`: successful state transitions worth auditing
- `warn`: degraded or recoverable behavior
- `error`: user-visible failures, startup failure, or integration failure

---

## Structured Logging

- Include stable identifiers such as `trace_id`, `cli_key`, `provider_id`, and
  `error_code` when they exist.
- Prefer structured fields over string-only logs for gateway and command flows.
- When logging cleanup or launcher behavior, log the lifecycle event, not the
  file contents.

---

## What to Log

- Startup and shutdown state transitions
- OAuth / opener failures
- Gateway circuit and routing transitions
- Gateway request/response transformations that change semantics should be recorded in `special_settings_json`
  (example marker: `claude_auth_injection`)
- Explicit cleanup failures that could leave drift or stale files behind

---

## What NOT to Log

- API keys, bearer tokens, refresh tokens, or temp config file contents
- Full prompt/request bodies unless explicitly sanitized for diagnostics
- Secrets copied into launcher scripts or temp JSON

---

## Internal Gateway Helper Traffic

Requests such as Claude `/v1/messages/count_tokens`, warmup probes, and other
gateway-generated helper traffic are **infra traffic by default**, not normal
user-visible request history.

- Do not treat `excluded_from_stats=true` as meaning "safe to still show in the
  default UI". Visibility and statistics are separate contracts.
- Infra-only helper traffic should not emit the normal
  `gateway:request_start`, `gateway:attempt`, or `gateway:request` events used
  by overview cards, logs pages, and task-complete heuristics.
- Infra-only helper traffic should not be written into the default request-log
  list unless there is an explicit diagnostic requirement.
- If diagnostic retention is required, route it to a debug-only surface or a
  separately labeled log path so the main request history stays focused on
  user-visible work.

## Lifecycle-Backed Request History

If a CLI needs vendor-style "in progress" request history, the backend must own
that lifecycle explicitly instead of asking the frontend to infer it from
realtime events.

- Create the user-visible request-log row at request start with the final
  `trace_id`, then update that same row when the request finishes.
- For Claude, only `/v1/messages` participates in this lifecycle. Helper paths
  and probe traffic must still stay out of the default history.
- Do not let the frontend render the same request twice through two different
  contracts such as `gateway:request_start` cards plus `request_logs` rows.
- If request-log rows are updated in place, the consumer side must support
  seeing those updates. An `id > afterId` poll alone is insufficient while any
  row is still in progress.

## Filtered Providers vs Failed Upstreams

Provider gate decisions such as circuit-open, cooldown, and rate-limit skips
are not the same thing as an upstream request failure.

- Keep gate-filtered attempts in `attempts_json` / `error_details_json` so the
  operator can see why a provider was skipped.
- Do not finalize a request as `GW_UPSTREAM_ALL_FAILED` when every recorded
  attempt is only a pre-send skip. Use `GW_ALL_PROVIDERS_UNAVAILABLE` instead.
- Preserve retry-after semantics for unavailable states so repeated CLI retries
  hit `RecentErrorCache` instead of generating a new request-log row every few
  hundred milliseconds.
- Review Home/log surfaces after changing terminal error families. A logging bug
  here often looks like a frontend duplicate, even when the real issue is
  backend classification drift.
