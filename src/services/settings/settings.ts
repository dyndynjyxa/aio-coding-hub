import type { HomeUsagePeriod } from "../../constants/homeUsagePeriods";
import { commands } from "../../generated/bindings";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type GatewayListenMode = "localhost" | "wsl_auto" | "lan" | "custom";

export type WslHostAddressMode = "auto" | "custom";
export type CodexHomeMode = "user_home_default" | "follow_codex_home" | "custom";
export type { HomeUsagePeriod };

export type WslTargetCli = {
  claude: boolean;
  codex: boolean;
  gemini: boolean;
};

export type AppSettings = {
  schema_version: number;
  preferred_port: number;
  show_home_heatmap: boolean;
  show_home_usage: boolean;
  home_usage_period: HomeUsagePeriod;
  gateway_listen_mode: GatewayListenMode;
  gateway_custom_listen_address: string;
  wsl_auto_config: boolean;
  wsl_target_cli: WslTargetCli;
  cli_priority_order: CliKey[];
  wsl_host_address_mode: WslHostAddressMode;
  wsl_custom_host_address: string;
  codex_home_mode: CodexHomeMode;
  codex_home_override: string;
  auto_start: boolean;
  start_minimized: boolean;
  tray_enabled: boolean;
  enable_cli_proxy_startup_recovery: boolean;
  log_retention_days: number;
  provider_cooldown_seconds: number;
  provider_base_url_ping_cache_ttl_seconds: number;
  upstream_first_byte_timeout_seconds: number;
  upstream_stream_idle_timeout_seconds: number;
  upstream_request_timeout_non_streaming_seconds: number;
  update_releases_url: string;
  failover_max_attempts_per_provider: number;
  failover_max_providers_to_try: number;
  circuit_breaker_failure_threshold: number;
  circuit_breaker_open_duration_minutes: number;
  enable_circuit_breaker_notice: boolean;
  verbose_provider_error: boolean;
  intercept_anthropic_warmup_requests: boolean;
  enable_thinking_signature_rectifier: boolean;
  enable_thinking_budget_rectifier: boolean;
  enable_billing_header_rectifier: boolean;
  enable_codex_session_id_completion: boolean;
  enable_claude_metadata_user_id_injection: boolean;
  enable_cache_anomaly_monitor: boolean;
  enable_task_complete_notify: boolean;
  enable_notification_sound: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
  response_fixer_max_json_depth: number;
  response_fixer_max_fix_size: number;
  cx2cc_fallback_model_opus: string;
  cx2cc_fallback_model_sonnet: string;
  cx2cc_fallback_model_haiku: string;
  cx2cc_fallback_model_main: string;
  cx2cc_model_reasoning_effort: string;
  cx2cc_service_tier: string;
  cx2cc_disable_response_storage: boolean;
  cx2cc_enable_reasoning_to_thinking: boolean;
  cx2cc_drop_stop_sequences: boolean;
  cx2cc_clean_schema: boolean;
  cx2cc_filter_batch_tool: boolean;
};

export type SettingsSetInput = {
  preferredPort: number;
  showHomeHeatmap?: boolean;
  showHomeUsage?: boolean;
  homeUsagePeriod?: HomeUsagePeriod;
  gatewayListenMode?: GatewayListenMode;
  gatewayCustomListenAddress?: string;
  autoStart: boolean;
  startMinimized?: boolean;
  trayEnabled?: boolean;
  enableCliProxyStartupRecovery?: boolean;
  logRetentionDays: number;
  providerCooldownSeconds?: number;
  providerBaseUrlPingCacheTtlSeconds?: number;
  upstreamFirstByteTimeoutSeconds?: number;
  upstreamStreamIdleTimeoutSeconds?: number;
  upstreamRequestTimeoutNonStreamingSeconds?: number;
  verboseProviderError?: boolean;
  interceptAnthropicWarmupRequests?: boolean;
  enableThinkingSignatureRectifier?: boolean;
  enableThinkingBudgetRectifier?: boolean;
  enableBillingHeaderRectifier?: boolean;
  enableCacheAnomalyMonitor?: boolean;
  enableTaskCompleteNotify?: boolean;
  enableNotificationSound?: boolean;
  enableResponseFixer?: boolean;
  responseFixerFixEncoding?: boolean;
  responseFixerFixSseFormat?: boolean;
  responseFixerFixTruncatedJson?: boolean;
  enableClaudeMetadataUserIdInjection?: boolean;
  updateReleasesUrl?: string;
  failoverMaxAttemptsPerProvider: number;
  failoverMaxProvidersToTry: number;
  circuitBreakerFailureThreshold?: number;
  circuitBreakerOpenDurationMinutes?: number;
  wslAutoConfig?: boolean;
  wslTargetCli?: WslTargetCli;
  cliPriorityOrder?: CliKey[];
  wslHostAddressMode?: WslHostAddressMode;
  wslCustomHostAddress?: string;
  codexHomeMode?: CodexHomeMode;
  codexHomeOverride?: string;
  cx2ccFallbackModelOpus?: string;
  cx2ccFallbackModelSonnet?: string;
  cx2ccFallbackModelHaiku?: string;
  cx2ccFallbackModelMain?: string;
  cx2ccModelReasoningEffort?: string;
  cx2ccServiceTier?: string;
  cx2ccDisableResponseStorage?: boolean;
  cx2ccEnableReasoningToThinking?: boolean;
  cx2ccDropStopSequences?: boolean;
  cx2ccCleanSchema?: boolean;
  cx2ccFilterBatchTool?: boolean;
};

export async function settingsGet() {
  return invokeGeneratedIpc<AppSettings>({
    title: "读取设置失败",
    cmd: "settings_get",
    invoke: () => commands.settingsGet() as Promise<GeneratedCommandResult<AppSettings>>,
  });
}

export async function settingsSet(input: SettingsSetInput) {
  return invokeGeneratedIpc<AppSettings>({
    title: "更新设置失败",
    cmd: "settings_set",
    args: { update: input },
    invoke: () =>
      commands.settingsSet(input as Parameters<typeof commands.settingsSet>[0]) as Promise<
        GeneratedCommandResult<AppSettings>
      >,
  });
}
