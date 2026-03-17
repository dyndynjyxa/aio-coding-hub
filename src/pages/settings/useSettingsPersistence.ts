import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { AppAboutInfo } from "../../services/appAbout";
import { cliProxySyncEnabled } from "../../services/cliProxy";
import { logToConsole } from "../../services/consoleLog";
import {
  gatewayCheckPortAvailable,
  gatewayStart,
  gatewayStop,
  type GatewayStatus,
} from "../../services/gateway";
import { useSettingsQuery, useSettingsSetMutation } from "../../query/settings";

type PersistedSettings = {
  preferred_port: number;
  show_home_heatmap: boolean;
  auto_start: boolean;
  start_minimized: boolean;
  tray_enabled: boolean;
  log_retention_days: number;
  provider_cooldown_seconds: number;
  provider_base_url_ping_cache_ttl_seconds: number;
  upstream_first_byte_timeout_seconds: number;
  upstream_stream_idle_timeout_seconds: number;
  upstream_request_timeout_non_streaming_seconds: number;
  intercept_anthropic_warmup_requests: boolean;
  enable_thinking_signature_rectifier: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
  failover_max_attempts_per_provider: number;
  failover_max_providers_to_try: number;
  circuit_breaker_failure_threshold: number;
  circuit_breaker_open_duration_minutes: number;
};

const DEFAULT_SETTINGS: PersistedSettings = {
  preferred_port: 37123,
  show_home_heatmap: true,
  auto_start: false,
  start_minimized: false,
  tray_enabled: true,
  log_retention_days: 7,
  provider_cooldown_seconds: 30,
  provider_base_url_ping_cache_ttl_seconds: 60,
  upstream_first_byte_timeout_seconds: 0,
  upstream_stream_idle_timeout_seconds: 0,
  upstream_request_timeout_non_streaming_seconds: 0,
  intercept_anthropic_warmup_requests: false,
  enable_thinking_signature_rectifier: true,
  enable_response_fixer: true,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
  failover_max_attempts_per_provider: 5,
  failover_max_providers_to_try: 5,
  circuit_breaker_failure_threshold: 5,
  circuit_breaker_open_duration_minutes: 30,
};

type PersistKey = keyof PersistedSettings;

export function useSettingsPersistence(options: {
  gateway: GatewayStatus | null;
  about: AppAboutInfo | null;
}) {
  const { gateway, about } = options;

  const settingsQuery = useSettingsQuery();
  const settingsSetMutation = useSettingsSetMutation();

  const [settingsReady, setSettingsReady] = useState(false);

  const [port, setPort] = useState<number>(DEFAULT_SETTINGS.preferred_port);
  const [showHomeHeatmap, setShowHomeHeatmap] = useState<boolean>(
    DEFAULT_SETTINGS.show_home_heatmap
  );
  const [autoStart, setAutoStart] = useState<boolean>(DEFAULT_SETTINGS.auto_start);
  const [startMinimized, setStartMinimized] = useState<boolean>(DEFAULT_SETTINGS.start_minimized);
  const [trayEnabled, setTrayEnabled] = useState<boolean>(DEFAULT_SETTINGS.tray_enabled);
  const [logRetentionDays, setLogRetentionDays] = useState<number>(
    DEFAULT_SETTINGS.log_retention_days
  );

  const persistedSettingsRef = useRef<PersistedSettings>(DEFAULT_SETTINGS);
  const desiredSettingsRef = useRef<PersistedSettings>(DEFAULT_SETTINGS);

  const persistQueueRef = useRef<{
    inFlight: boolean;
    pending: PersistedSettings | null;
  }>({ inFlight: false, pending: null });

  useEffect(() => {
    if (settingsReady) return;
    if (settingsQuery.isLoading) return;

    if (settingsQuery.isError) {
      logToConsole("error", "读取设置失败", { error: String(settingsQuery.error) });
      toast("读取设置失败：请检查 settings.json；修改任一配置将尝试覆盖写入修复");
      setSettingsReady(true);
      return;
    }

    const settingsValue = settingsQuery.data ?? null;
    if (!settingsValue) {
      setSettingsReady(true);
      return;
    }

    const nextSettings: PersistedSettings = {
      preferred_port: settingsValue.preferred_port,
      show_home_heatmap: settingsValue.show_home_heatmap ?? DEFAULT_SETTINGS.show_home_heatmap,
      auto_start: settingsValue.auto_start,
      start_minimized: settingsValue.start_minimized ?? DEFAULT_SETTINGS.start_minimized,
      tray_enabled: settingsValue.tray_enabled ?? DEFAULT_SETTINGS.tray_enabled,
      log_retention_days: settingsValue.log_retention_days,
      provider_cooldown_seconds:
        settingsValue.provider_cooldown_seconds ?? DEFAULT_SETTINGS.provider_cooldown_seconds,
      provider_base_url_ping_cache_ttl_seconds:
        settingsValue.provider_base_url_ping_cache_ttl_seconds ??
        DEFAULT_SETTINGS.provider_base_url_ping_cache_ttl_seconds,
      upstream_first_byte_timeout_seconds:
        settingsValue.upstream_first_byte_timeout_seconds ??
        DEFAULT_SETTINGS.upstream_first_byte_timeout_seconds,
      upstream_stream_idle_timeout_seconds:
        settingsValue.upstream_stream_idle_timeout_seconds ??
        DEFAULT_SETTINGS.upstream_stream_idle_timeout_seconds,
      upstream_request_timeout_non_streaming_seconds:
        settingsValue.upstream_request_timeout_non_streaming_seconds ??
        DEFAULT_SETTINGS.upstream_request_timeout_non_streaming_seconds,
      intercept_anthropic_warmup_requests:
        settingsValue.intercept_anthropic_warmup_requests ??
        DEFAULT_SETTINGS.intercept_anthropic_warmup_requests,
      enable_thinking_signature_rectifier:
        settingsValue.enable_thinking_signature_rectifier ??
        DEFAULT_SETTINGS.enable_thinking_signature_rectifier,
      enable_response_fixer:
        settingsValue.enable_response_fixer ?? DEFAULT_SETTINGS.enable_response_fixer,
      response_fixer_fix_encoding:
        settingsValue.response_fixer_fix_encoding ?? DEFAULT_SETTINGS.response_fixer_fix_encoding,
      response_fixer_fix_sse_format:
        settingsValue.response_fixer_fix_sse_format ??
        DEFAULT_SETTINGS.response_fixer_fix_sse_format,
      response_fixer_fix_truncated_json:
        settingsValue.response_fixer_fix_truncated_json ??
        DEFAULT_SETTINGS.response_fixer_fix_truncated_json,
      failover_max_attempts_per_provider:
        settingsValue.failover_max_attempts_per_provider ??
        DEFAULT_SETTINGS.failover_max_attempts_per_provider,
      failover_max_providers_to_try:
        settingsValue.failover_max_providers_to_try ??
        DEFAULT_SETTINGS.failover_max_providers_to_try,
      circuit_breaker_failure_threshold:
        settingsValue.circuit_breaker_failure_threshold ??
        DEFAULT_SETTINGS.circuit_breaker_failure_threshold,
      circuit_breaker_open_duration_minutes:
        settingsValue.circuit_breaker_open_duration_minutes ??
        DEFAULT_SETTINGS.circuit_breaker_open_duration_minutes,
    };

    persistedSettingsRef.current = nextSettings;
    desiredSettingsRef.current = nextSettings;

    setPort(nextSettings.preferred_port);
    setShowHomeHeatmap(nextSettings.show_home_heatmap);
    setAutoStart(nextSettings.auto_start);
    setStartMinimized(nextSettings.start_minimized);
    setTrayEnabled(nextSettings.tray_enabled);
    setLogRetentionDays(nextSettings.log_retention_days);
    setSettingsReady(true);
  }, [
    settingsQuery.data,
    settingsQuery.error,
    settingsQuery.isError,
    settingsQuery.isLoading,
    settingsReady,
  ]);

  function diffKeys(before: PersistedSettings, after: PersistedSettings): PersistKey[] {
    const keys = Object.keys(before) as PersistKey[];
    const out: PersistKey[] = [];
    for (const key of keys) {
      if (before[key] !== after[key]) out.push(key);
    }
    return out;
  }

  function setSetting<K extends PersistKey>(
    target: PersistedSettings,
    key: K,
    value: PersistedSettings[K]
  ) {
    target[key] = value;
  }

  function applySettingToState(key: PersistKey, value: PersistedSettings[PersistKey]) {
    switch (key) {
      case "preferred_port":
        setPort(value as number);
        return;
      case "auto_start":
        setAutoStart(value as boolean);
        return;
      case "show_home_heatmap":
        setShowHomeHeatmap(value as boolean);
        return;
      case "start_minimized":
        setStartMinimized(value as boolean);
        return;
      case "tray_enabled":
        setTrayEnabled(value as boolean);
        return;
      case "log_retention_days":
        setLogRetentionDays(value as number);
        return;
    }
  }

  function revertKeys(keys: PersistKey[]) {
    if (keys.length === 0) return;
    const base = persistedSettingsRef.current;
    const nextDesired = { ...desiredSettingsRef.current };
    for (const key of keys) {
      setSetting(nextDesired, key, base[key]);
      applySettingToState(key, base[key]);
    }
    desiredSettingsRef.current = nextDesired;
  }

  function validateDesiredForKeys(desired: PersistedSettings, keys: PersistKey[]) {
    if (keys.includes("preferred_port")) {
      if (
        !Number.isFinite(desired.preferred_port) ||
        desired.preferred_port < 1024 ||
        desired.preferred_port > 65535
      ) {
        return "端口号必须为 1024-65535";
      }
    }

    if (keys.includes("log_retention_days")) {
      if (
        !Number.isFinite(desired.log_retention_days) ||
        desired.log_retention_days < 1 ||
        desired.log_retention_days > 3650
      ) {
        return "日志保留必须为 1-3650 天";
      }
    }

    if (keys.includes("provider_cooldown_seconds")) {
      if (
        !Number.isFinite(desired.provider_cooldown_seconds) ||
        desired.provider_cooldown_seconds < 0 ||
        desired.provider_cooldown_seconds > 3600
      ) {
        return "短熔断冷却必须为 0-3600 秒";
      }
    }

    if (keys.includes("provider_base_url_ping_cache_ttl_seconds")) {
      if (
        !Number.isFinite(desired.provider_base_url_ping_cache_ttl_seconds) ||
        desired.provider_base_url_ping_cache_ttl_seconds < 1 ||
        desired.provider_base_url_ping_cache_ttl_seconds > 3600
      ) {
        return "Ping 选择缓存 TTL 必须为 1-3600 秒";
      }
    }

    if (keys.includes("upstream_first_byte_timeout_seconds")) {
      if (
        !Number.isFinite(desired.upstream_first_byte_timeout_seconds) ||
        desired.upstream_first_byte_timeout_seconds < 0 ||
        desired.upstream_first_byte_timeout_seconds > 3600
      ) {
        return "上游首字节超时必须为 0-3600 秒";
      }
    }

    if (keys.includes("upstream_stream_idle_timeout_seconds")) {
      if (
        !Number.isFinite(desired.upstream_stream_idle_timeout_seconds) ||
        desired.upstream_stream_idle_timeout_seconds < 0 ||
        desired.upstream_stream_idle_timeout_seconds > 3600
      ) {
        return "上游流式空闲超时必须为 0-3600 秒";
      }
    }

    if (keys.includes("upstream_request_timeout_non_streaming_seconds")) {
      if (
        !Number.isFinite(desired.upstream_request_timeout_non_streaming_seconds) ||
        desired.upstream_request_timeout_non_streaming_seconds < 0 ||
        desired.upstream_request_timeout_non_streaming_seconds > 86400
      ) {
        return "上游非流式总超时必须为 0-86400 秒";
      }
    }

    if (keys.includes("circuit_breaker_failure_threshold")) {
      if (
        !Number.isFinite(desired.circuit_breaker_failure_threshold) ||
        desired.circuit_breaker_failure_threshold < 1 ||
        desired.circuit_breaker_failure_threshold > 50
      ) {
        return "熔断阈值必须为 1-50";
      }
    }

    if (keys.includes("circuit_breaker_open_duration_minutes")) {
      if (
        !Number.isFinite(desired.circuit_breaker_open_duration_minutes) ||
        desired.circuit_breaker_open_duration_minutes < 1 ||
        desired.circuit_breaker_open_duration_minutes > 1440
      ) {
        return "熔断时长必须为 1-1440 分钟";
      }
    }

    return null;
  }

  function revertSettledKeys(desiredSnapshot: PersistedSettings, keysToConsider: PersistKey[]) {
    const desiredNow = desiredSettingsRef.current;
    const settledKeys = keysToConsider.filter((key) => desiredNow[key] === desiredSnapshot[key]);
    if (settledKeys.length > 0) revertKeys(settledKeys);
    if (persistQueueRef.current.pending) {
      persistQueueRef.current.pending = desiredSettingsRef.current;
    }
  }

  function enqueuePersist(desiredSnapshot: PersistedSettings) {
    if (!settingsReady) return;

    const queue = persistQueueRef.current;
    if (queue.inFlight) {
      queue.pending = desiredSnapshot;
      return;
    }

    queue.inFlight = true;
    void persistSettings(desiredSnapshot).finally(() => {
      const next = queue.pending;
      queue.pending = null;
      queue.inFlight = false;
      if (next) enqueuePersist(next);
    });
  }

  function requestPersist(patch: Partial<PersistedSettings>) {
    if (!settingsReady) return;
    const previous = desiredSettingsRef.current;
    const next = { ...previous, ...patch };
    desiredSettingsRef.current = next;
    enqueuePersist(next);
  }

  function commitNumberField(options: {
    key: "preferred_port" | "log_retention_days";
    next: number;
    min: number;
    max: number;
    invalidMessage: string;
  }) {
    if (!settingsReady) return;
    const normalized = Math.floor(options.next);
    if (!Number.isFinite(normalized) || normalized < options.min || normalized > options.max) {
      toast(options.invalidMessage);
      applySettingToState(options.key, desiredSettingsRef.current[options.key]);
      return;
    }

    applySettingToState(options.key, normalized as PersistedSettings[PersistKey]);
    requestPersist({ [options.key]: normalized } as Partial<PersistedSettings>);
  }

  async function persistSettings(desiredSnapshot: PersistedSettings) {
    const before = persistedSettingsRef.current;
    let desired = desiredSnapshot;
    let changedKeys = diffKeys(before, desired);
    if (changedKeys.length === 0) return;

    const validationError = validateDesiredForKeys(desired, changedKeys);
    if (validationError) {
      toast(validationError);
      revertSettledKeys(desired, changedKeys);
      return;
    }

    if (
      changedKeys.includes("preferred_port") &&
      !(gateway?.running && gateway.port === desired.preferred_port)
    ) {
      if (desiredSettingsRef.current.preferred_port !== desired.preferred_port) {
        return;
      }

      const available = await gatewayCheckPortAvailable(desired.preferred_port);
      if (available === false) {
        if (desiredSettingsRef.current.preferred_port === desired.preferred_port) {
          toast(`端口 ${desired.preferred_port} 已被占用，请换一个端口`);
          revertSettledKeys(desired, ["preferred_port"]);
          desired = { ...desired, preferred_port: before.preferred_port };
        } else {
          return;
        }
      }
    }

    changedKeys = diffKeys(before, desired);
    if (changedKeys.length === 0) return;

    try {
      const nextSettings = await settingsSetMutation.mutateAsync({
        preferredPort: desired.preferred_port,
        showHomeHeatmap: desired.show_home_heatmap,
        autoStart: desired.auto_start,
        startMinimized: desired.start_minimized,
        trayEnabled: desired.tray_enabled,
        logRetentionDays: desired.log_retention_days,
        providerCooldownSeconds: desired.provider_cooldown_seconds,
        providerBaseUrlPingCacheTtlSeconds: desired.provider_base_url_ping_cache_ttl_seconds,
        upstreamFirstByteTimeoutSeconds: desired.upstream_first_byte_timeout_seconds,
        upstreamStreamIdleTimeoutSeconds: desired.upstream_stream_idle_timeout_seconds,
        upstreamRequestTimeoutNonStreamingSeconds:
          desired.upstream_request_timeout_non_streaming_seconds,
        failoverMaxAttemptsPerProvider: desired.failover_max_attempts_per_provider,
        failoverMaxProvidersToTry: desired.failover_max_providers_to_try,
        circuitBreakerFailureThreshold: desired.circuit_breaker_failure_threshold,
        circuitBreakerOpenDurationMinutes: desired.circuit_breaker_open_duration_minutes,
      });

      if (!nextSettings) {
        revertSettledKeys(desired, changedKeys);
        return;
      }

      const after: PersistedSettings = {
        preferred_port: nextSettings.preferred_port,
        show_home_heatmap: nextSettings.show_home_heatmap ?? desired.show_home_heatmap,
        auto_start: nextSettings.auto_start,
        start_minimized: nextSettings.start_minimized ?? desired.start_minimized,
        tray_enabled: nextSettings.tray_enabled ?? desired.tray_enabled,
        log_retention_days: nextSettings.log_retention_days,
        provider_cooldown_seconds:
          nextSettings.provider_cooldown_seconds ?? desired.provider_cooldown_seconds,
        provider_base_url_ping_cache_ttl_seconds:
          nextSettings.provider_base_url_ping_cache_ttl_seconds ??
          desired.provider_base_url_ping_cache_ttl_seconds,
        upstream_first_byte_timeout_seconds:
          nextSettings.upstream_first_byte_timeout_seconds ??
          desired.upstream_first_byte_timeout_seconds,
        upstream_stream_idle_timeout_seconds:
          nextSettings.upstream_stream_idle_timeout_seconds ??
          desired.upstream_stream_idle_timeout_seconds,
        upstream_request_timeout_non_streaming_seconds:
          nextSettings.upstream_request_timeout_non_streaming_seconds ??
          desired.upstream_request_timeout_non_streaming_seconds,
        intercept_anthropic_warmup_requests:
          nextSettings.intercept_anthropic_warmup_requests ??
          desired.intercept_anthropic_warmup_requests,
        enable_thinking_signature_rectifier:
          nextSettings.enable_thinking_signature_rectifier ??
          desired.enable_thinking_signature_rectifier,
        enable_response_fixer: nextSettings.enable_response_fixer ?? desired.enable_response_fixer,
        response_fixer_fix_encoding:
          nextSettings.response_fixer_fix_encoding ?? desired.response_fixer_fix_encoding,
        response_fixer_fix_sse_format:
          nextSettings.response_fixer_fix_sse_format ?? desired.response_fixer_fix_sse_format,
        response_fixer_fix_truncated_json:
          nextSettings.response_fixer_fix_truncated_json ??
          desired.response_fixer_fix_truncated_json,
        failover_max_attempts_per_provider:
          nextSettings.failover_max_attempts_per_provider ??
          desired.failover_max_attempts_per_provider,
        failover_max_providers_to_try:
          nextSettings.failover_max_providers_to_try ?? desired.failover_max_providers_to_try,
        circuit_breaker_failure_threshold:
          nextSettings.circuit_breaker_failure_threshold ??
          desired.circuit_breaker_failure_threshold,
        circuit_breaker_open_duration_minutes:
          nextSettings.circuit_breaker_open_duration_minutes ??
          desired.circuit_breaker_open_duration_minutes,
      };

      persistedSettingsRef.current = after;

      const desiredNow = desiredSettingsRef.current;
      const settledKeys = changedKeys.filter((key) => desiredNow[key] === desired[key]);
      if (settledKeys.length > 0) {
        const nextDesired = { ...desiredNow };
        for (const key of settledKeys) {
          setSetting(nextDesired, key, after[key]);
          applySettingToState(key, after[key]);
        }
        desiredSettingsRef.current = nextDesired;
      }

      const portSettled = settledKeys.includes("preferred_port");

      logToConsole("info", "更新设置", { changed: changedKeys, settings: after });

      const circuitSettled =
        settledKeys.includes("circuit_breaker_failure_threshold") ||
        settledKeys.includes("circuit_breaker_open_duration_minutes");
      if (circuitSettled && gateway?.running && !portSettled) {
        toast("熔断参数已保存：重启网关后生效");
      }

      if (settledKeys.includes("auto_start")) {
        if (after.auto_start !== desired.auto_start) {
          toast("开机自启设置失败，已回退");
        } else if (after.auto_start && about?.run_mode === "portable") {
          toast("portable 模式开启自启：移动应用位置可能导致自启失效");
        }
      }

      if (portSettled) {
        if (!gateway?.running) {
          const baseOrigin = `http://127.0.0.1:${after.preferred_port}`;
          const syncResults = await cliProxySyncEnabled(baseOrigin);
          if (syncResults) {
            const okCount = syncResults.filter((r) => r.ok).length;
            logToConsole("info", "端口变更，已同步 CLI 代理配置", {
              base_origin: baseOrigin,
              ok_count: okCount,
              total: syncResults.length,
            });
            if (syncResults.length > 0) {
              toast(`已同步 ${okCount}/${syncResults.length} 个 CLI 代理配置`);
            }
          }
        } else {
          logToConsole("info", "端口变更，自动重启网关", {
            from: before.preferred_port,
            to: after.preferred_port,
          });

          const stopped = await gatewayStop();
          if (!stopped) {
            toast("自动重启失败：无法停止网关");
            return;
          }

          const started = await gatewayStart(after.preferred_port);
          if (!started) {
            toast("自动重启失败：无法启动网关");
            return;
          }

          const baseOrigin =
            started.base_url ?? `http://127.0.0.1:${started.port ?? after.preferred_port}`;
          const syncResults = await cliProxySyncEnabled(baseOrigin);
          if (syncResults) {
            const okCount = syncResults.filter((r) => r.ok).length;
            logToConsole("info", "端口变更，已同步 CLI 代理配置", {
              base_origin: baseOrigin,
              ok_count: okCount,
              total: syncResults.length,
            });
            if (syncResults.length > 0) {
              toast(`已同步 ${okCount}/${syncResults.length} 个 CLI 代理配置`);
            }
          }

          if (started.port && started.port !== after.preferred_port) {
            toast(`端口被占用，已切换到 ${started.port}`);
          } else {
            toast("网关已按新端口重启");
          }
        }
      }
    } catch (err) {
      logToConsole("error", "更新设置失败", { error: String(err) });
      toast("更新设置失败：请稍后重试");
      revertSettledKeys(desired, changedKeys);
    }
  }

  return {
    settingsReady,

    port,
    setPort,
    showHomeHeatmap,
    setShowHomeHeatmap,
    autoStart,
    setAutoStart,
    startMinimized,
    setStartMinimized,
    trayEnabled,
    setTrayEnabled,
    logRetentionDays,
    setLogRetentionDays,

    requestPersist,
    commitNumberField,
  };
}
