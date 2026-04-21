import {
  commands,
  type ClaudeModels as GeneratedClaudeModels,
  type DailyResetMode as GeneratedDailyResetMode,
  type ProviderAuthMode as GeneratedProviderAuthMode,
  type ProviderBaseUrlMode as GeneratedProviderBaseUrlMode,
  type ProviderOAuthDisconnectResult,
  type ProviderOAuthLimitsResult,
  type ProviderOAuthRefreshResult,
  type ProviderOAuthStartFlowResult,
  type ProviderOAuthStatusResult,
  type ProviderSummary as GeneratedProviderSummary,
  type ProviderUpsertInput as GeneratedProviderUpsertInput,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

type Override<TValue, TOverrides> = Omit<TValue, keyof TOverrides> & TOverrides;

type NullableGeneratedKeys<TValue extends object> = {
  [TKey in keyof TValue]-?: null extends TValue[TKey] ? TKey : never;
}[keyof TValue];

type NonNullableGeneratedKeys<TValue extends object> = Exclude<
  keyof TValue,
  NullableGeneratedKeys<TValue>
>;

type OptionalNullableGeneratedFields<TValue extends object> = Pick<
  TValue,
  NonNullableGeneratedKeys<TValue>
> &
  Partial<Pick<TValue, NullableGeneratedKeys<TValue>>>;

type RemapGeneratedKeys<
  TValue extends object,
  TMap extends Partial<Record<keyof TValue, PropertyKey>>,
> = {
  [TKey in keyof TValue as TKey extends keyof TMap ? TMap[TKey] & PropertyKey : TKey]: TValue[TKey];
};

export type {
  ProviderOAuthDisconnectResult,
  ProviderOAuthLimitsResult,
  ProviderOAuthRefreshResult,
  ProviderOAuthStartFlowResult,
  ProviderOAuthStatusResult,
};

export type CliKey = "claude" | "codex" | "gemini";

export type ClaudeModels = GeneratedClaudeModels;
export type DailyResetMode = GeneratedDailyResetMode;
export type ProviderAuthMode = GeneratedProviderAuthMode;
export type ProviderBaseUrlMode = GeneratedProviderBaseUrlMode;

export type ProviderSummary = Override<
  GeneratedProviderSummary,
  {
    cli_key: CliKey;
    auth_mode: ProviderAuthMode;
    api_key_configured?: GeneratedProviderSummary["api_key_configured"];
  }
>;

export async function providersList(cliKey: CliKey) {
  return invokeGeneratedIpc<ProviderSummary[]>({
    title: "读取供应商列表失败",
    cmd: "providers_list",
    args: { cliKey },
    invoke: () =>
      commands.providersList(cliKey) as Promise<GeneratedCommandResult<ProviderSummary[]>>,
  });
}

const providerUpsertFieldMap = {
  providerId: "provider_id",
  cliKey: "cli_key",
  name: "name",
  baseUrls: "base_urls",
  baseUrlMode: "base_url_mode",
  authMode: "auth_mode",
  apiKey: "api_key",
  enabled: "enabled",
  costMultiplier: "cost_multiplier",
  priority: "priority",
  claudeModels: "claude_models",
  limit5hUsd: "limit_5h_usd",
  limitDailyUsd: "limit_daily_usd",
  dailyResetMode: "daily_reset_mode",
  dailyResetTime: "daily_reset_time",
  limitWeeklyUsd: "limit_weekly_usd",
  limitMonthlyUsd: "limit_monthly_usd",
  limitTotalUsd: "limit_total_usd",
  tags: "tags",
  note: "note",
  sourceProviderId: "source_provider_id",
  bridgeType: "bridge_type",
  streamIdleTimeoutSeconds: "stream_idle_timeout_seconds",
} as const satisfies Record<keyof GeneratedProviderUpsertInput, string>;

type ProviderUpsertTransportInput =
  OptionalNullableGeneratedFields<GeneratedProviderUpsertInput>;

export type ProviderUpsertInput = Override<
  RemapGeneratedKeys<ProviderUpsertTransportInput, typeof providerUpsertFieldMap>,
  {
    cli_key: CliKey;
    auth_mode?: ProviderAuthMode | null;
    base_url_mode: ProviderBaseUrlMode;
    claude_models?: ClaudeModels | null;
    limit_5h_usd: number | null;
    limit_daily_usd: number | null;
    daily_reset_mode: DailyResetMode;
    daily_reset_time: string;
    limit_weekly_usd: number | null;
    limit_monthly_usd: number | null;
    limit_total_usd: number | null;
  }
>;

export async function providerUpsert(input: ProviderUpsertInput) {
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
  return invokeGeneratedIpc<number>({
    title: "测试 Base URL 延迟失败",
    cmd: "base_url_ping_ms",
    args: { baseUrl },
    invoke: () => commands.baseUrlPingMs(baseUrl) as Promise<GeneratedCommandResult<number>>,
  });
}

export async function providerSetEnabled(
  providerId: number,
  enabled: boolean
): Promise<ProviderSummary | null> {
  return invokeGeneratedIpc<ProviderSummary>({
    title: "更新供应商启用状态失败",
    cmd: "provider_set_enabled",
    args: { providerId, enabled },
    invoke: () =>
      commands.providerSetEnabled(providerId, enabled) as Promise<
        GeneratedCommandResult<ProviderSummary>
      >,
  });
}

export async function providerDelete(providerId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "删除供应商失败",
    cmd: "provider_delete",
    args: { providerId },
    invoke: () =>
      commands.providerDelete(providerId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function providersReorder(
  cliKey: CliKey,
  orderedProviderIds: number[]
): Promise<ProviderSummary[] | null> {
  return invokeGeneratedIpc<ProviderSummary[]>({
    title: "调整供应商顺序失败",
    cmd: "providers_reorder",
    args: { cliKey, orderedProviderIds },
    invoke: () =>
      commands.providersReorder(cliKey, orderedProviderIds) as Promise<
        GeneratedCommandResult<ProviderSummary[]>
      >,
  });
}

export async function providerDuplicate(providerId: number): Promise<ProviderSummary | null> {
  return invokeGeneratedIpc<ProviderSummary>({
    title: "复制供应商失败",
    cmd: "provider_duplicate",
    args: { providerId },
    invoke: () =>
      commands.providerDuplicate(providerId) as Promise<GeneratedCommandResult<ProviderSummary>>,
  });
}

export async function providerCopyApiKeyToClipboard(providerId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "复制 API Key 失败",
    cmd: "provider_copy_api_key_to_clipboard",
    args: { providerId },
    invoke: () =>
      commands.providerCopyApiKeyToClipboard(providerId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function providerClaudeTerminalLaunchCommand(providerId: number) {
  return invokeGeneratedIpc<string>({
    title: "生成 Claude 终端启动命令失败",
    cmd: "provider_claude_terminal_launch_command",
    args: { providerId },
    invoke: () =>
      commands.providerClaudeTerminalLaunchCommand(providerId) as Promise<
        GeneratedCommandResult<string>
      >,
  });
}

export async function providerOAuthStartFlow(
  cliKey: string,
  providerId: number
): Promise<ProviderOAuthStartFlowResult> {
  return invokeGeneratedIpc<ProviderOAuthStartFlowResult>({
    title: "启动 OAuth 登录失败",
    cmd: "provider_oauth_start_flow",
    args: { cliKey, providerId },
    invoke: () =>
      commands.providerOauthStartFlow(cliKey, providerId) as Promise<
        GeneratedCommandResult<ProviderOAuthStartFlowResult>
      >,
  });
}

export async function providerOAuthRefresh(providerId: number): Promise<ProviderOAuthRefreshResult> {
  return invokeGeneratedIpc<ProviderOAuthRefreshResult>({
    title: "刷新 OAuth 登录失败",
    cmd: "provider_oauth_refresh",
    args: { providerId },
    invoke: () =>
      commands.providerOauthRefresh(providerId) as Promise<
        GeneratedCommandResult<ProviderOAuthRefreshResult>
      >,
  });
}

export async function providerOAuthDisconnect(
  providerId: number
): Promise<ProviderOAuthDisconnectResult> {
  return invokeGeneratedIpc<ProviderOAuthDisconnectResult>({
    title: "断开 OAuth 登录失败",
    cmd: "provider_oauth_disconnect",
    args: { providerId },
    invoke: () =>
      commands.providerOauthDisconnect(providerId) as Promise<
        GeneratedCommandResult<ProviderOAuthDisconnectResult>
      >,
  });
}

export async function providerOAuthStatus(providerId: number): Promise<ProviderOAuthStatusResult> {
  return invokeGeneratedIpc<ProviderOAuthStatusResult>({
    title: "读取 OAuth 状态失败",
    cmd: "provider_oauth_status",
    args: { providerId },
    invoke: () =>
      commands.providerOauthStatus(providerId) as Promise<
        GeneratedCommandResult<ProviderOAuthStatusResult>
      >,
  });
}

export type OAuthLimitsResult = ProviderOAuthLimitsResult;

export async function providerOAuthFetchLimits(
  providerId: number
): Promise<OAuthLimitsResult | null> {
  return invokeGeneratedIpc<OAuthLimitsResult>({
    title: "读取 OAuth 限额失败",
    cmd: "provider_oauth_fetch_limits",
    args: { providerId },
    invoke: () =>
      commands.providerOauthFetchLimits(providerId) as Promise<
        GeneratedCommandResult<OAuthLimitsResult>
      >,
  });
}

// ---------------------------------------------------------------------------
// Provider Type Info — centralised auth-mode / bridge derivation
// ---------------------------------------------------------------------------

export interface ProviderTypeInfo {
  /** Whether this is a CX2CC bridge (has source_provider_id or bridge_type is cx2cc) */
  isCx2cc: boolean;
  /** Whether this is a CX2CC gateway (bridge_type=cx2cc but no source_provider_id) */
  isCx2ccGateway: boolean;
  /** Whether this is OAuth mode */
  isOAuth: boolean;
  /** Effective auth mode: api_key / oauth / cx2cc */
  effectiveAuthMode: "api_key" | "oauth" | "cx2cc";
}

export function getProviderTypeInfo(
  provider:
    | Pick<ProviderSummary, "auth_mode" | "bridge_type" | "source_provider_id">
    | null
    | undefined
): ProviderTypeInfo {
  if (!provider) {
    return { isCx2cc: false, isCx2ccGateway: false, isOAuth: false, effectiveAuthMode: "api_key" };
  }
  const isCx2cc = provider.source_provider_id != null || provider.bridge_type === "cx2cc";
  const isCx2ccGateway = provider.bridge_type === "cx2cc" && provider.source_provider_id == null;
  const isOAuth = provider.auth_mode === "oauth";
  const effectiveAuthMode: ProviderTypeInfo["effectiveAuthMode"] = isCx2cc
    ? "cx2cc"
    : isOAuth
      ? "oauth"
      : "api_key";
  return { isCx2cc, isCx2ccGateway, isOAuth, effectiveAuthMode };
}
