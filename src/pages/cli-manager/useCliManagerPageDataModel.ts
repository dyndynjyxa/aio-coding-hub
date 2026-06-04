// Usage: Data-model hook for CLI manager page orchestration.

import { useEffect, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { toast } from "sonner";
import {
  type ClaudeSettingsPatch,
  type CodexConfigPatch,
  type GeminiConfigPatch,
} from "../../services/cli/cliManager";
import { logToConsole } from "../../services/consoleLog";
import { openDesktopSinglePath } from "../../services/desktop/dialog";
import { openDesktopPath } from "../../services/desktop/opener";
import { type GatewayRectifierSettingsPatch } from "../../services/settings/settingsGatewayRectifier";
import type { AppSettings, SensitiveStringUpdate } from "../../services/settings/settings";
import {
  getSettingsReadProtection,
  SETTINGS_READONLY_MESSAGE,
  useSettingsCircuitBreakerNoticeSetMutation,
  useSettingsCodexSessionIdCompletionSetMutation,
  useSettingsGatewayRectifierSetMutation,
  useSettingsPatchMutation,
  useSettingsQuery,
} from "../../query/settings";
import { useProvidersListQuery } from "../../query/providers";
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
} from "../../query/cliManager";
import { formatActionFailureToast } from "../../utils/errors";

export type CliManagerTabKey = "general" | "claude" | "codex" | "cx2cc" | "gemini";

export const CLI_MANAGER_TABS: Array<{ key: CliManagerTabKey; label: string }> = [
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

export function useCliManagerPageDataModel() {
  const [tab, setTab] = useState<CliManagerTabKey>("general");

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
  const commonSettingsMutation = useSettingsPatchMutation();

  const rectifierSaving = rectifierMutation.isPending;
  const circuitBreakerNoticeSaving = circuitBreakerNoticeMutation.isPending;
  const codexSessionIdCompletionSaving = codexSessionIdCompletionMutation.isPending;
  const commonSettingsSaving = commonSettingsMutation.isPending;

  const [rectifier, setRectifier] = useState<GatewayRectifierSettingsPatch>(DEFAULT_RECTIFIER);
  const [circuitBreakerNoticeEnabled, setCircuitBreakerNoticeEnabled] = useState(false);
  const [codexSessionIdCompletionEnabled, setCodexSessionIdCompletionEnabled] = useState(true);
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
  const cacheAnomalyMonitorEnabled = appSettings?.enable_cache_anomaly_monitor ?? false;
  const taskCompleteNotifyEnabled = appSettings?.enable_task_complete_notify ?? true;
  const notificationSoundEnabled = appSettings?.enable_notification_sound ?? true;

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

    try {
      const updated = await persistCommonSettings({ enable_cache_anomaly_monitor: enable });
      if (!updated) return;
      const next = updated.enable_cache_anomaly_monitor ?? enable;
      toast(next ? "已开启缓存异常监测（实验）" : "已关闭缓存异常监测（实验）");
    } catch {}
  }

  async function persistTaskCompleteNotify(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (commonSettingsSaving) return;
    if (rectifierAvailable !== "available") return;

    try {
      const updated = await persistCommonSettings({ enable_task_complete_notify: enable });
      if (!updated) return;
      const next = updated.enable_task_complete_notify ?? enable;
      toast(next ? "已开启任务结束提醒" : "已关闭任务结束提醒");
    } catch {}
  }

  async function persistNotificationSound(enable: boolean) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (commonSettingsSaving) return;
    if (rectifierAvailable !== "available") return;

    try {
      const updated = await persistCommonSettings({ enable_notification_sound: enable });
      if (!updated) return;
      const next = updated.enable_notification_sound ?? enable;
      toast(next ? "已开启通知音效" : "已关闭通知音效");
    } catch {}
  }

  async function persistCommonSettings(
    patch: Partial<AppSettings> & { upstream_proxy_password?: SensitiveStringUpdate }
  ): Promise<AppSettings | null> {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return null;
    }
    if (commonSettingsSaving) return null;
    if (rectifierAvailable !== "available") return null;
    if (!appSettings) return null;

    const prev = appSettings;
    try {
      const updated = await commonSettingsMutation.mutateAsync({
        ...patch,
        upstream_proxy_password: patch.upstream_proxy_password ?? { mode: "preserve" },
      });

      if (!updated) {
        return null;
      }
      const updatedSettings = updated.settings;

      setUpstreamFirstByteTimeoutSeconds(updatedSettings.upstream_first_byte_timeout_seconds);
      setUpstreamStreamIdleTimeoutSeconds(updatedSettings.upstream_stream_idle_timeout_seconds);
      setUpstreamRequestTimeoutNonStreamingSeconds(
        updatedSettings.upstream_request_timeout_non_streaming_seconds
      );
      setProviderCooldownSeconds(updatedSettings.provider_cooldown_seconds);
      setProviderBaseUrlPingCacheTtlSeconds(
        updatedSettings.provider_base_url_ping_cache_ttl_seconds
      );
      setCircuitBreakerFailureThreshold(updatedSettings.circuit_breaker_failure_threshold);
      setCircuitBreakerOpenDurationMinutes(updatedSettings.circuit_breaker_open_duration_minutes);
      toast("已保存");
      return updatedSettings;
    } catch (err) {
      const formatted = formatActionFailureToast("更新通用网关参数", err);
      logToConsole("error", "更新通用网关参数失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
      });
      toast(formatted.toast);
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
    toast("Codex 目录已切换");
    return true;
  }

  async function persistCodexOauthCompatibleProxyMode(enabled: boolean) {
    const updated = await persistCommonSettings({
      codex_oauth_compatible_proxy_mode: enabled,
    });
    if (!updated) {
      return false;
    }

    await refreshCodex();
    toast(enabled ? "已开启 Codex OAuth 兼容代理模式" : "已关闭 Codex OAuth 兼容代理模式");
    return true;
  }

  async function pickCodexHomeDirectory(initialPath?: string): Promise<string | null> {
    try {
      return await openDesktopSinglePath({
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
      await openDesktopPath(dir);
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
      await openDesktopPath(codexConfig.config_dir);
    } catch (err) {
      logToConsole("error", "打开 Codex 配置目录失败", { error: String(err) });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") e.currentTarget.blur();
  }

  return {
    tab,
    setTab,
    generalTabProps: {
      rectifierAvailable,
      settingsReadErrorMessage,
      settingsWriteBlocked,
      rectifierSaving,
      rectifier,
      onPersistRectifier: persistRectifier,
      circuitBreakerNoticeEnabled,
      circuitBreakerNoticeSaving,
      onPersistCircuitBreakerNotice: persistCircuitBreakerNotice,
      codexSessionIdCompletionEnabled,
      codexSessionIdCompletionSaving,
      onPersistCodexSessionIdCompletion: persistCodexSessionIdCompletion,
      cacheAnomalyMonitorEnabled,
      cacheAnomalyMonitorSaving: commonSettingsSaving || settingsWriteBlocked,
      onPersistCacheAnomalyMonitor: persistCacheAnomalyMonitor,
      taskCompleteNotifyEnabled,
      taskCompleteNotifySaving: commonSettingsSaving || settingsWriteBlocked,
      onPersistTaskCompleteNotify: persistTaskCompleteNotify,
      notificationSoundEnabled,
      notificationSoundSaving: commonSettingsSaving || settingsWriteBlocked,
      onPersistNotificationSound: persistNotificationSound,
      appSettings,
      commonSettingsSaving: commonSettingsSaving || settingsWriteBlocked,
      onPersistCommonSettings: persistCommonSettings,
      upstreamFirstByteTimeoutSeconds,
      setUpstreamFirstByteTimeoutSeconds,
      upstreamStreamIdleTimeoutSeconds,
      setUpstreamStreamIdleTimeoutSeconds,
      upstreamRequestTimeoutNonStreamingSeconds,
      setUpstreamRequestTimeoutNonStreamingSeconds,
      providerCooldownSeconds,
      setProviderCooldownSeconds,
      providerBaseUrlPingCacheTtlSeconds,
      setProviderBaseUrlPingCacheTtlSeconds,
      circuitBreakerFailureThreshold,
      setCircuitBreakerFailureThreshold,
      circuitBreakerOpenDurationMinutes,
      setCircuitBreakerOpenDurationMinutes,
      blurOnEnter,
    },
    claudeTabProps: {
      claudeAvailable,
      claudeLoading,
      claudeInfo,
      claudeSettingsLoading,
      claudeSettingsSaving,
      claudeSettings,
      providers: claudeProviders,
      refreshClaude,
      openClaudeConfigDir,
      persistClaudeSettings,
    },
    codexTabProps: {
      codexAvailable,
      codexLoading,
      codexConfigLoading,
      codexConfigSaving,
      codexConfigTomlLoading,
      codexConfigTomlSaving,
      codexInfo,
      codexConfig,
      codexConfigToml,
      appSettings,
      codexHomeSettingsSaving: commonSettingsSaving || settingsWriteBlocked,
      refreshCodex,
      openCodexConfigDir,
      persistCodexConfig,
      persistCodexConfigToml,
      persistCodexHomeSettings,
      persistCodexOauthCompatibleProxyMode,
      pickCodexHomeDirectory,
    },
    cx2ccTabProps: {
      appSettings,
      commonSettingsSaving,
      onPersistCommonSettings: persistCommonSettings,
    },
    geminiTabProps: {
      geminiAvailable,
      geminiLoading,
      geminiInfo,
      geminiConfigLoading,
      geminiConfigSaving,
      geminiConfig,
      refreshGeminiInfo,
      persistGeminiConfig,
    },
  };
}
