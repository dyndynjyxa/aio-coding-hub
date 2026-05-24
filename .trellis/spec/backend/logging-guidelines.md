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

## CLI Proxy Status Diagnostics

CLI proxy status is a configuration diagnostic signal, not proof that current
traffic can or cannot reach the gateway.

- Request logs prove that a request reached the gateway; they do not prove
  `applied_to_current_gateway`.
- For status-card drift warnings, trace the full chain:
  `HomeWorkStatusCard -> useCliProxy -> cliProxyStatusAll -> cli_proxy_status_all -> cli_proxy::status_all`.
- Keep `enabled` and `applied_to_current_gateway` separate. `enabled` is read
  from the CLI proxy manifest; `applied_to_current_gateway` is recomputed from
  the current target files and the running gateway origin.
- For Codex, inspect `codex_home_mode` and `codex_home_override` before reading
  `~/.codex`. A stale custom Codex home can make the status card report drift
  even when the real Codex CLI process is using a valid `~/.codex/config.toml`.
- Codex applied checks depend on the resolved `config.toml` base URL/provider.
  In default API-key-placeholder mode they also require a readable `auth.json`
  with `OPENAI_API_KEY`; in Codex OAuth compatible proxy mode they must not
  read or require `auth.json`, and should instead fail drift if the root
  `preferred_auth_method = "apikey"` proxy preference remains.
- Repair actions write to the currently resolved Codex home. Surface the target
  path or a reason code before telling users to repair a drifted Codex proxy.

### Scenario: Codex OAuth compatible CLI proxy mode

#### 1. Scope / Trigger

- Trigger: Codex CLI proxy can run in a compatibility mode that preserves
  Codex's own ChatGPT/OAuth `auth.json` instead of replacing it with AIO's
  API-key placeholder.

#### 2. Signatures

- Persisted setting: `AppSettings.codex_oauth_compatible_proxy_mode: bool`.
- Settings update field: `SettingsUpdate.codexOauthCompatibleProxyMode:
  boolean | null`.
- CLI proxy target kinds:
  - default mode: `codex_config_toml`, `codex_auth_json`
  - OAuth compatible mode: `codex_config_toml` only

#### 3. Contracts

- OAuth compatible apply/sync writes `config.toml` only.
- OAuth compatible apply/sync must not create, backup, parse, write, delete, or
  restore `auth.json`.
- OAuth compatible `config.toml` writes:
  ```toml
  model_provider = "aio"

  [model_providers.aio]
  name = "aio"
  base_url = "http://127.0.0.1:<port>/v1"
  wire_api = "responses"
  requires_openai_auth = true
  ```
- Do not write `preferred_auth_method = "chatgpt"`. Remove only the old
  AIO-owned root `preferred_auth_method = "apikey"` when switching the config
  into OAuth compatible mode; preserve other user values.

#### 4. Validation & Error Matrix

- `config.toml` missing current `<base_origin>/v1` -> `applied_to_current_gateway = false`.
- provider table missing `model_providers.aio` / remote-compaction provider ->
  `applied_to_current_gateway = false`.
- OAuth compatible mode with root `preferred_auth_method = "apikey"` still
  present -> `applied_to_current_gateway = false`.
- Default mode with unreadable `auth.json` or missing `OPENAI_API_KEY` ->
  `applied_to_current_gateway = false`.

#### 5. Good/Base/Bad Cases

- Good: OAuth compatible mode writes the AIO provider table, keeps existing
  OAuth tokens untouched, and reports applied without reading `auth.json`.
- Base: default mode keeps existing behavior: backup both Codex files, write the
  API-key placeholder auth file, and restore via merge on disable.
- Bad: OAuth compatible disable restores or removes `auth.json`; this can
  destroy a user's Codex OAuth login and must not happen.

#### 6. Tests Required

- Builder test removes only `preferred_auth_method = "apikey"` and preserves a
  user `preferred_auth_method = "chatgpt"`.
- Enable test proves OAuth compatible mode writes config only and does not
  create `auth.json`.
- Status test proves OAuth compatible mode does not need `auth.json`, but
  reports drift when the old API-key preference remains.
- Mode-switch test proves OAuth compatible -> default mode adds an auth backup
  before writing placeholder auth.
- Disable test proves OAuth compatible mode restores config and leaves
  `auth.json` unchanged.

#### 7. Wrong vs Correct

Wrong:

```toml
preferred_auth_method = "chatgpt"
```

Correct:

```toml
model_provider = "aio"

[model_providers.aio]
requires_openai_auth = true
```

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
