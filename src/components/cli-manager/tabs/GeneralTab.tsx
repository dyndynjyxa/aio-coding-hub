import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { toast } from "sonner";
import type { AppSettings } from "../../../services/settings";
import type { GatewayRectifierSettingsPatch } from "../../../services/settingsGatewayRectifier";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { SettingsRow } from "../../../ui/SettingsRow";
import { Switch } from "../../../ui/Switch";
import { NetworkSettingsCard } from "../NetworkSettingsCard";
import { WslSettingsCard } from "../WslSettingsCard";
import { AlertTriangle, Shield } from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerGeneralTabProps = {
  rectifierAvailable: CliManagerAvailability;
  rectifierSaving: boolean;
  rectifier: GatewayRectifierSettingsPatch;
  onPersistRectifier: (patch: Partial<GatewayRectifierSettingsPatch>) => Promise<void> | void;

  circuitBreakerNoticeEnabled: boolean;
  circuitBreakerNoticeSaving: boolean;
  onPersistCircuitBreakerNotice: (enable: boolean) => Promise<void> | void;

  codexSessionIdCompletionEnabled: boolean;
  codexSessionIdCompletionSaving: boolean;
  onPersistCodexSessionIdCompletion: (enable: boolean) => Promise<void> | void;

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
  rectifierSaving,
  rectifier,
  onPersistRectifier,
  circuitBreakerNoticeEnabled,
  circuitBreakerNoticeSaving,
  onPersistCircuitBreakerNotice,
  codexSessionIdCompletionEnabled,
  codexSessionIdCompletionSaving,
  onPersistCodexSessionIdCompletion,
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
  return (
    <div className="space-y-6">
      <div className="grid gap-6 md:grid-cols-2">
        <Card className="md:col-span-2 relative overflow-hidden">
          <div className="absolute top-0 right-0 p-4 opacity-5">
            <Shield className="h-32 w-32" />
          </div>
          <div className="relative z-10">
            <div className="mb-4 border-b border-slate-100 pb-4">
              <h2 className="text-lg font-semibold text-slate-900 flex items-center gap-2">
                <Shield className="h-5 w-5 text-blue-500" />
                网关整流器
              </h2>
              <p className="mt-1 text-sm text-slate-500">
                优化与 AI 服务的连接稳定性，自动修复常见响应问题。
              </p>
            </div>

            {rectifierAvailable === "unavailable" ? (
              <div className="text-sm text-slate-600 bg-slate-50 p-4 rounded-lg">
                仅在 Tauri Desktop 环境可用
              </div>
            ) : (
              <div className="space-y-4">
                <SettingsRow label="拦截 Anthropic Warmup 请求">
                  <Switch
                    checked={rectifier.intercept_anthropic_warmup_requests}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ intercept_anthropic_warmup_requests: checked })
                    }
                    disabled={rectifierSaving || rectifierAvailable !== "available"}
                  />
                </SettingsRow>
                <SettingsRow label="Thinking 签名整流器">
                  <Switch
                    checked={rectifier.enable_thinking_signature_rectifier}
                    onCheckedChange={(checked) =>
                      void onPersistRectifier({ enable_thinking_signature_rectifier: checked })
                    }
                    disabled={rectifierSaving || rectifierAvailable !== "available"}
                  />
                </SettingsRow>
                <div className="rounded-lg bg-slate-50 p-4 border border-slate-100">
                  <SettingsRow label="响应整流（FluxFix）">
                    <Switch
                      checked={rectifier.enable_response_fixer}
                      onCheckedChange={(checked) =>
                        void onPersistRectifier({ enable_response_fixer: checked })
                      }
                      disabled={rectifierSaving || rectifierAvailable !== "available"}
                    />
                  </SettingsRow>
                  {rectifier.enable_response_fixer && (
                    <div className="mt-2 space-y-2 pl-4 border-l-2 border-slate-200 ml-1">
                      <SettingsRow label="修复编码问题">
                        <Switch
                          checked={rectifier.response_fixer_fix_encoding}
                          onCheckedChange={(checked) =>
                            void onPersistRectifier({ response_fixer_fix_encoding: checked })
                          }
                          disabled={rectifierSaving || rectifierAvailable !== "available"}
                        />
                      </SettingsRow>
                      <SettingsRow label="修复 SSE 格式">
                        <Switch
                          checked={rectifier.response_fixer_fix_sse_format}
                          onCheckedChange={(checked) =>
                            void onPersistRectifier({ response_fixer_fix_sse_format: checked })
                          }
                          disabled={rectifierSaving || rectifierAvailable !== "available"}
                        />
                      </SettingsRow>
                      <SettingsRow label="修复截断的 JSON">
                        <Switch
                          checked={rectifier.response_fixer_fix_truncated_json}
                          onCheckedChange={(checked) =>
                            void onPersistRectifier({ response_fixer_fix_truncated_json: checked })
                          }
                          disabled={rectifierSaving || rectifierAvailable !== "available"}
                        />
                      </SettingsRow>
                    </div>
                  )}
                </div>

                <div className="rounded-lg bg-slate-50 p-4 border border-slate-100">
                  <SettingsRow label="Codex Session ID 补全">
                    <Switch
                      checked={codexSessionIdCompletionEnabled}
                      onCheckedChange={(checked) => void onPersistCodexSessionIdCompletion(checked)}
                      disabled={
                        codexSessionIdCompletionSaving || rectifierAvailable !== "available"
                      }
                    />
                  </SettingsRow>
                  <p className="mt-2 text-xs text-slate-500">
                    当 Codex 请求仅提供 session_id / x-session-id（请求头）或
                    prompt_cache_key（请求体）之一时，
                    自动补全另一侧；若两者均缺失，则生成并在短时间内稳定复用的会话标识。
                  </p>
                </div>
              </div>
            )}
          </div>
        </Card>

        <Card className="md:col-span-2">
          <div className="mb-4 flex items-start gap-4">
            <div className="p-2 bg-amber-50 rounded-lg text-amber-600">
              <AlertTriangle className="h-6 w-6" />
            </div>
            <div className="flex-1">
              <h3 className="text-base font-semibold text-slate-900">熔断通知</h3>
              <p className="mt-1 text-sm text-slate-500">
                当服务熔断触发或恢复时，主动发送系统通知。
                <br />
                <span className="text-xs text-amber-600/80">* 需在系统设置中授予通知权限</span>
              </p>
            </div>
            <div className="pt-1">
              {rectifierAvailable === "unavailable" ? (
                <span className="text-xs text-slate-400">不可用</span>
              ) : (
                <Switch
                  checked={circuitBreakerNoticeEnabled}
                  onCheckedChange={(checked) => void onPersistCircuitBreakerNotice(checked)}
                  disabled={circuitBreakerNoticeSaving || rectifierAvailable !== "available"}
                />
              )}
            </div>
          </div>
        </Card>

        {appSettings ? (
          <>
            <NetworkSettingsCard
              available={rectifierAvailable === "available"}
              saving={commonSettingsSaving}
              settings={appSettings}
              onPersistSettings={onPersistCommonSettings}
            />
            <WslSettingsCard
              available={rectifierAvailable === "available"}
              saving={commonSettingsSaving}
              settings={appSettings}
              onPersistSettings={onPersistCommonSettings}
            />
          </>
        ) : null}

        <Card className="md:col-span-2">
          <div className="mb-4 border-b border-slate-100 pb-4">
            <div className="font-semibold text-slate-900">超时策略</div>
            <p className="mt-1 text-sm text-slate-500">
              控制上游请求的超时行为。0 表示禁用（交由上游/网络自行超时）。
            </p>
          </div>

          {rectifierAvailable === "unavailable" ? (
            <div className="text-sm text-slate-600 bg-slate-50 p-4 rounded-lg">
              仅在 Tauri Desktop 环境可用
            </div>
          ) : (
            <div className="space-y-1">
              <SettingsRow label="首字节超时（0=禁用）">
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
                    className="w-24"
                    min={0}
                    max={3600}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">秒</span>
                </div>
              </SettingsRow>

              <SettingsRow label="流式空闲超时（0=禁用）">
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
                      if (!Number.isFinite(next) || next < 0 || next > 3600) {
                        toast("上游流式空闲超时必须为 0-3600 秒");
                        setUpstreamStreamIdleTimeoutSeconds(
                          appSettings.upstream_stream_idle_timeout_seconds
                        );
                        return;
                      }
                      void onPersistCommonSettings({ upstream_stream_idle_timeout_seconds: next });
                    }}
                    onKeyDown={blurOnEnter}
                    className="w-24"
                    min={0}
                    max={3600}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">秒</span>
                </div>
              </SettingsRow>

              <SettingsRow label="非流式总超时（0=禁用）">
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    value={upstreamRequestTimeoutNonStreamingSeconds}
                    onChange={(e) => {
                      const next = e.currentTarget.valueAsNumber;
                      if (Number.isFinite(next)) setUpstreamRequestTimeoutNonStreamingSeconds(next);
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
                    className="w-24"
                    min={0}
                    max={86400}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">秒</span>
                </div>
              </SettingsRow>
            </div>
          )}
        </Card>

        <Card className="md:col-span-2">
          <div className="mb-4 border-b border-slate-100 pb-4">
            <div className="font-semibold text-slate-900">熔断与重试</div>
            <p className="mt-1 text-sm text-slate-500">
              控制 Provider 失败后的冷却、重试与熔断行为。修改后建议重启网关以完全生效。
            </p>
          </div>

          {rectifierAvailable === "unavailable" ? (
            <div className="text-sm text-slate-600 bg-slate-50 p-4 rounded-lg">
              仅在 Tauri Desktop 环境可用
            </div>
          ) : (
            <div className="space-y-1">
              <SettingsRow label="Provider 冷却">
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
                    className="w-24"
                    min={0}
                    max={3600}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">秒</span>
                </div>
              </SettingsRow>

              <SettingsRow label="Ping 选择缓存 TTL">
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
                    className="w-24"
                    min={1}
                    max={3600}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">秒</span>
                </div>
              </SettingsRow>

              <SettingsRow label="熔断阈值">
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
                    className="w-24"
                    min={1}
                    max={50}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">次</span>
                </div>
              </SettingsRow>

              <SettingsRow label="熔断时长">
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
                    className="w-24"
                    min={1}
                    max={1440}
                    disabled={commonSettingsSaving || rectifierAvailable !== "available"}
                  />
                  <span className="text-sm text-slate-500">分钟</span>
                </div>
              </SettingsRow>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
