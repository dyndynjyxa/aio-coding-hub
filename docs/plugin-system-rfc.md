# aio coding hub Plugin System RFC

## 1. Purpose

The plugin system lets aio coding hub support community extensions for gateway request processing, gateway response inspection, log redaction, local configuration, and GUI management. The first production version focuses on low-risk declarative plugins and stable host contracts before any third-party code runtime is enabled.

The priority scenarios are:

- Prompt optimization before a gateway request is sent upstream.
- Safety inspection of model responses, including streamed chunks.
- Redaction of sensitive URLs, keys, tokens, connection strings, logs, and GUI-visible data.

## 2. Non-Goals

- 短期不执行任意 JavaScript/TypeScript.
- Node.js and Deno are not default plugin runtimes.
- Third-party native dynamic libraries are not loaded into the Rust main process.
- 第三方代码不得直接进入主进程或 WebView.
- Plugins do not run inside the Tauri WebView.
- Plugins do not intercept local CLI input boxes or internal CLI UI events.
- The current Skill 市场 is not converted into this runtime.
- Enterprise RBAC, organization policy centers, and commercial settlement are separate future projects.

## 3. Existing Architecture Fit

The current host already has the right boundaries for a plugin system:

- GUI: Tauri 2, React, Vite, TypeScript.
- Backend host: Rust main process with an Axum gateway.
- Request entry: `src-tauri/src/gateway/routes.rs`.
- Proxy handler: `src-tauri/src/gateway/proxy/handler/mod.rs`.
- Body read middleware: `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`.
- Provider resolution middleware: `src-tauri/src/gateway/proxy/handler/middleware/provider_resolution.rs`.
- Upstream send path: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/send.rs`.
- Response routing: `src-tauri/src/gateway/proxy/handler/failover_loop/response/response_router.rs`.
- Non-stream response handling: `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_non_stream.rs`.
- Stream handling: `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_event_stream.rs` and `src-tauri/src/gateway/streams/usage_tee.rs`.
- IPC command registry: `src-tauri/src/commands/registry.rs`.
- Data root: `~/.aio-coding-hub`, resolved through `src-tauri/src/infra/app_paths.rs`.
- Persistence: SQLite plus `settings.json`.

The plan document listed some historical paths. Implementation must follow the current `failover_loop` layout above.

## 4. Hook Boundary Principles

Prompt optimization can only be reliably implemented at gateway request stages. It cannot reliably alter text that a CLI edits internally before the gateway receives HTTP traffic. Therefore, 提示词优化只能在网关请求阶段可靠实现.

Hook insertion points must be explicit and tested. No request-body hook may be inserted at an unrecorded or untested point. The final M2 implementation must update this RFC with the exact gateway hook chain diagram after integration tests freeze the route.

Initial hook names:

- `gateway.request.received`
- `gateway.request.afterBodyRead`
- `gateway.request.beforeProviderResolution`
- `gateway.request.beforeSend`
- `gateway.response.headers`
- `gateway.response.chunk`
- `gateway.response.after`
- `gateway.error`
- `log.beforePersist`

### Final Gateway Hook Chain

The M2 implementation freezes the tested hook chain below. Hooks marked as active are wired into the hot path; reserved hooks remain part of the v1 contract and must be wired with tests before they can mutate runtime data.

```mermaid
flowchart TD
    A["HTTP request enters routes.rs"] --> B["gateway.request.received (reserved metadata hook)"]
    B --> C["BodyReaderMiddleware reads and introspects body"]
    C --> D["gateway.request.afterBodyRead (active body/header mutation)"]
    D --> E["gateway.request.beforeProviderResolution (reserved provider-resolution hook)"]
    E --> F["Provider selection and failover loop"]
    F --> G["gateway.request.beforeSend (active upstream header/body mutation)"]
    G --> H["Upstream provider request"]
    H --> I{"Response type"}
    I --> J["gateway.response.headers (reserved header hook)"]
    J --> K["gateway.response.after (active non-stream body/header mutation)"]
    J --> L["gateway.response.chunk (active SSE chunk inspect/modify/block)"]
    K --> M["Client response"]
    L --> M
    F --> N["gateway.error (active standard gateway error response mutation)"]
    N --> M
    M --> O["log.beforePersist (active request log redaction before enqueue)"]
```

Active request and response hooks execute through a timeout-bounded pipeline with permission-trimmed contexts, fail-open defaults, audit events, and circuit-open skips. Stream chunks run after gzip/protocol/response-fixer processing and before usage accounting, so safety and redaction plugins affect both client-visible chunks and downstream stream accounting. `log.beforePersist` receives a serialized request-log payload and may redact persisted fields without changing gateway status classification.

## 5. Runtime Roadmap

### Short Term: Declarative Rules

The first runtime executes no community code. Plugins declare rules in JSON:

- JSON path selection.
- Regex detection with bounded input and timeout protection.
- Replace, warn, block, and append-message actions.
- Configuration schema and permission declarations.

This runtime is enough for community prompt helpers, response safety checks, and log redactors that can be expressed as bounded JSON path and regex rules.

### Medium Term: WASM

WASM plugins run in a sandboxed runtime, initially Wasmtime. WASM execution must:

- Disable default filesystem access.
- Disable default network access.
- Enforce memory limits.
- Enforce execution timeouts.
- Access host capabilities only through imported functions that check granted permissions.
- Use a versioned ABI for hook input, hook output, errors, logs, and config reads.

### Long Term: Managed Process Runtime

An independent process runtime may be used for a proof of concept, but only as a host-managed worker:

- JSON-RPC over stdio.
- Every request carries a `trace_id`.
- Startup timeout, hook timeout, crash limits, and idle recycle are mandatory.
- The worker must not become a daemon, login item, scheduled task, or detached background service.
- The PoC does not mean marketplace plugins can use background tasks by default.

## 6. Security Principles

- Least privilege: plugins see only context allowed by their granted permissions and hook type.
- Sensitive headers such as `Authorization` and `Cookie` are hidden unless `request.header.readSensitive` is explicitly granted.
- Request and response bodies are hidden unless body-read permissions are granted.
- Write actions are rejected unless matching write permissions are granted.
- High-risk permissions require second confirmation.
- Upgrades that add permissions require renewed authorization.
- Audit logs record install, enable, disable, config changes, permission changes, hook errors, hook timeouts, blocks, and high-risk mutations.
- Audit logs must not store sensitive raw values.
- Consecutive failures can quarantine a plugin.
- Safety-class plugins may use fail-closed behavior. Decorative plugins default to fail-open.
- `log.beforePersist` falls back to host redaction if plugin redaction fails.

## 7. Cross-Platform Principles

- All plugin paths are below `~/.aio-coding-hub/plugins`.
- Paths are built with `PathBuf` joins, not hard-coded `/`.
- Plugin IDs are validated before they become path segments.
- Canonical path checks prevent zip slip and directory traversal.
- Windows file locks must fail safely during update and rollback.
- macOS quarantine or code signing behavior must not require executing unsigned native binaries.
- Linux AppImage/deb/rpm layouts must write plugin data only to user data directories, never installation directories.

## 8. Skill Market Boundary

The Skill 市场 and this plugin system are separate:

- Skills are agent instruction/tooling assets.
- Plugins are user-installed runtime extensions for aio coding hub gateway, logs, settings, and GUI management.
- Skill repository cache code can inspire plugin package cache logic, but plugin packages, manifests, permissions, hooks, and runtime isolation are independent contracts.
- A Skill cannot bypass plugin permissions by being installed as a plugin.

## 9. Failure Policy Matrix

| Hook | Default policy | Configurable | Failure behavior |
| --- | --- | --- | --- |
| `gateway.request.received` | fail-open | yes | Skip plugin, audit failure. |
| `gateway.request.afterBodyRead` | fail-open | yes | Continue with original request, audit failure. |
| `gateway.request.beforeProviderResolution` | fail-open | yes | Ignore routing suggestions, audit failure. |
| `gateway.request.beforeSend` | fail-open | security plugins may fail-closed | Continue for decorative plugins; block for fail-closed security plugins. |
| `gateway.response.headers` | fail-open | yes | Return original headers. |
| `gateway.response.chunk` | security fail-closed, non-security fail-open | yes | Terminate stream or emit safe error for security failures. |
| `gateway.response.after` | security fail-closed, non-security fail-open | yes | Return safe error for security failures. |
| `gateway.error` | fail-open | no | Never hide the host error. |
| `log.beforePersist` | fail-closed-to-host-redaction | no | Use host redaction before persistence. |

## 10. Observability

Every hook execution records:

- `trace_id`
- plugin ID and version
- hook name
- elapsed time
- outcome
- failure policy
- mutation summary
- warning or block summary

English logs should be concise and operational, for example:

- `plugin hook timed out`
- `plugin hook blocked request`
- `plugin entered quarantine after repeated failures`
