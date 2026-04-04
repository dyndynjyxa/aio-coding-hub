import { commands } from "../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "./generatedIpc";
import { invokeService } from "./invokeServiceCommand";
import { invokeTauriOrNull } from "./tauriInvoke";

export type CliKey = "claude" | "codex" | "gemini";

export type ClaudeModels = {
  main_model?: string | null;
  reasoning_model?: string | null;
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
};

export type ProviderSummary = {
  id: number;
  cli_key: CliKey;
  name: string;
  base_urls: string[];
  base_url_mode: "order" | "ping";
  claude_models: ClaudeModels;
  enabled: boolean;
  priority: number;
  cost_multiplier: number;
  limit_5h_usd: number | null;
  limit_daily_usd: number | null;
  daily_reset_mode: "fixed" | "rolling";
  daily_reset_time: string;
  limit_weekly_usd: number | null;
  limit_monthly_usd: number | null;
  limit_total_usd: number | null;
  tags: string[];
  note: string;
  created_at: number;
  updated_at: number;
  auth_mode: "api_key" | "oauth";
  oauth_provider_type: string | null;
  oauth_email: string | null;
  oauth_expires_at: number | null;
  oauth_last_error: string | null;
  source_provider_id: number | null;
  bridge_type: string | null;
  stream_idle_timeout_seconds: number | null;
};

export async function providersList(cliKey: CliKey) {
  return invokeGeneratedIpc<ProviderSummary[]>({
    title: "读取供应商列表失败",
    cmd: "providers_list",
    args: { cliKey },
    invoke: () =>
      commands.providersList(cliKey) as Promise<GeneratedCommandResult<ProviderSummary[]>>,
  });
}

export async function providerUpsert(input: {
  provider_id?: number | null;
  cli_key: CliKey;
  name: string;
  base_urls: string[];
  base_url_mode: "order" | "ping";
  auth_mode?: "api_key" | "oauth" | null;
  api_key?: string | null;
  enabled: boolean;
  cost_multiplier: number;
  priority?: number | null;
  claude_models?: ClaudeModels | null;
  limit_5h_usd: number | null;
  limit_daily_usd: number | null;
  daily_reset_mode: "fixed" | "rolling";
  daily_reset_time: string;
  limit_weekly_usd: number | null;
  limit_monthly_usd: number | null;
  limit_total_usd: number | null;
  tags?: string[];
  note?: string;
  source_provider_id?: number | null;
  bridge_type?: string | null;
  stream_idle_timeout_seconds?: number | null;
}) {
  const streamIdleTimeoutSeconds = Object.prototype.hasOwnProperty.call(
    input,
    "stream_idle_timeout_seconds"
  )
    ? (input.stream_idle_timeout_seconds ?? 0)
    : undefined;
  const payload = {
    providerId: input.provider_id ?? null,
    cliKey: input.cli_key,
    name: input.name,
    baseUrls: input.base_urls,
    baseUrlMode: input.base_url_mode,
    authMode: input.auth_mode ?? null,
    apiKey: input.api_key ?? null,
    enabled: input.enabled,
    costMultiplier: input.cost_multiplier,
    priority: input.priority ?? null,
    claudeModels: input.claude_models ?? null,
    limit5hUsd: input.limit_5h_usd,
    limitDailyUsd: input.limit_daily_usd,
    dailyResetMode: input.daily_reset_mode,
    dailyResetTime: input.daily_reset_time,
    limitWeeklyUsd: input.limit_weekly_usd,
    limitMonthlyUsd: input.limit_monthly_usd,
    limitTotalUsd: input.limit_total_usd,
    tags: input.tags ?? null,
    note: input.note ?? null,
    sourceProviderId: input.source_provider_id ?? null,
    bridgeType: input.bridge_type ?? null,
    streamIdleTimeoutSeconds: streamIdleTimeoutSeconds as number | null,
  };
  const logPayload = {
    ...payload,
    apiKey: payload.apiKey == null ? payload.apiKey : "[REDACTED]",
  };

  return invokeGeneratedIpc<ProviderSummary>({
    title: "保存供应商失败",
    cmd: "provider_upsert",
    args: { input: logPayload },
    invoke: () =>
      commands.providerUpsert(payload) as Promise<GeneratedCommandResult<ProviderSummary>>,
  });
}

export async function baseUrlPingMs(baseUrl: string) {
  return invokeService<number>("测试 Base URL 延迟失败", "base_url_ping_ms", { baseUrl });
}

export async function providerSetEnabled(providerId: number, enabled: boolean) {
  return invokeService<ProviderSummary>("更新供应商启用状态失败", "provider_set_enabled", {
    providerId,
    enabled,
  });
}

export async function providerDelete(providerId: number) {
  return invokeService<boolean>("删除供应商失败", "provider_delete", { providerId });
}

export async function providersReorder(cliKey: CliKey, orderedProviderIds: number[]) {
  return invokeService<ProviderSummary[]>("调整供应商顺序失败", "providers_reorder", {
    cliKey,
    orderedProviderIds,
  });
}

export async function providerGetApiKey(providerId: number) {
  return invokeService<string>("读取 API Key 失败", "provider_get_api_key", { providerId });
}

export async function providerClaudeTerminalLaunchCommand(providerId: number) {
  return invokeService<string>(
    "生成 Claude 终端启动命令失败",
    "provider_claude_terminal_launch_command",
    { providerId }
  );
}

export async function providerOAuthStartFlow(
  cliKey: string,
  providerId: number
): Promise<{ success: boolean; provider_type?: string; expires_at?: number } | null> {
  return invokeTauriOrNull<{ success: boolean; provider_type?: string; expires_at?: number }>(
    "provider_oauth_start_flow",
    { cliKey, providerId },
    { timeoutMs: 0 }
  );
}

export async function providerOAuthRefresh(
  providerId: number
): Promise<{ success: boolean; expires_at?: number } | null> {
  return invokeTauriOrNull<{ success: boolean; expires_at?: number }>("provider_oauth_refresh", {
    providerId,
  });
}

export async function providerOAuthDisconnect(
  providerId: number
): Promise<{ success: boolean } | null> {
  return invokeTauriOrNull<{ success: boolean }>("provider_oauth_disconnect", { providerId });
}

export async function providerOAuthStatus(providerId: number): Promise<{
  connected: boolean;
  provider_type?: string;
  email?: string;
  expires_at?: number;
  has_refresh_token?: boolean;
} | null> {
  return invokeTauriOrNull<{
    connected: boolean;
    provider_type?: string;
    email?: string;
    expires_at?: number;
    has_refresh_token?: boolean;
  }>("provider_oauth_status", { providerId });
}

export type OAuthLimitsResult = {
  limit_short_label?: string | null;
  limit_5h_text?: string | null;
  limit_weekly_text?: string | null;
  limit_5h_reset_at?: number | null;
  limit_weekly_reset_at?: number | null;
  raw_json?: Record<string, unknown> | null;
};

export async function providerOAuthFetchLimits(
  providerId: number
): Promise<OAuthLimitsResult | null> {
  return invokeTauriOrNull<OAuthLimitsResult>("provider_oauth_fetch_limits", { providerId });
}
