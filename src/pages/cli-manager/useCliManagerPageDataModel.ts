// Usage: Data-model hook for CLI manager page orchestration.

import { useEffect, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { toast } from "sonner";
import {
  type ClaudeSettingsPatch,
  type CodexConfigPatch,
  type CodexConfigState,
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
  useCliManagerCodexModelCatalogQuery,
  useCliManagerCodexModelCatalogRefresh,
  useCliManagerGeminiConfigQuery,
  useCliManagerGeminiConfigSetMutation,
  useCliManagerGeminiInfoQuery,
} from "../../query/cliManager";
import { formatActionFailureToast } from "../../utils/errors";
import { useGrokTabDataModel } from "../../components/cli-manager/tabs/useGrokTabDataModel";

export type CliManagerTabKey = "general" | "claude" | "codex" | "cx2cc" | "gemini" | "grok";

export const CLI_MANAGER_TABS: Array<{ key: CliManagerTabKey; label: string }> = [
  { key: "general", label: "通用" },
  { key: "claude", label: "Claude Code" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
  { key: "grok", label: "Grok" },
  { key: "cx2cc", label: "CX2CC" },
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

type GeneralSettingsDraft = {
  rectifier: GatewayRectifierSettingsPatch;
  circuitBreakerNoticeEnabled: boolean;
  codexSessionIdCompletionEnabled: boolean;
  upstreamFirstByteTimeoutSeconds: number;
  upstreamStreamIdleTimeoutSeconds: number;
  upstreamRequestTimeoutNonStreamingSeconds: number;
  providerCooldownSeconds: number;
  providerBaseUrlPingCacheTtlSeconds: number;
  circuitBreakerFailureThreshold: number;
  circuitBreakerOpenDurationMinutes: number;
};

const DEFAULT_GENERAL_SETTINGS_DRAFT: GeneralSettingsDraft = {
  rectifier: DEFAULT_RECTIFIER,
  circuitBreakerNoticeEnabled: false,
  codexSessionIdCompletionEnabled: true,
  upstreamFirstByteTimeoutSeconds: 0,
  upstreamStreamIdleTimeoutSeconds: 0,
  upstreamRequestTimeoutNonStreamingSeconds: 0,
  providerCooldownSeconds: 30,
  providerBaseUrlPingCacheTtlSeconds: 60,
  circuitBreakerFailureThreshold: 5,
  circuitBreakerOpenDurationMinutes: 30,
};

function appSettingsToGeneralSettingsDraft(appSettings: AppSettings): GeneralSettingsDraft {
  return {
    rectifier: {
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
    },
    circuitBreakerNoticeEnabled: appSettings.enable_circuit_breaker_notice ?? false,
    codexSessionIdCompletionEnabled: appSettings.enable_codex_session_id_completion ?? true,
    upstreamFirstByteTimeoutSeconds: appSettings.upstream_first_byte_timeout_seconds,
    upstreamStreamIdleTimeoutSeconds: appSettings.upstream_stream_idle_timeout_seconds,
    upstreamRequestTimeoutNonStreamingSeconds:
      appSettings.upstream_request_timeout_non_streaming_seconds,
    providerCooldownSeconds: appSettings.provider_cooldown_seconds,
    providerBaseUrlPingCacheTtlSeconds: appSettings.provider_base_url_ping_cache_ttl_seconds,
    circuitBreakerFailureThreshold: appSettings.circuit_breaker_failure_threshold,
    circuitBreakerOpenDurationMinutes: appSettings.circuit_breaker_open_duration_minutes,
  };
}

function generalSettingsDraftPatchFromAppSettings(
  appSettings: AppSettings
): Pick<
  GeneralSettingsDraft,
  | "upstreamFirstByteTimeoutSeconds"
  | "upstreamStreamIdleTimeoutSeconds"
  | "upstreamRequestTimeoutNonStreamingSeconds"
  | "providerCooldownSeconds"
  | "providerBaseUrlPingCacheTtlSeconds"
  | "circuitBreakerFailureThreshold"
  | "circuitBreakerOpenDurationMinutes"
> {
  return {
    upstreamFirstByteTimeoutSeconds: appSettings.upstream_first_byte_timeout_seconds,
    upstreamStreamIdleTimeoutSeconds: appSettings.upstream_stream_idle_timeout_seconds,
    upstreamRequestTimeoutNonStreamingSeconds:
      appSettings.upstream_request_timeout_non_streaming_seconds,
    providerCooldownSeconds: appSettings.provider_cooldown_seconds,
    providerBaseUrlPingCacheTtlSeconds: appSettings.provider_base_url_ping_cache_ttl_seconds,
    circuitBreakerFailureThreshold: appSettings.circuit_breaker_failure_threshold,
    circuitBreakerOpenDurationMinutes: appSettings.circuit_breaker_open_duration_minutes,
  };
}

function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
  if (e.key === "Enter") e.currentTarget.blur();
}

export function useCliManagerPageDataModel() {
  const [tab, setTab] = useState<CliManagerTabKey>("general");
  const grokTabProps = useGrokTabDataModel({ enabled: tab === "grok" });

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

  const [generalSettingsDraft, setGeneralSettingsDraft] = useState<GeneralSettingsDraft>(
    DEFAULT_GENERAL_SETTINGS_DRAFT
  );
  const {
    rectifier,
    circuitBreakerNoticeEnabled,
    codexSessionIdCompletionEnabled,
    upstreamFirstByteTimeoutSeconds,
    upstreamStreamIdleTimeoutSeconds,
    upstreamRequestTimeoutNonStreamingSeconds,
    providerCooldownSeconds,
    providerBaseUrlPingCacheTtlSeconds,
    circuitBreakerFailureThreshold,
    circuitBreakerOpenDurationMinutes,
  } = generalSettingsDraft;
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
  const refreshCodexModelCatalog = useCliManagerCodexModelCatalogRefresh();
  const codexModelCatalogQuery = useCliManagerCodexModelCatalogQuery({
    enabled:
      tab === "codex" && codexInfoQuery.data?.found === true && codexConfigQuery.data != null,
    snapshot: {
      configPath: codexConfigQuery.data?.config_path,
      executablePath: codexInfoQuery.data?.executable_path,
      cliVersion: codexInfoQuery.data?.version,
    },
  });

  const codexInfo = codexInfoQuery.data ?? null;
  const codexConfig = codexConfigQuery.data ?? null;
  const codexConfigToml = codexConfigTomlQuery.data ?? null;
  const codexModelCatalog = codexModelCatalogQuery.isError
    ? null
    : (codexModelCatalogQuery.data ?? null);
  const codexAvailable: "checking" | "available" | "unavailable" =
    codexInfoQuery.isFetching && !codexInfo
      ? "checking"
      : codexInfo?.found === true
        ? "available"
        : "unavailable";
  const codexLoading = codexInfoQuery.isFetching;
  const codexConfigLoading = codexConfigQuery.isFetching;
  const codexConfigTomlLoading = codexConfigTomlQuery.isFetching;
  const codexConfigTomlSaving = codexConfigTomlSetMutation.isPending;
  const codexConfigWriting = codexConfigSetMutation.isPending || codexConfigTomlSaving;
  const codexConfigSaving = codexConfigWriting;
  const codexModelCatalogLoading = codexModelCatalogQuery.isFetching;
  const codexModelCatalogError = codexModelCatalogQuery.isError;

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
    setGeneralSettingsDraft(appSettingsToGeneralSettingsDraft(appSettings));
  }, [appSettings]);

  function updateGeneralSettingsDraft(patch: Partial<GeneralSettingsDraft>) {
    setGeneralSettingsDraft((draft) => ({ ...draft, ...patch }));
  }

  function updateRectifierDraft(rectifier: GatewayRectifierSettingsPatch) {
    setGeneralSettingsDraft((draft) => ({ ...draft, rectifier }));
  }

  function setDraftNumber<K extends keyof GeneralSettingsDraft>(key: K, value: number) {
    setGeneralSettingsDraft((draft) => ({ ...draft, [key]: value }));
  }

  async function persistRectifier(patch: Partial<GatewayRectifierSettingsPatch>) {
    if (settingsWriteBlocked) {
      blockSettingsWrite();
      return;
    }
    if (rectifierSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = rectifier;
    const next = { ...prev, ...patch };
    updateRectifierDraft(next);
    try {
      const updated = await rectifierMutation.mutateAsync(next);
      if (!updated) {
        updateRectifierDraft(prev);
        return;
      }

      updateRectifierDraft({
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
      updateRectifierDraft(prev);
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
    updateGeneralSettingsDraft({ circuitBreakerNoticeEnabled: enable });
    try {
      const updated = await circuitBreakerNoticeMutation.mutateAsync(enable);
      if (!updated) {
        updateGeneralSettingsDraft({ circuitBreakerNoticeEnabled: prev });
        return;
      }

      updateGeneralSettingsDraft({
        circuitBreakerNoticeEnabled: updated.enable_circuit_breaker_notice ?? enable,
      });
      toast(enable ? "已开启熔断通知" : "已关闭熔断通知");
    } catch (err) {
      logToConsole("error", "更新熔断通知配置失败", { error: String(err) });
      toast("更新熔断通知配置失败：请稍后重试");
      updateGeneralSettingsDraft({ circuitBreakerNoticeEnabled: prev });
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
    updateGeneralSettingsDraft({ codexSessionIdCompletionEnabled: enable });
    try {
      const updated = await codexSessionIdCompletionMutation.mutateAsync(enable);
      if (!updated) {
        updateGeneralSettingsDraft({ codexSessionIdCompletionEnabled: prev });
        return;
      }

      updateGeneralSettingsDraft({
        codexSessionIdCompletionEnabled: updated.enable_codex_session_id_completion ?? enable,
      });
      toast(enable ? "已开启 Codex Session ID 补全" : "已关闭 Codex Session ID 补全");
    } catch (err) {
      logToConsole("error", "更新 Codex Session ID 补全配置失败", { error: String(err) });
      toast("更新 Codex Session ID 补全配置失败：请稍后重试");
      updateGeneralSettingsDraft({ codexSessionIdCompletionEnabled: prev });
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

      updateGeneralSettingsDraft(generalSettingsDraftPatchFromAppSettings(updatedSettings));
      toast("已保存");
      return updatedSettings;
    } catch (err) {
      const formatted = formatActionFailureToast("更新通用网关参数", err);
      logToConsole("error", "更新通用网关参数失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
      });
      toast(formatted.toast);
      updateGeneralSettingsDraft(generalSettingsDraftPatchFromAppSettings(prev));
      return null;
    }
  }

  async function refreshClaude() {
    await Promise.all([claudeSettingsQuery.refetch(), claudeInfoQuery.refetch()]);
  }

  async function refreshCodex() {
    if (codexConfigWriting) return;
    const [configResult, , infoResult] = await Promise.all([
      codexConfigQuery.refetch(),
      codexConfigTomlQuery.refetch(),
      codexInfoQuery.refetch(),
    ]);
    const nextConfig = configResult.data ?? null;
    const nextInfo = infoResult.data ?? null;
    if (configResult.isError || infoResult.isError || !nextConfig || nextInfo?.found !== true) {
      return;
    }
    await refreshCodexModelCatalog({
      configPath: nextConfig.config_path,
      executablePath: nextInfo.executable_path,
      cliVersion: nextInfo.version,
    });
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

  async function persistCodexConfig(patch: CodexConfigPatch): Promise<CodexConfigState | null> {
    if (codexConfigWriting) return null;
    if (!codexConfig) return null;

    try {
      const updated = await codexConfigSetMutation.mutateAsync(patch);
      if (!updated) {
        return null;
      }
      toast("已更新 Codex 配置");
      return updated;
    } catch (err) {
      const formatted = formatActionFailureToast("更新 Codex 配置", err);
      logToConsole("error", "更新 Codex 配置失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        patch,
      });
      toast(formatted.toast);
      return null;
    }
  }

  async function persistCodexConfigToml(toml: string): Promise<boolean> {
    if (codexConfigWriting) return false;
    if (!codexConfig) return false;

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
      setUpstreamFirstByteTimeoutSeconds: (value: number) =>
        setDraftNumber("upstreamFirstByteTimeoutSeconds", value),
      upstreamStreamIdleTimeoutSeconds,
      setUpstreamStreamIdleTimeoutSeconds: (value: number) =>
        setDraftNumber("upstreamStreamIdleTimeoutSeconds", value),
      upstreamRequestTimeoutNonStreamingSeconds,
      setUpstreamRequestTimeoutNonStreamingSeconds: (value: number) =>
        setDraftNumber("upstreamRequestTimeoutNonStreamingSeconds", value),
      providerCooldownSeconds,
      setProviderCooldownSeconds: (value: number) =>
        setDraftNumber("providerCooldownSeconds", value),
      providerBaseUrlPingCacheTtlSeconds,
      setProviderBaseUrlPingCacheTtlSeconds: (value: number) =>
        setDraftNumber("providerBaseUrlPingCacheTtlSeconds", value),
      circuitBreakerFailureThreshold,
      setCircuitBreakerFailureThreshold: (value: number) =>
        setDraftNumber("circuitBreakerFailureThreshold", value),
      circuitBreakerOpenDurationMinutes,
      setCircuitBreakerOpenDurationMinutes: (value: number) =>
        setDraftNumber("circuitBreakerOpenDurationMinutes", value),
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
      codexModelCatalogLoading,
      codexModelCatalogError,
      codexInfo,
      codexConfig,
      codexConfigToml,
      codexModelCatalog,
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
    grokTabProps,
  };
}
