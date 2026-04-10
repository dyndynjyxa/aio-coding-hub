// Usage: Shared MSW in-memory state for tests that run through `invoke` -> fetch -> MSW handlers.

import type { AppAboutInfo } from "../../services/app/appAbout";
import type { CliProxyResult, CliProxyStatus } from "../../services/cli/cliProxy";
import type { DbDiskUsage } from "../../services/app/dataManagement";
import type { EnvConflict } from "../../services/cli/envConflicts";
import type { GatewayStatus } from "../../services/gateway/gateway";
import type { CliKey, ProviderSummary } from "../../services/providers/providers";
import type { AppSettings } from "../../services/settings/settings";
import type { SortModeActiveRow, SortModeSummary } from "../../services/providers/sortModes";
import type { UsageSummary } from "../../services/usage/usage";
import type { WorkspacesListResult } from "../../services/workspace/workspaces";

const DEFAULT_BASE_ORIGIN = "http://127.0.0.1:37123";

const DEFAULT_CLI_PROXY_STATUS: CliProxyStatus[] = [
  { cli_key: "claude", enabled: false, base_origin: null, applied_to_current_gateway: null },
  { cli_key: "codex", enabled: false, base_origin: null, applied_to_current_gateway: null },
  { cli_key: "gemini", enabled: false, base_origin: null, applied_to_current_gateway: null },
];

// Default settings matching the Rust backend defaults.
const DEFAULT_SETTINGS: AppSettings = {
  schema_version: 29,
  preferred_port: 37123,
  show_home_heatmap: true,
  show_home_usage: true,
  home_usage_period: "last15",
  gateway_listen_mode: "localhost",
  gateway_custom_listen_address: "",
  wsl_auto_config: false,
  wsl_target_cli: { claude: true, codex: true, gemini: true },
  cli_priority_order: ["claude", "codex", "gemini"],
  wsl_host_address_mode: "auto",
  wsl_custom_host_address: "127.0.0.1",
  codex_home_mode: "user_home_default",
  codex_home_override: "",
  auto_start: false,
  start_minimized: false,
  tray_enabled: true,
  enable_cli_proxy_startup_recovery: true,
  log_retention_days: 7,
  provider_cooldown_seconds: 30,
  provider_base_url_ping_cache_ttl_seconds: 60,
  upstream_first_byte_timeout_seconds: 30,
  upstream_stream_idle_timeout_seconds: 120,
  upstream_request_timeout_non_streaming_seconds: 0,
  update_releases_url: "https://github.com/dyndynjyxa/aio-coding-hub/releases",
  failover_max_attempts_per_provider: 5,
  failover_max_providers_to_try: 5,
  circuit_breaker_failure_threshold: 5,
  circuit_breaker_open_duration_minutes: 30,
  enable_circuit_breaker_notice: false,
  verbose_provider_error: true,
  intercept_anthropic_warmup_requests: true,
  enable_thinking_signature_rectifier: true,
  enable_thinking_budget_rectifier: true,
  enable_billing_header_rectifier: true,
  enable_codex_session_id_completion: true,
  enable_claude_metadata_user_id_injection: true,
  enable_cache_anomaly_monitor: false,
  enable_task_complete_notify: true,
  enable_notification_sound: true,
  enable_response_fixer: true,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
  response_fixer_max_json_depth: 200,
  response_fixer_max_fix_size: 1048576,
  cx2cc_fallback_model_opus: "gpt-5.4",
  cx2cc_fallback_model_sonnet: "gpt-5.4",
  cx2cc_fallback_model_haiku: "gpt-5.4",
  cx2cc_fallback_model_main: "gpt-5.4",
  cx2cc_model_reasoning_effort: "",
  cx2cc_service_tier: "",
  cx2cc_disable_response_storage: true,
  cx2cc_enable_reasoning_to_thinking: true,
  cx2cc_drop_stop_sequences: true,
  cx2cc_clean_schema: true,
  cx2cc_filter_batch_tool: true,
};

const DEFAULT_GATEWAY_STATUS: GatewayStatus = {
  running: false,
  port: null,
  base_url: null,
  listen_addr: null,
};

const DEFAULT_APP_ABOUT: AppAboutInfo = {
  os: "darwin",
  arch: "aarch64",
  profile: "debug",
  app_version: "0.0.0-test",
  bundle_type: null,
  run_mode: "development",
};

const DEFAULT_DB_DISK_USAGE: DbDiskUsage = {
  db_bytes: 0,
  wal_bytes: 0,
  shm_bytes: 0,
  total_bytes: 0,
};

const DEFAULT_USAGE_SUMMARY: UsageSummary = {
  requests_total: 0,
  requests_with_usage: 0,
  requests_success: 0,
  requests_failed: 0,
  avg_duration_ms: null,
  avg_ttfb_ms: null,
  avg_output_tokens_per_second: null,
  input_tokens: 0,
  output_tokens: 0,
  io_total_tokens: 0,
  total_tokens: 0,
  cache_read_input_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_creation_5m_input_tokens: 0,
};

let traceCounter = 0;
let cliProxyStatusAllState: CliProxyStatus[] = JSON.parse(JSON.stringify(DEFAULT_CLI_PROXY_STATUS));
let envConflictsState: EnvConflict[] = [];
let settingsState: AppSettings = clone(DEFAULT_SETTINGS);
let gatewayStatusState: GatewayStatus = clone(DEFAULT_GATEWAY_STATUS);
let providersState: Map<CliKey, ProviderSummary[]> = new Map();
let usageSummaryState: UsageSummary = clone(DEFAULT_USAGE_SUMMARY);
let appAboutState: AppAboutInfo = clone(DEFAULT_APP_ABOUT);
let dbDiskUsageState: DbDiskUsage = clone(DEFAULT_DB_DISK_USAGE);
let sortModesState: SortModeSummary[] = [];
let sortModeActiveState: SortModeActiveRow[] = [];
let workspacesState: Map<CliKey, WorkspacesListResult> = new Map();

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function nextTraceId(): string {
  traceCounter += 1;
  return `msw-${traceCounter}`;
}

export function resetMswState() {
  traceCounter = 0;
  cliProxyStatusAllState = clone(DEFAULT_CLI_PROXY_STATUS);
  envConflictsState = [];
  settingsState = clone(DEFAULT_SETTINGS);
  gatewayStatusState = clone(DEFAULT_GATEWAY_STATUS);
  providersState = new Map();
  usageSummaryState = clone(DEFAULT_USAGE_SUMMARY);
  appAboutState = clone(DEFAULT_APP_ABOUT);
  dbDiskUsageState = clone(DEFAULT_DB_DISK_USAGE);
  sortModesState = [];
  sortModeActiveState = [];
  workspacesState = new Map();
}

export function getCliProxyStatusAllState(): CliProxyStatus[] {
  return clone(cliProxyStatusAllState);
}

export function setCliProxyStatusAllState(next: CliProxyStatus[]) {
  cliProxyStatusAllState = clone(next);
}

export function getEnvConflictsState(): EnvConflict[] {
  return clone(envConflictsState);
}

export function setEnvConflictsState(next: EnvConflict[]) {
  envConflictsState = clone(next);
}

// -- Settings --

export function getSettingsState(): AppSettings {
  return clone(settingsState);
}

export function setSettingsState(next: AppSettings) {
  settingsState = clone(next);
}

export function mergeSettingsState(partial: Partial<AppSettings>): AppSettings {
  settingsState = { ...settingsState, ...partial };
  return clone(settingsState);
}

// -- Gateway --

export function getGatewayStatusState(): GatewayStatus {
  return clone(gatewayStatusState);
}

export function setGatewayStatusState(next: GatewayStatus) {
  gatewayStatusState = clone(next);
}

// -- Providers --

export function getProvidersState(cliKey: CliKey): ProviderSummary[] {
  return clone(providersState.get(cliKey) ?? []);
}

export function setProvidersState(cliKey: CliKey, next: ProviderSummary[]) {
  providersState.set(cliKey, clone(next));
}

// -- Usage --

export function getUsageSummaryState(): UsageSummary {
  return clone(usageSummaryState);
}

export function setUsageSummaryState(next: UsageSummary) {
  usageSummaryState = clone(next);
}

// -- App About --

export function getAppAboutState(): AppAboutInfo {
  return clone(appAboutState);
}

export function setAppAboutState(next: AppAboutInfo) {
  appAboutState = clone(next);
}

// -- DB Disk Usage --

export function getDbDiskUsageState(): DbDiskUsage {
  return clone(dbDiskUsageState);
}

export function setDbDiskUsageState(next: DbDiskUsage) {
  dbDiskUsageState = clone(next);
}

// -- Sort Modes --

export function getSortModesState(): SortModeSummary[] {
  return clone(sortModesState);
}

export function setSortModesState(next: SortModeSummary[]) {
  sortModesState = clone(next);
}

export function getSortModeActiveState(): SortModeActiveRow[] {
  return clone(sortModeActiveState);
}

export function setSortModeActiveState(next: SortModeActiveRow[]) {
  sortModeActiveState = clone(next);
}

// -- Workspaces --

export function getWorkspacesState(cliKey: CliKey): WorkspacesListResult {
  return clone(workspacesState.get(cliKey) ?? { active_id: null, items: [] });
}

export function setWorkspacesState(cliKey: CliKey, next: WorkspacesListResult) {
  workspacesState.set(cliKey, clone(next));
}

export function setCliProxyEnabledState(cliKey: CliKey, enabled: boolean): CliProxyStatus[] {
  const rowIndex = cliProxyStatusAllState.findIndex((row) => row.cli_key === cliKey);
  const baseOrigin = enabled ? DEFAULT_BASE_ORIGIN : null;
  if (rowIndex < 0) {
    cliProxyStatusAllState = [
      {
        cli_key: cliKey,
        enabled,
        base_origin: baseOrigin,
        applied_to_current_gateway: enabled ? true : null,
      },
      ...cliProxyStatusAllState,
    ];
    return getCliProxyStatusAllState();
  }

  const next = clone(cliProxyStatusAllState);
  next[rowIndex] = {
    ...next[rowIndex],
    enabled,
    base_origin: baseOrigin,
    applied_to_current_gateway: enabled ? true : null,
  };
  cliProxyStatusAllState = next;
  return getCliProxyStatusAllState();
}

export function buildCliProxySetEnabledResult(input: {
  cli_key: string;
  enabled: boolean;
}): CliProxyResult {
  const cliKey = input.cli_key;
  const enabled = input.enabled;

  if (cliKey !== "claude" && cliKey !== "codex" && cliKey !== "gemini") {
    return {
      trace_id: nextTraceId(),
      cli_key: cliKey as CliKey,
      enabled,
      ok: false,
      error_code: "UNSUPPORTED_CLI",
      message: `unsupported cli_key: ${cliKey}`,
      base_origin: null,
    };
  }

  const cli_key = cliKey as CliKey;
  const base_origin = enabled ? DEFAULT_BASE_ORIGIN : null;
  setCliProxyEnabledState(cli_key, enabled);

  return {
    trace_id: nextTraceId(),
    cli_key,
    enabled,
    ok: true,
    error_code: null,
    message: "",
    base_origin,
  };
}
