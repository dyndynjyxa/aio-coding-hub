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

---

## React Root Boundary Checklist

Use this when touching `src/main.tsx`, `src/App.tsx`, or global event wiring.

- Keep the root component **composition-only**: providers, router, toasts, boundaries.
- Move startup side effects into a dedicated hook such as `useAppBootstrap`.
- Keep route declarations in a dedicated module such as `src/app/AppRoutes.tsx`.
- Split unrelated synchronization work into separate effects instead of one “startup soup” effect.

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
- Log the failure with enough context to diagnose file corruption or migration
  drift.
- Add one test that proves a read failure does not silently erase unrelated
  fields on the next save.

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

---

## Checklist for Cross-Layer Features

Before implementation:
- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens
- [ ] Decided whether Specta bindings must be regenerated

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
- [ ] Classified helper/probe routes as user-visible vs infra-only and verified
      logs, events, stats, and provider-health side effects match that choice
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

---

## When to Create Flow Documentation

Create detailed flow docs when:
- Feature spans 3+ layers
- Multiple teams are involved
- Data format is complex
- Feature has caused bugs before
- One Tauri command is used by multiple pages or services
