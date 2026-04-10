// Usage: MSW handlers emulating Tauri commands via `http://tauri.local/<command>` fetch bridge.

import { http, HttpResponse } from "msw";
import { TAURI_ENDPOINT } from "../tauriEndpoint";
import type { CliKey, ClaudeModels, ProviderSummary } from "../../services/providers/providers";
import {
  buildCliProxySetEnabledResult,
  getAppAboutState,
  getCliProxyStatusAllState,
  getDbDiskUsageState,
  getEnvConflictsState,
  getGatewayStatusState,
  getProvidersState,
  getSettingsState,
  setProvidersState,
  getSortModeActiveState,
  getSortModesState,
  getUsageSummaryState,
  getWorkspacesState,
  mergeSettingsState,
} from "./state";

const withJson = async <T>(request: Request): Promise<T> => {
  try {
    const raw = await request.text();
    if (!raw) return {} as T;
    return JSON.parse(raw) as T;
  } catch {
    return {} as T;
  }
};

export const handlers = [
  // ---- CLI Proxy ----
  http.post(`${TAURI_ENDPOINT}/cli_proxy_status_all`, () =>
    HttpResponse.json(getCliProxyStatusAllState())
  ),

  http.post(`${TAURI_ENDPOINT}/cli_proxy_set_enabled`, async ({ request }) => {
    const payload = await withJson<{ cliKey?: string; enabled?: boolean }>(request);
    return HttpResponse.json(
      buildCliProxySetEnabledResult({
        cli_key: payload.cliKey ?? "",
        enabled: Boolean(payload.enabled),
      })
    );
  }),

  // ---- Environment Conflicts ----
  http.post(`${TAURI_ENDPOINT}/env_conflicts_check`, () =>
    HttpResponse.json(getEnvConflictsState())
  ),

  // ---- Settings ----
  http.post(`${TAURI_ENDPOINT}/settings_get`, () => HttpResponse.json(getSettingsState())),

  http.post(`${TAURI_ENDPOINT}/settings_set`, async ({ request }) => {
    const payload = await withJson<{ update?: Partial<Record<string, unknown>> }>(request);
    if (payload.update) {
      const normalizedUpdate = Object.fromEntries(
        Object.entries(payload.update).map(([key, value]) => [
          key.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`),
          value,
        ])
      );
      mergeSettingsState(normalizedUpdate as any);
    }
    return HttpResponse.json(getSettingsState());
  }),

  // Settings sub-commands that return AppSettings.
  http.post(`${TAURI_ENDPOINT}/settings_circuit_breaker_notice_set`, () =>
    HttpResponse.json(getSettingsState())
  ),
  http.post(`${TAURI_ENDPOINT}/settings_gateway_rectifier_set`, () =>
    HttpResponse.json(getSettingsState())
  ),
  http.post(`${TAURI_ENDPOINT}/settings_codex_session_id_completion_set`, () =>
    HttpResponse.json(getSettingsState())
  ),

  // ---- Gateway ----
  http.post(`${TAURI_ENDPOINT}/gateway_status`, () => HttpResponse.json(getGatewayStatusState())),

  http.post(`${TAURI_ENDPOINT}/gateway_start`, () => {
    // Simulate a started gateway.
    return HttpResponse.json({
      ...getGatewayStatusState(),
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    });
  }),

  http.post(`${TAURI_ENDPOINT}/gateway_stop`, () => {
    return HttpResponse.json({
      running: false,
      port: null,
      base_url: null,
      listen_addr: null,
    });
  }),

  http.post(`${TAURI_ENDPOINT}/gateway_check_port_available`, () => HttpResponse.json(true)),

  http.post(`${TAURI_ENDPOINT}/gateway_sessions_list`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/gateway_circuit_status`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/gateway_circuit_reset_provider`, () => HttpResponse.json(true)),

  http.post(`${TAURI_ENDPOINT}/gateway_circuit_reset_cli`, () => HttpResponse.json(0)),

  // ---- Providers ----
  http.post(`${TAURI_ENDPOINT}/providers_list`, async ({ request }) => {
    const payload = await withJson<{ cliKey?: CliKey }>(request);
    return HttpResponse.json(getProvidersState(payload.cliKey ?? "claude"));
  }),

  http.post(`${TAURI_ENDPOINT}/provider_upsert`, async ({ request }) => {
    const payload = await withJson<{ input?: Record<string, unknown> }>(request);
    const input = payload.input;
    if (!input) {
      return HttpResponse.json({ error: "missing provider_upsert input" }, { status: 400 });
    }

    const cliKeyRaw = input.cliKey;
    if (cliKeyRaw !== "claude" && cliKeyRaw !== "codex" && cliKeyRaw !== "gemini") {
      return HttpResponse.json({ error: "invalid provider_upsert cliKey" }, { status: 400 });
    }
    const cliKey = cliKeyRaw as CliKey;

    if (typeof input.name !== "string" || !Array.isArray(input.baseUrls)) {
      return HttpResponse.json({ error: "invalid provider_upsert payload" }, { status: 400 });
    }

    const current = getProvidersState(cliKey);
    const requestedId = typeof input.providerId === "number" ? input.providerId : null;
    const existing =
      requestedId == null ? null : (current.find((row) => row.id === requestedId) ?? null);
    const nextId = requestedId ?? Math.max(0, ...current.map((row) => row.id)) + 1;
    const now = Date.now();
    const summary: ProviderSummary = {
      id: nextId,
      cli_key: cliKey,
      name: input.name,
      base_urls: input.baseUrls.map((value) => String(value)),
      base_url_mode: input.baseUrlMode === "ping" ? "ping" : "order",
      claude_models:
        input.claudeModels && typeof input.claudeModels === "object"
          ? (input.claudeModels as ClaudeModels)
          : {},
      enabled: Boolean(input.enabled),
      priority: typeof input.priority === "number" ? input.priority : (existing?.priority ?? 100),
      cost_multiplier:
        typeof input.costMultiplier === "number"
          ? input.costMultiplier
          : (existing?.cost_multiplier ?? 1),
      limit_5h_usd:
        typeof input.limit5hUsd === "number"
          ? input.limit5hUsd
          : typeof input.limit5HUsd === "number"
            ? input.limit5HUsd
            : null,
      limit_daily_usd: typeof input.limitDailyUsd === "number" ? input.limitDailyUsd : null,
      daily_reset_mode: input.dailyResetMode === "rolling" ? "rolling" : "fixed",
      daily_reset_time:
        typeof input.dailyResetTime === "string" ? input.dailyResetTime : "00:00:00",
      limit_weekly_usd: typeof input.limitWeeklyUsd === "number" ? input.limitWeeklyUsd : null,
      limit_monthly_usd: typeof input.limitMonthlyUsd === "number" ? input.limitMonthlyUsd : null,
      limit_total_usd: typeof input.limitTotalUsd === "number" ? input.limitTotalUsd : null,
      tags: Array.isArray(input.tags) ? input.tags.map((value) => String(value)) : [],
      note: typeof input.note === "string" ? input.note : "",
      created_at: existing?.created_at ?? now,
      updated_at: now,
      auth_mode: input.authMode === "oauth" ? "oauth" : "api_key",
      oauth_provider_type: existing?.oauth_provider_type ?? null,
      oauth_email: existing?.oauth_email ?? null,
      oauth_expires_at: existing?.oauth_expires_at ?? null,
      oauth_last_error: existing?.oauth_last_error ?? null,
      source_provider_id:
        typeof input.sourceProviderId === "number" ? input.sourceProviderId : null,
      bridge_type: typeof input.bridgeType === "string" ? input.bridgeType : null,
      stream_idle_timeout_seconds:
        typeof input.streamIdleTimeoutSeconds === "number"
          ? input.streamIdleTimeoutSeconds > 0
            ? input.streamIdleTimeoutSeconds
            : null
          : (existing?.stream_idle_timeout_seconds ?? null),
    };

    setProvidersState(
      cliKey,
      existing ? current.map((row) => (row.id === nextId ? summary : row)) : [...current, summary]
    );
    return HttpResponse.json(summary);
  }),

  http.post(`${TAURI_ENDPOINT}/provider_set_enabled`, () => HttpResponse.json(null)),

  http.post(`${TAURI_ENDPOINT}/provider_delete`, () => HttpResponse.json(true)),

  http.post(`${TAURI_ENDPOINT}/providers_reorder`, async ({ request }) => {
    const payload = await withJson<{ cliKey?: CliKey }>(request);
    return HttpResponse.json(getProvidersState(payload.cliKey ?? "claude"));
  }),

  http.post(`${TAURI_ENDPOINT}/base_url_ping_ms`, () => HttpResponse.json(50)),

  // ---- Usage ----
  http.post(`${TAURI_ENDPOINT}/usage_summary`, () => HttpResponse.json(getUsageSummaryState())),

  http.post(`${TAURI_ENDPOINT}/usage_summary_v2`, () => HttpResponse.json(getUsageSummaryState())),

  http.post(`${TAURI_ENDPOINT}/usage_leaderboard_provider`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/usage_leaderboard_day`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/usage_hourly_series`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/usage_leaderboard_v2`, () => HttpResponse.json([])),

  http.post(`${TAURI_ENDPOINT}/usage_provider_cache_rate_trend_v1`, () => HttpResponse.json([])),

  // ---- Cost ----
  http.post(`${TAURI_ENDPOINT}/cost_summary_v1`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cost_trend_v1`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/cost_breakdown_provider_v1`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/cost_breakdown_model_v1`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/cost_top_requests_v1`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/cost_scatter_cli_provider_model_v1`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/cost_backfill_missing_v1`, () => HttpResponse.json(null)),

  // ---- Provider Limit Usage ----
  http.post(`${TAURI_ENDPOINT}/provider_limit_usage_v1`, () => HttpResponse.json([])),

  // ---- Request Logs ----
  http.post(`${TAURI_ENDPOINT}/request_logs_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/request_logs_list_all`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/request_logs_list_after_id`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/request_logs_list_after_id_all`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/request_log_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/request_log_get_by_trace_id`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/request_attempt_logs_by_trace_id`, () => HttpResponse.json([])),

  // ---- Sort Modes ----
  http.post(`${TAURI_ENDPOINT}/sort_modes_list`, () => HttpResponse.json(getSortModesState())),
  http.post(`${TAURI_ENDPOINT}/sort_mode_create`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/sort_mode_rename`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/sort_mode_delete`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/sort_mode_active_list`, () =>
    HttpResponse.json(getSortModeActiveState())
  ),
  http.post(`${TAURI_ENDPOINT}/sort_mode_active_set`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/sort_mode_providers_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/sort_mode_providers_set_order`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/sort_mode_provider_set_enabled`, () => HttpResponse.json(null)),

  // ---- Workspaces ----
  http.post(`${TAURI_ENDPOINT}/workspaces_list`, async ({ request }) => {
    const payload = await withJson<{ cliKey?: CliKey }>(request);
    return HttpResponse.json(getWorkspacesState(payload.cliKey ?? "claude"));
  }),
  http.post(`${TAURI_ENDPOINT}/workspace_create`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/workspace_rename`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/workspace_delete`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/workspace_preview`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/workspace_apply`, () => HttpResponse.json(null)),

  // ---- Prompts ----
  http.post(`${TAURI_ENDPOINT}/prompts_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/prompts_default_sync_from_files`, () =>
    HttpResponse.json({ items: [] })
  ),
  http.post(`${TAURI_ENDPOINT}/prompt_upsert`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/prompt_set_enabled`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/prompt_delete`, () => HttpResponse.json(true)),

  // ---- MCP Servers ----
  http.post(`${TAURI_ENDPOINT}/mcp_servers_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/mcp_server_upsert`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/mcp_server_set_enabled`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/mcp_server_delete`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/mcp_parse_json`, () => HttpResponse.json({ servers: [] })),
  http.post(`${TAURI_ENDPOINT}/mcp_import_servers`, () =>
    HttpResponse.json({ inserted: 0, updated: 0 })
  ),
  http.post(`${TAURI_ENDPOINT}/mcp_import_from_workspace_cli`, () =>
    HttpResponse.json({ inserted: 0, updated: 0 })
  ),

  // ---- Skills ----
  http.post(`${TAURI_ENDPOINT}/skill_repos_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/skill_repo_upsert`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/skill_repo_delete`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/skills_installed_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/skills_discover_available`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/skill_install`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/skill_install_to_local`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/skill_set_enabled`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/skill_uninstall`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/skill_return_to_local`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/skills_local_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/skill_local_delete`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/skill_import_local`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/skills_import_local_batch`, () =>
    HttpResponse.json({ imported: [], skipped: [], failed: [] })
  ),
  http.post(`${TAURI_ENDPOINT}/skills_paths_get`, () => HttpResponse.json(null)),

  // ---- App About ----
  http.post(`${TAURI_ENDPOINT}/app_about_get`, () => HttpResponse.json(getAppAboutState())),

  // ---- Data Management ----
  http.post(`${TAURI_ENDPOINT}/db_disk_usage_get`, () => HttpResponse.json(getDbDiskUsageState())),
  http.post(`${TAURI_ENDPOINT}/request_logs_clear_all`, () =>
    HttpResponse.json({ request_logs_deleted: 0, request_attempt_logs_deleted: 0 })
  ),
  http.post(`${TAURI_ENDPOINT}/app_data_reset`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/app_data_dir_get`, () => HttpResponse.json("/tmp/aio-test-data")),
  http.post(`${TAURI_ENDPOINT}/app_exit`, () => HttpResponse.json(true)),
  http.post(`${TAURI_ENDPOINT}/app_restart`, () => HttpResponse.json(true)),

  // ---- Model Prices ----
  http.post(`${TAURI_ENDPOINT}/model_prices_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/model_prices_sync_basellm`, () =>
    HttpResponse.json({
      status: "not_modified",
      inserted: 0,
      updated: 0,
      skipped: 0,
      total: 0,
    })
  ),
  http.post(`${TAURI_ENDPOINT}/model_price_aliases_get`, () =>
    HttpResponse.json({ version: 1, rules: [] })
  ),
  http.post(`${TAURI_ENDPOINT}/model_price_aliases_set`, () =>
    HttpResponse.json({ version: 1, rules: [] })
  ),

  // ---- CLI Manager ----
  http.post(`${TAURI_ENDPOINT}/cli_manager_claude_info_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_info_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_config_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_config_set`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_config_toml_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_config_toml_validate`, () =>
    HttpResponse.json({ ok: true, error: null })
  ),
  http.post(`${TAURI_ENDPOINT}/cli_manager_codex_config_toml_set`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_gemini_info_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_claude_env_set`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_claude_settings_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/cli_manager_claude_settings_set`, () => HttpResponse.json(null)),

  // ---- Claude Model Validation ----
  http.post(`${TAURI_ENDPOINT}/claude_provider_validate_model`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/claude_provider_get_api_key_plaintext`, () =>
    HttpResponse.json(null)
  ),
  http.post(`${TAURI_ENDPOINT}/claude_validation_history_list`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/claude_validation_history_clear_provider`, () =>
    HttpResponse.json(true)
  ),

  // ---- WSL ----
  http.post(`${TAURI_ENDPOINT}/wsl_detect`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/wsl_host_address_get`, () => HttpResponse.json(null)),
  http.post(`${TAURI_ENDPOINT}/wsl_distro_config_status`, () => HttpResponse.json([])),
  http.post(`${TAURI_ENDPOINT}/wsl_configure_clients`, () => HttpResponse.json(null)),

  // ---- Frontend Error Reporter ----
  http.post(`${TAURI_ENDPOINT}/app_frontend_error_report`, () => HttpResponse.json(true)),

  // ---- Notice ----
  http.post(`${TAURI_ENDPOINT}/notice_send`, () => HttpResponse.json(true)),

  // ---- CLI Proxy Sync ----
  http.post(`${TAURI_ENDPOINT}/cli_proxy_sync_enabled`, () => HttpResponse.json([])),

  // Catch-all: return `null` for any unimplemented command to keep tests stable by default.
  http.post(`${TAURI_ENDPOINT}/:command`, () => HttpResponse.json(null)),
];
