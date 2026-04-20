import { describe, expect, it } from "vitest";
import { appEventNames } from "../../constants/appEvents";
import { HOME_USAGE_PERIOD_VALUES } from "../../constants/homeUsagePeriods";
import bindingsSource from "../bindings.ts?raw";
import heartbeatSource from "../../../src-tauri/src/app/heartbeat_watchdog.rs?raw";
import noticeSource from "../../../src-tauri/src/app/notice.rs?raw";

function extractStringUnionLiterals(source: string, typeName: string) {
  const match = source.match(new RegExp(`export type ${typeName} = (.+)$`, "m"));
  expect(match).toBeTruthy();
  return Array.from(match![1].matchAll(/"([^"]+)"/g), (item) => item[1]);
}

function extractRustStringConst(source: string, constName: string) {
  const match = source.match(new RegExp(`const ${constName}: &str = "([^"]+)";`));
  expect(match).toBeTruthy();
  return match![1];
}

describe("generated/bindings.ts contract", () => {
  it("documents the generated IPC ownership surface", () => {
    expect(bindingsSource).toContain(
      "NOTE: Generated IPC contract for settings, config migration, desktop, app management, gateway, request-log, CLI update, CLI proxy, provider, WSL, sort-mode, provider-limit, usage, cost, model-price, prompt, workspace, skills, MCP, CLI manager, CLI sessions, Claude validation, notice, and env-conflict command families."
    );
    expect(bindingsSource).toContain("settings_get");
    expect(bindingsSource).toContain("settings_gateway_rectifier_set");
    expect(bindingsSource).toContain("settings_circuit_breaker_notice_set");
    expect(bindingsSource).toContain("settings_codex_session_id_completion_set");
    expect(bindingsSource).toContain("desktop_clipboard_write_text");
    expect(bindingsSource).toContain("desktop_dialog_open");
    expect(bindingsSource).toContain("desktop_dialog_save");
    expect(bindingsSource).toContain("desktop_notification_notify");
    expect(bindingsSource).toContain("desktop_opener_open_path");
    expect(bindingsSource).toContain("desktop_opener_open_url");
    expect(bindingsSource).toContain("desktop_opener_reveal_item_in_dir");
    expect(bindingsSource).toContain("desktop_updater_check");
    expect(bindingsSource).toContain("app_about_get");
    expect(bindingsSource).toContain("app_heartbeat_pong");
    expect(bindingsSource).toContain("app_frontend_error_report");
    expect(bindingsSource).toContain("app_data_reset");
    expect(bindingsSource).toContain("gateway_status");
    expect(bindingsSource).toContain("gateway_sessions_list");
    expect(bindingsSource).toContain("gateway_upstream_proxy_validate");
    expect(bindingsSource).toContain("request_logs_list_all");
    expect(bindingsSource).toContain("request_attempt_logs_by_trace_id");
    expect(bindingsSource).toContain("cli_proxy_status_all");
    expect(bindingsSource).toContain("cli_proxy_set_enabled");
    expect(bindingsSource).toContain("provider_upsert");
    expect(bindingsSource).toContain("provider_oauth_fetch_limits");
    expect(bindingsSource).toContain("wsl_detect");
    expect(bindingsSource).toContain("sort_modes_list");
    expect(bindingsSource).toContain("provider_limit_usage_v1");
    expect(bindingsSource).toContain("usage_summary_v2");
    expect(bindingsSource).toContain("cost_summary_v1");
    expect(bindingsSource).toContain("model_prices_list");
    expect(bindingsSource).toContain("prompts_list");
    expect(bindingsSource).toContain("workspaces_list");
    expect(bindingsSource).toContain("skills_installed_list");
    expect(bindingsSource).toContain("mcp_servers_list");
    expect(bindingsSource).toContain("cli_manager_codex_config_get");
    expect(bindingsSource).toContain("cli_sessions_projects_list");
    expect(bindingsSource).toContain("claude_provider_validate_model");
    expect(bindingsSource).toContain("notice_send");
    expect(bindingsSource).toContain("env_conflicts_check");
    expect(bindingsSource).toContain("config_export");
    expect(bindingsSource).toContain("cli_update");
    expect(bindingsSource).toContain("providers_reorder");
  });

  it("exports the home usage period literals used by runtime settings", () => {
    expect(extractStringUnionLiterals(bindingsSource, "HomeUsagePeriod")).toEqual([
      ...HOME_USAGE_PERIOD_VALUES,
    ]);
    expect(bindingsSource).not.toContain('"last_7"');
    expect(bindingsSource).not.toContain('"last_15"');
    expect(bindingsSource).not.toContain('"last_30"');
  });

  it("includes secret-safe upstream proxy settings in the generated settings contract", () => {
    expect(bindingsSource).toContain("upstream_proxy_enabled");
    expect(bindingsSource).toContain("upstream_proxy_url");
    expect(bindingsSource).toContain("upstream_proxy_username");
    expect(bindingsSource).toContain("upstream_proxy_password_configured");
    expect(bindingsSource).toContain("upstreamProxyEnabled");
    expect(bindingsSource).toContain("upstreamProxyUrl");
    expect(bindingsSource).toContain("upstreamProxyUsername");
    expect(bindingsSource).toContain("upstreamProxyPassword: SensitiveStringUpdate | null");
    expect(bindingsSource).toContain("export type SensitiveStringUpdate");
    expect(bindingsSource).toContain("export type SettingsMutationResult");
  });

  it("leaves updater install outside generated bindings when a Channel callback is required", () => {
    expect(bindingsSource).toContain("desktop_updater_check");
    expect(bindingsSource).not.toContain("desktop_updater_download_and_install");
    expect(bindingsSource).not.toContain("desktopUpdaterDownloadAndInstall");
  });

  it("keeps Rust app event emitters aligned with shared frontend constants", () => {
    expect(extractRustStringConst(heartbeatSource, "HEARTBEAT_EVENT_NAME")).toBe(
      appEventNames.heartbeat
    );
    expect(extractRustStringConst(noticeSource, "NOTICE_EVENT_NAME")).toBe(appEventNames.notice);
  });
});
