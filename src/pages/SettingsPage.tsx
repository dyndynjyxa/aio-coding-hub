// Usage: App settings and gateway controls. Backend commands: `settings_*`, `gateway_*`, `cli_proxy_*`, `model_prices_*`, `usage_*`, `app_data_*`.

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { AIO_RELEASES_URL } from "../constants/urls";
import { cliProxySyncEnabled } from "../services/cliProxy";
import { logToConsole } from "../services/consoleLog";
import { updateCheckNow, useUpdateMeta } from "../hooks/useUpdateMeta";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { SettingsRow } from "../ui/SettingsRow";
import { Switch } from "../ui/Switch";
import { gatewayCheckPortAvailable, gatewayStart, gatewayStop } from "../services/gateway";
import { settingsGet, settingsSet } from "../services/settings";
import { gatewayMetaSetPreferredPort, useGatewayMeta } from "../hooks/useGatewayMeta";
import {
  modelPricesList,
  modelPricesSyncBasellm,
  subscribeModelPricesUpdated,
  type ModelPricesSyncReport,
} from "../services/modelPrices";
import { usageSummary } from "../services/usage";
import {
  appDataDirGet,
  appDataReset,
  appExit,
  dbDiskUsageGet,
  requestLogsClearAll,
  type DbDiskUsage,
} from "../services/dataManagement";
import { noticeSend } from "../services/notice";
import { cn } from "../utils/cn";
import { formatBytes } from "../utils/formatters";
import { ModelPriceAliasesDialog } from "../components/settings/ModelPriceAliasesDialog";

type NoticePermissionStatus = "checking" | "granted" | "not_granted" | "denied" | "unknown";

type NotificationPluginModule = typeof import("@tauri-apps/plugin-notification");

let notificationPluginPromise: Promise<NotificationPluginModule> | null = null;

function loadNotificationPlugin(): Promise<NotificationPluginModule> {
  if (notificationPluginPromise) return notificationPluginPromise;
  notificationPluginPromise = import("@tauri-apps/plugin-notification").catch((err) => {
    notificationPluginPromise = null;
    throw err;
  });
  return notificationPluginPromise;
}

type PersistedSettings = {
  preferred_port: number;
  auto_start: boolean;
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
  auto_start: false,
  tray_enabled: true,
  log_retention_days: 30,
  provider_cooldown_seconds: 30,
  provider_base_url_ping_cache_ttl_seconds: 60,
  upstream_first_byte_timeout_seconds: 0,
  upstream_stream_idle_timeout_seconds: 0,
  upstream_request_timeout_non_streaming_seconds: 0,
  intercept_anthropic_warmup_requests: false,
  enable_thinking_signature_rectifier: false,
  enable_response_fixer: false,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
  failover_max_attempts_per_provider: 5,
  failover_max_providers_to_try: 5,
  circuit_breaker_failure_threshold: 5,
  circuit_breaker_open_duration_minutes: 30,
};

export function SettingsPage() {
  const { gateway, gatewayAvailable } = useGatewayMeta();
  const [settingsReady, setSettingsReady] = useState(false);
  const [port, setPort] = useState<number>(37123);
  const [autoStart, setAutoStart] = useState<boolean>(false);
  const [trayEnabled, setTrayEnabled] = useState<boolean>(true);
  const [logRetentionDays, setLogRetentionDays] = useState<number>(30);
  const updateMeta = useUpdateMeta();
  const about = updateMeta.about;
  const [syncingModelPrices, setSyncingModelPrices] = useState(false);
  const [lastModelPricesSyncReport, setLastModelPricesSyncReport] =
    useState<ModelPricesSyncReport | null>(null);
  const [lastModelPricesSyncError, setLastModelPricesSyncError] = useState<string | null>(null);
  const [modelPriceAliasesDialogOpen, setModelPriceAliasesDialogOpen] = useState(false);
  const [modelPricesAvailable, setModelPricesAvailable] = useState<
    "checking" | "available" | "unavailable"
  >("checking");
  const [modelPricesCount, setModelPricesCount] = useState<number | null>(null);
  const [todayRequestsAvailable, setTodayRequestsAvailable] = useState<
    "checking" | "available" | "unavailable"
  >("checking");
  const [todayRequestsTotal, setTodayRequestsTotal] = useState<number | null>(null);
  const [dbDiskUsageAvailable, setDbDiskUsageAvailable] = useState<
    "checking" | "available" | "unavailable"
  >("checking");
  const [dbDiskUsage, setDbDiskUsage] = useState<DbDiskUsage | null>(null);
  const [clearRequestLogsDialogOpen, setClearRequestLogsDialogOpen] = useState(false);
  const [clearingRequestLogs, setClearingRequestLogs] = useState(false);
  const [resetAllDialogOpen, setResetAllDialogOpen] = useState(false);
  const [resettingAll, setResettingAll] = useState(false);
  const [noticePermissionStatus, setNoticePermissionStatus] =
    useState<NoticePermissionStatus>("checking");
  const [requestingNoticePermission, setRequestingNoticePermission] = useState(false);
  const [sendingNoticeTest, setSendingNoticeTest] = useState(false);

  const persistedSettingsRef = useRef<PersistedSettings>(DEFAULT_SETTINGS);
  const desiredSettingsRef = useRef<PersistedSettings>(DEFAULT_SETTINGS);
  const persistQueueRef = useRef<{
    inFlight: boolean;
    pending: PersistedSettings | null;
  }>({ inFlight: false, pending: null });

  function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") e.currentTarget.blur();
  }

  useEffect(() => {
    let cancelled = false;
    settingsGet()
      .then((settingsValue) => {
        if (cancelled) return;
        if (!settingsValue) {
          setSettingsReady(true);
          return;
        }

        const nextSettings: PersistedSettings = {
          preferred_port: settingsValue.preferred_port,
          auto_start: settingsValue.auto_start,
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
            settingsValue.response_fixer_fix_encoding ??
            DEFAULT_SETTINGS.response_fixer_fix_encoding,
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
        setAutoStart(nextSettings.auto_start);
        setTrayEnabled(nextSettings.tray_enabled);
        setLogRetentionDays(nextSettings.log_retention_days);
        setSettingsReady(true);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取设置失败", { error: String(err) });
        toast("读取设置失败：请检查 settings.json；修改任一配置将尝试覆盖写入修复");
        setSettingsReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    loadNotificationPlugin()
      .then(async ({ isPermissionGranted }) => {
        const granted = await isPermissionGranted();
        if (cancelled) return;
        setNoticePermissionStatus(granted ? "granted" : "not_granted");
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "检查系统通知权限失败", { error: String(err) });
        setNoticePermissionStatus("unknown");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function requestSystemNotificationPermission() {
    if (requestingNoticePermission) return;
    setRequestingNoticePermission(true);

    try {
      const { requestPermission } = await loadNotificationPlugin();
      const permission = await requestPermission();
      const granted = permission === "granted";
      setNoticePermissionStatus(granted ? "granted" : "denied");
      toast(granted ? "系统通知权限已授权" : "系统通知权限已拒绝");
    } catch (err) {
      logToConsole("error", "请求系统通知权限失败", { error: String(err) });
      toast("请求系统通知权限失败：请查看控制台日志");
      setNoticePermissionStatus("unknown");
    } finally {
      setRequestingNoticePermission(false);
    }
  }

  async function sendSystemNotificationTest() {
    if (sendingNoticeTest) return;
    setSendingNoticeTest(true);

    try {
      const { isPermissionGranted } = await loadNotificationPlugin();
      const granted = await isPermissionGranted();
      if (!granted) {
        setNoticePermissionStatus("not_granted");
        toast("请先在「系统通知」中授权通知权限");
        return;
      }

      const ok = await noticeSend({
        level: "info",
        title: "测试通知",
        body: "这是一条来自 AIO Coding Hub 的系统通知",
      });
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      toast("已发送测试通知");
    } catch (err) {
      logToConsole("error", "发送测试通知失败", { error: String(err) });
      toast("发送测试通知失败：请查看控制台日志");
    } finally {
      setSendingNoticeTest(false);
    }
  }

  async function openUpdateLog() {
    const url = AIO_RELEASES_URL;

    try {
      await openUrl(url);
    } catch (err) {
      logToConsole("error", "打开更新日志失败", { error: String(err), url });
      toast("打开更新日志失败");
    }
  }

  async function openAppDataDir() {
    try {
      const dir = await appDataDirGet();
      if (!dir) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      await openPath(dir);
    } catch (err) {
      logToConsole("error", "打开数据目录失败", { error: String(err) });
      toast("打开数据目录失败：请查看控制台日志");
    }
  }

  async function checkUpdate() {
    try {
      if (!about) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      if (about.run_mode === "portable") {
        toast("portable 模式请手动下载");
        await openUpdateLog();
        return;
      }

      await updateCheckNow({ silent: false, openDialogIfUpdate: true });
    } catch {
      // noop: errors/toasts are handled in updateCheckNow
    }
  }

  const refreshModelPricesCount = useCallback(async () => {
    setModelPricesAvailable("checking");
    try {
      const [codex, claude, gemini] = await Promise.all([
        modelPricesList("codex"),
        modelPricesList("claude"),
        modelPricesList("gemini"),
      ]);

      if (!codex || !claude || !gemini) {
        setModelPricesAvailable("unavailable");
        setModelPricesCount(null);
        return;
      }

      setModelPricesAvailable("available");
      setModelPricesCount(codex.length + claude.length + gemini.length);
    } catch {
      setModelPricesAvailable("unavailable");
      setModelPricesCount(null);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    setTodayRequestsAvailable("checking");
    usageSummary("today")
      .then((summary) => {
        if (cancelled) return;
        if (!summary) {
          setTodayRequestsAvailable("unavailable");
          setTodayRequestsTotal(null);
          return;
        }
        setTodayRequestsAvailable("available");
        setTodayRequestsTotal(summary.requests_total);
      })
      .catch(() => {
        if (cancelled) return;
        setTodayRequestsAvailable("unavailable");
        setTodayRequestsTotal(null);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    refreshModelPricesCount().catch(() => {});
  }, [refreshModelPricesCount]);

  const refreshDbDiskUsage = useCallback(async () => {
    setDbDiskUsageAvailable("checking");
    try {
      const usage = await dbDiskUsageGet();
      if (!usage) {
        setDbDiskUsageAvailable("unavailable");
        setDbDiskUsage(null);
        return;
      }
      setDbDiskUsageAvailable("available");
      setDbDiskUsage(usage);
    } catch {
      setDbDiskUsageAvailable("unavailable");
      setDbDiskUsage(null);
    }
  }, []);

  useEffect(() => {
    refreshDbDiskUsage().catch(() => {});
  }, [refreshDbDiskUsage]);

  async function clearRequestLogs() {
    if (clearingRequestLogs) return;
    setClearingRequestLogs(true);

    try {
      const result = await requestLogsClearAll();
      if (!result) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      toast(
        `已清理请求日志：request_logs ${result.request_logs_deleted} 条，request_attempt_logs ${result.request_attempt_logs_deleted} 条`
      );
      logToConsole("info", "清理请求日志", result);
      setClearRequestLogsDialogOpen(false);
      refreshDbDiskUsage().catch(() => {});
    } catch (err) {
      logToConsole("error", "清理请求日志失败", { error: String(err) });
      toast("清理请求日志失败：请稍后重试");
    } finally {
      setClearingRequestLogs(false);
    }
  }

  async function resetAllData() {
    if (resettingAll) return;
    setResettingAll(true);

    try {
      const ok = await appDataReset();
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      logToConsole("info", "清理全部信息", { ok: true });
      toast("已清理全部信息：应用即将退出，请重新打开");
      setResetAllDialogOpen(false);

      window.setTimeout(() => {
        appExit().catch(() => {});
      }, 1000);
    } catch (err) {
      logToConsole("error", "清理全部信息失败", { error: String(err) });
      toast("清理全部信息失败：请稍后重试");
    } finally {
      setResettingAll(false);
    }
  }

  useEffect(() => {
    return subscribeModelPricesUpdated(() => {
      refreshModelPricesCount().catch(() => {});
    });
  }, [refreshModelPricesCount]);

  type PersistKey = keyof PersistedSettings;

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
      const nextSettings = await settingsSet({
        preferred_port: desired.preferred_port,
        auto_start: desired.auto_start,
        tray_enabled: desired.tray_enabled,
        log_retention_days: desired.log_retention_days,
        provider_cooldown_seconds: desired.provider_cooldown_seconds,
        provider_base_url_ping_cache_ttl_seconds: desired.provider_base_url_ping_cache_ttl_seconds,
        upstream_first_byte_timeout_seconds: desired.upstream_first_byte_timeout_seconds,
        upstream_stream_idle_timeout_seconds: desired.upstream_stream_idle_timeout_seconds,
        upstream_request_timeout_non_streaming_seconds:
          desired.upstream_request_timeout_non_streaming_seconds,
        failover_max_attempts_per_provider: desired.failover_max_attempts_per_provider,
        failover_max_providers_to_try: desired.failover_max_providers_to_try,
        circuit_breaker_failure_threshold: desired.circuit_breaker_failure_threshold,
        circuit_breaker_open_duration_minutes: desired.circuit_breaker_open_duration_minutes,
      });

      if (!nextSettings) {
        toast("仅在 Tauri Desktop 环境可用");
        revertSettledKeys(desired, changedKeys);
        return;
      }

      const after: PersistedSettings = {
        preferred_port: nextSettings.preferred_port,
        auto_start: nextSettings.auto_start,
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
      if (portSettled) {
        gatewayMetaSetPreferredPort(after.preferred_port);
      }

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

  async function syncModelPrices(force: boolean) {
    if (syncingModelPrices) return;
    setSyncingModelPrices(true);
    setLastModelPricesSyncError(null);

    try {
      const report = await modelPricesSyncBasellm(force);
      if (!report) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setLastModelPricesSyncReport(report);
      if (report.status !== "not_modified") {
        await refreshModelPricesCount();
      }

      if (report.status === "not_modified") {
        toast("模型定价已是最新（无变更）");
        return;
      }

      toast(`同步完成：新增 ${report.inserted}，更新 ${report.updated}，跳过 ${report.skipped}`);
    } catch (err) {
      logToConsole("error", "同步模型定价失败", { error: String(err) });
      toast("同步模型定价失败：请稍后重试");
      setLastModelPricesSyncError(String(err));
    } finally {
      setSyncingModelPrices(false);
    }
  }

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-start">
      <div className="lg:col-span-12">
        <h1 className="text-2xl font-semibold tracking-tight text-slate-900">设置</h1>
      </div>

      {/* 左侧：主要配置 */}
      <div className="space-y-6 lg:col-span-8">
        {/* 网关服务 */}
        <Card>
          <div className="mb-4 flex items-center justify-between border-b border-slate-100 pb-4">
            <div className="font-semibold text-slate-900">网关服务</div>
            <span
              className={cn(
                "rounded-full px-2.5 py-0.5 text-xs font-medium",
                gatewayAvailable === "checking" || gatewayAvailable === "unavailable"
                  ? "bg-slate-100 text-slate-600"
                  : gateway?.running
                    ? "bg-emerald-50 text-emerald-700"
                    : "bg-slate-100 text-slate-600"
              )}
            >
              {gatewayAvailable === "checking"
                ? "检查中"
                : gatewayAvailable === "unavailable"
                  ? "不可用"
                  : gateway?.running
                    ? "运行中"
                    : "未运行"}
            </span>
          </div>

          <div className="space-y-1">
            <SettingsRow label="服务状态">
              <div className="flex gap-2">
                <Button
                  onClick={async () => {
                    const desiredPort = Math.floor(port);
                    if (
                      !Number.isFinite(desiredPort) ||
                      desiredPort < 1024 ||
                      desiredPort > 65535
                    ) {
                      toast("端口号必须为 1024-65535");
                      return;
                    }

                    if (gateway?.running) {
                      const stopped = await gatewayStop();
                      if (!stopped) {
                        toast("重启失败：无法停止网关");
                        return;
                      }
                    }

                    const status = await gatewayStart(desiredPort);
                    if (!status) {
                      toast("启动失败：当前环境不可用或 command 未注册");
                      return;
                    }
                    logToConsole("info", "启动本地网关", {
                      port: status.port,
                      base_url: status.base_url,
                    });
                    toast(gateway?.running ? "本地网关已重启" : "本地网关已启动");
                  }}
                  variant={gateway?.running ? "secondary" : "primary"}
                  size="sm"
                  disabled={gatewayAvailable !== "available"}
                >
                  {gateway?.running ? "重启" : "启动"}
                </Button>
                <Button
                  onClick={async () => {
                    const status = await gatewayStop();
                    if (!status) {
                      toast("停止失败：当前环境不可用或 command 未注册");
                      return;
                    }
                    logToConsole("info", "停止本地网关");
                    toast("本地网关已停止");
                  }}
                  variant="secondary"
                  size="sm"
                  disabled={gatewayAvailable !== "available" || !gateway?.running}
                >
                  停止
                </Button>
              </div>
            </SettingsRow>

            <SettingsRow label="监听端口">
              <Input
                type="number"
                value={port}
                onChange={(e) => {
                  const next = e.currentTarget.valueAsNumber;
                  if (Number.isFinite(next)) setPort(next);
                }}
                onBlur={(e) =>
                  commitNumberField({
                    key: "preferred_port",
                    next: e.currentTarget.valueAsNumber,
                    min: 1024,
                    max: 65535,
                    invalidMessage: "端口号必须为 1024-65535",
                  })
                }
                onKeyDown={blurOnEnter}
                className="w-28 font-mono"
                min={1024}
                max={65535}
                disabled={!settingsReady}
              />
            </SettingsRow>
          </div>
        </Card>

        {/* 参数配置 */}
        <Card>
          <div className="mb-4 border-b border-slate-100 pb-4">
            <div className="font-semibold text-slate-900">参数配置</div>
          </div>

          <div className="space-y-8">
            {/* 系统偏好 */}
            <div>
              <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500">
                系统偏好
              </h3>
              <div className="space-y-1">
                <SettingsRow label="开机自启">
                  <Switch
                    checked={autoStart}
                    onCheckedChange={(checked) => {
                      setAutoStart(checked);
                      requestPersist({ auto_start: checked });
                    }}
                    disabled={!settingsReady}
                  />
                </SettingsRow>
                <SettingsRow label="托盘常驻">
                  <Switch
                    checked={trayEnabled}
                    onCheckedChange={(checked) => {
                      setTrayEnabled(checked);
                      requestPersist({ tray_enabled: checked });
                    }}
                    disabled={!settingsReady}
                  />
                </SettingsRow>
                <SettingsRow label="日志保留">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={logRetentionDays}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setLogRetentionDays(next);
                      }}
                      onBlur={(e) =>
                        commitNumberField({
                          key: "log_retention_days",
                          next: e.currentTarget.valueAsNumber,
                          min: 1,
                          max: 3650,
                          invalidMessage: "日志保留必须为 1-3650 天",
                        })
                      }
                      onKeyDown={blurOnEnter}
                      className="w-24"
                      min={1}
                      max={3650}
                      disabled={!settingsReady}
                    />
                    <span className="text-sm text-slate-500">天</span>
                  </div>
                </SettingsRow>
              </div>
            </div>

            {/* 系统通知 */}
            <div>
              <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500">
                系统通知
              </h3>
              <div className="space-y-1">
                <SettingsRow label="权限状态">
                  <span
                    className={cn(
                      "rounded-full px-2.5 py-0.5 text-xs font-medium",
                      noticePermissionStatus === "granted"
                        ? "bg-emerald-50 text-emerald-700"
                        : noticePermissionStatus === "checking" ||
                            noticePermissionStatus === "unknown"
                          ? "bg-slate-100 text-slate-600"
                          : "bg-amber-50 text-amber-700"
                    )}
                  >
                    {noticePermissionStatus === "checking"
                      ? "检查中"
                      : noticePermissionStatus === "granted"
                        ? "已授权"
                        : noticePermissionStatus === "denied"
                          ? "已拒绝"
                          : noticePermissionStatus === "not_granted"
                            ? "未授权"
                            : "未知"}
                  </span>
                </SettingsRow>
                <SettingsRow label="请求权限">
                  <Button
                    onClick={() => void requestSystemNotificationPermission()}
                    variant="secondary"
                    size="sm"
                    disabled={requestingNoticePermission}
                  >
                    {requestingNoticePermission ? "请求中…" : "请求通知权限"}
                  </Button>
                </SettingsRow>
                <SettingsRow label="测试通知">
                  <Button
                    onClick={() => void sendSystemNotificationTest()}
                    variant="secondary"
                    size="sm"
                    disabled={sendingNoticeTest}
                  >
                    {sendingNoticeTest ? "发送中…" : "发送测试通知"}
                  </Button>
                </SettingsRow>
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* 右侧：信息与数据 */}
      <div className="space-y-6 lg:col-span-4">
        {/* 关于应用 */}
        <Card>
          <div className="mb-4 font-semibold text-slate-900">关于应用</div>
          {about ? (
            <div className="grid gap-2 text-sm text-slate-700">
              <div className="flex items-center justify-between gap-4">
                <span className="text-slate-500">版本</span>
                <span className="font-mono">{about.app_version}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-slate-500">构建</span>
                <span className="font-mono">{about.profile}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-slate-500">平台</span>
                <span className="font-mono">
                  {about.os}/{about.arch}
                </span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-slate-500">Bundle</span>
                <span className="font-mono">{about.bundle_type ?? "—"}</span>
              </div>
              <div className="flex items-center justify-between gap-4">
                <span className="text-slate-500">运行模式</span>
                <span className="font-mono">{about.run_mode}</span>
              </div>
            </div>
          ) : (
            <div className="text-sm text-slate-600">仅在 Tauri Desktop 环境可用。</div>
          )}
        </Card>

        {/* 软件更新 */}
        <Card>
          <div className="mb-4 font-semibold text-slate-900">软件更新</div>
          <div className="divide-y divide-slate-100">
            <SettingsRow label={about?.run_mode === "portable" ? "获取新版本" : "检查更新"}>
              <Button
                onClick={checkUpdate}
                variant="secondary"
                size="sm"
                disabled={updateMeta.checkingUpdate || !about}
              >
                {updateMeta.checkingUpdate
                  ? "检查中…"
                  : about?.run_mode === "portable"
                    ? "打开"
                    : "检查"}
              </Button>
            </SettingsRow>
          </div>
        </Card>

        {/* 数据管理 */}
        <Card>
          <div className="mb-4 flex items-center justify-between gap-2">
            <div className="font-semibold text-slate-900">数据管理</div>
            <Button
              onClick={() => void openAppDataDir()}
              variant="secondary"
              size="sm"
              disabled={!about}
            >
              打开目录
            </Button>
          </div>
          <div className="divide-y divide-slate-100">
            <SettingsRow label="数据磁盘占用">
              <span className="font-mono text-sm text-slate-900">
                {dbDiskUsageAvailable === "checking"
                  ? "加载中…"
                  : dbDiskUsageAvailable === "unavailable"
                    ? "—"
                    : formatBytes(dbDiskUsage?.total_bytes ?? 0)}
              </span>
              <Button
                onClick={() => refreshDbDiskUsage().catch(() => {})}
                variant="secondary"
                size="sm"
                disabled={!about || dbDiskUsageAvailable === "checking"}
              >
                刷新
              </Button>
            </SettingsRow>
            <SettingsRow label="清理请求日志">
              <span className="text-xs text-slate-500">不可撤销</span>
              <Button
                onClick={() => setClearRequestLogsDialogOpen(true)}
                variant="warning"
                size="sm"
                disabled={!about}
              >
                清理
              </Button>
            </SettingsRow>
            <SettingsRow label="清理全部信息">
              <span className="text-xs text-rose-700">不可撤销</span>
              <Button
                onClick={() => setResetAllDialogOpen(true)}
                variant="danger"
                size="sm"
                disabled={!about}
              >
                清理
              </Button>
            </SettingsRow>
          </div>
        </Card>

        {/* 数据与同步 */}
        <Card>
          <div className="mb-4 font-semibold text-slate-900">数据与同步</div>
          <div className="divide-y divide-slate-100">
            <SettingsRow label="模型定价">
              <span className="font-mono text-sm text-slate-900">
                {modelPricesAvailable === "checking"
                  ? "加载中…"
                  : modelPricesAvailable === "unavailable"
                    ? "—"
                    : modelPricesCount === 0
                      ? "未同步"
                      : `${modelPricesCount} 条`}
              </span>
              {lastModelPricesSyncError ? (
                <span className="text-xs text-rose-600">失败</span>
              ) : lastModelPricesSyncReport ? (
                <span className="text-xs text-slate-500">
                  {lastModelPricesSyncReport.status === "not_modified"
                    ? "最新"
                    : `+${lastModelPricesSyncReport.inserted} / ~${lastModelPricesSyncReport.updated}`}
                </span>
              ) : null}
            </SettingsRow>
            <SettingsRow label="定价匹配">
              <span className="text-xs text-slate-500">prefix / wildcard / exact</span>
              <Button
                onClick={() => setModelPriceAliasesDialogOpen(true)}
                variant="secondary"
                size="sm"
                disabled={!about}
              >
                配置
              </Button>
            </SettingsRow>
            <SettingsRow label="今日请求">
              <span className="font-mono text-sm text-slate-900">
                {todayRequestsAvailable === "checking"
                  ? "加载中…"
                  : todayRequestsAvailable === "unavailable"
                    ? "—"
                    : String(todayRequestsTotal ?? 0)}
              </span>
            </SettingsRow>
            <SettingsRow label="同步定价">
              <div className="flex gap-2">
                <Button
                  onClick={() => syncModelPrices(false)}
                  variant="secondary"
                  size="sm"
                  disabled={syncingModelPrices}
                >
                  {syncingModelPrices ? "同步中" : "同步"}
                </Button>
                <Button
                  onClick={() => syncModelPrices(true)}
                  variant="secondary"
                  size="sm"
                  disabled={syncingModelPrices}
                >
                  强制
                </Button>
              </div>
            </SettingsRow>
          </div>
        </Card>
      </div>

      <ModelPriceAliasesDialog
        open={modelPriceAliasesDialogOpen}
        onOpenChange={setModelPriceAliasesDialogOpen}
      />

      <Dialog
        open={clearRequestLogsDialogOpen}
        onOpenChange={(open) => {
          if (!open && clearingRequestLogs) return;
          setClearRequestLogsDialogOpen(open);
          if (!open) setClearingRequestLogs(false);
        }}
        title="确认清理请求日志"
        description="将清空 request_logs 与 request_attempt_logs。此操作不可撤销。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="text-sm text-slate-700">
            说明：仅影响请求日志与明细，不会影响 Providers、Prompts、MCP 等配置。
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 pt-3">
            <Button
              onClick={() => setClearRequestLogsDialogOpen(false)}
              variant="secondary"
              disabled={clearingRequestLogs}
            >
              取消
            </Button>
            <Button onClick={clearRequestLogs} variant="warning" disabled={clearingRequestLogs}>
              {clearingRequestLogs ? "清理中…" : "确认清理"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={resetAllDialogOpen}
        onOpenChange={(open) => {
          if (!open && resettingAll) return;
          setResetAllDialogOpen(open);
          if (!open) setResettingAll(false);
        }}
        title="确认清理全部信息"
        description="将删除本地数据库与 settings.json，并在完成后退出应用。下次启动会以默认配置重新初始化。此操作不可撤销。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800">
            注意：此操作会清空所有本地数据与配置。完成后应用会自动退出，需要手动重新打开。
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 pt-3">
            <Button
              onClick={() => setResetAllDialogOpen(false)}
              variant="secondary"
              disabled={resettingAll}
            >
              取消
            </Button>
            <Button onClick={resetAllData} variant="danger" disabled={resettingAll}>
              {resettingAll ? "清理中…" : "确认清理并退出"}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
