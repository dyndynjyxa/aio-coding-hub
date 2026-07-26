import type {
  AppSettings,
  HomeUsagePeriod,
  SettingsSetInput,
  SettingsViewBackedInputKey,
} from "../../services/settings/settings";
import type { CliKey } from "../../services/providers/providers";
import { DEFAULT_GATEWAY_PORT } from "../../constants/gateway";
import {
  DEFAULT_CLI_PRIORITY_ORDER,
  normalizeCliPriorityOrder,
} from "../../services/cli/cliPriorityOrder";
import { pickSettingsSetInputFieldsFromView } from "../../services/settings/settings";
import { DEFAULT_HOME_USAGE_PERIOD } from "../../utils/homeUsagePeriod";

export type PersistedSettings = {
  preferred_port: number;
  show_home_heatmap: boolean;
  show_home_usage: boolean;
  home_usage_period: HomeUsagePeriod;
  cli_priority_order: CliKey[];
  auto_start: boolean;
  start_minimized: boolean;
  tray_enabled: boolean;
  log_retention_days: number;
  request_log_retention_days: number;
  provider_cooldown_seconds: number;
  provider_base_url_ping_cache_ttl_seconds: number;
  upstream_first_byte_timeout_seconds: number;
  upstream_stream_idle_timeout_seconds: number;
  upstream_request_timeout_non_streaming_seconds: number;
  intercept_anthropic_warmup_requests: boolean;
  enable_thinking_signature_rectifier: boolean;
  enable_debug_log: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
  failover_max_attempts_per_provider: number;
  failover_max_providers_to_try: number;
  circuit_breaker_failure_threshold: number;
  circuit_breaker_open_duration_minutes: number;
};

export type PersistKey = keyof PersistedSettings;
export type PersistedSettingsPatch = Partial<PersistedSettings>;

export const DEFAULT_PERSISTED_SETTINGS: PersistedSettings = {
  preferred_port: DEFAULT_GATEWAY_PORT,
  show_home_heatmap: true,
  show_home_usage: true,
  home_usage_period: DEFAULT_HOME_USAGE_PERIOD,
  cli_priority_order: DEFAULT_CLI_PRIORITY_ORDER,
  auto_start: false,
  start_minimized: false,
  tray_enabled: true,
  log_retention_days: 7,
  request_log_retention_days: 0,
  provider_cooldown_seconds: 30,
  provider_base_url_ping_cache_ttl_seconds: 60,
  upstream_first_byte_timeout_seconds: 0,
  upstream_stream_idle_timeout_seconds: 0,
  upstream_request_timeout_non_streaming_seconds: 0,
  intercept_anthropic_warmup_requests: false,
  enable_thinking_signature_rectifier: true,
  enable_debug_log: false,
  enable_response_fixer: true,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
  failover_max_attempts_per_provider: 5,
  failover_max_providers_to_try: 5,
  circuit_breaker_failure_threshold: 5,
  circuit_breaker_open_duration_minutes: 30,
};

const MAX_FAILOVER_ATTEMPTS_PER_PROVIDER = 20;
const MAX_FAILOVER_PROVIDERS_TO_TRY = 20;
const MAX_FAILOVER_TOTAL_ATTEMPTS = 100;

const PERSISTED_SETTINGS_INPUT_KEYS = [
  "preferredPort",
  "showHomeHeatmap",
  "showHomeUsage",
  "homeUsagePeriod",
  "cliPriorityOrder",
  "autoStart",
  "startMinimized",
  "trayEnabled",
  "logRetentionDays",
  "requestLogRetentionDays",
  "providerCooldownSeconds",
  "providerBaseUrlPingCacheTtlSeconds",
  "upstreamFirstByteTimeoutSeconds",
  "upstreamStreamIdleTimeoutSeconds",
  "upstreamRequestTimeoutNonStreamingSeconds",
  "interceptAnthropicWarmupRequests",
  "enableThinkingSignatureRectifier",
  "enableDebugLog",
  "enableResponseFixer",
  "responseFixerFixEncoding",
  "responseFixerFixSseFormat",
  "responseFixerFixTruncatedJson",
  "failoverMaxAttemptsPerProvider",
  "failoverMaxProvidersToTry",
  "circuitBreakerFailureThreshold",
  "circuitBreakerOpenDurationMinutes",
] as const satisfies readonly SettingsViewBackedInputKey[];

function persistedSettingValuesEqual(
  left: PersistedSettings[PersistKey],
  right: PersistedSettings[PersistKey]
) {
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((item, index) => item === right[index]);
  }

  return left === right;
}

export function diffPersistedSettings(before: PersistedSettings, after: PersistedSettings) {
  return (Object.keys(before) as PersistKey[]).filter(
    (key) => !persistedSettingValuesEqual(before[key], after[key])
  );
}

export function applyPersistedSettingsPatch(
  base: PersistedSettings,
  patch: PersistedSettingsPatch
) {
  return { ...base, ...patch };
}

function assignPersistedSetting<K extends PersistKey>(
  target: PersistedSettings,
  key: K,
  value: PersistedSettings[K]
) {
  target[key] = value;
}

export function replacePersistedSettingsKeys(
  base: PersistedSettings,
  source: PersistedSettings,
  keys: PersistKey[]
) {
  if (keys.length === 0) return base;

  const next = { ...base };
  for (const key of keys) {
    assignPersistedSetting(next, key, source[key]);
  }
  return next;
}

export function buildPersistedSettingsSnapshot(
  settingsValue: AppSettings,
  fallback: PersistedSettings = DEFAULT_PERSISTED_SETTINGS
): PersistedSettings {
  return {
    preferred_port: settingsValue.preferred_port,
    show_home_heatmap: settingsValue.show_home_heatmap ?? fallback.show_home_heatmap,
    show_home_usage: settingsValue.show_home_usage ?? fallback.show_home_usage,
    home_usage_period: settingsValue.home_usage_period ?? fallback.home_usage_period,
    cli_priority_order: normalizeCliPriorityOrder(
      settingsValue.cli_priority_order ?? fallback.cli_priority_order
    ),
    auto_start: settingsValue.auto_start,
    start_minimized: settingsValue.start_minimized ?? fallback.start_minimized,
    tray_enabled: settingsValue.tray_enabled ?? fallback.tray_enabled,
    log_retention_days: settingsValue.log_retention_days,
    request_log_retention_days:
      settingsValue.request_log_retention_days ?? fallback.request_log_retention_days,
    provider_cooldown_seconds:
      settingsValue.provider_cooldown_seconds ?? fallback.provider_cooldown_seconds,
    provider_base_url_ping_cache_ttl_seconds:
      settingsValue.provider_base_url_ping_cache_ttl_seconds ??
      fallback.provider_base_url_ping_cache_ttl_seconds,
    upstream_first_byte_timeout_seconds:
      settingsValue.upstream_first_byte_timeout_seconds ??
      fallback.upstream_first_byte_timeout_seconds,
    upstream_stream_idle_timeout_seconds:
      settingsValue.upstream_stream_idle_timeout_seconds ??
      fallback.upstream_stream_idle_timeout_seconds,
    upstream_request_timeout_non_streaming_seconds:
      settingsValue.upstream_request_timeout_non_streaming_seconds ??
      fallback.upstream_request_timeout_non_streaming_seconds,
    intercept_anthropic_warmup_requests:
      settingsValue.intercept_anthropic_warmup_requests ??
      fallback.intercept_anthropic_warmup_requests,
    enable_thinking_signature_rectifier:
      settingsValue.enable_thinking_signature_rectifier ??
      fallback.enable_thinking_signature_rectifier,
    enable_debug_log: settingsValue.enable_debug_log ?? fallback.enable_debug_log,
    enable_response_fixer: settingsValue.enable_response_fixer ?? fallback.enable_response_fixer,
    response_fixer_fix_encoding:
      settingsValue.response_fixer_fix_encoding ?? fallback.response_fixer_fix_encoding,
    response_fixer_fix_sse_format:
      settingsValue.response_fixer_fix_sse_format ?? fallback.response_fixer_fix_sse_format,
    response_fixer_fix_truncated_json:
      settingsValue.response_fixer_fix_truncated_json ?? fallback.response_fixer_fix_truncated_json,
    failover_max_attempts_per_provider:
      settingsValue.failover_max_attempts_per_provider ??
      fallback.failover_max_attempts_per_provider,
    failover_max_providers_to_try:
      settingsValue.failover_max_providers_to_try ?? fallback.failover_max_providers_to_try,
    circuit_breaker_failure_threshold:
      settingsValue.circuit_breaker_failure_threshold ?? fallback.circuit_breaker_failure_threshold,
    circuit_breaker_open_duration_minutes:
      settingsValue.circuit_breaker_open_duration_minutes ??
      fallback.circuit_breaker_open_duration_minutes,
  };
}

export function buildPersistedSettingsMutationInput(desired: PersistedSettings): SettingsSetInput {
  return pickSettingsSetInputFieldsFromView(desired, PERSISTED_SETTINGS_INPUT_KEYS);
}

function isIntegerInRange(value: number, min: number, max: number) {
  return Number.isInteger(value) && value >= min && value <= max;
}

export function validatePersistedSettings(desired: PersistedSettings, keys: PersistKey[]) {
  if (keys.includes("preferred_port")) {
    if (!isIntegerInRange(desired.preferred_port, 1024, 65535)) {
      return "端口号必须为 1024-65535";
    }
  }

  if (keys.includes("log_retention_days")) {
    if (!isIntegerInRange(desired.log_retention_days, 1, 3650)) {
      return "日志保留必须为 1-3650 天";
    }
  }

  if (keys.includes("request_log_retention_days")) {
    if (!isIntegerInRange(desired.request_log_retention_days, 0, 3650)) {
      return "请求记录保留必须为 0（永久）或 1-3650 天";
    }
  }

  if (keys.includes("provider_cooldown_seconds")) {
    if (!isIntegerInRange(desired.provider_cooldown_seconds, 0, 3600)) {
      return "短熔断冷却必须为 0-3600 秒";
    }
  }

  if (keys.includes("provider_base_url_ping_cache_ttl_seconds")) {
    if (!isIntegerInRange(desired.provider_base_url_ping_cache_ttl_seconds, 1, 3600)) {
      return "Ping 选择缓存 TTL 必须为 1-3600 秒";
    }
  }

  if (keys.includes("upstream_first_byte_timeout_seconds")) {
    if (!isIntegerInRange(desired.upstream_first_byte_timeout_seconds, 0, 3600)) {
      return "上游首字节超时必须为 0-3600 秒";
    }
  }

  if (keys.includes("upstream_stream_idle_timeout_seconds")) {
    if (
      !isIntegerInRange(desired.upstream_stream_idle_timeout_seconds, 0, 3600) ||
      (desired.upstream_stream_idle_timeout_seconds > 0 &&
        desired.upstream_stream_idle_timeout_seconds < 60)
    ) {
      return "上游流式空闲超时必须为 0（禁用）或 60-3600 秒";
    }
  }

  if (keys.includes("upstream_request_timeout_non_streaming_seconds")) {
    if (!isIntegerInRange(desired.upstream_request_timeout_non_streaming_seconds, 0, 86400)) {
      return "上游非流式总超时必须为 0-86400 秒";
    }
  }

  if (keys.includes("failover_max_attempts_per_provider")) {
    if (
      !isIntegerInRange(
        desired.failover_max_attempts_per_provider,
        1,
        MAX_FAILOVER_ATTEMPTS_PER_PROVIDER
      )
    ) {
      return `单个 Provider 重试次数必须为 1-${MAX_FAILOVER_ATTEMPTS_PER_PROVIDER}`;
    }
  }

  if (keys.includes("failover_max_providers_to_try")) {
    if (
      !isIntegerInRange(desired.failover_max_providers_to_try, 1, MAX_FAILOVER_PROVIDERS_TO_TRY)
    ) {
      return `Provider 尝试数量必须为 1-${MAX_FAILOVER_PROVIDERS_TO_TRY}`;
    }
  }

  if (
    (keys.includes("failover_max_attempts_per_provider") ||
      keys.includes("failover_max_providers_to_try")) &&
    desired.failover_max_attempts_per_provider * desired.failover_max_providers_to_try >
      MAX_FAILOVER_TOTAL_ATTEMPTS
  ) {
    return `Provider 重试总量必须不超过 ${MAX_FAILOVER_TOTAL_ATTEMPTS}`;
  }

  if (keys.includes("circuit_breaker_failure_threshold")) {
    if (!isIntegerInRange(desired.circuit_breaker_failure_threshold, 1, 50)) {
      return "熔断阈值必须为 1-50";
    }
  }

  if (keys.includes("circuit_breaker_open_duration_minutes")) {
    if (!isIntegerInRange(desired.circuit_breaker_open_duration_minutes, 1, 1440)) {
      return "熔断时长必须为 1-1440 分钟";
    }
  }

  return null;
}
