import {
  commands,
  type CodexHomeMode,
  type GatewayListenMode,
  type HomeUsagePeriod,
  type SensitiveStringUpdate,
  type SettingsMutationResult as GeneratedSettingsMutationResult,
  type SettingsMutationRuntime as GeneratedSettingsMutationRuntime,
  type SettingsUpdate as GeneratedSettingsUpdate,
  type SettingsView as GeneratedAppSettings,
  type WslHostAddressMode,
  type WslTargetCli,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { type OptionalNullableGeneratedFields } from "../generatedTypeUtils";
import { validateSettingsSetInput } from "./settingsValidation";

export type {
  CodexHomeMode,
  GatewayListenMode,
  HomeUsagePeriod,
  SensitiveStringUpdate,
  WslHostAddressMode,
  WslTargetCli,
};

export type AppSettings = GeneratedAppSettings;
export type SettingsMutationRuntime = GeneratedSettingsMutationRuntime;

export type SettingsMutationResult = GeneratedSettingsMutationResult;
export type SettingsSetInput = OptionalNullableGeneratedFields<GeneratedSettingsUpdate>;

export type AppSettingsPatch = Partial<AppSettings> & {
  upstream_proxy_password?: SensitiveStringUpdate;
};

type AssertNever<TValue extends never> = TValue;

export type SettingsViewBackedInputKey = Exclude<
  keyof GeneratedSettingsUpdate,
  "upstreamProxyPassword"
>;

const SETTINGS_VIEW_TO_UPDATE_FIELD_MAP = {
  preferredPort: "preferred_port",
  showHomeHeatmap: "show_home_heatmap",
  showHomeUsage: "show_home_usage",
  homeUsagePeriod: "home_usage_period",
  gatewayListenMode: "gateway_listen_mode",
  gatewayCustomListenAddress: "gateway_custom_listen_address",
  autoStart: "auto_start",
  startMinimized: "start_minimized",
  trayEnabled: "tray_enabled",
  enableCliProxyStartupRecovery: "enable_cli_proxy_startup_recovery",
  logRetentionDays: "log_retention_days",
  providerCooldownSeconds: "provider_cooldown_seconds",
  providerBaseUrlPingCacheTtlSeconds: "provider_base_url_ping_cache_ttl_seconds",
  upstreamFirstByteTimeoutSeconds: "upstream_first_byte_timeout_seconds",
  upstreamStreamIdleTimeoutSeconds: "upstream_stream_idle_timeout_seconds",
  upstreamRequestTimeoutNonStreamingSeconds: "upstream_request_timeout_non_streaming_seconds",
  verboseProviderError: "verbose_provider_error",
  interceptAnthropicWarmupRequests: "intercept_anthropic_warmup_requests",
  enableThinkingSignatureRectifier: "enable_thinking_signature_rectifier",
  enableThinkingBudgetRectifier: "enable_thinking_budget_rectifier",
  enableBillingHeaderRectifier: "enable_billing_header_rectifier",
  enableClaudeMetadataUserIdInjection: "enable_claude_metadata_user_id_injection",
  enableCacheAnomalyMonitor: "enable_cache_anomaly_monitor",
  enableDebugLog: "enable_debug_log",
  enableTaskCompleteNotify: "enable_task_complete_notify",
  enableNotificationSound: "enable_notification_sound",
  enableResponseFixer: "enable_response_fixer",
  responseFixerFixEncoding: "response_fixer_fix_encoding",
  responseFixerFixSseFormat: "response_fixer_fix_sse_format",
  responseFixerFixTruncatedJson: "response_fixer_fix_truncated_json",
  updateReleasesUrl: "update_releases_url",
  failoverMaxAttemptsPerProvider: "failover_max_attempts_per_provider",
  failoverMaxProvidersToTry: "failover_max_providers_to_try",
  circuitBreakerFailureThreshold: "circuit_breaker_failure_threshold",
  circuitBreakerOpenDurationMinutes: "circuit_breaker_open_duration_minutes",
  wslAutoConfig: "wsl_auto_config",
  wslTargetCli: "wsl_target_cli",
  cliPriorityOrder: "cli_priority_order",
  wslHostAddressMode: "wsl_host_address_mode",
  wslCustomHostAddress: "wsl_custom_host_address",
  codexHomeMode: "codex_home_mode",
  codexHomeOverride: "codex_home_override",
  cx2CcFallbackModelOpus: "cx2cc_fallback_model_opus",
  cx2CcFallbackModelSonnet: "cx2cc_fallback_model_sonnet",
  cx2CcFallbackModelHaiku: "cx2cc_fallback_model_haiku",
  cx2CcFallbackModelMain: "cx2cc_fallback_model_main",
  cx2CcModelReasoningEffort: "cx2cc_model_reasoning_effort",
  cx2CcServiceTier: "cx2cc_service_tier",
  cx2CcDisableResponseStorage: "cx2cc_disable_response_storage",
  cx2CcEnableReasoningToThinking: "cx2cc_enable_reasoning_to_thinking",
  cx2CcDropStopSequences: "cx2cc_drop_stop_sequences",
  cx2CcCleanSchema: "cx2cc_clean_schema",
  cx2CcFilterBatchTool: "cx2cc_filter_batch_tool",
  upstreamProxyEnabled: "upstream_proxy_enabled",
  upstreamProxyUrl: "upstream_proxy_url",
  upstreamProxyUsername: "upstream_proxy_username",
} as const satisfies Record<SettingsViewBackedInputKey, keyof GeneratedAppSettings>;

const SETTINGS_VIEW_BACKED_INPUT_KEYS = Object.keys(
  SETTINGS_VIEW_TO_UPDATE_FIELD_MAP
) as SettingsViewBackedInputKey[];

type SettingsViewKeysHandledByCreateInput =
  (typeof SETTINGS_VIEW_TO_UPDATE_FIELD_MAP)[SettingsViewBackedInputKey];

type SettingsViewKeysHandledOutsideCreateInput =
  | "schema_version"
  | "enable_circuit_breaker_notice"
  | "enable_codex_session_id_completion"
  | "response_fixer_max_json_depth"
  | "response_fixer_max_fix_size"
  | "upstream_proxy_password_configured";

export type __AssertNoUnhandledSettingsViewKeys = AssertNever<
  Exclude<
    keyof GeneratedAppSettings,
    SettingsViewKeysHandledByCreateInput | SettingsViewKeysHandledOutsideCreateInput
  >
>;
export type __AssertNoStaleHandledSettingsViewKeys = AssertNever<
  Exclude<
    SettingsViewKeysHandledByCreateInput | SettingsViewKeysHandledOutsideCreateInput,
    keyof GeneratedAppSettings
  >
>;

function validateRequiredSettingsSetInput(input: SettingsSetInput): string | null {
  for (const [fieldLabel, value] of [
    ["preferredPort", input.preferredPort],
    ["autoStart", input.autoStart],
    ["logRetentionDays", input.logRetentionDays],
    ["failoverMaxAttemptsPerProvider", input.failoverMaxAttemptsPerProvider],
    ["failoverMaxProvidersToTry", input.failoverMaxProvidersToTry],
  ] as const) {
    if (value == null) {
      return `SEC_INVALID_INPUT: ${fieldLabel} is required`;
    }
  }
  return null;
}

export function pickSettingsSetInputFieldsFromView<
  const TKeys extends readonly SettingsViewBackedInputKey[],
>(
  source: Pick<GeneratedAppSettings, (typeof SETTINGS_VIEW_TO_UPDATE_FIELD_MAP)[TKeys[number]]>,
  keys: TKeys
): Pick<SettingsSetInput, TKeys[number]> {
  const next = {} as Pick<SettingsSetInput, TKeys[number]>;

  for (const key of keys) {
    const inputKey = key as TKeys[number];
    const sourceKey = SETTINGS_VIEW_TO_UPDATE_FIELD_MAP[
      inputKey
    ] as (typeof SETTINGS_VIEW_TO_UPDATE_FIELD_MAP)[TKeys[number]];
    next[inputKey] = source[sourceKey] as unknown as SettingsSetInput[TKeys[number]];
  }

  return next;
}

function toGeneratedSettingsUpdate(input: SettingsSetInput): GeneratedSettingsUpdate {
  const update: GeneratedSettingsUpdate = {
    preferredPort: input.preferredPort,
    showHomeHeatmap: input.showHomeHeatmap ?? null,
    showHomeUsage: input.showHomeUsage ?? null,
    homeUsagePeriod: input.homeUsagePeriod ?? null,
    gatewayListenMode: input.gatewayListenMode ?? null,
    gatewayCustomListenAddress: input.gatewayCustomListenAddress ?? null,
    autoStart: input.autoStart,
    startMinimized: input.startMinimized ?? null,
    trayEnabled: input.trayEnabled ?? null,
    enableCliProxyStartupRecovery: input.enableCliProxyStartupRecovery ?? null,
    logRetentionDays: input.logRetentionDays,
    providerCooldownSeconds: input.providerCooldownSeconds ?? null,
    providerBaseUrlPingCacheTtlSeconds: input.providerBaseUrlPingCacheTtlSeconds ?? null,
    upstreamFirstByteTimeoutSeconds: input.upstreamFirstByteTimeoutSeconds ?? null,
    upstreamStreamIdleTimeoutSeconds: input.upstreamStreamIdleTimeoutSeconds ?? null,
    upstreamRequestTimeoutNonStreamingSeconds:
      input.upstreamRequestTimeoutNonStreamingSeconds ?? null,
    interceptAnthropicWarmupRequests: input.interceptAnthropicWarmupRequests ?? null,
    enableThinkingSignatureRectifier: input.enableThinkingSignatureRectifier ?? null,
    enableThinkingBudgetRectifier: input.enableThinkingBudgetRectifier ?? null,
    enableBillingHeaderRectifier: input.enableBillingHeaderRectifier ?? null,
    enableClaudeMetadataUserIdInjection: input.enableClaudeMetadataUserIdInjection ?? null,
    enableCacheAnomalyMonitor: input.enableCacheAnomalyMonitor ?? null,
    enableDebugLog: input.enableDebugLog ?? null,
    enableTaskCompleteNotify: input.enableTaskCompleteNotify ?? null,
    enableNotificationSound: input.enableNotificationSound ?? null,
    enableResponseFixer: input.enableResponseFixer ?? null,
    responseFixerFixEncoding: input.responseFixerFixEncoding ?? null,
    responseFixerFixSseFormat: input.responseFixerFixSseFormat ?? null,
    responseFixerFixTruncatedJson: input.responseFixerFixTruncatedJson ?? null,
    verboseProviderError: input.verboseProviderError ?? null,
    failoverMaxAttemptsPerProvider: input.failoverMaxAttemptsPerProvider,
    failoverMaxProvidersToTry: input.failoverMaxProvidersToTry,
    circuitBreakerFailureThreshold: input.circuitBreakerFailureThreshold ?? null,
    circuitBreakerOpenDurationMinutes: input.circuitBreakerOpenDurationMinutes ?? null,
    updateReleasesUrl: input.updateReleasesUrl ?? null,
    wslAutoConfig: input.wslAutoConfig ?? null,
    wslTargetCli: input.wslTargetCli ?? null,
    cliPriorityOrder: input.cliPriorityOrder ?? null,
    wslHostAddressMode: input.wslHostAddressMode ?? null,
    wslCustomHostAddress: input.wslCustomHostAddress ?? null,
    codexHomeMode: input.codexHomeMode ?? null,
    codexHomeOverride: input.codexHomeOverride ?? null,
    cx2CcFallbackModelOpus: input.cx2CcFallbackModelOpus ?? null,
    cx2CcFallbackModelSonnet: input.cx2CcFallbackModelSonnet ?? null,
    cx2CcFallbackModelHaiku: input.cx2CcFallbackModelHaiku ?? null,
    cx2CcFallbackModelMain: input.cx2CcFallbackModelMain ?? null,
    cx2CcModelReasoningEffort: input.cx2CcModelReasoningEffort ?? null,
    cx2CcServiceTier: input.cx2CcServiceTier ?? null,
    cx2CcDisableResponseStorage: input.cx2CcDisableResponseStorage ?? null,
    cx2CcEnableReasoningToThinking: input.cx2CcEnableReasoningToThinking ?? null,
    cx2CcDropStopSequences: input.cx2CcDropStopSequences ?? null,
    cx2CcCleanSchema: input.cx2CcCleanSchema ?? null,
    cx2CcFilterBatchTool: input.cx2CcFilterBatchTool ?? null,
    upstreamProxyEnabled: input.upstreamProxyEnabled ?? null,
    upstreamProxyUrl: input.upstreamProxyUrl ?? null,
    upstreamProxyUsername: input.upstreamProxyUsername ?? null,
    upstreamProxyPassword: input.upstreamProxyPassword ?? null,
  };
  return update;
}

export function createSettingsSetInput(
  current: AppSettings,
  patch: AppSettingsPatch = {}
): SettingsSetInput {
  const next: AppSettings = { ...current, ...patch };
  return {
    ...pickSettingsSetInputFieldsFromView(next, SETTINGS_VIEW_BACKED_INPUT_KEYS),
    upstreamProxyPassword: patch.upstream_proxy_password ?? { mode: "preserve" },
  };
}

export async function settingsGet() {
  return invokeGeneratedIpc<AppSettings>({
    title: "读取设置失败",
    cmd: "settings_get",
    invoke: () => commands.settingsGet() as Promise<GeneratedCommandResult<AppSettings>>,
  });
}

export async function settingsSet(input: SettingsSetInput) {
  const requiredMessage = validateRequiredSettingsSetInput(input);
  if (requiredMessage) {
    throw new Error(requiredMessage);
  }

  const validationMessage = validateSettingsSetInput(input);
  if (validationMessage) {
    throw new Error(validationMessage);
  }

  const update = toGeneratedSettingsUpdate(input);
  return invokeGeneratedIpc<SettingsMutationResult>({
    title: "更新设置失败",
    cmd: "settings_set",
    args: { update },
    invoke: () =>
      commands.settingsSet(update) as Promise<GeneratedCommandResult<SettingsMutationResult>>,
  });
}
