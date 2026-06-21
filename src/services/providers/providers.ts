import {
  commands,
  type ClaudeModels as GeneratedClaudeModels,
  type DailyResetMode as GeneratedDailyResetMode,
  type ProviderAuthMode as GeneratedProviderAuthMode,
  type ProviderAvailabilityResult,
  type ProviderBaseUrlMode as GeneratedProviderBaseUrlMode,
  type ProviderOAuthDeviceCodeCancelResult as GeneratedProviderOAuthDeviceCodeCancelResult,
  type ProviderOAuthDeviceCodePollResult as GeneratedProviderOAuthDeviceCodePollResult,
  type ProviderOAuthDeviceCodeStartResult as GeneratedProviderOAuthDeviceCodeStartResult,
  type ProviderOAuthDisconnectResult,
  type ProviderOAuthLimitsResult,
  type ProviderOAuthRefreshResult,
  type ProviderOAuthResetCodexQuotaResult,
  type ProviderOAuthStartFlowResult,
  type ProviderOAuthStatusResult,
  type ProviderSummary as GeneratedProviderSummary,
  type ProviderUpsertInput as GeneratedProviderUpsertInput,
} from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import {
  narrowGeneratedStringUnion,
  type NullableGeneratedKeys,
  type RemapGeneratedKeys,
  type Override,
} from "../generatedTypeUtils";
import { createRiskyIpcConfirm } from "../ipcConfirm";

export type {
  ProviderAvailabilityResult,
  GeneratedProviderOAuthDeviceCodePollResult as ProviderOAuthDeviceCodePollResult,
  GeneratedProviderOAuthDeviceCodeStartResult as ProviderOAuthDeviceCodeStartResult,
  GeneratedProviderOAuthDeviceCodeCancelResult as ProviderOAuthDeviceCodeCancelResult,
  ProviderOAuthDisconnectResult,
  ProviderOAuthLimitsResult,
  ProviderOAuthRefreshResult,
  ProviderOAuthResetCodexQuotaResult,
  ProviderOAuthStartFlowResult,
  ProviderOAuthStatusResult,
};

export type CliKey = "claude" | "codex" | "gemini";

export type ClaudeModels = GeneratedClaudeModels;
export type DailyResetMode = GeneratedDailyResetMode;
export type ProviderAuthMode = GeneratedProviderAuthMode;
export type ProviderBaseUrlMode = GeneratedProviderBaseUrlMode;

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];
const PROVIDER_AUTH_MODE_VALUES = [
  "api_key",
  "oauth",
] as const satisfies readonly ProviderAuthMode[];
export const MAX_PROVIDER_ORDER_IDS = 512;

export type ProviderSummary = Override<
  GeneratedProviderSummary,
  {
    cli_key: CliKey;
    auth_mode: ProviderAuthMode;
  }
>;

export type ProviderRouteRow = {
  provider_id: number;
};

type ProviderDeleteCommandArgs = Parameters<typeof commands.providerDelete>;

export type ProviderDeleteOptions = {
  clearUsageStats?: ProviderDeleteCommandArgs[1] | null;
};

type ProviderUpsertFieldMap = {
  providerId: "providerId";
  cliKey: "cliKey";
  name: "name";
  baseUrls: "baseUrls";
  baseUrlMode: "baseUrlMode";
  authMode: "authMode";
  apiKey: "apiKey";
  enabled: "enabled";
  costMultiplier: "costMultiplier";
  priority: "priority";
  claudeModels: "claudeModels";
  limit5hUsd: "limit5hUsd";
  limitDailyUsd: "limitDailyUsd";
  dailyResetMode: "dailyResetMode";
  dailyResetTime: "dailyResetTime";
  limitWeeklyUsd: "limitWeeklyUsd";
  limitMonthlyUsd: "limitMonthlyUsd";
  limitTotalUsd: "limitTotalUsd";
  tags: "tags";
  note: "note";
  sourceProviderId: "sourceProviderId";
  bridgeType: "bridgeType";
  streamIdleTimeoutSeconds: "streamIdleTimeoutSeconds";
};

type ProviderUpsertAuthority = RemapGeneratedKeys<
  GeneratedProviderUpsertInput,
  ProviderUpsertFieldMap &
    Record<keyof GeneratedProviderUpsertInput, keyof GeneratedProviderUpsertInput>
>;

type ProviderUpsertOptionalKeys =
  | NullableGeneratedKeys<ProviderUpsertAuthority>
  | "streamIdleTimeoutSeconds";

export type ProviderUpsertInput = Omit<
  ProviderUpsertAuthority,
  ProviderUpsertOptionalKeys | "cliKey"
> & {
  cliKey: CliKey;
} & Partial<Pick<ProviderUpsertAuthority, ProviderUpsertOptionalKeys>>;

type ProviderUpsertTransportInput = Omit<
  GeneratedProviderUpsertInput,
  "streamIdleTimeoutSeconds"
> & {
  streamIdleTimeoutSeconds?: GeneratedProviderUpsertInput["streamIdleTimeoutSeconds"];
};

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value, CLI_KEY_VALUES, label);
}

export function validateProviderCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function toProviderAuthMode(value: string, label: string): ProviderAuthMode {
  return narrowGeneratedStringUnion(value, PROVIDER_AUTH_MODE_VALUES, label);
}

function toProviderSummary(value: GeneratedProviderSummary): ProviderSummary {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "providers.cli_key"),
    auth_mode: toProviderAuthMode(value.auth_mode, "providers.auth_mode"),
  };
}

export function validateProviderId(providerId: number, label = "providerId"): number {
  if (!Number.isSafeInteger(providerId) || providerId <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${providerId}`);
  }
  return providerId;
}

function toProviderUpsertPayload(input: ProviderUpsertInput): ProviderUpsertTransportInput {
  const providerId = input.providerId == null ? null : validateProviderId(input.providerId);
  const sourceProviderId =
    input.sourceProviderId == null
      ? null
      : validateProviderId(input.sourceProviderId, "sourceProviderId");
  const cliKey = validateProviderCliKey(input.cliKey);

  const payloadBase = {
    providerId,
    cliKey,
    name: input.name,
    baseUrls: input.baseUrls,
    baseUrlMode: input.baseUrlMode,
    authMode: input.authMode ?? null,
    apiKey: input.apiKey ?? null,
    enabled: input.enabled,
    costMultiplier: input.costMultiplier,
    priority: input.priority ?? null,
    claudeModels: input.claudeModels ?? null,
    limit5hUsd: input.limit5hUsd ?? null,
    limitDailyUsd: input.limitDailyUsd ?? null,
    dailyResetMode: input.dailyResetMode ?? null,
    dailyResetTime: input.dailyResetTime ?? null,
    limitWeeklyUsd: input.limitWeeklyUsd ?? null,
    limitMonthlyUsd: input.limitMonthlyUsd ?? null,
    limitTotalUsd: input.limitTotalUsd ?? null,
    tags: input.tags ?? null,
    note: input.note ?? null,
    sourceProviderId,
    bridgeType: input.bridgeType ?? null,
  } satisfies Omit<GeneratedProviderUpsertInput, "streamIdleTimeoutSeconds">;

  if (Object.prototype.hasOwnProperty.call(input, "streamIdleTimeoutSeconds")) {
    return {
      ...payloadBase,
      streamIdleTimeoutSeconds: input.streamIdleTimeoutSeconds ?? 0,
    } satisfies ProviderUpsertTransportInput;
  }

  return payloadBase;
}

function validateOrderedProviderIds(orderedProviderIds: number[]) {
  if (orderedProviderIds.length > MAX_PROVIDER_ORDER_IDS) {
    throw new Error(
      `SEC_INVALID_INPUT: orderedProviderIds must contain at most ${MAX_PROVIDER_ORDER_IDS} entries`
    );
  }

  const seen = new Set<number>();
  for (const providerId of orderedProviderIds) {
    validateProviderId(providerId);
    if (seen.has(providerId)) {
      throw new Error(`SEC_INVALID_INPUT: duplicate providerId=${providerId}`);
    }
    seen.add(providerId);
  }
}

export async function providersList(cliKey: CliKey) {
  const normalizedCliKey = validateProviderCliKey(cliKey);

  return invokeGeneratedIpc<ProviderSummary[]>({
    title: "读取供应商列表失败",
    cmd: "providers_list",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.providersList(normalizedCliKey), (rows) =>
        rows.map(toProviderSummary)
      ),
  });
}

export async function providerUpsert(input: ProviderUpsertInput) {
  const payload = toProviderUpsertPayload(input);
  const logPayload = {
    ...payload,
    apiKey: payload.apiKey == null ? payload.apiKey : "[REDACTED]",
  };

  return invokeGeneratedIpc<ProviderSummary>({
    title: "保存供应商失败",
    cmd: "provider_upsert",
    args: { input: logPayload },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.providerUpsert(payload as GeneratedProviderUpsertInput),
        toProviderSummary
      ),
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
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderSummary>({
    title: "更新供应商启用状态失败",
    cmd: "provider_set_enabled",
    args: { providerId: normalizedProviderId, enabled },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.providerSetEnabled(normalizedProviderId, enabled),
        toProviderSummary
      ),
  });
}

export async function providerDelete(providerId: number, options: ProviderDeleteOptions = {}) {
  const normalizedProviderId = validateProviderId(providerId);
  const clearUsageStats = options.clearUsageStats === true;

  return invokeGeneratedIpc<boolean>({
    title: "删除供应商失败",
    cmd: "provider_delete",
    args: { providerId: normalizedProviderId, clearUsageStats },
    invoke: () =>
      commands.providerDelete(normalizedProviderId, clearUsageStats) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function providersReorder(
  cliKey: CliKey,
  orderedProviderIds: number[]
): Promise<ProviderSummary[] | null> {
  const normalizedCliKey = validateProviderCliKey(cliKey);
  validateOrderedProviderIds(orderedProviderIds);

  return invokeGeneratedIpc<ProviderSummary[]>({
    title: "调整供应商顺序失败",
    cmd: "providers_reorder",
    args: { cliKey: normalizedCliKey, orderedProviderIds },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.providersReorder(normalizedCliKey, orderedProviderIds),
        (rows) => rows.map(toProviderSummary)
      ),
  });
}

export async function defaultRouteProvidersList(cliKey: CliKey) {
  const normalizedCliKey = validateProviderCliKey(cliKey);

  return invokeGeneratedIpc<ProviderRouteRow[]>({
    title: "读取 Default 调用顺序失败",
    cmd: "default_route_providers_list",
    args: { cliKey: normalizedCliKey },
    invoke: () =>
      commands.defaultRouteProvidersList(normalizedCliKey) as Promise<
        GeneratedCommandResult<ProviderRouteRow[]>
      >,
  });
}

export async function defaultRouteProvidersSetOrder(cliKey: CliKey, orderedProviderIds: number[]) {
  const normalizedCliKey = validateProviderCliKey(cliKey);
  validateOrderedProviderIds(orderedProviderIds);

  return invokeGeneratedIpc<ProviderRouteRow[]>({
    title: "更新 Default 调用顺序失败",
    cmd: "default_route_providers_set_order",
    args: { cliKey: normalizedCliKey, orderedProviderIds },
    invoke: () =>
      commands.defaultRouteProvidersSetOrder(normalizedCliKey, orderedProviderIds) as Promise<
        GeneratedCommandResult<ProviderRouteRow[]>
      >,
  });
}

export async function providerDuplicate(providerId: number): Promise<ProviderSummary | null> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderSummary>({
    title: "复制供应商失败",
    cmd: "provider_duplicate",
    args: { providerId: normalizedProviderId },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.providerDuplicate(normalizedProviderId),
        toProviderSummary
      ),
  });
}

export async function providerCopyApiKeyToClipboard(providerId: number) {
  const normalizedProviderId = validateProviderId(providerId);
  const confirm = createRiskyIpcConfirm(
    "provider_copy_api_key_to_clipboard",
    `provider:${normalizedProviderId}:api_key`
  );
  return invokeGeneratedIpc<boolean>({
    title: "复制 API Key 失败",
    cmd: "provider_copy_api_key_to_clipboard",
    args: { providerId: normalizedProviderId, confirm },
    invoke: () =>
      commands.providerCopyApiKeyToClipboard(normalizedProviderId, confirm) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function providerClaudeTerminalLaunchCommand(providerId: number) {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<string>({
    title: "生成 Claude 终端启动命令失败",
    cmd: "provider_claude_terminal_launch_command",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerClaudeTerminalLaunchCommand(normalizedProviderId) as Promise<
        GeneratedCommandResult<string>
      >,
  });
}

export async function providerOAuthStartFlow(
  cliKey: string,
  providerId: number
): Promise<ProviderOAuthStartFlowResult> {
  const normalizedCliKey = validateProviderCliKey(cliKey);
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderOAuthStartFlowResult>({
    title: "启动 OAuth 登录失败",
    cmd: "provider_oauth_start_flow",
    args: { cliKey: normalizedCliKey, providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthStartFlow(normalizedCliKey, normalizedProviderId) as Promise<
        GeneratedCommandResult<ProviderOAuthStartFlowResult>
      >,
  });
}

export async function providerOAuthStartDeviceFlow(
  providerId: number
): Promise<GeneratedProviderOAuthDeviceCodeStartResult> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<GeneratedProviderOAuthDeviceCodeStartResult>({
    title: "启动设备码登录失败",
    cmd: "provider_oauth_start_device_flow",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthStartDeviceFlow(normalizedProviderId) as Promise<
        GeneratedCommandResult<GeneratedProviderOAuthDeviceCodeStartResult>
      >,
  });
}

export async function providerOAuthPollDeviceFlow(
  providerId: number,
  flowId: string,
  deviceCode: string,
  userCode: string
): Promise<GeneratedProviderOAuthDeviceCodePollResult> {
  const normalizedProviderId = validateProviderId(providerId);
  const normalizedFlowId = flowId.trim();
  if (!normalizedFlowId) {
    throw new Error("SEC_INVALID_INPUT: invalid flowId");
  }

  return invokeGeneratedIpc<GeneratedProviderOAuthDeviceCodePollResult>({
    title: "轮询设备码登录失败",
    cmd: "provider_oauth_poll_device_flow",
    args: { providerId: normalizedProviderId, flowId: normalizedFlowId, deviceCode, userCode },
    invoke: () =>
      commands.providerOauthPollDeviceFlow({
        providerId: normalizedProviderId,
        flowId: normalizedFlowId,
        deviceCode,
        userCode,
      }) as Promise<GeneratedCommandResult<GeneratedProviderOAuthDeviceCodePollResult>>,
  });
}

export async function providerOAuthCancelDeviceFlow(
  flowId: string
): Promise<GeneratedProviderOAuthDeviceCodeCancelResult> {
  const normalizedFlowId = flowId.trim();

  return invokeGeneratedIpc<GeneratedProviderOAuthDeviceCodeCancelResult>({
    title: "取消设备码登录失败",
    cmd: "provider_oauth_cancel_device_flow",
    args: { flowId: normalizedFlowId },
    invoke: () =>
      commands.providerOauthCancelDeviceFlow(normalizedFlowId) as Promise<
        GeneratedCommandResult<GeneratedProviderOAuthDeviceCodeCancelResult>
      >,
  });
}

export async function providerOAuthRefresh(
  providerId: number
): Promise<ProviderOAuthRefreshResult> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderOAuthRefreshResult>({
    title: "刷新 OAuth 登录失败",
    cmd: "provider_oauth_refresh",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthRefresh(normalizedProviderId) as Promise<
        GeneratedCommandResult<ProviderOAuthRefreshResult>
      >,
  });
}

export async function providerOAuthDisconnect(
  providerId: number
): Promise<ProviderOAuthDisconnectResult> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderOAuthDisconnectResult>({
    title: "断开 OAuth 登录失败",
    cmd: "provider_oauth_disconnect",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthDisconnect(normalizedProviderId) as Promise<
        GeneratedCommandResult<ProviderOAuthDisconnectResult>
      >,
  });
}

export async function providerOAuthStatus(providerId: number): Promise<ProviderOAuthStatusResult> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderOAuthStatusResult>({
    title: "读取 OAuth 状态失败",
    cmd: "provider_oauth_status",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthStatus(normalizedProviderId) as Promise<
        GeneratedCommandResult<ProviderOAuthStatusResult>
      >,
  });
}

export type OAuthLimitsResult = ProviderOAuthLimitsResult;

function parseLeadingOAuthQuotaNumber(text: string): [number, string] | null {
  const match = text.match(/^(\d+(?:\.\d+)?)(.*)$/);
  if (!match) return null;
  const value = Number.parseFloat(match[1].replace(/,/g, ""));
  if (!Number.isFinite(value)) return null;
  return [value, match[2] ?? ""];
}

export function isExhaustedOAuthQuotaText(value: string | null | undefined): boolean {
  const text = value?.trim();
  if (!text) return false;

  const parsed = parseLeadingOAuthQuotaNumber(text.replace(/,/g, ""));
  if (!parsed) return false;

  const [remaining, restRaw] = parsed;
  if (Math.abs(remaining) > Number.EPSILON) return false;

  const rest = restRaw.trimStart();
  if (!rest) return true;

  const first = rest[0];
  return first === "%" || first === "/" || /\p{L}/u.test(first);
}

export function hasInsufficientOAuthQuota(limits: OAuthLimitsResult | null): boolean {
  return (
    isExhaustedOAuthQuotaText(limits?.limit_5h_text) ||
    isExhaustedOAuthQuotaText(limits?.limit_weekly_text)
  );
}

export async function providerOAuthFetchLimits(
  providerId: number
): Promise<OAuthLimitsResult | null> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<OAuthLimitsResult>({
    title: "读取 OAuth 限额失败",
    cmd: "provider_oauth_fetch_limits",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerOauthFetchLimits(normalizedProviderId) as Promise<
        GeneratedCommandResult<OAuthLimitsResult>
      >,
  });
}

export async function providerOAuthResetCodexQuota(
  providerId: number
): Promise<ProviderOAuthResetCodexQuotaResult> {
  const normalizedProviderId = validateProviderId(providerId);
  const confirm = createRiskyIpcConfirm(
    "provider_oauth_reset_codex_quota",
    `provider:${normalizedProviderId}:codex_reset_credit`
  );

  return invokeGeneratedIpc<ProviderOAuthResetCodexQuotaResult>({
    title: "重置 Codex OAuth 额度失败",
    cmd: "provider_oauth_reset_codex_quota",
    args: { providerId: normalizedProviderId, confirm },
    invoke: () =>
      commands.providerOauthResetCodexQuota(normalizedProviderId, confirm) as Promise<
        GeneratedCommandResult<ProviderOAuthResetCodexQuotaResult>
      >,
  });
}

export async function providerTestAvailability(
  providerId: number
): Promise<ProviderAvailabilityResult | null> {
  const normalizedProviderId = validateProviderId(providerId);

  return invokeGeneratedIpc<ProviderAvailabilityResult>({
    title: "测试供应商可用性失败",
    cmd: "provider_test_availability",
    args: { providerId: normalizedProviderId },
    invoke: () =>
      commands.providerTestAvailability(normalizedProviderId) as Promise<
        GeneratedCommandResult<ProviderAvailabilityResult>
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
