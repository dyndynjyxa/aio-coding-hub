import { invokeService } from "./invokeServiceCommand";

export type CliKey = "claude" | "codex" | "gemini";

export type ClaudeModels = {
  main_model?: string | null;
  reasoning_model?: string | null;
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
};

export type AuthType = "api_key" | "oauth";
export type OAuthProviderType = "claude_oauth" | "codex_oauth" | "gemini_oauth";

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
  auth_type: AuthType;
  oauth_provider_type: OAuthProviderType | null;
  oauth_expires_at: number | null;
  oauth_last_error: string | null;
  oauth_quota_exceeded: boolean;
  oauth_quota_recover_at: number | null;
  created_at: number;
  updated_at: number;
};

export async function providersList(cliKey: CliKey) {
  return invokeService<ProviderSummary[]>("读取供应商列表失败", "providers_list", { cliKey });
}

export async function providerUpsert(input: {
  provider_id?: number | null;
  cli_key: CliKey;
  name: string;
  base_urls: string[];
  base_url_mode: "order" | "ping";
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
  auth_type?: AuthType;
  oauth_provider_type?: OAuthProviderType | null;
}) {
  return invokeService<ProviderSummary>("保存供应商失败", "provider_upsert", {
    providerId: input.provider_id ?? null,
    cliKey: input.cli_key,
    name: input.name,
    baseUrls: input.base_urls,
    baseUrlMode: input.base_url_mode,
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
    authType: input.auth_type ?? null,
    oauthProviderType: input.oauth_provider_type ?? null,
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

export async function providerClaudeTerminalLaunchCommand(providerId: number) {
  return invokeService<string>(
    "生成 Claude 终端启动命令失败",
    "provider_claude_terminal_launch_command",
    { providerId }
  );
}

export type OAuthProviderStatus = {
  has_token: boolean;
  expires_at: number | null;
  last_error: string | null;
  quota_exceeded: boolean;
  quota_recover_at: number | null;
};

export async function oauthStartLogin(providerId: number) {
  return invokeService<OAuthProviderStatus>("OAuth 登录失败", "oauth_start_login", { providerId });
}

export async function oauthProviderStatus(providerId: number) {
  return invokeService<OAuthProviderStatus>("读取 OAuth 状态失败", "oauth_provider_status", {
    providerId,
  });
}

export async function oauthProviderLogout(providerId: number) {
  return invokeService<OAuthProviderStatus>("OAuth 登出失败", "oauth_provider_logout", {
    providerId,
  });
}
