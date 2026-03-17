import { invokeService } from "./invokeServiceCommand";

export type GatewayListenMode = "localhost" | "wsl_auto" | "lan" | "custom";

export type WslHostAddressMode = "auto" | "custom";

export type WslTargetCli = {
  claude: boolean;
  codex: boolean;
  gemini: boolean;
};

export type AppSettings = {
  schema_version: number;
  preferred_port: number;
  show_home_heatmap: boolean;
  gateway_listen_mode: GatewayListenMode;
  gateway_custom_listen_address: string;
  wsl_auto_config: boolean;
  wsl_target_cli: WslTargetCli;
  wsl_host_address_mode: WslHostAddressMode;
  wsl_custom_host_address: string;
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
  enable_codex_session_id_completion: boolean;
  enable_claude_metadata_user_id_injection: boolean;
  enable_cache_anomaly_monitor: boolean;
  enable_task_complete_notify: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
  response_fixer_max_json_depth: number;
  response_fixer_max_fix_size: number;
};

export type SettingsSetInput = {
  preferredPort: number;
  showHomeHeatmap?: boolean;
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
  enableCacheAnomalyMonitor?: boolean;
  enableTaskCompleteNotify?: boolean;
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
  wslHostAddressMode?: WslHostAddressMode;
  wslCustomHostAddress?: string;
};

export async function settingsGet() {
  return invokeService<AppSettings>("读取设置失败", "settings_get");
}

export async function settingsSet(input: SettingsSetInput) {
  return invokeService<AppSettings>("更新设置失败", "settings_set", { update: input });
}
