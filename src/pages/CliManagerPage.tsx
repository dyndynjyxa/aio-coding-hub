// Usage: UI for configuring local CLI integrations and related app settings. Backend commands: `cli_manager_*`, `settings_*`, `cli_proxy_*`, `gateway_*`.

import {
  lazy,
  Suspense,
  useEffect,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import {
  type ClaudeSettingsPatch,
  type CodexConfigPatch,
  type GeminiConfigPatch,
} from "../services/cli/cliManager";
import { cliProxyRebindCodexHome, cliProxyStatusAll } from "../services/cli/cliProxy";
import { logToConsole } from "../services/consoleLog";
import { type GatewayRectifierSettingsPatch } from "../services/settings/settingsGatewayRectifier";
import type { AppSettings } from "../services/settings/settings";
import {
  setCacheAnomalyMonitorEnabled,
  useCacheAnomalyMonitorEnabled,
} from "../services/gateway/cacheAnomalyMonitor";
import {
  setTaskCompleteNotifyEnabled,
  useTaskCompleteNotifyEnabled,
} from "../services/notification/taskCompleteNotifyEvents";
import {
  setNotificationSoundEnabled,
  useNotificationSoundEnabled,
} from "../services/notification/notificationSound";
import {
  getSettingsReadProtection,
  SETTINGS_READONLY_MESSAGE,
  useSettingsCircuitBreakerNoticeSetMutation,
  useSettingsCodexSessionIdCompletionSetMutation,
  useSettingsGatewayRectifierSetMutation,
  useSettingsQuery,
  useSettingsSetMutation,
} from "../query/settings";
import { useProvidersListQuery } from "../query/providers";
import {
  useCliManagerClaudeInfoQuery,
  useCliManagerClaudeSettingsQuery,
  useCliManagerClaudeSettingsSetMutation,
  useCliManagerCodexConfigQuery,
  useCliManagerCodexConfigSetMutation,
  useCliManagerCodexConfigTomlQuery,
  useCliManagerCodexConfigTomlSetMutation,
  useCliManagerCodexInfoQuery,
  useCliManagerGeminiConfigQuery,
  useCliManagerGeminiConfigSetMutation,
  useCliManagerGeminiInfoQuery,
} from "../query/cliManager";
import { cliProxyKeys } from "../query/keys";
import { formatActionFailureToast } from "../utils/errors";
import { CliManagerGeneralTab } from "../components/cli-manager/tabs/GeneralTab";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";

type TabKey = "general" | "claude" | "codex" | "cx2cc" | "gemini";

const TABS: Array<{ key: TabKey; label: string }> = [
  { key: "general", label: "通用" },
  { key: "claude", label: "Claude Code" },
  { key: "codex", label: "Codex" },
  { key: "cx2cc", label: "CX2CC" },
  { key: "gemini", label: "Gemini" },
];

const DEFAULT_RECTIFIER: GatewayRectifierSettingsPatch = {
  verbose_provider_error: true,
  intercept_anthropic_warmup_requests: true,
  enable_thinking_signature_rectifier: true,
  enable_thinking_budget_rectifier: true,
  enable_billing_header_rectifier: true,
  enable_claude_metadata_user_id_injection: true,
  enable_response_fixer: true,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
  response_fixer_max_json_depth: 200,
  response_fixer_max_fix_size: 1024 * 1024,
};

const LazyClaudeTab = lazy(() =>
  import("../components/cli-manager/tabs/ClaudeTab").then((m) => ({
    default: m.CliManagerClaudeTab,
  }))
);

const LazyCodexTab = lazy(() =>
  import("../components/cli-manager/tabs/CodexTab").then((m) => ({
    default: m.CliManagerCodexTab,
  }))
);

const LazyCx2ccTab = lazy(() =>
  import("../components/cli-manager/tabs/Cx2ccTab").then((m) => ({
    default: m.CliManagerCx2ccTab,
  }))
);

const LazyGeminiTab = lazy(() =>
  import("../components/cli-manager/tabs/GeminiTab").then((m) => ({
    default: m.CliManagerGeminiTab,
  }))
);

const TAB_FALLBACK = <div className="p-6 text-sm text-slate-500 dark:text-slate-400">加载中…</div>;

export function CliManagerPage() {
  const [tab, setTab] = useState<TabKey>("general");
  const queryClient = useQueryClient();

  const settingsQuery = useSettingsQuery();
  const appSettings = settingsQuery.data ?? null;
  const { settingsReadErrorMessage, settingsWriteBlocked } =
    getSettingsReadProtection(settingsQuery);

  const rectifierAvailable: "checking" | "available" | "unavailable" = settingsQuery.isLoading
    ? "checking"
    : appSettings
      ? "available"
      : "unavailable";

  const rectifierMutation = useSettingsGatewayRectifierSetMutation();
  const circuitBreakerNoticeMutation = useSettingsCircuitBreakerNoticeSetMutation();
  const codexSessionIdCompletionMutation = useSettingsCodexSessionIdCompletionSetMutation();
  const commonSettingsMutation = useSettingsSetMutation();

  const rectifierSaving = rectifierMutation.isPending;
  const circuitBreakerNoticeSaving = circuitBreakerNoticeMutation.isPending;
  const codexSessionIdCompletionSaving = codexSessionIdCompletionMutation.isPending;
  const commonSettingsSaving = commonSettingsMutation.isPending;

  const [rectifier, setRectifier] = useState<GatewayRectifierSettingsPatch>(DEFAULT_RECTIFIER);
  const [circuitBreakerNoticeEnabled, setCircuitBreakerNoticeEnabled] = useState(false);
  const [codexSessionIdCompletionEnabled, setCodexSessionIdCompletionEnabled] = useState(true);
  const cacheAnomalyMonitorEnabled = useCacheAnomalyMonitorEnabled();
  const taskCompleteNotifyEnabled = useTaskCompleteNotifyEnabled();
  const notificationSoundEnabled = useNotificationSoundEnabled();
  const [upstreamFirstByteTimeoutSeconds, setUpstreamFirstByteTimeoutSeconds] = useState<number>(0);
  const [upstreamStreamIdleTimeoutSeconds, setUpstreamStreamIdleTimeoutSeconds] =
    useState<number>(0);
  const [upstreamRequestTimeoutNonStreamingSeconds, setUpstreamRequestTimeoutNonStreamingSeconds] =
    useState<number>(0);
  const [providerCooldownSeconds, setProviderCooldownSeconds] = useState<number>(30);
  const [providerBaseUrlPingCacheTtlSeconds, setProviderBaseUrlPingCacheTtlSeconds] =
    useState<number>(60);
  const [circuitBreakerFailureThreshold, setCircuitBreakerFailureThreshold] = useState<number>(5);
  const [circuitBreakerOpenDurationMinutes, setCircuitBreakerOpenDurationMinutes] =
    useState<number>(30);

  function blockSettingsWrite() {
    toast(settingsReadErrorMessage ?? SETTINGS_READONLY_MESSAGE);
  }

  const claudeInfoQuery = useCliManagerClaudeInfoQuery({ enabled: tab === "claude" });
  const claudeSettingsQuery = useCliManagerClaudeSettingsQuery({ enabled: tab === "claude" });
  const claudeSettingsSetMutation = useCliManagerClaudeSettingsSetMutation();
  const claudeProvidersQuery = useProvidersListQuery("claude", { enabled: tab === "claude" });

  const claudeInfo = claudeInfoQuery.data ?? null;
  const claudeSettings = claudeSettingsQuery.data ?? null;
  const claudeProviders = claudeProvidersQuery.data ?? null;
  const claudeAvailable: "checking" | "available" | "unavailable" =
    claudeInfoQuery.isFetching && !claudeInfo
      ? "checking"
      : claudeInfo
        ? "available"
        : "unavailable";
  const claudeLoading = claudeInfoQuery.isFetching;
  const claudeSettingsLoading = claudeSettingsQuery.isFetching;
  const claudeSettingsSaving = claudeSettingsSetMutation.isPending;

  const codexInfoQuery = useCliManagerCodexInfoQuery({ enabled: tab === "codex" });
  const codexConfigQuery = useCliManagerCodexConfigQuery({ enabled: tab === "codex" });
  const codexConfigTomlQuery = useCliManagerCodexConfigTomlQuery({ enabled: tab === "codex" });
  const codexConfigSetMutation = useCliManagerCodexConfigSetMutation();
  const codexConfigTomlSetMutation = useCliManagerCodexConfigTomlSetMutation();

  const codexInfo = codexInfoQuery.data ?? null;
  const codexConfig = codexConfigQuery.data ?? null;
  const codexConfigToml = codexConfigTomlQuery.data ?? null;
  const codexAvailable: "checking" | "available" | "unavailable" =
    codexInfoQuery.isFetching && !codexInfo ? "checking" : codexInfo ? "available" : "unavailable";
  const codexLoading = codexInfoQuery.isFetching;
  const codexConfigLoading = codexConfigQuery.isFetching;
  const codexConfigSaving = codexConfigSetMutation.isPending;
  const codexConfigTomlLoading = codexConfigTomlQuery.isFetching;
  const codexConfigTomlSaving = codexConfigTomlSetMutation.isPending;

  const geminiInfoQuery = useCliManagerGeminiInfoQuery({ enabled: tab === "gemini" });
  const geminiConfigQuery = useCliManagerGeminiConfigQuery({ enabled: tab === "gemini" });
  const geminiConfigSetMutation = useCliManagerGeminiConfigSetMutation();
  const geminiInfo = geminiInfoQuery.data ?? null;
  const geminiConfig = geminiConfigQuery.data ?? null;
  const geminiAvailable: "checking" | "available" | "unavailable" =
    geminiInfoQuery.isFetching && !geminiInfo
      ? "checking"
      : geminiInfo
        ? "available"
        : "unavailable";
  const geminiLoading = geminiInfoQuery.isFetching;
  const geminiConfigLoading = geminiConfigQuery.isFetching;
  const geminiConfigSaving = geminiConfigSetMutation.isPending;

  useEffect(() => {
    if (!appSettings) return;
    setRectifier({
      verbose_provider_error: appSettings.verbose_provider_error,
      intercept_anthropic_warmup_requests: appSettings.intercept_anthropic_warmup_requests,
      enable_thinking_signature_rectifier: appSettings.enable_thinking_signature_rectifier,
      enable_thinking_budget_rectifier: appSettings.enable_thinking_budget_rectifier,
      enable_billing_header_rectifier: appSettings.enable_billing_header_rectifier,
      enable_claude_metadata_user_id_injection:
        appSettings.enable_claude_metadata_user_id_injection,
      enable_response_fixer: appSettings.enable_response_fixer,
      response_fixer_fix_encoding: appSettings.response_fixer_fix_encoding,
      response_fixer_fix_sse_format: appSettings.response_fixer_fix_sse_format,
      response_fixer_fix_truncated_json: appSettings.response_fixer_fix_truncated_json,
      response_fixer_max_json_depth: appSettings.response_fixer_max_json_depth,
      response_fixer_max_fix_size: appSettings.response_fixer_max_fix_size,
    });
    setCircuitBreakerNoticeEnabled(appSettings.enable_circuit_breaker_notice ?? false);
    setCodexSessionIdCompletionEnabled(appSettings.enable_codex_session_id_completion ?? true);
    setCacheAnomalyMonitorEnabled(appSettings.enable_cache_anomaly_monitor ?? false);
    setTaskCompleteNotifyEnabled(appSettings.enable_task_complete_notify ?? true);
    setNotificationSoundEnabled(appSettings.enable_notification_sound ?? true);
    setUpstreamFirstByteTimeoutSeconds(appSettings.upstream_first_byte_timeout_seconds);
    setUpstreamStreamIdleTimeoutSeconds(appSettings.upstream_stream_idle_timeout_seconds);
    setUpstreamRequestTimeoutNonStreamingSeconds(
      appSettings.upstream_request_timeout_non_streaming_seconds
    );
    setProviderCooldownSeconds(appSettings.provider_cooldown_seconds);
    setProviderBaseUrlPingCacheTtlSeconds(appSettings.provider_base_url_ping_cache_ttl_seconds);
    setCircuitBreakerFailureThreshold(appSettings.circuit_breaker_failure_threshold);
    setCircuitBreakerOpenDurationMinutes(appSettings.circuit_breaker_open_duration_minutes);
  }, [appSettings]);

  async function persistRectifier(patch: Partial<GatewayRectifierSettingsPatch>) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (rectifierSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = rectifier;
    const next = { ...prev, ...patch };
    setRectifier(next);
    try {
      const updated = await rectifierMutation.mutateAsync(next);
      if (!updated) {
        setRectifier(prev);
        return;
      }

      setRectifier({
        verbose_provider_error: updated.verbose_provider_error,
        intercept_anthropic_warmup_requests: updated.intercept_anthropic_warmup_requests,
        enable_thinking_signature_rectifier: updated.enable_thinking_signature_rectifier,
        enable_thinking_budget_rectifier: updated.enable_thinking_budget_rectifier,
        enable_billing_header_rectifier: updated.enable_billing_header_rectifier,
        enable_claude_metadata_user_id_injection: updated.enable_claude_metadata_user_id_injection,
        enable_response_fixer: updated.enable_response_fixer,
        response_fixer_fix_encoding: updated.response_fixer_fix_encoding,
        response_fixer_fix_sse_format: updated.response_fixer_fix_sse_format,
        response_fixer_fix_truncated_json: updated.response_fixer_fix_truncated_json,
        response_fixer_max_json_depth: updated.response_fixer_max_json_depth,
        response_fixer_max_fix_size: updated.response_fixer_max_fix_size,
      });
    } catch (err) {
      logToConsole("error", "更新网关整流配置失败", { error: String(err) });
      toast("更新网关整流配置失败：请稍后重试");
      setRectifier(prev);
    }
  }

  async function persistCircuitBreakerNotice(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (circuitBreakerNoticeSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = circuitBreakerNoticeEnabled;
    setCircuitBreakerNoticeEnabled(enable);
    try {
      const updated = await circuitBreakerNoticeMutation.mutateAsync(enable);
      if (!updated) {
        setCircuitBreakerNoticeEnabled(prev);
        return;
      }

      setCircuitBreakerNoticeEnabled(updated.enable_circuit_breaker_notice ?? enable);
      toast(enable ? "已开启熔断通知" : "已关闭熔断通知");
    } catch (err) {
      logToConsole("error", "更新熔断通知配置失败", { error: String(err) });
      toast("更新熔断通知配置失败：请稍后重试");
      setCircuitBreakerNoticeEnabled(prev);
    }
  }

  async function persistCodexSessionIdCompletion(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (codexSessionIdCompletionSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = codexSessionIdCompletionEnabled;
    setCodexSessionIdCompletionEnabled(enable);
    try {
      const updated = await codexSessionIdCompletionMutation.mutateAsync(enable);
      if (!updated) {
        setCodexSessionIdCompletionEnabled(prev);
        return;
      }

      setCodexSessionIdCompletionEnabled(updated.enable_codex_session_id_completion ?? enable);
      toast(enable ? "已开启 Codex Session ID 补全" : "已关闭 Codex Session ID 补全");
    } catch (err) {
      logToConsole("error", "更新 Codex Session ID 补全配置失败", { error: String(err) });
      toast("更新 Codex Session ID 补全配置失败：请稍后重试");
      setCodexSessionIdCompletionEnabled(prev);
    }
  }

  async function persistCacheAnomalyMonitor(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (commonSettingsSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = cacheAnomalyMonitorEnabled;
    setCacheAnomalyMonitorEnabled(enable);
    try {
      const updated = await persistCommonSettings({ enable_cache_anomaly_monitor: enable });
      if (!updated) {
        setCacheAnomalyMonitorEnabled(prev);
        return;
      }

      const next = updated.enable_cache_anomaly_monitor ?? enable;
      setCacheAnomalyMonitorEnabled(next);
      toast(next ? "已开启缓存异常监测（实验）" : "已关闭缓存异常监测（实验）");
    } catch {
      setCacheAnomalyMonitorEnabled(prev);
    }
  }

  async function persistTaskCompleteNotify(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (commonSettingsSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = taskCompleteNotifyEnabled;
    setTaskCompleteNotifyEnabled(enable);
    try {
      const updated = await persistCommonSettings({ enable_task_complete_notify: enable });
      if (!updated) {
        setTaskCompleteNotifyEnabled(prev);
        return;
      }

      const next = updated.enable_task_complete_notify ?? enable;
      setTaskCompleteNotifyEnabled(next);
      toast(next ? "已开启任务结束提醒" : "已关闭任务结束提醒");
    } catch {
      setTaskCompleteNotifyEnabled(prev);
    }
  }

  async function persistNotificationSound(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (commonSettingsSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = notificationSoundEnabled;
    setNotificationSoundEnabled(enable);
    try {
      const updated = await persistCommonSettings({ enable_notification_sound: enable });
      if (!updated) {
        setNotificationSoundEnabled(prev);
        return;
      }

      const next = updated.enable_notification_sound ?? enable;
      setNotificationSoundEnabled(next);
      toast(next ? "已开启通知音效" : "已关闭通知音效");
    } catch {
      setNotificationSoundEnabled(prev);
    }
  }

  async function persistCommonSettings(patch: Partial<AppSettings>): Promise<AppSettings | null> {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return null;
    }
    if (commonSettingsSaving) return null;
    if (rectifierAvailable !== "available") return null;
    if (!appSettings) return null;

    const prev = appSettings;
    const next: AppSettings = { ...prev, ...patch };
    try {
      const updated = await commonSettingsMutation.mutateAsync({
        preferredPort: next.preferred_port,
        gatewayListenMode: next.gateway_listen_mode,
        gatewayCustomListenAddress: next.gateway_custom_listen_address,
        autoStart: next.auto_start,
        trayEnabled: next.tray_enabled,
        enableCliProxyStartupRecovery: next.enable_cli_proxy_startup_recovery,
        logRetentionDays: next.log_retention_days,
        providerCooldownSeconds: next.provider_cooldown_seconds,
        providerBaseUrlPingCacheTtlSeconds: next.provider_base_url_ping_cache_ttl_seconds,
        upstreamFirstByteTimeoutSeconds: next.upstream_first_byte_timeout_seconds,
        upstreamStreamIdleTimeoutSeconds: next.upstream_stream_idle_timeout_seconds,
        upstreamRequestTimeoutNonStreamingSeconds:
          next.upstream_request_timeout_non_streaming_seconds,
        enableCacheAnomalyMonitor: next.enable_cache_anomaly_monitor,
        enableTaskCompleteNotify: next.enable_task_complete_notify,
        enableNotificationSound: next.enable_notification_sound,
        failoverMaxAttemptsPerProvider: next.failover_max_attempts_per_provider,
        failoverMaxProvidersToTry: next.failover_max_providers_to_try,
        circuitBreakerFailureThreshold: next.circuit_breaker_failure_threshold,
        circuitBreakerOpenDurationMinutes: next.circuit_breaker_open_duration_minutes,
        wslAutoConfig: next.wsl_auto_config,
        wslTargetCli: next.wsl_target_cli,
        codexHomeMode: next.codex_home_mode,
        codexHomeOverride: next.codex_home_override,
        cx2ccFallbackModelOpus: next.cx2cc_fallback_model_opus,
        cx2ccFallbackModelSonnet: next.cx2cc_fallback_model_sonnet,
        cx2ccFallbackModelHaiku: next.cx2cc_fallback_model_haiku,
        cx2ccFallbackModelMain: next.cx2cc_fallback_model_main,
        cx2ccModelReasoningEffort: next.cx2cc_model_reasoning_effort,
        cx2ccServiceTier: next.cx2cc_service_tier,
        cx2ccDisableResponseStorage: next.cx2cc_disable_response_storage,
        cx2ccEnableReasoningToThinking: next.cx2cc_enable_reasoning_to_thinking,
        cx2ccDropStopSequences: next.cx2cc_drop_stop_sequences,
        cx2ccCleanSchema: next.cx2cc_clean_schema,
        cx2ccFilterBatchTool: next.cx2cc_filter_batch_tool,
      });

      if (!updated) {
        return null;
      }

      setUpstreamFirstByteTimeoutSeconds(updated.upstream_first_byte_timeout_seconds);
      setUpstreamStreamIdleTimeoutSeconds(updated.upstream_stream_idle_timeout_seconds);
      setUpstreamRequestTimeoutNonStreamingSeconds(
        updated.upstream_request_timeout_non_streaming_seconds
      );
      setProviderCooldownSeconds(updated.provider_cooldown_seconds);
      setProviderBaseUrlPingCacheTtlSeconds(updated.provider_base_url_ping_cache_ttl_seconds);
      setCircuitBreakerFailureThreshold(updated.circuit_breaker_failure_threshold);
      setCircuitBreakerOpenDurationMinutes(updated.circuit_breaker_open_duration_minutes);
      toast("已保存");
      return updated;
    } catch (err) {
      logToConsole("error", "更新通用网关参数失败", { error: String(err) });
      toast("更新通用网关参数失败：请稍后重试");
      setUpstreamFirstByteTimeoutSeconds(prev.upstream_first_byte_timeout_seconds);
      setUpstreamStreamIdleTimeoutSeconds(prev.upstream_stream_idle_timeout_seconds);
      setUpstreamRequestTimeoutNonStreamingSeconds(
        prev.upstream_request_timeout_non_streaming_seconds
      );
      setProviderCooldownSeconds(prev.provider_cooldown_seconds);
      setProviderBaseUrlPingCacheTtlSeconds(prev.provider_base_url_ping_cache_ttl_seconds);
      setCircuitBreakerFailureThreshold(prev.circuit_breaker_failure_threshold);
      setCircuitBreakerOpenDurationMinutes(prev.circuit_breaker_open_duration_minutes);
      return null;
    }
  }

  async function refreshClaude() {
    await Promise.all([claudeSettingsQuery.refetch(), claudeInfoQuery.refetch()]);
  }

  async function refreshCodex() {
    await Promise.all([
      codexConfigQuery.refetch(),
      codexConfigTomlQuery.refetch(),
      codexInfoQuery.refetch(),
    ]);
  }

  async function refreshGeminiInfo() {
    await Promise.all([geminiInfoQuery.refetch(), geminiConfigQuery.refetch()]);
  }

  async function persistGeminiConfig(patch: GeminiConfigPatch) {
    if (geminiConfigSaving) return;
    if (geminiAvailable !== "available") return;

    try {
      const updated = await geminiConfigSetMutation.mutateAsync(patch);
      if (!updated) {
        return;
      }
      toast("已更新 Gemini 配置");
    } catch (err) {
      const formatted = formatActionFailureToast("更新 Gemini 配置", err);
      logToConsole("error", "更新 Gemini 配置失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        patch,
      });
      toast(formatted.toast);
    }
  }

  async function repairCodexProxyAfterCodexHomeChange() {
    const codexProxyEnabled = await cliProxyStatusAll()
      .then((rows) => rows?.some((row) => row.cli_key === "codex" && row.enabled) ?? false)
      .catch((err) => {
        logToConsole("warn", "读取 CLI 代理状态失败，将直接尝试重绑 Codex 代理", {
          error: String(err),
        });
        return null;
      });

    try {
      if (codexProxyEnabled === false) {
        return false;
      }

      const codexResult = await cliProxyRebindCodexHome();

      if (!codexResult) {
        logToConsole("warn", "Codex Home 已切换，但 Codex 代理重绑未返回结果", {});
        toast("Codex 目录已切换，但代理重绑未返回结果；可稍后在首页点击“修复”重试");
        return false;
      }

      if (!codexResult.ok) {
        logToConsole("warn", "Codex Home 已切换，但 Codex 代理重绑失败", {
          error_code: codexResult.error_code ?? undefined,
          message: codexResult.message,
        });
        toast("Codex 目录已切换，但代理重绑失败；可稍后在首页点击“修复”重试");
        return false;
      }

      logToConsole("info", "Codex Home 已切换，已触发 Codex 代理重绑", {
        base_origin: codexResult.base_origin ?? undefined,
        trace_id: codexResult.trace_id,
        message: codexResult.message,
      });
      return true;
    } catch (err) {
      logToConsole("warn", "Codex Home 已切换，但 Codex 代理重绑失败", {
        error: String(err),
      });
      toast("Codex 目录已切换，但代理重绑失败；可稍后在首页点击“修复”重试");
      return false;
    } finally {
      void queryClient.invalidateQueries({ queryKey: cliProxyKeys.statusAll() });
    }
  }

  async function persistCodexHomeSettings(
    codexHomeMode: AppSettings["codex_home_mode"],
    codexHomeOverride: string
  ) {
    const updated = await persistCommonSettings({
      codex_home_mode: codexHomeMode,
      codex_home_override: codexHomeOverride,
    });
    if (!updated) {
      return false;
    }

    await refreshCodex();
    const rebound = await repairCodexProxyAfterCodexHomeChange();
    if (rebound) {
      await refreshCodex();
    }
    return true;
  }

  async function pickCodexHomeDirectory(initialPath?: string): Promise<string | null> {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "选择 Codex .codex 目录",
        defaultPath:
          initialPath ||
          codexConfig?.user_home_default_dir ||
          codexConfig?.follow_codex_home_dir ||
          codexConfig?.config_dir ||
          undefined,
      });

      if (!selected) {
        return null;
      }

      return Array.isArray(selected) ? (selected[0] ?? null) : selected;
    } catch (err) {
      logToConsole("error", "打开 Codex 目录选择器失败", { error: String(err) });
      toast("打开目录选择器失败：请稍后重试");
      return null;
    }
  }

  async function persistCodexConfig(patch: CodexConfigPatch) {
    if (codexConfigSaving) return;
    if (codexAvailable !== "available") return;

    try {
      const updated = await codexConfigSetMutation.mutateAsync(patch);
      if (!updated) {
        return;
      }
      toast("已更新 Codex 配置");
    } catch (err) {
      const formatted = formatActionFailureToast("更新 Codex 配置", err);
      logToConsole("error", "更新 Codex 配置失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        patch,
      });
      toast(formatted.toast);
    }
  }

  async function persistCodexConfigToml(toml: string): Promise<boolean> {
    if (codexConfigTomlSaving) return false;
    if (codexAvailable !== "available") return false;

    try {
      const updated = await codexConfigTomlSetMutation.mutateAsync({ toml });
      if (!updated) {
        return false;
      }
      toast("已保存 config.toml");
      return true;
    } catch (err) {
      const formatted = formatActionFailureToast("保存 config.toml", err);
      logToConsole("error", "保存 Codex config.toml 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
      });
      toast(formatted.toast);
      return false;
    }
  }

  async function persistClaudeSettings(patch: ClaudeSettingsPatch) {
    if (claudeSettingsSaving) return;
    if (claudeAvailable !== "available") return;

    try {
      const updated = await claudeSettingsSetMutation.mutateAsync(patch);
      if (!updated) {
        return;
      }
      toast("已更新 Claude Code 配置");
    } catch (err) {
      logToConsole("error", "更新 Claude Code settings.json 失败", { error: String(err) });
      toast("更新 Claude Code 配置失败：请稍后重试");
    }
  }

  async function openClaudeConfigDir() {
    const dir = claudeInfo?.config_dir ?? claudeSettings?.config_dir;
    if (!dir) return;
    try {
      await openPath(dir);
    } catch (err) {
      logToConsole("error", "打开 Claude 配置目录失败", { error: String(err) });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  async function openCodexConfigDir() {
    if (!codexConfig) return;
    if (!codexConfig.can_open_config_dir) {
      toast("受权限限制，无法自动打开该目录");
      return;
    }
    try {
      await openPath(codexConfig.config_dir);
    } catch (err) {
      logToConsole("error", "打开 Codex 配置目录失败", { error: String(err) });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") e.currentTarget.blur();
  }

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader
        title="CLI 管理"
        actions={
          <TabList ariaLabel="CLI 管理视图切换" items={TABS} value={tab} onChange={setTab} />
        }
      />

      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay">
        {tab === "general" ? (
          <CliManagerGeneralTab
            rectifierAvailable={rectifierAvailable}
            settingsReadErrorMessage={settingsReadErrorMessage}
            settingsWriteBlocked={settingsWriteBlocked}
            rectifierSaving={rectifierSaving}
            rectifier={rectifier}
            onPersistRectifier={persistRectifier}
            circuitBreakerNoticeEnabled={circuitBreakerNoticeEnabled}
            circuitBreakerNoticeSaving={circuitBreakerNoticeSaving}
            onPersistCircuitBreakerNotice={persistCircuitBreakerNotice}
            codexSessionIdCompletionEnabled={codexSessionIdCompletionEnabled}
            codexSessionIdCompletionSaving={codexSessionIdCompletionSaving}
            onPersistCodexSessionIdCompletion={persistCodexSessionIdCompletion}
            cacheAnomalyMonitorEnabled={cacheAnomalyMonitorEnabled}
            cacheAnomalyMonitorSaving={commonSettingsSaving || settingsWriteBlocked}
            onPersistCacheAnomalyMonitor={persistCacheAnomalyMonitor}
            taskCompleteNotifyEnabled={taskCompleteNotifyEnabled}
            taskCompleteNotifySaving={commonSettingsSaving || settingsWriteBlocked}
            onPersistTaskCompleteNotify={persistTaskCompleteNotify}
            notificationSoundEnabled={notificationSoundEnabled}
            notificationSoundSaving={commonSettingsSaving || settingsWriteBlocked}
            onPersistNotificationSound={persistNotificationSound}
            appSettings={appSettings}
            commonSettingsSaving={commonSettingsSaving || settingsWriteBlocked}
            onPersistCommonSettings={persistCommonSettings}
            upstreamFirstByteTimeoutSeconds={upstreamFirstByteTimeoutSeconds}
            setUpstreamFirstByteTimeoutSeconds={setUpstreamFirstByteTimeoutSeconds}
            upstreamStreamIdleTimeoutSeconds={upstreamStreamIdleTimeoutSeconds}
            setUpstreamStreamIdleTimeoutSeconds={setUpstreamStreamIdleTimeoutSeconds}
            upstreamRequestTimeoutNonStreamingSeconds={upstreamRequestTimeoutNonStreamingSeconds}
            setUpstreamRequestTimeoutNonStreamingSeconds={
              setUpstreamRequestTimeoutNonStreamingSeconds
            }
            providerCooldownSeconds={providerCooldownSeconds}
            setProviderCooldownSeconds={setProviderCooldownSeconds}
            providerBaseUrlPingCacheTtlSeconds={providerBaseUrlPingCacheTtlSeconds}
            setProviderBaseUrlPingCacheTtlSeconds={setProviderBaseUrlPingCacheTtlSeconds}
            circuitBreakerFailureThreshold={circuitBreakerFailureThreshold}
            setCircuitBreakerFailureThreshold={setCircuitBreakerFailureThreshold}
            circuitBreakerOpenDurationMinutes={circuitBreakerOpenDurationMinutes}
            setCircuitBreakerOpenDurationMinutes={setCircuitBreakerOpenDurationMinutes}
            blurOnEnter={blurOnEnter}
          />
        ) : null}

        {tab === "claude" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyClaudeTab
              claudeAvailable={claudeAvailable}
              claudeLoading={claudeLoading}
              claudeInfo={claudeInfo}
              claudeSettingsLoading={claudeSettingsLoading}
              claudeSettingsSaving={claudeSettingsSaving}
              claudeSettings={claudeSettings}
              providers={claudeProviders}
              refreshClaude={refreshClaude}
              openClaudeConfigDir={openClaudeConfigDir}
              persistClaudeSettings={persistClaudeSettings}
            />
          </Suspense>
        ) : null}

        {tab === "codex" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyCodexTab
              codexAvailable={codexAvailable}
              codexLoading={codexLoading}
              codexConfigLoading={codexConfigLoading}
              codexConfigSaving={codexConfigSaving}
              codexConfigTomlLoading={codexConfigTomlLoading}
              codexConfigTomlSaving={codexConfigTomlSaving}
              codexInfo={codexInfo}
              codexConfig={codexConfig}
              codexConfigToml={codexConfigToml}
              appSettings={appSettings}
              codexHomeSettingsSaving={commonSettingsSaving || settingsWriteBlocked}
              refreshCodex={refreshCodex}
              openCodexConfigDir={openCodexConfigDir}
              persistCodexConfig={persistCodexConfig}
              persistCodexConfigToml={persistCodexConfigToml}
              persistCodexHomeSettings={persistCodexHomeSettings}
              pickCodexHomeDirectory={pickCodexHomeDirectory}
            />
          </Suspense>
        ) : null}

        {tab === "cx2cc" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyCx2ccTab
              appSettings={appSettings}
              commonSettingsSaving={commonSettingsSaving}
              onPersistCommonSettings={persistCommonSettings}
            />
          </Suspense>
        ) : null}

        {tab === "gemini" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyGeminiTab
              geminiAvailable={geminiAvailable}
              geminiLoading={geminiLoading}
              geminiInfo={geminiInfo}
              geminiConfigLoading={geminiConfigLoading}
              geminiConfigSaving={geminiConfigSaving}
              geminiConfig={geminiConfig}
              refreshGeminiInfo={refreshGeminiInfo}
              persistGeminiConfig={persistGeminiConfig}
            />
          </Suspense>
        ) : null}
      </div>
    </div>
  );
}
