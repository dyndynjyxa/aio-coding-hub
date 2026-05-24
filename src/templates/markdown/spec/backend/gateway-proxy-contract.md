# Gateway Proxy Contract

> What the local gateway does to requests/responses, and what is (not) "passthrough".

---

## Why This Exists

The gateway is **not** a dumb TCP tunnel. It is an application-level proxy that:

- selects providers (failover, circuit breaker, session stickiness)
- rewrites authentication (fail-closed; never leak client-sent tokens upstream)
- applies compatibility rectifiers (Claude thinking fixes, metadata injection, etc.)
- optionally bridges protocols (CX2CC: Anthropic → OpenAI Responses and back)

This document makes those mutations explicit, so "透传/不透传" is not a guess.

---

## High-Level Request Lifecycle (Claude/Codex/Gemini)

```
Client CLI
  → proxy handler (entry / guards / introspection)
    → provider selection (sort mode + session binding)
      → failover loop
        → per-attempt rewrite (auth + protocol bridge + rectifiers)
          → upstream request
            → response handling (stream/non-stream, response fixer, translation)
```

Primary code entrypoints:

- Entry handler: `src-tauri/src/gateway/proxy/handler/mod.rs`
- Failover loop: `src-tauri/src/gateway/proxy/handler/failover_loop/mod.rs`
- Auth injection helper: `src-tauri/src/gateway/util.rs`

---

## Provider Selection and Session Binding

Session binding is a preference, not an authorization bypass. A bound provider
can only be reused after the current request has built its eligible provider
candidate list.

Contract:

- Always load the eligible providers for the active or session-bound route mode
  first. For the default provider route, eligibility is `providers.enabled = 1`.
  For a sort-template route, eligibility is `sort_mode_providers.enabled = 1`
  and must not depend on `providers.enabled`.
- Never reinsert a session-bound provider that is missing from the current
  candidate list. Missing means disabled, removed from the mode, deleted, or no
  longer valid for this CLI key.
- If the bound provider is missing, clear the stale session binding and let the
  failover loop continue with the remaining candidates.
- Provider create/save/toggle/delete and sort-template membership flows that
  change routing eligibility must clear the running gateway's route runtime
  state for that CLI key after the database write succeeds. Route runtime state
  includes session bindings plus recent `GW_ALL_PROVIDERS_UNAVAILABLE` errors;
  otherwise the recent-error cache can short-circuit the next request before it
  reaches failover/logging, even though a newly enabled provider is now
  eligible.
- If the bound provider is still present, run circuit-breaker gating before
  applying session preference. An open or cooling-down bound provider must not
  block fallback to later candidates.
- Forced provider selection is a separate explicit override. It must not be
  combined with stale session-binding reinsertion.

Codex-specific note:

- Codex sessions can be derived from `prompt_cache_key`, `previous_response_id`,
  metadata, or deterministic request fingerprints. Treat them exactly like other
  session ids: they preserve continuity only within the current eligible provider
  set.
- Regression tests for Codex provider selection must cover this case: an OAuth
  Team provider is bound to the session, then becomes disabled and circuit-open,
  while a later API-key provider is enabled. The gateway must choose the later
  provider without requiring a manual circuit reset.

### Route Runtime State Invalidation

#### 1. Scope / Trigger

- Trigger: a persisted provider or sort-template change can alter the provider
  candidates, candidate order, credentials, upstream endpoint, or active route
  view for a CLI key.

#### 2. Signatures

- App helper:
  `app_gateway_clear_cli_route_runtime_state(app, cli_key) -> GatewayRouteRuntimeClearResult`.
- Runtime helper:
  `GatewayRuntime::clear_cli_route_runtime_state(cli_key) -> GatewayRouteRuntimeClearResult`.
- Result fields: `cleared_sessions: usize`, `cleared_recent_errors: usize`.

#### 3. Contracts

- Clear the target CLI's session bindings after the database write succeeds.
- Clear recent unavailable-error cache entries in the running gateway. The cache
  is currently process-wide, so route-state invalidation may clear entries for
  other CLI keys too.
- Do not reset circuit-breaker state here. Circuit reset remains an explicit
  user/admin operation; the fix is to let newly eligible providers be tried.

#### 4. Validation & Error Matrix

- No running gateway -> return zero counts and keep the persisted change.
- Provider not found during delete -> return the existing DB_NOT_FOUND error and
  do not clear runtime state.
- Cache clear failure is not expected; poisoned locks must use the shared
  lock-or-recover path already used by gateway runtime state.

#### 5. Good/Base/Bad Cases

- Good: enable a new Codex provider after the only previous provider cached
  `GW_ALL_PROVIDERS_UNAVAILABLE`; the next request reaches failover and logs.
- Base: reordering providers clears sticky order and recent unavailable cache.
- Bad: only clear session bindings; recent-error cache still short-circuits the
  next request before request-start/log writes.

#### 6. Tests Required

- Unit-test the runtime helper clears target CLI sessions and recent-error cache
  together.
- Regression-test default provider routing and active sort-template routing
  separately; sort templates must not depend on `providers.enabled`.

#### 7. Wrong vs Correct

Wrong: `provider_set_enabled` calls only `clear_cli_session_bindings(cli_key)`.

Correct: provider and sort-template eligibility changes call the route runtime
state helper so sessions and recent unavailable-error cache are invalidated
together.

---

## What Is Always Modified (Not Passthrough)

### 1) Authentication (Fail-Closed)

The gateway **always clears** client-sent auth headers before sending upstream, then injects
credentials based on the selected provider:

- clears: `Authorization`, `x-api-key`, `x-goog-api-key`, `x-goog-api-client`
- injects:
  - Codex: `Authorization: Bearer <provider_credential>`
  - Claude (API key mode): `x-api-key: <provider_key>` (default)
  - Claude (OAuth mode): `Authorization: Bearer <access_token>` + required Claude headers via adapter
  - Gemini: `Authorization: Bearer <oauth_token>` OR `x-goog-api-key: <api_key>` depending on credential shape

**Rationale**: prevent accidental token leakage when clients send their own credentials.

### 2) Hop-by-Hop Headers

Hop-by-hop/proxy headers are stripped before forwarding (HTTP correctness):

- e.g. `connection`, `proxy-authorization`, `transfer-encoding`, `upgrade`, etc.

### 3) Content-Encoding When Body Mutates

If the gateway rewrites the request body, it will remove `Content-Encoding`
to avoid sending a compressed body with stale encoding metadata.

---

## Conditional Mutations (Depends on CLI / Provider / Response)

### Claude: API Key vs OAuth vs CX2CC

#### A) API Key mode (`auth_mode=api_key`, `source_provider_id=NULL`)

- Default auth scheme: `x-api-key`
- Fallback auth scheme (once): if upstream returns **401/403**, retry with:
  `Authorization: Bearer ...` (and remove `x-api-key`) to support strict relays.
- Observability: emits `special_settings_json` entries of type `claude_auth_injection`.

This is the only place where "same key, different auth header" can occur.

#### B) OAuth mode (`auth_mode=oauth`)

Uses the OAuth adapter to inject upstream headers (Claude-specific beta flags, UA/stainless headers, etc).

Important: OAuth mode should **not** send `x-api-key` upstream.

#### C) CX2CC bridge (`source_provider_id=...`)

CX2CC is **explicitly non-passthrough**:

- request: translate Anthropic Messages JSON into OpenAI Responses JSON
- routing: upstream becomes the **source** (Codex) provider, not the Claude bridge provider
- headers: strip Claude-specific headers when bridging `claude → codex`
- response: translate OpenAI responses/SSE back into Anthropic-shaped responses/SSE

This mode is isolated by `source_provider_id` and does not modify non-bridge providers.

---

## Body Rectifiers

The gateway may rewrite request JSON in a small number of controlled situations:

- **Billing header rectifier**: removes `x-anthropic-billing-header: ...` blocks from `system`
  (some non-Anthropic upstreams reject them with 400).
- **Metadata user_id injection**: injects `metadata.user_id` for `/v1/messages` when missing.
- **Model mapping**: rewrites model name based on provider slot config (body/query/path).
- **Thinking rectifiers**: after upstream 400 indicates a thinking/signature/budget issue,
  rewrites the request and retries (signature fields, thinking blocks, budget tokens).
- **Codex previous-response rectifier**: after a Codex upstream returns 400/404
  explicitly indicating the supplied `previous_response_id` is missing/invalid,
  removes only `previous_response_id` and retries the same provider once. This
  stale provider-scoped continuation error must not increment circuit failure
  counts or trigger cooldown for the newly selected provider.

Each rectifier must:

- be guarded (enabled flag + path checks)
- record a `special_settings_json` entry describing what happened (without secrets)
- have unit tests for edge cases (empty arrays, missing fields, etc.)

---

## Observability Contract

For troubleshooting "why did this request fail", the gateway relies on:

- request logs (`request_logs`, `request_attempt_logs`)
- structured gateway logs (`emit_gateway_log`)
- `special_settings_json` (per request) to record transformations

New/important `special_settings_json` markers include:

- `claude_auth_injection` (x-api-key default; 401/403 → Bearer fallback)
- `billing_header_rectifier`
- `claude_metadata_user_id_injection`
- `claude_model_mapping`
- `thinking_signature_rectifier`
- `thinking_budget_rectifier`
- `codex_session_id_completion`
- `codex_previous_response_id_rectifier`

Never include secrets (API keys, bearer tokens, refresh tokens) in any of these surfaces.

### Provider Gates, Skipped Attempts, and Terminal Logs

Provider gates run before an upstream request is sent. Circuit-open, cooldown,
and provider limit checks may append a skipped `FailoverAttempt` for diagnostics,
but that skipped attempt is not a real provider request.

Required behavior:

- `outcome=skipped` means the provider was not called
- skipped attempts may appear in `attempts_json` and route diagnostics
- all gate-only skips must finalize as `GW_ALL_PROVIDERS_UNAVAILABLE`
- the terminal request log should describe provider unavailability, not a failed
  upstream response from the skipped provider
- retry-after/recent-error cache should short-circuit repeated identical
  unavailable requests where possible

Forbidden pattern:

- Do not let skipped circuit-breaker attempts look like new upstream attempts in
  summary/detail UI.
- Do not use the last skipped provider as the user-facing final provider without
  also exposing the terminal unavailable state.

### Codex SSE Tail Errors After Completion

Codex `/v1/responses` streams can complete successfully from the user's
perspective and still surface a late read error while the gateway is draining
the tail. If the gateway has already observed stream output plus
`response.completed` and/or usage, that tail read error must not be rewritten
into `GW_STREAM_ERROR`/502 just because the socket closed during teardown.

Contract:

- Keep the terminal request log aligned with the user-visible stream result.
- Treat late tail read errors as transport noise once completion/usage has been
  observed for a 2xx Codex responses stream.
- Preserve the raw stream-read failure in debug logs or attempt details if
  needed, but do not surface it as the final user-facing request failure.
- Verify this path separately from terminal marker detection; a stream can have
  no error marker and still fail late during body drain.
