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
    preferredPort: next.preferred_port,
    showHomeHeatmap: next.show_home_heatmap,
    showHomeUsage: next.show_home_usage,
    homeUsagePeriod: next.home_usage_period,
    gatewayListenMode: next.gateway_listen_mode,
    gatewayCustomListenAddress: next.gateway_custom_listen_address,
    autoStart: next.auto_start,
    startMinimized: next.start_minimized,
    trayEnabled: next.tray_enabled,
    enableCliProxyStartupRecovery: next.enable_cli_proxy_startup_recovery,
    logRetentionDays: next.log_retention_days,
    providerCooldownSeconds: next.provider_cooldown_seconds,
    providerBaseUrlPingCacheTtlSeconds: next.provider_base_url_ping_cache_ttl_seconds,
    upstreamFirstByteTimeoutSeconds: next.upstream_first_byte_timeout_seconds,
    upstreamStreamIdleTimeoutSeconds: next.upstream_stream_idle_timeout_seconds,
    upstreamRequestTimeoutNonStreamingSeconds:
      next.upstream_request_timeout_non_streaming_seconds,
    verboseProviderError: next.verbose_provider_error,
    interceptAnthropicWarmupRequests: next.intercept_anthropic_warmup_requests,
    enableThinkingSignatureRectifier: next.enable_thinking_signature_rectifier,
    enableThinkingBudgetRectifier: next.enable_thinking_budget_rectifier,
    enableBillingHeaderRectifier: next.enable_billing_header_rectifier,
    enableClaudeMetadataUserIdInjection: next.enable_claude_metadata_user_id_injection,
    enableCacheAnomalyMonitor: next.enable_cache_anomaly_monitor,
    enableTaskCompleteNotify: next.enable_task_complete_notify,
    enableNotificationSound: next.enable_notification_sound,
    enableResponseFixer: next.enable_response_fixer,
    responseFixerFixEncoding: next.response_fixer_fix_encoding,
    responseFixerFixSseFormat: next.response_fixer_fix_sse_format,
    responseFixerFixTruncatedJson: next.response_fixer_fix_truncated_json,
    updateReleasesUrl: next.update_releases_url,
    failoverMaxAttemptsPerProvider: next.failover_max_attempts_per_provider,
    failoverMaxProvidersToTry: next.failover_max_providers_to_try,
    circuitBreakerFailureThreshold: next.circuit_breaker_failure_threshold,
    circuitBreakerOpenDurationMinutes: next.circuit_breaker_open_duration_minutes,
    wslAutoConfig: next.wsl_auto_config,
    wslTargetCli: next.wsl_target_cli,
    cliPriorityOrder: next.cli_priority_order,
    wslHostAddressMode: next.wsl_host_address_mode,
    wslCustomHostAddress: next.wsl_custom_host_address,
    codexHomeMode: next.codex_home_mode,
    codexHomeOverride: next.codex_home_override,
    cx2CcFallbackModelOpus: next.cx2cc_fallback_model_opus,
    cx2CcFallbackModelSonnet: next.cx2cc_fallback_model_sonnet,
    cx2CcFallbackModelHaiku: next.cx2cc_fallback_model_haiku,
    cx2CcFallbackModelMain: next.cx2cc_fallback_model_main,
    cx2CcModelReasoningEffort: next.cx2cc_model_reasoning_effort,
    cx2CcServiceTier: next.cx2cc_service_tier,
    cx2CcDisableResponseStorage: next.cx2cc_disable_response_storage,
    cx2CcEnableReasoningToThinking: next.cx2cc_enable_reasoning_to_thinking,
    cx2CcDropStopSequences: next.cx2cc_drop_stop_sequences,
    cx2CcCleanSchema: next.cx2cc_clean_schema,
    cx2CcFilterBatchTool: next.cx2cc_filter_batch_tool,
    upstreamProxyEnabled: next.upstream_proxy_enabled,
    upstreamProxyUrl: next.upstream_proxy_url,
    upstreamProxyUsername: next.upstream_proxy_username,
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
  const update = toGeneratedSettingsUpdate(input);
  return invokeGeneratedIpc<SettingsMutationResult>({
    title: "更新设置失败",
    cmd: "settings_set",
    args: { update },
    invoke: () =>
      commands.settingsSet(update) as Promise<GeneratedCommandResult<SettingsMutationResult>>,
  });
}
