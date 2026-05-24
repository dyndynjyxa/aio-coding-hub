# brainstorm: Codex OAuth compatible proxy mode

## Goal

Add a Codex OAuth compatible CLI proxy mode so AIO can point Codex at the local gateway without overwriting Codex's OAuth `auth.json` credentials. The mode should preserve existing Codex login state while still letting AIO manage the gateway provider entry in `config.toml`.

## What I already know

* User wants a mode: when enabled, AIO should not modify Codex `auth.json`; it should only modify Codex `config.toml`.
* Current Codex proxy enable writes both `config.toml` and `auth.json`.
* Current `config.toml` proxy patch writes `model_provider = "aio"`, `preferred_auth_method = "apikey"`, `[model_providers.aio]`, and Windows sandbox settings.
* Current `auth.json` proxy patch writes `OPENAI_API_KEY = "aio-coding-hub"`, `auth_mode = "apikey"`, and removes OAuth `tokens` / `last_refresh`.
* Gateway upstream auth injection clears client auth headers and injects AIO provider credentials, so preserved Codex OAuth credentials are primarily for Codex CLI local auth compatibility.
* Codex CLI local binary includes `apikey` and `chatgpt` auth-mode strings; `preferred_auth_method = "chatgpt"` appears compatible enough for design, but final behavior should be covered by tests and manual verification.

## Assumptions (temporary)

* The new mode defaults OFF to preserve current behavior for existing users.
* The mode is Codex-only; Claude and Gemini proxy behavior remains unchanged.
* The mode should be persistent, not a one-shot checkbox.
* In OAuth compatible mode, AIO still manages Codex `config.toml` gateway provider fields and still participates in startup repair/sync/rebind.
* In OAuth compatible mode, AIO should not parse, backup, write, delete, or restore Codex `auth.json` as part of CLI proxy takeover.`n* In OAuth compatible mode, AIO should not write `preferred_auth_method`; if AIO previously wrote `preferred_auth_method = "apikey"`, applying OAuth compatible mode should remove/restore that root key.

## Open Questions

* Decision: OAuth compatible mode should not write `preferred_auth_method = "chatgpt"`. It should omit/remove the AIO-managed `preferred_auth_method = "apikey"` so Codex uses its existing OpenAI authentication credentials with `requires_openai_auth = true`.

## Requirements (evolving)

* Add a Codex OAuth compatible proxy mode switch.
* When the mode is off, keep existing behavior exactly: config patch + auth placeholder patch.
* When the mode is on, Codex proxy enable/sync/rebind should only write Codex `config.toml` and must not mutate `auth.json`.
* The mode should survive app restart and be visible in the UI.
* Status/drift checks should still report Codex proxy applied when `config.toml` points to the current gateway and local Codex OAuth auth is present or the auth file is intentionally untouched.
* Disable/restore should only revert proxy-managed `config.toml` fields for OAuth compatible mode and must not remove or rewrite user's OAuth `auth.json`.
* Existing MCP sync after proxy toggle must still run and preserve `[mcp_servers.*]` entries.

## Acceptance Criteria (evolving)

* [ ] Default behavior remains unchanged when OAuth compatible mode is disabled.
* [ ] Enabling Codex proxy in OAuth compatible mode does not create or modify `auth.json`.
* [ ] Enabling Codex proxy in OAuth compatible mode writes/updates `[model_providers.aio]` `base_url = "<gateway>/v1"` in `config.toml`.
* [ ] Switching gateway port or Codex Home while OAuth compatible mode is enabled re-syncs `config.toml` only.
* [ ] Disabling Codex proxy in OAuth compatible mode restores/removes only proxy-managed `config.toml` fields and leaves `auth.json` intact.
* [ ] Unit tests cover auth-file untouched behavior, status detection, restore behavior, and regression for current API-key placeholder mode.
* [ ] Frontend exposes the mode with clear copy that it preserves Codex OAuth login and avoids modifying `auth.json`.

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* Changing AIO provider OAuth implementation.
* Importing or reading Codex's local OAuth token into AIO.
* Supporting Claude/Gemini variants of this mode.
* Changing Codex MCP sync semantics except preserving existing behavior.

## Technical Notes`n`n* Official Codex auth docs document ChatGPT and API-key sign-in, login caching, `auth.json`, `cli_auth_credentials_store`, and `forced_login_method = "chatgpt" # or "api"` for managed restrictions; they do not document `preferred_auth_method`.`n* Official Codex config reference documents `model_providers.<id>.requires_openai_auth = true` for custom providers using OpenAI authentication; this lets users sign in with ChatGPT or API key. This is the doc-aligned field for AIO proxy provider auth.`n
* Research: `.trellis/tasks/05-24-codex-oauth-compatible-proxy-mode/research/codex-oauth-compatible-proxy-mode.md`
* Current Codex config path resolution: `src-tauri/src/infra/codex_paths.rs`
* Current proxy implementation: `src-tauri/src/infra/cli_proxy/mod.rs`, `src-tauri/src/infra/cli_proxy/codex.rs`
* Current settings model: `src-tauri/src/infra/settings/types.rs`, `src-tauri/src/app/settings_service.rs`
* Current Codex UI: `src/components/cli-manager/tabs/CodexTab.tsx`
* Gateway auth behavior: `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs`

