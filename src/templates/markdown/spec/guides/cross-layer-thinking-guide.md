# Cross-Layer Thinking Guide

> **Purpose**: Think through data flow across layers before implementing.

---

## The Problem

**Most bugs happen at layer boundaries**, not within layers.

Common cross-layer bugs:
- API returns format A, frontend expects format B
- Database stores X, service transforms to Y, but loses data
- Multiple layers implement the same logic differently
- Tauri command signatures drift from frontend wrappers after one side changes

---

## Before Implementing Cross-Layer Features

### Step 1: Map the Data Flow

Draw out how data moves:

```
Source → Transform → Store → Retrieve → Transform → Display
```

For each arrow, ask:
- What format is the data in?
- What could go wrong?
- Who is responsible for validation?

### Step 2: Identify Boundaries

| Boundary | Common Issues |
|----------|---------------|
| API ↔ Service | Type mismatches, missing fields |
| Service ↔ Database | Format conversions, null handling |
| Backend ↔ Frontend | Serialization, date formats, command drift |
| Component ↔ Component | Props shape changes |

### Step 3: Define Contracts

For each boundary:
- What is the exact input format?
- What is the exact output format?
- What errors can occur?
- Which file owns the contract?

---

## Tauri IPC Contract Checklist

Use this checklist whenever a Tauri command is added or changed.

### Input shape

- Use a **single DTO struct** when a command carries more than 3 business fields.
- Prefer `#[serde(rename_all = "camelCase")]` on command DTOs so the JS side keeps a stable shape.
- Keep UI form models and IPC DTOs separate when the UI needs different naming or defaults.

### Output shape

- Return domain DTOs with explicit field ownership instead of ad-hoc JSON maps.
- When a command is part of the stable desktop contract, add `#[specta::specta]` and export bindings.
- If Rust exposes `i64` / `u64`, decide the TypeScript bigint strategy **explicitly** during export.

### Ownership

- The Tauri command layer owns IPC shape adaptation.
- The domain layer owns validation and persistence rules.
- The frontend service layer owns the final JS wrapper used by pages/hooks.
- Keep runtime command registration and Specta export coverage derived from one
  registry module. If those lists diverge, the desktop contract is already
  drifting even if tests still compile.
- Generated bindings only protect the commands and types they actually export.
  If Specta covers only a subset, document that boundary explicitly and keep
  service-layer contract tests for the remaining commands.
- Treat generated bindings as authoritative only when runtime code actually
  imports them or a generated wrapper sits directly under the service layer.
  Raw-file snapshot tests alone do not prevent handwritten runtime wrappers from
  drifting away from the exported contract.
- If a command intentionally stays outside Specta, keep one explicit owner file
  for the handwritten DTO on the frontend and add a targeted contract test that
  names the Rust command and the JS wrapper together.
- Keep runtime-only exceptions rare and named. In this project,
  `desktop_updater_download_and_install` is the known handwritten command path
  because it depends on a Tauri `Channel` callback.

---

## React Root Boundary Checklist

Use this when touching `src/main.tsx`, `src/App.tsx`, or global event wiring.

- Keep the root component **composition-only**: providers, router, toasts, boundaries.
- Move startup side effects into a dedicated hook such as `useAppBootstrap`.
- Keep route declarations in a dedicated module such as `src/app/AppRoutes.tsx`.
- Split unrelated synchronization work into separate effects instead of one “startup soup” effect.
- If root code needs to update runtime-only module singletons, pass one
  normalized snapshot into a runtime controller instead of calling several
  setters inline.

---

## Common Cross-Layer Mistakes

### Mistake 1: Implicit Format Assumptions

**Bad**: Assuming date format without checking

**Good**: Explicit format conversion at boundaries

### Mistake 2: Scattered Validation

**Bad**: Validating the same thing in multiple layers

**Good**: Validate once at the entry point

### Mistake 3: Leaky Abstractions

**Bad**: Component knows about database schema

**Good**: Each layer only knows its neighbors

### Mistake 4: Wide Tauri Command Signatures

**Bad**: Changing one positional field forces fragile updates across Rust, JS wrappers, and tests

**Good**: One request object, one stable export, one wrapper mapping layer

### Mistake 5: Gating Upstream Contracts on the Wrong Identity

**Bad**: A request enters as protocol A, gets translated to protocol B, but
post-translation helpers still gate on the original `cli_key`. Upstream-only
fields like `prompt_cache_key`, `session_id`, `cache_control`, or provider
metadata then disappear silently.

**Good**: After protocol translation, re-evaluate what the *actual upstream*
expects. Run upstream-specific completion/normalization on the translated
body/headers, and keep stable cache/session identifiers across the bridge.

Bridge/failover checklist:
- When routing changes protocol, list which fields must be preserved or
  re-derived for the new upstream contract.
- Do not gate upstream helpers only on the inbound identity if failover or
  bridge logic can switch protocol later.
- Rebuild or strip protocol-specific headers when the upstream protocol changes.
  Do not forward Claude-only headers into Codex/OpenAI backends, and make sure
  target-specific identity headers such as `User-Agent`, `originator`, and
  account identifiers are switched to the actual upstream.
- Verify translated headers/body still contain stable cache/session identifiers
  before the request is sent upstream.

### Mistake 6: Treating Generated Bindings as Broader Than They Are

**Bad**: Assume `src/generated/bindings.ts` is the authoritative desktop contract
while only a few commands are actually exported through Specta.

**Good**: Make it explicit which commands are protected by generated bindings and
which still rely on handwritten service wrappers plus targeted tests.

### Mistake 7: Fail-Open Settings Merges

**Bad**: If persisted settings cannot be read, silently fall back to defaults,
merge the next write against those defaults, and overwrite the user's original
config without an explicit recovery step.

**Good**: Treat unreadable persisted config as a blocking state for save flows,
or route through a visible recovery/import-reset path before writing anything
back to disk.

Config write checklist:
- Decide whether read failure should block writes, offer reset, or restore from
  backup. Do not let `unwrap_or_default()` make that choice implicitly.
- For multi-file config apply flows, parse and build every target output before
  writing the first file. A parse failure in a later file must not leave an
  earlier file partially switched to proxy/managed state.
- Log the failure with enough context to diagnose file corruption or migration
  drift.
- Add one test that proves a read failure does not silently erase unrelated
  fields on the next save.
- Add one test where the first target can be built but a later target fails to
  parse, proving no target file was modified.
- For app-owned settings files, persist the full managed snapshot. Do not drop
  keys just because they equal today's default.
- For third-party config bridges, document whether each field is explicitly
  managed or intentionally follows upstream defaults.
- If the target format supports explicit `false`, do not encode "disabled" as
  key deletion.
- Add upgrade-drift tests that distinguish explicit `false` from a missing key.

### Mistake 8: Letting Composition Roots Become Feature Hosts

**Bad**: Keep Tauri plugin wiring, startup recovery, gateway auto-start, WSL
bootstrapping, cleanup, and large command registration inside one root file or
one React bootstrap hook until every cross-layer change touches the same place.

**Good**: Keep roots composition-only. Once startup logic grows beyond one
feature area, split into dedicated registrars such as `command_registry`,
`startup/bootstrap`, or `platform_init` modules and let the root only compose
them.

Root-boundary checklist:
- If a root file owns app lifecycle plus feature logic plus platform branches,
  extract feature-specific startup modules.
- If the command registry grows, group command lists by feature and generate the
  final handler from smaller registrars.
- Review blast radius: adding one feature should not require editing unrelated
  startup branches.

### Mistake 12: Letting Root Bridges Drive Multiple Runtime Singletons Directly

**Bad**: A root bridge reads settings and directly calls
`setCacheAnomalyMonitorEnabled`, `setTaskCompleteNotifyEnabled`,
`setNotificationSoundEnabled`, and future runtime setters one by one.

**Good**: Normalize the query snapshot once and hand it to a runtime controller
that owns singleton fan-out, de-duplication, and future toggle growth.

Runtime-bridge checklist:
- The root hook should see one query snapshot and one controller call.
- The controller should own de-duplication and normalization.
- Runtime singleton setters should not leak into app bootstrap or route code.

### Mistake 7b: Treating Missing Managed Settings as a Stable Meaning

**Bad**: A settings writer omits fields that currently equal defaults, or a CLI
bridge uses `false => delete key`. A later release changes defaults and the same
persisted file is reinterpreted as a different user choice.

**Good**: Separate "explicitly managed value" from "follow upstream default" in
the storage contract. App-owned settings should persist full snapshots.
Third-party bridges should use deliberate per-field semantics instead of
reusing key deletion as a generic "off" state.

Upgrade-persistence checklist:
- For internal settings, missing keys should mean "legacy or corrupted data",
  not "safe to recompute from the latest defaults".
- For upstream-managed config files, define whether the product owns each field
  or intentionally defers to upstream defaults.
- If product intentionally supports "follow upstream default", model that as an
  explicit third state in the app contract instead of overloading `false`.
- Add at least one regression test that simulates a future default change and
  proves persisted user choices stay stable across upgrades.

### Mistake 13: Letting Tauri Commands Become Application Services

**Bad**: `commands/settings.rs` or `commands/cli_proxy.rs` owns persistence,
runtime rollback, gateway rebind, CLI sync, and session cleanup directly.

**Good**: Keep `commands/*` as IPC wrappers and move orchestration into
`app/*_service.rs` so the same service can be reused by startup flows, tests,
or future non-Tauri entrypoints.

### Mistake 9: Letting Event Names Bypass the Shared Contract

**Bad**: Define a shared `gatewayEventNames` map, but still add raw
`"gateway:*"` strings in feature modules.

**Good**: Subscribe through the shared event bus and central constants so
event-name changes fail in one place instead of silently drifting.

### Mistake 10: Letting Internal Helper Requests Leak Into User-Facing Observability

**Bad**: Treat internal helper traffic such as Claude
`/v1/messages/count_tokens`, warmup probes, or bridge housekeeping as if it
were a normal user request. The gateway then emits the usual
`request_start` / `attempt` / `request` events, writes default request-log
rows, and may even mutate provider health for traffic the user never actually
asked to inspect.

**Good**: Classify each request at the gateway boundary as either
user-visible or infra-only, then keep observability and provider-health side
effects aligned with that classification.

Internal helper checklist:
- Decide request visibility at handler entry, not later in the UI.
- If a route is infra-only, skip default `gateway:request_start`,
  `gateway:attempt`, `gateway:request`, and default request-log persistence.
- Do not let infra-only helper failures change provider cooldown / circuit
  state unless product requirements explicitly say they count toward provider
  health.
- If helper traffic must remain inspectable, expose it only through explicit
  diagnostics, not the default overview/log surfaces.
- If product wants vendor-style "in progress" logs, create one lifecycle row at
  request start and update it by `trace_id` on completion. Do not model that
  request as both a realtime card and a separate request-log record.
- When logs are updated in place, verify the frontend polling strategy can
  observe row updates. `afterId`-only polling misses status transitions on an
  existing row.

### Mistake 11: Treating Additive Analytics Fields as "Safe Enough" to Skip Contract Updates

**Bad**: Backend adds a new metrics field such as
`cache_creation_1h_input_tokens`, but frontend service types, view models, and
summary cards keep the old shape because the existing UI still renders.

**Good**: Treat additive analytics fields as contract changes. Update the owning
service type, query tests, and the first consumer surface in the same change.

Analytics contract checklist:
- If Rust adds or renames a serialized field, update the frontend service type
  in the same PR.
- If the field is intentionally backend-only, document that choice next to the
  Rust DTO instead of relying on silent extra JSON fields.
- Prefer one owning TypeScript type per IPC payload and derive page/view-model
  types from it instead of re-declaring subsets.
- Add at least one contract-focused test that fails when the new field is
  missing from the frontend payload shape.

### Mistake 12: Hardcoding Support Matrices Across TS, Rust, and SQLite

**Bad**: A new CLI or workspace-scoped sync object requires edits to TypeScript
union types, Rust string arrays, SQL columns like `enabled_claude`, and
multiple page branches. The feature works only after a wide copy-paste sweep.

**Good**: Keep the support matrix owned by one registry/descriptor model and let
UI, validation, and persistence derive from it where possible.

Extension-matrix checklist:
- If adding one CLI key requires touching frontend constants, backend
  validation, migration schema, and tests separately, stop and re-evaluate the
  design before shipping.

### Mistake 13: Treating Gate-Filtered Providers as Real Upstream Failures

**Bad**: The failover loop records circuit-open / cooldown / rate-limit skips in
`attempts`, then terminal classification checks only `attempts.is_empty()`.
Skip-only requests are finalized as `GW_UPSTREAM_ALL_FAILED`, bypass the recent
error cache, and flood Home request history with repeated failures while the
provider is still unavailable.

**Good**: Distinguish "provider filtered before send" from "upstream request
actually failed". Preserve filtered attempts in `attempts_json` for diagnostics,
but finalize skip-only loops as `GW_ALL_PROVIDERS_UNAVAILABLE` so retry-after
cache and UI dedupe continue to work.

Failover observability checklist:
- Terminal classification must answer: "did any upstream request actually get
  sent?" instead of "is the attempts array non-empty?".
- Circuit-open / cooldown / rate-limit skips are diagnostic breadcrumbs, not
  proof of upstream failure.
- If every candidate was filtered before send, keep `attempts_json` detail but
  use the unavailable error family and retry-after semantics.
- When terminal state changes from `upstream_failed` to `unavailable`, verify
  recent-error cache keys and Home/log polling behavior still align with that
  state.
- Prefer data-driven enablement tables over one boolean column per CLI when the
  set is expected to evolve.
- Keep one authoritative definition for supported identities and generate or
  derive secondary views from it.
- When schema constraints force duplication, document every mirrored ownership
  point in the same PR.

### Mistake 14: Two-Phase Writes Without Orphan Recovery

**Bad**: Backend writes a placeholder row (`status=NULL`) at request start and
relies on a second upsert to finalize it. If the second write is lost (crash,
backpressure drop, channel disconnect), the placeholder persists indefinitely.
Frontend treats `status==null` as "in progress" with no time bound, causing
permanent UI artifacts and polling degradation.

**Good**: Any two-phase write pattern must account for the second phase never
arriving. Define an explicit staleness contract across layers.

Two-phase write checklist:
- If backend writes a placeholder that expects a later update, define the
  maximum expected lifetime of the placeholder state.
- Frontend must enforce a staleness guard: after the threshold, treat the row
  as abandoned rather than in-progress.
- Backend should periodically scan for orphaned placeholders (e.g. on startup
  or via a background sweep) and finalize them with a dedicated error code
  such as `GW_ORPHANED`.
- The second-phase write must have equal or higher delivery priority than the
  first phase. If backpressure drops the completion but keeps the placeholder,
  the system state is worse than if neither was written.
- When `shouldUseFullRefresh` or similar polling-mode decisions depend on
  in-progress detection, verify that a stuck placeholder does not permanently
  degrade polling performance.

### Mistake 15: Exposing Runtime Settings That Never Reach the Real Runtime Boundary

**Bad**: A setting is persisted in `settings.json` and rendered in the UI, but
the real consumer reads only static plugin config or startup-time state. Users
think they changed live behavior, yet the effective endpoint or plugin state
never moves.

**Good**: For every user-facing setting, identify the real runtime owner and
wire the final side effect in the same change. If the boundary is build-time or
startup-only, either make that explicit in the product or stop exposing it as a
normal live setting.

Runtime-setting checklist:
- If a setting controls a Tauri plugin, confirm whether that plugin reads
  `tauri.conf.json`, startup-time builder state, or live command input.
- Do not keep a UI toggle or text field once the actual runtime consumer is
  known to ignore it.
- Add one test that changes the setting and verifies the real side effect, not
  just the stored JSON.
- When a value is display-only, document that ownership next to the setting type
  instead of implying runtime control.

### Mistake 16: Mixing Generated and Handwritten IPC Contracts Without an Ownership Map

**Bad**: Some command families use Specta-generated bindings, others still use
handwritten `invoke` wrappers, and pages/components import both styles directly.
Maintainers then talk about a "stable IPC contract" as if one generated file
protected the whole desktop boundary.

**Good**: Keep one explicit ownership map for the desktop contract. Decide which
command families are generated-first, which remain handwritten, and which must
stay behind service adapters. Pages should consume service functions, not pick
their own IPC style.

IPC-ownership checklist:
- Group command families under one of: generated binding, handwritten wrapper,
  plugin API wrapper, or event-only contract.
- If Specta coverage is partial, document that boundary in code and docs next to
  the generated file.
- Keep one targeted contract test for every handwritten command family that
  names the Rust command and the TypeScript wrapper together.
- Do not let pages/components import both generated IPC and raw `invoke` for
  the same feature area.

### Mistake 17: Driving Downstream Side Effects from Persisted Settings Instead of the Active Runtime Snapshot

**Bad**: Persist new host/port settings and immediately push those values into
WSL, CLI proxy, updater, or other downstream sync flows while the active
gateway/runtime listener is still bound to the old address.

**Good**: Separate "next persisted config" from "current active runtime
snapshot". If a setting needs rebind/restart to take effect, downstream sync
must either use the active snapshot or wait until the rebind succeeds.

Runtime-rebind checklist:
- Model persisted config and active runtime state as separate concepts.
- Use the active runtime snapshot for downstream sync until rebind completes.
- Add one integration test that edits host/port while the runtime is already
  running and verifies which value external sync receives.
- If live rebind is unsupported, surface that as explicit UX instead of
  pretending the new persisted value is already in effect.

### Mistake 18: Broadcasting High-Frequency Events Without Visibility or Payload Ownership

**Bad**: Backend emits large realtime payloads to every window even when the
window is hidden, no page is subscribed, or the UI only needs a small summary.

**Good**: Classify events by freshness and payload cost. Use push only for the
small state users must see immediately, and let heavier views re-fetch by ID or
cursor when visible.

Realtime-event checklist:
- Decide which events are summary signals and which are detail payloads.
- If a window is hidden or no subscriber is active, skip or coalesce expensive
  events.
- Prefer push-summary + pull-detail for traces, logs, and other high-volume
  streams.
- Add simple event-rate instrumentation so regression shows up before UI jank
  becomes a user report.

### Mistake 19: Assuming One Enable Flag Owns Every Route View

**Bad**: A route has multiple enable flags (`providers.enabled`,
`sort_mode_providers.enabled`, workspace-scoped flags, etc.), and code assumes
one of them is always the global source of truth. In this project, the default
provider list and sort templates intentionally have different ownership.

**Good**: Define the eligibility owner for each route view explicitly. The
default provider route is gated by `providers.enabled`; a sort-template route is
gated by `sort_mode_providers.enabled` and must not silently inherit the
default route's global switch. Runtime route state must be cleared after
successful changes to the eligibility owner for that route view. Route state is
not only session bindings; recent `GW_ALL_PROVIDERS_UNAVAILABLE` cache entries
also short-circuit requests before failover/logging and must be invalidated.

Provider-eligibility checklist:
- Before changing a gateway candidate query, write down whether it represents
  the default provider route or a sort-template route.
- Default provider queries must apply `providers.enabled = 1`.
- Sort-template gateway queries must apply `sort_mode_providers.enabled = 1`
  and must not depend on `providers.enabled`.
- Provider create/save/toggle/delete and sort-mode membership changes clear
  runtime route state for the affected CLI key after persistence succeeds when
  they change that route view's eligibility.
- Runtime route-state invalidation must clear both session bindings and recent
  unavailable-error cache entries; otherwise enabling another provider can still
  return the old cached unavailable response without writing a new request log.
- Regression tests cover both the default provider list and active sort-mode
  paths.

### Mistake 20: Treating Provider-Scoped Continuation IDs as Route-Scoped State

**Bad**: After a circuit-open Codex provider is disabled and another provider is
enabled, forward the next `/v1/responses` request with the old
`previous_response_id` and count the new provider's 400/404 as a health failure.
The response id belongs to the upstream provider that created it, not to the
route, session binding, or local gateway.

**Good**: When a Codex upstream returns a specific missing/invalid previous
response error for a request carrying `previous_response_id`, strip only that
field and retry the same provider once. Record the mutation in
`special_settings_json`, do not increment circuit/cooldown for the discarded
attempt, and keep enough per-provider retry budget for other internal retries
such as OAuth reactive refresh.

Provider-continuation checklist:
- Treat upstream-generated continuation handles (`previous_response_id`,
  vendor response ids, bridge conversation ids) as provider-scoped unless the
  upstream contract explicitly says they are portable.
- On provider switch/failover, distinguish "stale provider-scoped continuation"
  from true provider failure before mutating circuit-breaker state.
- Gate body repair by CLI, status, request field presence, and upstream error
  text; do not retry arbitrary 400/404 responses.
- If a repair consumes an internal retry, reserve retry budget independently
  from normal provider failover attempts.

### Mistake 21: Treating Process-Wide Test Environment as Test-Local State

**Bad**: A test fixture mutates process-wide environment variables while a
mutex guard is dropped before the restore guard. Parallel `cargo test` runs can
then acquire the lock while stale env vars are still active, or have their env
vars restored out from under them, causing temp test paths to be written into
real user config.

**Good**: For struct-owned test fixtures, declare the env restore guard before
the mutex guard so Rust drops the restore guard first and releases the lock only
after the environment is clean.

Test-env checklist:
- Any fixture that mutates `AIO_CODING_HUB_HOME_DIR`,
  `AIO_CODING_HUB_DOTDIR_NAME`, `AIO_CODING_HUB_TEST_HOME`, `CODEX_HOME`, or
  similar process-wide env vars must hold one shared env lock.
- In structs, declare restore guards before lock guards. Do not rely on field
  names; Rust drops struct fields in declaration order.
- When a full test run unexpectedly changes real user config, compare the file
  mtime with session logs for `cargo test` / `pnpm tauri:test` and search for
  temp fixture names before assuming the user edited the file.

### Mistake 22: Editing Third-Party Configs Through a Narrow Projection

**Bad**: Read a third-party config node into a simplified UI model, then write
that simplified model back over the whole node. Any upstream-supported fields
outside the UI model, such as unknown handler types, conditions, async flags, or
future extension fields, disappear after the user edits one visible row.

**Good**: Treat third-party config as a preserve-by-default document. The UI may
present a supported subset, but save flows must patch only the selected item or
carry unknown fields through a raw JSON model.

Third-party config checklist:
- Before adding a GUI editor for a third-party config file, compare the UI model
  against the upstream schema and decide which fields are first-class, read-only,
  or pass-through.
- Round-trip tests must include unknown fields and unsupported item types. A
  save of one visible item must preserve sibling items and extra fields.
- If the product intentionally supports only a subset, expose unsupported rows as
  read-only instead of silently dropping them on save.
- Invalid JSON must fail closed. Do not downgrade parse failures into `{}` and
  then write defaults back over user config.

### Mistake 23: Treating Provider Probe Status Codes as the Whole Contract

**Bad**: Mark a provider available whenever the probe returns any non-401/403
HTTP response. Some upstreams report invalid credentials as 400 with an auth
message, while 5xx proves the endpoint responded but not that the provider is
usable.

**Good**: Classify provider probe results with both status and body semantics:
explicit auth failures fail closed, upstream 5xx is unavailable, and expected
model/rate-limit errors can still prove the route and credential reached the
provider.

Provider-probe checklist:
- Keep auth-failure detection provider-aware enough to cover body-level errors
  such as "API key not valid" and `invalid_api_key`.
- Treat 5xx probe responses as unavailable unless product requirements define a
  separate degraded state.
- Regression tests must cover model-not-found/rate-limit success, 5xx failure,
  and body-level auth failure with a 4xx status.

### Mistake 24: Cache Hit-Rate Denominators Drift Across Layers

**Bad**: Backend usage summaries, provider trends, Home cards, Usage tables,
and realtime trace cards each calculate "cache rate" from whichever fields are
nearby. One screen shows cached-token share, another shows read-hit rate, and
bridge providers get counted differently from Codex/Gemini.

**Good**: Treat cache hit rate as a cross-layer metric contract:

- Formula: `cache_read_input_tokens / (effective_input_tokens + cache_creation_input_tokens + cache_read_input_tokens)`.
- `effective_input_tokens` subtracts cache reads for Codex/Gemini and any
  backend-classified bridge/source provider where cached reads are already a
  subset of input tokens.
- Rust usage aggregation owns SQL-level effective input and total token
  expressions in `src-tauri/src/domain/usage_stats/tokens.rs`.
- Frontend display helpers own UI-level math in `src/utils/cacheRateMetrics.ts`;
  components should not duplicate the formula.
- Usage summary/leaderboard DTOs already expose effective `input_tokens`; do
  not subtract cache reads again in components that consume those DTOs.
- Raw realtime logs/traces may still need frontend effective-input correction
  before display because they carry unaggregated gateway metrics.

Cache metric checklist:
- [ ] Decide whether the data source is raw gateway metrics or already
      aggregated usage DTO data
- [ ] For backend trend rows, expose `denom_tokens` from the backend and let
      charts divide `cache_read_input_tokens / denom_tokens`
- [ ] For frontend summary/table rows, use `computeCacheHitRate` and confirm the
      input argument is already the effective input from Rust aggregation
- [ ] For raw trace/log cards, use the shared effective-input helper instead of
      local CLI string checks
- [ ] Add regression tests that distinguish old cached-token share from the
      read-hit formula
- [ ] Include Codex/Gemini and bridge/source-provider examples when touching
      denominator logic

---

## Checklist for Cross-Layer Features

Before implementation:
- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens
- [ ] Decided whether Specta bindings must be regenerated
- [ ] For request logs, mapped `gateway event → request_logs row → generated
      binding → service type → list card → detail dialog`

After implementation:
- [ ] Tested with edge cases (null, empty, invalid)
- [ ] Verified error handling at each boundary
- [ ] Checked data survives round-trip
- [ ] Updated generated bindings or documented why not
- [ ] Verified generated bindings are actually consumed where they are claimed
      to be authoritative
- [ ] Verified config read failures do not silently downgrade into
      default-overwrite saves
- [ ] Confirmed event names and error-code constants still come from the shared source
- [ ] Confirmed additive analytics / observability fields are reflected in the
      owning frontend payload type
- [ ] Confirmed extension matrices (CLI keys, workspace sync scopes, enabled
      flags) are still owned centrally instead of drifting across layers
- [ ] Confirmed each route/provider view uses its documented enable owner
      (default provider route vs sort-template route) and invalidates stale
      runtime route state after successful eligibility changes, including
      session bindings and recent unavailable-error cache
- [ ] Checked provider-scoped continuation ids before provider-health mutation:
      stale Codex `previous_response_id` errors are repaired once and recorded
      as a guarded body mutation, not as circuit-breaker evidence
- [ ] For request logs, separated real upstream attempts from provider-gate
      skips in user-facing wording
- [ ] For request logs, documented unsupported CLI folder lookup instead of
      treating it as a lookup miss
- [ ] For Codex SSE responses, distinguished terminal error markers from late
      tail read failures after `response.completed`; do not let teardown I/O
      noise overwrite a successful user-visible completion
- [ ] Classified helper/probe routes as user-visible vs infra-only and verified
      logs, events, stats, and provider-health side effects match that choice
- [ ] Classified provider availability probe results by status and body
      semantics, including 5xx and body-level auth errors
- [ ] If displaying cache hit rate or cache denominator data, verified the
      formula owner and whether the source data is raw gateway metrics or
      already-aggregated effective usage
- [ ] If the change touches gateway/proxy paths, explicitly list all non-passthrough
      mutations (headers, path/query, body JSON, response translation) and ensure each
      mutation is either:
      - guarded + observable (`special_settings_json`), or
      - removed as unnecessary
- [ ] Confirm that provider auth/bridge modes do not silently affect each other
      (e.g. API key vs OAuth vs protocol bridge should have clear boundaries)
- [ ] Reviewed root bootstrap / command registry blast radius after the change
- [ ] If any write uses a two-phase pattern (placeholder + update), verified
      that orphan recovery exists and frontend enforces a staleness guard
- [ ] Confirmed each user-facing setting reaches the real runtime owner
      (startup builder, plugin config, live command path, or documented
      display-only field)
- [ ] Documented IPC ownership for the touched command family
      (generated, handwritten, plugin wrapper, or event-only)
- [ ] Verified downstream sync reads the active runtime snapshot instead of
      assuming persisted settings are already applied
- [ ] Verified high-frequency events have an explicit payload owner and
      visibility/backpressure rule
- [ ] If editing a third-party config file, verified unsupported fields and
      unknown item types survive read-modify-write round trips

---

## When to Create Flow Documentation

Create detailed flow docs when:
- Feature spans 3+ layers
- Multiple teams are involved
- Data format is complex
- Feature has caused bugs before
- One Tauri command is used by multiple pages or services
