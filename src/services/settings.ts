import { invokeTauriOrNull } from "./tauriInvoke";

export type GatewayListenMode = "localhost" | "wsl_auto" | "lan" | "custom";

export type WslTargetCli = {
  claude: boolean;
  codex: boolean;
  gemini: boolean;
};

export type AppSettings = {
  schema_version: number;
  preferred_port: number;
  gateway_listen_mode: GatewayListenMode;
  gateway_custom_listen_address: string;
  wsl_auto_config: boolean;
  wsl_target_cli: WslTargetCli;
  auto_start: boolean;
  tray_enabled: boolean;
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
  intercept_anthropic_warmup_requests: boolean;
  enable_thinking_signature_rectifier: boolean;
  enable_codex_session_id_completion: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
};

export async function settingsGet() {
  return invokeTauriOrNull<AppSettings>("settings_get");
}

export async function settingsSet(input: {
  preferred_port: number;
  gateway_listen_mode?: GatewayListenMode;
  gateway_custom_listen_address?: string;
  auto_start: boolean;
  tray_enabled: boolean;
  log_retention_days: number;
  provider_cooldown_seconds: number;
  provider_base_url_ping_cache_ttl_seconds: number;
  upstream_first_byte_timeout_seconds: number;
  upstream_stream_idle_timeout_seconds: number;
  upstream_request_timeout_non_streaming_seconds: number;
  intercept_anthropic_warmup_requests?: boolean;
  enable_thinking_signature_rectifier?: boolean;
  enable_response_fixer?: boolean;
  response_fixer_fix_encoding?: boolean;
  response_fixer_fix_sse_format?: boolean;
  response_fixer_fix_truncated_json?: boolean;
  update_releases_url?: string;
  failover_max_attempts_per_provider: number;
  failover_max_providers_to_try: number;
  circuit_breaker_failure_threshold: number;
  circuit_breaker_open_duration_minutes: number;
  wsl_auto_config?: boolean;
  wsl_target_cli?: WslTargetCli;
}) {
  const args: Record<string, unknown> = {
    preferredPort: input.preferred_port,
    autoStart: input.auto_start,
    trayEnabled: input.tray_enabled,
    logRetentionDays: input.log_retention_days,
    providerCooldownSeconds: input.provider_cooldown_seconds,
    providerBaseUrlPingCacheTtlSeconds: input.provider_base_url_ping_cache_ttl_seconds,
    upstreamFirstByteTimeoutSeconds: input.upstream_first_byte_timeout_seconds,
    upstreamStreamIdleTimeoutSeconds: input.upstream_stream_idle_timeout_seconds,
    upstreamRequestTimeoutNonStreamingSeconds: input.upstream_request_timeout_non_streaming_seconds,
    failoverMaxAttemptsPerProvider: input.failover_max_attempts_per_provider,
    failoverMaxProvidersToTry: input.failover_max_providers_to_try,
    circuitBreakerFailureThreshold: input.circuit_breaker_failure_threshold,
    circuitBreakerOpenDurationMinutes: input.circuit_breaker_open_duration_minutes,
  };

  if (input.gateway_listen_mode !== undefined) {
    args.gatewayListenMode = input.gateway_listen_mode;
  }
  if (input.gateway_custom_listen_address !== undefined) {
    args.gatewayCustomListenAddress = input.gateway_custom_listen_address;
  }

  if (input.intercept_anthropic_warmup_requests !== undefined) {
    args.interceptAnthropicWarmupRequests = input.intercept_anthropic_warmup_requests;
  }
  if (input.enable_thinking_signature_rectifier !== undefined) {
    args.enableThinkingSignatureRectifier = input.enable_thinking_signature_rectifier;
  }
  if (input.enable_response_fixer !== undefined) {
    args.enableResponseFixer = input.enable_response_fixer;
  }
  if (input.response_fixer_fix_encoding !== undefined) {
    args.responseFixerFixEncoding = input.response_fixer_fix_encoding;
  }
  if (input.response_fixer_fix_sse_format !== undefined) {
    args.responseFixerFixSseFormat = input.response_fixer_fix_sse_format;
  }
  if (input.response_fixer_fix_truncated_json !== undefined) {
    args.responseFixerFixTruncatedJson = input.response_fixer_fix_truncated_json;
  }

  if (input.update_releases_url !== undefined) {
    args.updateReleasesUrl = input.update_releases_url;
  }

  if (input.wsl_auto_config !== undefined) {
    args.wslAutoConfig = input.wsl_auto_config;
  }
  if (input.wsl_target_cli !== undefined) {
    args.wslTargetCli = input.wsl_target_cli;
  }

  return invokeTauriOrNull<AppSettings>("settings_set", args);
}
