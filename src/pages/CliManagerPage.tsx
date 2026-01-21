// Usage: UI for configuring local CLI integrations and related app settings. Backend commands: `cli_manager_*`, `settings_*`, `cli_proxy_*`, `gateway_*`.

import { useEffect, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import {
  cliManagerClaudeEnvSet,
  cliManagerClaudeInfoGet,
  cliManagerCodexInfoGet,
  cliManagerGeminiInfoGet,
  type ClaudeCliInfo,
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
import { CliManagerGeneralTab } from "../components/cli-manager/tabs/GeneralTab";
import { CliManagerClaudeTab } from "../components/cli-manager/tabs/ClaudeTab";
import { SimpleCliTab } from "../components/cli-manager/tabs/SimpleCliTab";
import { TabList } from "../ui/TabList";
import { Terminal, Cpu } from "lucide-react";

type TabKey = "general" | "claude" | "codex" | "gemini";

const TABS: Array<{ key: TabKey; label: string }> = [
  { key: "general", label: "通用" },
  { key: "claude", label: "Claude Code" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
];

const DEFAULT_RECTIFIER: GatewayRectifierSettingsPatch = {
  intercept_anthropic_warmup_requests: false,
  enable_thinking_signature_rectifier: false,
  enable_response_fixer: false,
  response_fixer_fix_encoding: true,
  response_fixer_fix_sse_format: true,
  response_fixer_fix_truncated_json: true,
};

const MAX_CLAUDE_MCP_TIMEOUT_MS = 24 * 60 * 60 * 1000;

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
  const [codexSessionIdCompletionEnabled, setCodexSessionIdCompletionEnabled] = useState(false);
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
  const [claudeSaving, setClaudeSaving] = useState(false);
  const [claudeInfo, setClaudeInfo] = useState<ClaudeCliInfo | null>(null);
  const [claudeMcpTimeoutMsText, setClaudeMcpTimeoutMsText] = useState<string>("");

  const [codexAvailable, setCodexAvailable] = useState<"checking" | "available" | "unavailable">(
    "checking"
  );
  const [codexLoading, setCodexLoading] = useState(false);
  const [codexInfo, setCodexInfo] = useState<SimpleCliInfo | null>(null);

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
        });
        setCircuitBreakerNoticeEnabled(settings.enable_circuit_breaker_notice ?? false);
        setCodexSessionIdCompletionEnabled(settings.enable_codex_session_id_completion ?? false);
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
    if (claudeAvailable !== "checking") return;
    void refreshClaudeInfo();
  }, [tab, claudeAvailable]);

  useEffect(() => {
    if (tab !== "codex") return;
    if (codexAvailable !== "checking") return;
    void refreshCodexInfo();
  }, [tab, codexAvailable]);

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
    setClaudeMcpTimeoutMsText(info.mcp_timeout_ms == null ? "" : String(info.mcp_timeout_ms));
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

  async function persistClaudeEnv(input: {
    mcp_timeout_ms: number | null;
    disable_error_reporting: boolean;
  }) {
    if (claudeSaving) return;
    if (claudeAvailable !== "available") return;

    const prev = claudeInfo;
    setClaudeSaving(true);
    try {
      const updated = await cliManagerClaudeEnvSet({
        mcp_timeout_ms: input.mcp_timeout_ms,
        disable_error_reporting: input.disable_error_reporting,
      });
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        if (prev) applyClaudeInfo(prev);
        return;
      }
      if (prev) {
        applyClaudeInfo({
          ...prev,
          config_dir: updated.config_dir,
          settings_path: updated.settings_path,
          mcp_timeout_ms: updated.mcp_timeout_ms,
          disable_error_reporting: updated.disable_error_reporting,
        });
      } else {
        applyClaudeInfo({
          found: false,
          executable_path: null,
          version: null,
          error: null,
          shell: null,
          resolved_via: "unavailable",
          config_dir: updated.config_dir,
          settings_path: updated.settings_path,
          mcp_timeout_ms: updated.mcp_timeout_ms,
          disable_error_reporting: updated.disable_error_reporting,
        });
      }
      toast("已更新 Claude Code 配置");
    } catch (err) {
      logToConsole("error", "更新 Claude Code 配置失败", { error: String(err) });
      toast("更新 Claude Code 配置失败：请稍后重试");
      if (prev) applyClaudeInfo(prev);
    } finally {
      setClaudeSaving(false);
    }
  }

  async function openClaudeConfigDir() {
    if (!claudeInfo) return;
    try {
      await openPath(claudeInfo.config_dir);
    } catch (err) {
      logToConsole("error", "打开 Claude 配置目录失败", { error: String(err) });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") e.currentTarget.blur();
  }

  function normalizeClaudeMcpTimeoutMsOrNull(raw: string): number | null {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    const n = Math.floor(Number(trimmed));
    if (!Number.isFinite(n) || n < 0) return NaN;
    if (n === 0) return null;
    if (n > MAX_CLAUDE_MCP_TIMEOUT_MS) return Infinity;
    return n;
  }

  return (
    <div className="space-y-4 pb-10">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold tracking-tight">CLI 管理</h1>
          <p className="mt-1 text-sm text-slate-600">
            统一管理 CLI 工具的配置与状态（支持 Claude / Codex / Gemini）。
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <TabList ariaLabel="CLI 管理视图切换" items={TABS} value={tab} onChange={setTab} />
        </div>
      </div>

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
          <CliManagerClaudeTab
            claudeAvailable={claudeAvailable}
            claudeLoading={claudeLoading}
            claudeSaving={claudeSaving}
            claudeInfo={claudeInfo}
            claudeMcpTimeoutMsText={claudeMcpTimeoutMsText}
            setClaudeMcpTimeoutMsText={setClaudeMcpTimeoutMsText}
            refreshClaudeInfo={refreshClaudeInfo}
            openClaudeConfigDir={openClaudeConfigDir}
            persistClaudeEnv={persistClaudeEnv}
            normalizeClaudeMcpTimeoutMsOrNull={normalizeClaudeMcpTimeoutMsOrNull}
            blurOnEnter={blurOnEnter}
            maxMcpTimeoutMs={MAX_CLAUDE_MCP_TIMEOUT_MS}
          />
        ) : null}

        {tab === "codex" ? (
          <SimpleCliTab
            title="Codex"
            Icon={Terminal}
            available={codexAvailable}
            loading={codexLoading}
            info={codexInfo}
            onRefresh={refreshCodexInfo}
          />
        ) : null}

        {tab === "gemini" ? (
          <SimpleCliTab
            title="Gemini"
            Icon={Cpu}
            available={geminiAvailable}
            loading={geminiLoading}
            info={geminiInfo}
            onRefresh={refreshGeminiInfo}
          />
        ) : null}
      </div>
    </div>
  );
}
