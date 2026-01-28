// Usage: UI for configuring local CLI integrations and related app settings. Backend commands: `cli_manager_*`, `settings_*`, `cli_proxy_*`, `gateway_*`.

import {
  lazy,
  Suspense,
  useEffect,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import {
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexInfoGet,
  cliManagerGeminiInfoGet,
  type ClaudeCliInfo,
  type ClaudeSettingsPatch,
  type ClaudeSettingsState,
  type CodexConfigPatch,
  type CodexConfigState,
  type SimpleCliInfo,
} from "../services/cliManager";
import { logToConsole } from "../services/consoleLog";
import { settingsGet, settingsSet, type AppSettings } from "../services/settings";
import { settingsCodexSessionIdCompletionSet } from "../services/settingsCodexSessionIdCompletion";
import { settingsCircuitBreakerNoticeSet } from "../services/settingsCircuitBreakerNotice";
import {
  settingsGatewayRectifierSet,
  type GatewayRectifierSettingsPatch,
} from "../services/settingsGatewayRectifier";
import { formatActionFailureToast } from "../utils/errors";
import { CliManagerGeneralTab } from "../components/cli-manager/tabs/GeneralTab";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";

type TabKey = "general" | "claude" | "codex" | "gemini";

const TABS: Array<{ key: TabKey; label: string }> = [
  { key: "general", label: "通用" },
  { key: "claude", label: "Claude Code" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
];

const DEFAULT_RECTIFIER: GatewayRectifierSettingsPatch = {
  intercept_anthropic_warmup_requests: false,
  enable_thinking_signature_rectifier: true,
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

const LazyGeminiTab = lazy(() =>
  import("../components/cli-manager/tabs/GeminiTab").then((m) => ({
    default: m.CliManagerGeminiTab,
  }))
);

const TAB_FALLBACK = <div className="p-6 text-sm text-slate-500">加载中…</div>;

export function CliManagerPage() {
  const [tab, setTab] = useState<TabKey>("general");
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);

  const [rectifierAvailable, setRectifierAvailable] = useState<
    "checking" | "available" | "unavailable"
  >("checking");
  const [rectifierSaving, setRectifierSaving] = useState(false);
  const [rectifier, setRectifier] = useState<GatewayRectifierSettingsPatch>(DEFAULT_RECTIFIER);
  const [circuitBreakerNoticeEnabled, setCircuitBreakerNoticeEnabled] = useState(false);
  const [circuitBreakerNoticeSaving, setCircuitBreakerNoticeSaving] = useState(false);
  const [codexSessionIdCompletionEnabled, setCodexSessionIdCompletionEnabled] = useState(true);
  const [codexSessionIdCompletionSaving, setCodexSessionIdCompletionSaving] = useState(false);
  const [commonSettingsSaving, setCommonSettingsSaving] = useState(false);
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

  const [claudeAvailable, setClaudeAvailable] = useState<"checking" | "available" | "unavailable">(
    "checking"
  );
  const [claudeLoading, setClaudeLoading] = useState(false);
  const [claudeInfo, setClaudeInfo] = useState<ClaudeCliInfo | null>(null);
  const [claudeSettingsLoading, setClaudeSettingsLoading] = useState(false);
  const [claudeSettingsSaving, setClaudeSettingsSaving] = useState(false);
  const [claudeSettings, setClaudeSettings] = useState<ClaudeSettingsState | null>(null);
  const [claudeSettingsAttempted, setClaudeSettingsAttempted] = useState(false);

  const [codexAvailable, setCodexAvailable] = useState<"checking" | "available" | "unavailable">(
    "checking"
  );
  const [codexLoading, setCodexLoading] = useState(false);
  const [codexInfo, setCodexInfo] = useState<SimpleCliInfo | null>(null);
  const [codexConfigLoading, setCodexConfigLoading] = useState(false);
  const [codexConfigSaving, setCodexConfigSaving] = useState(false);
  const [codexConfig, setCodexConfig] = useState<CodexConfigState | null>(null);
  const [codexConfigAttempted, setCodexConfigAttempted] = useState(false);

  const [geminiAvailable, setGeminiAvailable] = useState<"checking" | "available" | "unavailable">(
    "checking"
  );
  const [geminiLoading, setGeminiLoading] = useState(false);
  const [geminiInfo, setGeminiInfo] = useState<SimpleCliInfo | null>(null);

  useEffect(() => {
    let cancelled = false;
    setRectifierAvailable("checking");
    settingsGet()
      .then((settings) => {
        if (cancelled) return;
        if (!settings) {
          setRectifierAvailable("unavailable");
          setAppSettings(null);
          return;
        }
        setRectifierAvailable("available");
        setAppSettings(settings);
        setRectifier({
          intercept_anthropic_warmup_requests: settings.intercept_anthropic_warmup_requests,
          enable_thinking_signature_rectifier: settings.enable_thinking_signature_rectifier,
          enable_response_fixer: settings.enable_response_fixer,
          response_fixer_fix_encoding: settings.response_fixer_fix_encoding,
          response_fixer_fix_sse_format: settings.response_fixer_fix_sse_format,
          response_fixer_fix_truncated_json: settings.response_fixer_fix_truncated_json,
          response_fixer_max_json_depth: settings.response_fixer_max_json_depth,
          response_fixer_max_fix_size: settings.response_fixer_max_fix_size,
        });
        setCircuitBreakerNoticeEnabled(settings.enable_circuit_breaker_notice ?? false);
        setCodexSessionIdCompletionEnabled(settings.enable_codex_session_id_completion ?? true);
        setUpstreamFirstByteTimeoutSeconds(settings.upstream_first_byte_timeout_seconds);
        setUpstreamStreamIdleTimeoutSeconds(settings.upstream_stream_idle_timeout_seconds);
        setUpstreamRequestTimeoutNonStreamingSeconds(
          settings.upstream_request_timeout_non_streaming_seconds
        );
        setProviderCooldownSeconds(settings.provider_cooldown_seconds);
        setProviderBaseUrlPingCacheTtlSeconds(settings.provider_base_url_ping_cache_ttl_seconds);
        setCircuitBreakerFailureThreshold(settings.circuit_breaker_failure_threshold);
        setCircuitBreakerOpenDurationMinutes(settings.circuit_breaker_open_duration_minutes);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取网关整流配置失败", { error: String(err) });
        setRectifierAvailable("available");
        setAppSettings(null);
        toast("读取网关整流配置失败：请查看控制台日志");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (tab !== "claude") return;
    if (!claudeSettings && !claudeSettingsAttempted && !claudeSettingsLoading) {
      void refreshClaudeSettings();
      return;
    }
    if (claudeSettingsLoading) return;
    if (claudeAvailable === "checking") void refreshClaudeInfo();
  }, [tab, claudeAvailable, claudeSettings, claudeSettingsAttempted, claudeSettingsLoading]);

  useEffect(() => {
    if (tab !== "codex") return;
    if (!codexConfig && !codexConfigAttempted && !codexConfigLoading) {
      void refreshCodexConfig();
      return;
    }
    if (codexConfigLoading) return;
    if (codexAvailable === "checking") void refreshCodexInfo();
  }, [tab, codexAvailable, codexConfig, codexConfigAttempted, codexConfigLoading]);

  useEffect(() => {
    if (tab !== "gemini") return;
    if (geminiAvailable !== "checking") return;
    void refreshGeminiInfo();
  }, [tab, geminiAvailable]);

  async function persistRectifier(patch: Partial<GatewayRectifierSettingsPatch>) {
    if (rectifierSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = rectifier;
    const next = { ...prev, ...patch };
    setRectifier(next);
    setRectifierSaving(true);
    try {
      const updated = await settingsGatewayRectifierSet(next);
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        setRectifier(prev);
        return;
      }

      setAppSettings(updated);
      setRectifier({
        intercept_anthropic_warmup_requests: updated.intercept_anthropic_warmup_requests,
        enable_thinking_signature_rectifier: updated.enable_thinking_signature_rectifier,
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
    } finally {
      setRectifierSaving(false);
    }
  }

  async function persistCircuitBreakerNotice(enable: boolean) {
    if (circuitBreakerNoticeSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = circuitBreakerNoticeEnabled;
    setCircuitBreakerNoticeEnabled(enable);
    setCircuitBreakerNoticeSaving(true);
    try {
      const updated = await settingsCircuitBreakerNoticeSet(enable);
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        setCircuitBreakerNoticeEnabled(prev);
        return;
      }

      setAppSettings(updated);
      setCircuitBreakerNoticeEnabled(updated.enable_circuit_breaker_notice ?? enable);
      toast(enable ? "已开启熔断通知" : "已关闭熔断通知");
    } catch (err) {
      logToConsole("error", "更新熔断通知配置失败", { error: String(err) });
      toast("更新熔断通知配置失败：请稍后重试");
      setCircuitBreakerNoticeEnabled(prev);
    } finally {
      setCircuitBreakerNoticeSaving(false);
    }
  }

  async function persistCodexSessionIdCompletion(enable: boolean) {
    if (codexSessionIdCompletionSaving) return;
    if (rectifierAvailable !== "available") return;

    const prev = codexSessionIdCompletionEnabled;
    setCodexSessionIdCompletionEnabled(enable);
    setCodexSessionIdCompletionSaving(true);
    try {
      const updated = await settingsCodexSessionIdCompletionSet(enable);
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        setCodexSessionIdCompletionEnabled(prev);
        return;
      }

      setAppSettings(updated);
      setCodexSessionIdCompletionEnabled(updated.enable_codex_session_id_completion ?? enable);
      toast(enable ? "已开启 Codex Session ID 补全" : "已关闭 Codex Session ID 补全");
    } catch (err) {
      logToConsole("error", "更新 Codex Session ID 补全配置失败", { error: String(err) });
      toast("更新 Codex Session ID 补全配置失败：请稍后重试");
      setCodexSessionIdCompletionEnabled(prev);
    } finally {
      setCodexSessionIdCompletionSaving(false);
    }
  }

  async function persistCommonSettings(patch: Partial<AppSettings>): Promise<AppSettings | null> {
    if (commonSettingsSaving) return null;
    if (rectifierAvailable !== "available") return null;
    if (!appSettings) return null;

    const prev = appSettings;
    const next: AppSettings = { ...prev, ...patch };
    setAppSettings(next);
    setCommonSettingsSaving(true);
    try {
      const updated = await settingsSet({
        preferred_port: next.preferred_port,
        gateway_listen_mode: next.gateway_listen_mode,
        gateway_custom_listen_address: next.gateway_custom_listen_address,
        auto_start: next.auto_start,
        tray_enabled: next.tray_enabled,
        log_retention_days: next.log_retention_days,
        provider_cooldown_seconds: next.provider_cooldown_seconds,
        provider_base_url_ping_cache_ttl_seconds: next.provider_base_url_ping_cache_ttl_seconds,
        upstream_first_byte_timeout_seconds: next.upstream_first_byte_timeout_seconds,
        upstream_stream_idle_timeout_seconds: next.upstream_stream_idle_timeout_seconds,
        upstream_request_timeout_non_streaming_seconds:
          next.upstream_request_timeout_non_streaming_seconds,
        failover_max_attempts_per_provider: next.failover_max_attempts_per_provider,
        failover_max_providers_to_try: next.failover_max_providers_to_try,
        circuit_breaker_failure_threshold: next.circuit_breaker_failure_threshold,
        circuit_breaker_open_duration_minutes: next.circuit_breaker_open_duration_minutes,
        wsl_auto_config: next.wsl_auto_config,
        wsl_target_cli: next.wsl_target_cli,
      });

      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        setAppSettings(prev);
        return null;
      }

      setAppSettings(updated);
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
      setAppSettings(prev);
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
    } finally {
      setCommonSettingsSaving(false);
    }
  }

  function applyClaudeInfo(info: ClaudeCliInfo) {
    setClaudeInfo(info);
  }

  async function refreshClaudeInfo() {
    if (claudeLoading) return;
    setClaudeLoading(true);
    setClaudeAvailable("checking");
    try {
      const info = await cliManagerClaudeInfoGet();
      if (!info) {
        setClaudeAvailable("unavailable");
        setClaudeInfo(null);
        return;
      }
      setClaudeAvailable("available");
      applyClaudeInfo(info);
    } catch (err) {
      logToConsole("error", "读取 Claude Code 信息失败", { error: String(err) });
      setClaudeAvailable("available");
      toast("读取 Claude Code 信息失败：请查看控制台日志");
    } finally {
      setClaudeLoading(false);
    }
  }

  async function refreshClaudeSettings() {
    if (claudeSettingsLoading) return;
    setClaudeSettingsAttempted(true);
    setClaudeSettingsLoading(true);
    try {
      const settings = await cliManagerClaudeSettingsGet();
      if (!settings) {
        setClaudeSettings(null);
        return;
      }
      setClaudeSettings(settings);
    } catch (err) {
      logToConsole("error", "读取 Claude Code settings.json 失败", { error: String(err) });
      toast("读取 Claude Code 配置失败：请查看控制台日志");
    } finally {
      setClaudeSettingsLoading(false);
    }
  }

  async function refreshClaude() {
    await refreshClaudeSettings();
    await refreshClaudeInfo();
  }

  async function refreshCodexInfo() {
    if (codexLoading) return;
    setCodexLoading(true);
    setCodexAvailable("checking");
    try {
      const info = await cliManagerCodexInfoGet();
      if (!info) {
        setCodexAvailable("unavailable");
        setCodexInfo(null);
        return;
      }
      setCodexAvailable("available");
      setCodexInfo(info);
    } catch (err) {
      logToConsole("error", "读取 Codex 信息失败", { error: String(err) });
      setCodexAvailable("available");
      toast("读取 Codex 信息失败：请查看控制台日志");
    } finally {
      setCodexLoading(false);
    }
  }

  async function refreshCodexConfig() {
    if (codexConfigLoading) return;
    setCodexConfigAttempted(true);
    setCodexConfigLoading(true);
    try {
      const cfg = await cliManagerCodexConfigGet();
      if (!cfg) {
        setCodexConfig(null);
        return;
      }
      setCodexConfig(cfg);
    } catch (err) {
      logToConsole("error", "读取 Codex 配置失败", { error: String(err) });
      toast("读取 Codex 配置失败：请查看控制台日志");
    } finally {
      setCodexConfigLoading(false);
    }
  }

  async function refreshCodex() {
    await refreshCodexConfig();
    await refreshCodexInfo();
  }

  async function refreshGeminiInfo() {
    if (geminiLoading) return;
    setGeminiLoading(true);
    setGeminiAvailable("checking");
    try {
      const info = await cliManagerGeminiInfoGet();
      if (!info) {
        setGeminiAvailable("unavailable");
        setGeminiInfo(null);
        return;
      }
      setGeminiAvailable("available");
      setGeminiInfo(info);
    } catch (err) {
      logToConsole("error", "读取 Gemini 信息失败", { error: String(err) });
      setGeminiAvailable("available");
      toast("读取 Gemini 信息失败：请查看控制台日志");
    } finally {
      setGeminiLoading(false);
    }
  }

  async function persistCodexConfig(patch: CodexConfigPatch) {
    if (codexConfigSaving) return;
    if (codexAvailable !== "available") return;

    const prev = codexConfig;
    setCodexConfigSaving(true);
    try {
      const updated = await cliManagerCodexConfigSet(patch);
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        if (prev) setCodexConfig(prev);
        return;
      }
      setCodexConfig(updated);
      toast("已更新 Codex 配置");
    } catch (err) {
      const formatted = formatActionFailureToast("更新 Codex 配置", err);
      logToConsole("error", "更新 Codex 配置失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        patch,
      });
      toast(formatted.toast);
      if (prev) setCodexConfig(prev);
    } finally {
      setCodexConfigSaving(false);
    }
  }

  async function persistClaudeSettings(patch: ClaudeSettingsPatch) {
    if (claudeSettingsSaving) return;
    if (claudeAvailable !== "available") return;

    const prev = claudeSettings;
    setClaudeSettingsSaving(true);
    try {
      const updated = await cliManagerClaudeSettingsSet(patch);
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        if (prev) setClaudeSettings(prev);
        return;
      }
      setClaudeSettings(updated);
      toast("已更新 Claude Code 配置");
    } catch (err) {
      logToConsole("error", "更新 Claude Code settings.json 失败", { error: String(err) });
      toast("更新 Claude Code 配置失败：请稍后重试");
      if (prev) setClaudeSettings(prev);
    } finally {
      setClaudeSettingsSaving(false);
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
      toast("受权限限制，无法自动打开该目录（仅允许 $HOME/.codex 下的路径）");
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
    <div className="space-y-6 pb-10">
      <PageHeader
        title="CLI 管理"
        actions={
          <TabList ariaLabel="CLI 管理视图切换" items={TABS} value={tab} onChange={setTab} />
        }
      />

      <div className="min-h-[400px]">
        {tab === "general" ? (
          <CliManagerGeneralTab
            rectifierAvailable={rectifierAvailable}
            rectifierSaving={rectifierSaving}
            rectifier={rectifier}
            onPersistRectifier={persistRectifier}
            circuitBreakerNoticeEnabled={circuitBreakerNoticeEnabled}
            circuitBreakerNoticeSaving={circuitBreakerNoticeSaving}
            onPersistCircuitBreakerNotice={persistCircuitBreakerNotice}
            codexSessionIdCompletionEnabled={codexSessionIdCompletionEnabled}
            codexSessionIdCompletionSaving={codexSessionIdCompletionSaving}
            onPersistCodexSessionIdCompletion={persistCodexSessionIdCompletion}
            appSettings={appSettings}
            commonSettingsSaving={commonSettingsSaving}
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
              codexInfo={codexInfo}
              codexConfig={codexConfig}
              refreshCodex={refreshCodex}
              openCodexConfigDir={openCodexConfigDir}
              persistCodexConfig={persistCodexConfig}
            />
          </Suspense>
        ) : null}

        {tab === "gemini" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyGeminiTab
              geminiAvailable={geminiAvailable}
              geminiLoading={geminiLoading}
              geminiInfo={geminiInfo}
              refreshGeminiInfo={refreshGeminiInfo}
            />
          </Suspense>
        ) : null}
      </div>
    </div>
  );
}
