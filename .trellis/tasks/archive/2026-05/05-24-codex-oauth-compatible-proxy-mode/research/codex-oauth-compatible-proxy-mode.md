# Codex OAuth compatible proxy mode research

## Question

How should AIO's Codex CLI proxy takeover behave when the user wants to preserve Codex OAuth credentials instead of replacing `auth.json` with an API-key placeholder?

## Existing behavior

- Enabling the Codex CLI proxy currently targets both Codex files:
  - `config.toml` as `codex_config_toml`
  - `auth.json` as `codex_auth_json`
- The current `config.toml` proxy patch writes:
  - root `model_provider = "aio"`
  - root `preferred_auth_method = "apikey"`
  - `[model_providers.aio]` with `name`, `base_url = "<gateway>/v1"`, `wire_api = "responses"`, `requires_openai_auth = true`
  - `[windows] sandbox = "elevated"` on Windows only
- The current `auth.json` proxy patch writes:
  - `OPENAI_API_KEY = "aio-coding-hub"`
  - `auth_mode = "apikey"`
  - removes `tokens` and `last_refresh`
- Restore logic already merge-restores Codex proxy-managed keys and restores OAuth `tokens` / `last_refresh` from backup.

## Local Codex CLI observations

- Installed Codex CLI: `codex-cli 0.133.0`.
- Binary strings contain both `apikey` and `chatgpt` near auth-related structures, consistent with two auth modes.
- Local `codex login --help` exposes ChatGPT/browser login and API key/access-token login paths.
- Running `codex --strict-config -c 'preferred_auth_method="chatgpt"' doctor --summary` in a temporary `CODEX_HOME` loads config successfully. This is not a full semantic proof because invalid values did not fail either, but it supports that `chatgpt` is at least recognized in the binary.

## Gateway-side compatibility

- Gateway failover attempt auth injection clears all client auth headers before upstream forwarding.
- The upstream request receives provider auth from AIO's configured provider, including AIO-managed OAuth provider tokens when provider auth_mode is `oauth`.
- Therefore, preserving Codex's local OAuth token is mainly to make Codex CLI willing to call AIO's local provider. AIO does not need to forward that client token upstream.

## Recommended design

Add a Codex-specific CLI proxy auth strategy/mode, defaulting to the current behavior:

- `api_key_placeholder` / default:
  - current behavior; patch both `config.toml` and `auth.json`.
- `oauth_compatible`:
  - patch `config.toml` only.
  - do not target, parse, write, or backup `auth.json` as part of proxy enable/apply/sync/rebind.
  - in `config.toml`, set provider table to AIO gateway, but do not write `preferred_auth_method = "apikey"`; instead write `preferred_auth_method = "chatgpt"` or remove/restore the proxy-owned key. Product decision needed.

## Implementation hotspots

- Backend proxy takeover:
  - `src-tauri/src/infra/cli_proxy/mod.rs`
  - `src-tauri/src/infra/cli_proxy/codex.rs`
  - `src-tauri/src/app/cli_proxy_service.rs`
- Settings persistence if implemented as global setting:
  - `src-tauri/src/infra/settings/types.rs`
  - `src-tauri/src/app/settings_service.rs`
  - `src/services/settings/settings.ts`
- Frontend UI likely placement:
  - `src/components/cli-manager/tabs/CodexTab.tsx` near Codex Windows config / Codex feature settings, or `src/pages/settings/SettingsMainColumn.tsx` for a global app setting.
- Bindings/tests:
  - generated bindings under `src/generated/bindings.ts` may need regeneration.
  - tests under `src-tauri/src/infra/cli_proxy/tests.rs`, settings tests, service/query/component tests.

## Official docs check (2026-05-24)

- OpenAI Codex Authentication docs document two CLI sign-in methods: ChatGPT and API key.
- They document login caching at `~/.codex/auth.json` or OS credential store, and `cli_auth_credentials_store`.
- They document `forced_login_method = "chatgpt" # or "api"` as a managed-environment restriction, not as a normal user preference.
- They document alternative/custom providers using `requires_openai_auth = true`; with that setting users can sign in with ChatGPT or an API key.
- The official config reference did not contain `preferred_auth_method`.

Conclusion: OAuth compatible mode should not write `preferred_auth_method = "chatgpt"`. The safer, doc-aligned config is to keep `requires_openai_auth = true` on `[model_providers.aio]` and remove/avoid the old AIO-managed `preferred_auth_method = "apikey"` root key.
