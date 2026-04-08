import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { toast } from "sonner";
import { useNavigate } from "react-router-dom";
import { CACHE_ANOMALY_MONITOR_GUIDE_COPY } from "../../../services/cacheAnomalyMonitorConfig";
import type { AppSettings } from "../../../services/settings";
import type { GatewayRectifierSettingsPatch } from "../../../services/settingsGatewayRectifier";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { SettingsRow } from "../../../ui/SettingsRow";
import { Switch } from "../../../ui/Switch";
import { NetworkSettingsCard } from "../NetworkSettingsCard";
import { WslSettingsCard } from "../WslSettingsCard";
import { Bell, Shield, TrendingDown } from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerGeneralTabProps = {
  rectifierAvailable: CliManagerAvailability;
  settingsReadErrorMessage: string | null;
  settingsWriteBlocked: boolean;
  rectifierSaving: boolean;
  rectifier: GatewayRectifierSettingsPatch;
  onPersistRectifier: (patch: Partial<GatewayRectifierSettingsPatch>) => Promise<void> | void;

  circuitBreakerNoticeEnabled: boolean;
  circuitBreakerNoticeSaving: boolean;
  onPersistCircuitBreakerNotice: (enable: boolean) => Promise<void> | void;

  codexSessionIdCompletionEnabled: boolean;
  codexSessionIdCompletionSaving: boolean;
  onPersistCodexSessionIdCompletion: (enable: boolean) => Promise<void> | void;

  cacheAnomalyMonitorEnabled: boolean;
  cacheAnomalyMonitorSaving: boolean;
  onPersistCacheAnomalyMonitor: (enable: boolean) => Promise<void> | void;

  taskCompleteNotifyEnabled: boolean;
  taskCompleteNotifySaving: boolean;
  onPersistTaskCompleteNotify: (enable: boolean) => Promise<void> | void;

  notificationSoundEnabled: boolean;
  notificationSoundSaving: boolean;
  onPersistNotificationSound: (enable: boolean) => Promise<void> | void;

  appSettings: AppSettings | null;
  commonSettingsSaving: boolean;
  onPersistCommonSettings: (patch: Partial<AppSettings>) => Promise<AppSettings | null>;

  upstreamFirstByteTimeoutSeconds: number;
  setUpstreamFirstByteTimeoutSeconds: (value: number) => void;
  upstreamStreamIdleTimeoutSeconds: number;
  setUpstreamStreamIdleTimeoutSeconds: (value: number) => void;
  upstreamRequestTimeoutNonStreamingSeconds: number;
  setUpstreamRequestTimeoutNonStreamingSeconds: (value: number) => void;

  providerCooldownSeconds: number;
  setProviderCooldownSeconds: (value: number) => void;
  providerBaseUrlPingCacheTtlSeconds: number;
  setProviderBaseUrlPingCacheTtlSeconds: (value: number) => void;
  circuitBreakerFailureThreshold: number;
  setCircuitBreakerFailureThreshold: (value: number) => void;
  circuitBreakerOpenDurationMinutes: number;
  setCircuitBreakerOpenDurationMinutes: (value: number) => void;

  blurOnEnter: (e: ReactKeyboardEvent<HTMLInputElement>) => void;
};

export function CliManagerGeneralTab({
  rectifierAvailable,
  settingsReadErrorMessage,
  settingsWriteBlocked,
  rectifierSaving,
  rectifier,
  onPersistRectifier,
  circuitBreakerNoticeEnabled,
  circuitBreakerNoticeSaving,
  onPersistCircuitBreakerNotice,
  codexSessionIdCompletionEnabled,
  codexSessionIdCompletionSaving,
  onPersistCodexSessionIdCompletion,
  cacheAnomalyMonitorEnabled,
  cacheAnomalyMonitorSaving,
  onPersistCacheAnomalyMonitor,
  taskCompleteNotifyEnabled,
  taskCompleteNotifySaving,
  onPersistTaskCompleteNotify,
  notificationSoundEnabled,
  notificationSoundSaving,
  onPersistNotificationSound,
  appSettings,
  commonSettingsSaving,
  onPersistCommonSettings,
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
}: CliManagerGeneralTabProps) {
  const navigate = useNavigate();
  const settingsUnavailable = rectifierAvailable !== "available";
  const rectifierDisabled = rectifierSaving || settingsUnavailable || settingsWriteBlocked;
  const circuitNoticeDisabled =
    circuitBreakerNoticeSaving || settingsUnavailable || settingsWriteBlocked;
  const codexCompletionDisabled =
    codexSessionIdCompletionSaving || settingsUnavailable || settingsWriteBlocked;
  const taskNotifyDisabled =
    taskCompleteNotifySaving || settingsUnavailable || settingsWriteBlocked;
  const notificationSoundDisabled =
    notificationSoundSaving || settingsUnavailable || settingsWriteBlocked;
  const cacheMonitorDisabled =
    cacheAnomalyMonitorSaving || settingsUnavailable || settingsWriteBlocked;
  const commonSettingsDisabled =
    commonSettingsSaving || settingsUnavailable || settingsWriteBlocked;

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-slate-100 dark:border-slate-700 p-6">
          <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">通用配置</h2>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            网关整流、通知、超时与熔断策略。
          </p>
        </div>

        {settingsReadErrorMessage ? (
          <div className="border-b border-amber-200 bg-amber-50 px-6 py-4 text-sm text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200">
            {settingsReadErrorMessage}
          </div>
        ) : null}

        {rectifierAvailable === "unavailable" ? (
          <div className="text-sm text-slate-600 dark:text-slate-400 text-center py-8">
            数据不可用
          </div>
        ) : (
          <div className="p-6 space-y-6">
            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                <Shield className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                网关整流器
              </h3>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingsRow label="详细供应商错误信息" subtitle="在日志中显示完整的上游错误详情。">
                  <Switch
                    checked={rectifier.verbose_provider_error}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ verbose_provider_error: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="拦截 Anthropic Warmup 请求"
                  subtitle="自动拦截并响应 Anthropic 的预热请求，避免计费。"
                >
                  <Switch
                    checked={rectifier.intercept_anthropic_warmup_requests}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ intercept_anthropic_warmup_requests: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="Thinking 签名整流器"
                  subtitle="自动修复 extended thinking 相关的签名问题。"
                >
                  <Switch
                    checked={rectifier.enable_thinking_signature_rectifier}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ enable_thinking_signature_rectifier: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="Thinking 预算整流器"
                  subtitle="自动修复 thinking budget 相关的参数问题。"
                >
                  <Switch
                    checked={rectifier.enable_thinking_budget_rectifier}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ enable_thinking_budget_rectifier: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="Billing Header 整流器"
                  subtitle="自动移除 Claude 请求里的 billing header system 块。适合OAuth用户"
                >
                  <Switch
                    checked={rectifier.enable_billing_header_rectifier}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ enable_billing_header_rectifier: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="Claude metadata.user_id 注入"
                  subtitle="为 Claude 请求自动注入 metadata.user_id 字段。"
                >
                  <Switch
                    checked={rectifier.enable_claude_metadata_user_id_injection}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({
                        enable_claude_metadata_user_id_injection: checked,
                      })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="响应整流（FluxFix）"
                  subtitle="自动修复编码、SSE 格式、截断 JSON 等常见响应问题。"
                >
                  <Switch
                    checked={rectifier.enable_response_fixer}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ enable_response_fixer: checked })
                    }
                    disabled={rectifierDisabled}
                  />
                </SettingsRow>
                {rectifier.enable_response_fixer && (
                  <>
                    <SettingsRow label="修复编码问题" className="pl-6">
                      <Switch
                        checked={rectifier.response_fixer_fix_encoding}
                        onCheckedChange={(checked) =>
                          void onPersistRectifier({ response_fixer_fix_encoding: checked })
                        }
                        disabled={rectifierDisabled}
                      />
                    </SettingsRow>
                    <SettingsRow label="修复 SSE 格式" className="pl-6">
                      <Switch
                        checked={rectifier.response_fixer_fix_sse_format}
                        onCheckedChange={(checked) =>
                          void onPersistRectifier({ response_fixer_fix_sse_format: checked })
                        }
                        disabled={rectifierDisabled}
                      />
                    </SettingsRow>
                    <SettingsRow label="修复截断的 JSON" className="pl-6">
                      <Switch
                        checked={rectifier.response_fixer_fix_truncated_json}
                        onCheckedChange={(checked) =>
                          void onPersistRectifier({ response_fixer_fix_truncated_json: checked })
                        }
                        disabled={rectifierDisabled}
                      />
                    </SettingsRow>
                  </>
                )}
                <SettingsRow
                  label="Codex Session ID 补全"
                  subtitle="当 Codex 请求仅提供 session_id 或 prompt_cache_key 之一时，自动补全另一侧；若两者均缺失，则生成并稳定复用会话标识。"
                >
                  <Switch
                    checked={codexSessionIdCompletionEnabled}
                    onCheckedChange={(checked) => void onPersistCodexSessionIdCompletion(checked)}
                    disabled={codexCompletionDisabled}
                  />
                </SettingsRow>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-1">
                <Bell className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                通知
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                控制系统通知与音效提醒行为。
                <span className="ml-1 text-amber-600/80 dark:text-amber-400/80">
                  * 需在系统设置中授予通知权限
                </span>
              </p>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingsRow
                  label="任务结束提醒"
                  subtitle="当 AI CLI 工具（Claude/Gemini：30 秒；Codex：120 秒）请求结束后静默无新请求时，发送系统通知提醒。"
                >
                  <Switch
                    checked={taskCompleteNotifyEnabled}
                    onCheckedChange={(checked) => void onPersistTaskCompleteNotify(checked)}
                    disabled={taskNotifyDisabled}
                  />
                </SettingsRow>
                <SettingsRow label="熔断通知" subtitle="当服务熔断触发或恢复时，主动发送系统通知。">
                  <Switch
                    checked={circuitBreakerNoticeEnabled}
                    onCheckedChange={(checked) => void onPersistCircuitBreakerNotice(checked)}
                    disabled={circuitNoticeDisabled}
                  />
                </SettingsRow>
                <SettingsRow
                  label="通知音效"
                  subtitle="使用自定义提示音代替系统默认通知音效，避免重复响铃。"
                >
                  <Switch
                    checked={notificationSoundEnabled}
                    onCheckedChange={(checked) => void onPersistNotificationSound(checked)}
                    disabled={notificationSoundDisabled}
                  />
                </SettingsRow>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-1">
                <TrendingDown className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                缓存异常监测（实验）
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                {CACHE_ANOMALY_MONITOR_GUIDE_COPY.overview}
              </p>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingsRow
                  label="启用缓存异常监测"
                  subtitle={`${CACHE_ANOMALY_MONITOR_GUIDE_COPY.trigger} ${CACHE_ANOMALY_MONITOR_GUIDE_COPY.metric}`}
                >
                  <Switch
                    checked={cacheAnomalyMonitorEnabled}
                    onCheckedChange={(checked) => void onPersistCacheAnomalyMonitor(checked)}
                    disabled={cacheMonitorDisabled}
                  />
                </SettingsRow>
              </div>
              <div className="mt-3 space-y-1 text-xs text-slate-500 dark:text-slate-400">
                <p>{CACHE_ANOMALY_MONITOR_GUIDE_COPY.coldStart}</p>
                <p>{CACHE_ANOMALY_MONITOR_GUIDE_COPY.nonCachingModel}</p>
                <p>{CACHE_ANOMALY_MONITOR_GUIDE_COPY.thresholds}</p>
              </div>
              <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                <span>
                  提示：告警会以 <span className="font-mono">WARN</span>{" "}
                  写入「控制台」页（无需开启调试日志）。
                </span>
                <Button size="sm" variant="secondary" onClick={() => navigate("/console")}>
                  打开控制台
                </Button>
              </div>
            </div>

            {appSettings ? (
              <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
                <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                  <Shield className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                  启动与恢复
                </h3>
                <div className="divide-y divide-slate-100 dark:divide-slate-700">
                  <SettingsRow
                    label="启动时 CLI 代理自愈"
                    subtitle="应用启动后仅修复异常退出导致的 CLI 代理残留状态，不会主动改写当前配置。建议保持开启。"
                  >
                    <Switch
                      checked={appSettings.enable_cli_proxy_startup_recovery}
                      onCheckedChange={(checked) =>
                        void onPersistCommonSettings({
                          enable_cli_proxy_startup_recovery: checked,
                        })
                      }
                      disabled={commonSettingsDisabled}
                    />
                  </SettingsRow>
                </div>
              </div>
            ) : null}

            {appSettings ? (
              <>
                <NetworkSettingsCard
                  available={rectifierAvailable === "available"}
                  saving={commonSettingsDisabled}
                  settings={appSettings}
                  onPersistSettings={onPersistCommonSettings}
                />
                <WslSettingsCard
                  available={rectifierAvailable === "available"}
                  saving={commonSettingsDisabled}
                  settings={appSettings}
                />
              </>
            ) : null}

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-1">
                <Shield className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                超时策略
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                控制上游请求的超时行为。0 表示禁用（交由上游/网络自行超时）。
              </p>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingsRow
                  label="首字节超时（0=禁用）"
                  subtitle="等待上游返回第一个字节的最大时间。"
                >
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={upstreamFirstByteTimeoutSeconds}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setUpstreamFirstByteTimeoutSeconds(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 0 || next > 3600) {
                          toast("上游首字节超时必须为 0-3600 秒");
                          setUpstreamFirstByteTimeoutSeconds(
                            appSettings.upstream_first_byte_timeout_seconds
                          );
                          return;
                        }
                        void onPersistCommonSettings({ upstream_first_byte_timeout_seconds: next });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={0}
                      max={3600}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">秒</span>
                  </div>
                </SettingsRow>

                <SettingsRow
                  label="流式空闲超时（0=禁用，启用时最小60秒）"
                  subtitle="流式响应中两次数据之间的最大静默时间。"
                >
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={upstreamStreamIdleTimeoutSeconds}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setUpstreamStreamIdleTimeoutSeconds(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (
                          !Number.isFinite(next) ||
                          next < 0 ||
                          next > 3600 ||
                          (next > 0 && next < 60)
                        ) {
                          toast("上游流式空闲超时必须为 0（禁用）或 60-3600 秒");
                          setUpstreamStreamIdleTimeoutSeconds(
                            appSettings.upstream_stream_idle_timeout_seconds
                          );
                          return;
                        }
                        void onPersistCommonSettings({
                          upstream_stream_idle_timeout_seconds: next,
                        });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={0}
                      max={3600}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">秒</span>
                  </div>
                </SettingsRow>

                <SettingsRow label="非流式总超时（0=禁用）" subtitle="非流式请求的总超时时间。">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={upstreamRequestTimeoutNonStreamingSeconds}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next))
                          setUpstreamRequestTimeoutNonStreamingSeconds(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 0 || next > 86400) {
                          toast("上游非流式总超时必须为 0-86400 秒");
                          setUpstreamRequestTimeoutNonStreamingSeconds(
                            appSettings.upstream_request_timeout_non_streaming_seconds
                          );
                          return;
                        }
                        void onPersistCommonSettings({
                          upstream_request_timeout_non_streaming_seconds: next,
                        });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={0}
                      max={86400}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">秒</span>
                  </div>
                </SettingsRow>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-1">
                <Shield className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                熔断与重试
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                控制 Provider 失败后的冷却、重试与熔断行为。修改后建议重启网关以完全生效。
              </p>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingsRow label="Provider 冷却" subtitle="单个 Provider 失败后的短暂冷却时间。">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={providerCooldownSeconds}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setProviderCooldownSeconds(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 0 || next > 3600) {
                          toast("短熔断冷却必须为 0-3600 秒");
                          setProviderCooldownSeconds(appSettings.provider_cooldown_seconds);
                          return;
                        }
                        void onPersistCommonSettings({ provider_cooldown_seconds: next });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={0}
                      max={3600}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">秒</span>
                  </div>
                </SettingsRow>

                <SettingsRow
                  label="Ping 选择缓存 TTL"
                  subtitle="Provider 可用性 ping 结果的缓存有效期。"
                >
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={providerBaseUrlPingCacheTtlSeconds}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setProviderBaseUrlPingCacheTtlSeconds(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 1 || next > 3600) {
                          toast("Ping 选择缓存 TTL 必须为 1-3600 秒");
                          setProviderBaseUrlPingCacheTtlSeconds(
                            appSettings.provider_base_url_ping_cache_ttl_seconds
                          );
                          return;
                        }
                        void onPersistCommonSettings({
                          provider_base_url_ping_cache_ttl_seconds: next,
                        });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={1}
                      max={3600}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">秒</span>
                  </div>
                </SettingsRow>

                <SettingsRow label="熔断阈值" subtitle="连续失败达到此次数后触发熔断。">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={circuitBreakerFailureThreshold}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setCircuitBreakerFailureThreshold(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 1 || next > 50) {
                          toast("熔断阈值必须为 1-50");
                          setCircuitBreakerFailureThreshold(
                            appSettings.circuit_breaker_failure_threshold
                          );
                          return;
                        }
                        void onPersistCommonSettings({ circuit_breaker_failure_threshold: next });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={1}
                      max={50}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">次</span>
                  </div>
                </SettingsRow>

                <SettingsRow label="熔断时长" subtitle="触发熔断后暂停该 Provider 的持续时间。">
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={circuitBreakerOpenDurationMinutes}
                      onChange={(e) => {
                        const next = e.currentTarget.valueAsNumber;
                        if (Number.isFinite(next)) setCircuitBreakerOpenDurationMinutes(next);
                      }}
                      onBlur={(e) => {
                        if (!appSettings) return;
                        const next = e.currentTarget.valueAsNumber;
                        if (!Number.isFinite(next) || next < 1 || next > 1440) {
                          toast("熔断时长必须为 1-1440 分钟");
                          setCircuitBreakerOpenDurationMinutes(
                            appSettings.circuit_breaker_open_duration_minutes
                          );
                          return;
                        }
                        void onPersistCommonSettings({
                          circuit_breaker_open_duration_minutes: next,
                        });
                      }}
                      onKeyDown={blurOnEnter}
                      style={{ width: "5rem" }}
                      min={1}
                      max={1440}
                      disabled={commonSettingsDisabled}
                    />
                    <span className="w-8 text-sm text-slate-500 dark:text-slate-400">分钟</span>
                  </div>
                </SettingsRow>
              </div>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
