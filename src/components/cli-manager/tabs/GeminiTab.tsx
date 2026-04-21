import { useEffect, useState, type ReactNode } from "react";
import { toast } from "sonner";
import type {
  GeminiConfigPatch,
  GeminiConfigState,
  SimpleCliInfo,
} from "../../../services/cli/cliManager";
import { cn } from "../../../utils/cn";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { Select } from "../../../ui/Select";
import { Switch } from "../../../ui/Switch";
import { CliVersionBadge } from "../CliVersionBadge";
import {
  AlertTriangle,
  CheckCircle2,
  Cpu,
  FileJson,
  FolderOpen,
  RefreshCw,
  Settings,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerGeminiTabProps = {
  geminiAvailable: CliManagerAvailability;
  geminiLoading: boolean;
  geminiInfo: SimpleCliInfo | null;
  geminiConfigLoading: boolean;
  geminiConfigSaving: boolean;
  geminiConfig: GeminiConfigState | null;
  refreshGeminiInfo: () => Promise<void> | void;
  persistGeminiConfig: (patch: GeminiConfigPatch) => Promise<void> | void;
};

function SettingItem({
  label,
  subtitle,
  children,
  className,
}: {
  label: string;
  subtitle: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between",
        className
      )}
    >
      <div className="min-w-0">
        <div className="text-sm text-slate-700 dark:text-slate-300">{label}</div>
        <div className="mt-1 text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
          {subtitle}
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function boolOrDefault(value: boolean | null, fallback = false) {
  return value ?? fallback;
}

function stringOrDefault(value: string | null, fallback = "") {
  return (value ?? fallback).trim();
}

function formatNumberInput(value: number | null) {
  return value == null ? "" : String(value);
}

export function CliManagerGeminiTab({
  geminiAvailable,
  geminiLoading,
  geminiInfo,
  geminiConfigLoading,
  geminiConfigSaving,
  geminiConfig,
  refreshGeminiInfo,
  persistGeminiConfig,
}: CliManagerGeminiTabProps) {
  const [versionRefreshToken, setVersionRefreshToken] = useState(0);
  const [modelNameText, setModelNameText] = useState("");
  const [defaultApprovalModeText, setDefaultApprovalModeText] = useState("default");
  const [maxAttemptsText, setMaxAttemptsText] = useState("");
  const [maxSessionTurnsText, setMaxSessionTurnsText] = useState("");
  const [compressionThresholdText, setCompressionThresholdText] = useState("");
  const [uiThemeText, setUiThemeText] = useState("");
  const [uiInlineThinkingModeText, setUiInlineThinkingModeText] = useState("off");
  const [sessionRetentionMaxAgeText, setSessionRetentionMaxAgeText] = useState("");
  const [authTypeText, setAuthTypeText] = useState("");

  useEffect(() => {
    if (!geminiConfig) return;
    setModelNameText(geminiConfig.modelName ?? "");
    setDefaultApprovalModeText(stringOrDefault(geminiConfig.defaultApprovalMode, "default"));
    setMaxAttemptsText(formatNumberInput(geminiConfig.maxAttempts));
    setMaxSessionTurnsText(formatNumberInput(geminiConfig.modelMaxSessionTurns));
    setCompressionThresholdText(formatNumberInput(geminiConfig.modelCompressionThreshold));
    setUiThemeText(geminiConfig.uiTheme ?? "");
    setUiInlineThinkingModeText(stringOrDefault(geminiConfig.uiInlineThinkingMode, "off"));
    setSessionRetentionMaxAgeText(geminiConfig.sessionRetentionMaxAge ?? "");
    setAuthTypeText(geminiConfig.securityAuthSelectedType ?? "");
  }, [geminiConfig]);

  const loading = geminiLoading || geminiConfigLoading;
  const saving = geminiConfigSaving;
  const configDir = geminiConfig?.configDir ?? "—";
  const configPath = geminiConfig?.configPath ?? "—";

  async function refreshGeminiStatus() {
    try {
      await refreshGeminiInfo();
    } finally {
      setVersionRefreshToken((value) => value + 1);
    }
  }

  function revertNumberField(
    setter: (value: string) => void,
    currentValue: number | null | undefined
  ) {
    setter(currentValue == null ? "" : String(currentValue));
  }

  function parseIntegerInput(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    const value = Number(trimmed);
    if (!Number.isFinite(value) || !Number.isInteger(value)) return null;
    return value;
  }

  function parseFloatInput(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    const value = Number(trimmed);
    if (!Number.isFinite(value)) return null;
    return value;
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-slate-100 dark:border-slate-700">
          <div className="flex flex-col gap-4 p-6">
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="h-14 w-14 rounded-xl bg-slate-900/5 dark:bg-slate-700 flex items-center justify-center text-slate-700 dark:text-slate-300">
                  <Cpu className="h-8 w-8" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">Gemini</h2>
                  <div className="flex items-center gap-2 mt-1 flex-wrap">
                    {geminiAvailable === "available" && geminiInfo?.found ? (
                      <>
                        <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 dark:bg-green-900/30 px-2.5 py-0.5 text-xs font-medium text-green-700 dark:text-green-400 ring-1 ring-inset ring-green-600/20">
                          <CheckCircle2 className="h-3 w-3" />
                          已安装 {geminiInfo.version}
                        </span>
                        <CliVersionBadge
                          cliKey="gemini"
                          installedVersion={geminiInfo.version}
                          refreshToken={versionRefreshToken}
                          onUpdateComplete={refreshGeminiStatus}
                        />
                      </>
                    ) : geminiAvailable === "checking" || loading ? (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 dark:bg-blue-900/30 px-2.5 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400 ring-1 ring-inset ring-blue-600/20">
                        <RefreshCw className="h-3 w-3 animate-spin" />
                        检测中...
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-0.5 text-xs font-medium text-slate-600 dark:text-slate-400 ring-1 ring-inset ring-slate-500/10">
                        未检测到
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <Button
                onClick={() => void refreshGeminiStatus()}
                variant="secondary"
                size="sm"
                disabled={loading}
                className="gap-2"
              >
                <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
                刷新状态
              </Button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 mt-2">
              <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                  <FolderOpen className="h-3 w-3" />
                  配置目录
                </div>
                <div
                  className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                  title={configDir}
                >
                  {configDir}
                </div>
              </div>

              <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                  <FileJson className="h-3 w-3" />
                  settings.json
                </div>
                <div
                  className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                  title={configPath}
                >
                  {configPath}
                </div>
                {geminiConfig ? (
                  <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                    {geminiConfig.exists ? "已存在" : "不存在（保存时自动创建）"}
                  </div>
                ) : null}
              </div>

              <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                  <Cpu className="h-3 w-3" />
                  可执行文件
                </div>
                <div
                  className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                  title={geminiInfo?.executable_path ?? "—"}
                >
                  {geminiInfo?.executable_path ?? "—"}
                </div>
              </div>

              <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                  <Settings className="h-3 w-3" />
                  解析方式
                </div>
                <div
                  className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                  title={geminiInfo?.resolved_via ?? "—"}
                >
                  {geminiInfo?.resolved_via ?? "—"}
                </div>
                <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                  SHELL: {geminiInfo?.shell ?? "—"}
                </div>
              </div>
            </div>
          </div>
        </div>

        {geminiAvailable === "unavailable" ? (
          <div className="text-sm text-slate-600 dark:text-slate-400 text-center py-8">
            数据不可用
          </div>
        ) : !geminiInfo ? (
          <div className="text-sm text-slate-500 dark:text-slate-400 text-center py-8">
            暂无信息，请尝试刷新
          </div>
        ) : (
          <div className="p-6 space-y-6">
            {geminiConfig ? (
              <>
                <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
                  <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                    <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                    模型与行为
                  </h3>
                  <div className="divide-y divide-slate-100 dark:divide-slate-700">
                    <SettingItem
                      label="默认模型 (model.name)"
                      subtitle="留空会删除该配置，交给 Gemini CLI 默认行为。"
                    >
                      <Input
                        value={modelNameText}
                        onChange={(e) => setModelNameText(e.currentTarget.value)}
                        onBlur={() => void persistGeminiConfig({ modelName: modelNameText.trim() })}
                        placeholder="例如：gemini-2.5-pro"
                        className="font-mono w-[280px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="审批模式 (general.defaultApprovalMode)"
                      subtitle="default / auto_edit / plan。"
                    >
                      <Select
                        value={defaultApprovalModeText}
                        onChange={(e) => {
                          const value = e.currentTarget.value;
                          setDefaultApprovalModeText(value);
                          void persistGeminiConfig({ defaultApprovalMode: value });
                        }}
                        className="w-[220px] max-w-full font-mono"
                        disabled={saving}
                      >
                        <option value="default">default</option>
                        <option value="auto_edit">auto_edit</option>
                        <option value="plan">plan</option>
                      </Select>
                    </SettingItem>

                    <SettingItem
                      label="最大尝试次数 (general.maxAttempts)"
                      subtitle="输入整数；留空会恢复当前显示值。"
                    >
                      <Input
                        type="number"
                        value={maxAttemptsText}
                        onChange={(e) => setMaxAttemptsText(e.currentTarget.value)}
                        onBlur={() => {
                          const next = parseIntegerInput(maxAttemptsText);
                          if (next == null) {
                            revertNumberField(setMaxAttemptsText, geminiConfig.maxAttempts);
                            if (maxAttemptsText.trim()) {
                              toast.error("maxAttempts 必须为整数");
                            }
                            return;
                          }
                          void persistGeminiConfig({ maxAttempts: next });
                        }}
                        className="font-mono w-[180px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="会话轮次上限 (model.maxSessionTurns)"
                      subtitle="输入整数，支持 -1 表示 unlimited。"
                    >
                      <Input
                        type="number"
                        value={maxSessionTurnsText}
                        onChange={(e) => setMaxSessionTurnsText(e.currentTarget.value)}
                        onBlur={() => {
                          const next = parseIntegerInput(maxSessionTurnsText);
                          if (next == null) {
                            revertNumberField(
                              setMaxSessionTurnsText,
                              geminiConfig.modelMaxSessionTurns
                            );
                            if (maxSessionTurnsText.trim()) {
                              toast.error("maxSessionTurns 必须为整数");
                            }
                            return;
                          }
                          void persistGeminiConfig({ modelMaxSessionTurns: next });
                        }}
                        className="font-mono w-[180px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="压缩阈值 (model.compressionThreshold)"
                      subtitle="输入数字，例如 0.7。"
                    >
                      <Input
                        type="number"
                        step="0.1"
                        value={compressionThresholdText}
                        onChange={(e) => setCompressionThresholdText(e.currentTarget.value)}
                        onBlur={() => {
                          const next = parseFloatInput(compressionThresholdText);
                          if (next == null) {
                            revertNumberField(
                              setCompressionThresholdText,
                              geminiConfig.modelCompressionThreshold
                            );
                            if (compressionThresholdText.trim()) {
                              toast.error("compressionThreshold 必须为数字");
                            }
                            return;
                          }
                          void persistGeminiConfig({ modelCompressionThreshold: next });
                        }}
                        className="font-mono w-[180px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>
                  </div>
                </div>

                <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
                  <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                    <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                    界面设置
                  </h3>
                  <div className="divide-y divide-slate-100 dark:divide-slate-700">
                    <SettingItem label="主题 (ui.theme)" subtitle="输入主题名；留空会删除该配置。">
                      <Input
                        value={uiThemeText}
                        onChange={(e) => setUiThemeText(e.currentTarget.value)}
                        onBlur={() => void persistGeminiConfig({ uiTheme: uiThemeText.trim() })}
                        className="font-mono w-[220px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="隐藏 Banner (ui.hideBanner)"
                      subtitle="开启后隐藏欢迎 Banner。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.uiHideBanner)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ uiHideBanner: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem label="隐藏 Tips (ui.hideTips)" subtitle="开启后隐藏提示文本。">
                      <Switch
                        checked={boolOrDefault(geminiConfig.uiHideTips)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ uiHideTips: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem label="显示行号 (ui.showLineNumbers)" subtitle="开启后显示行号。">
                      <Switch
                        checked={boolOrDefault(geminiConfig.uiShowLineNumbers)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ uiShowLineNumbers: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="思考展示模式 (ui.inlineThinkingMode)"
                      subtitle="off / full。"
                    >
                      <Select
                        value={uiInlineThinkingModeText}
                        onChange={(e) => {
                          const value = e.currentTarget.value;
                          setUiInlineThinkingModeText(value);
                          void persistGeminiConfig({ uiInlineThinkingMode: value });
                        }}
                        className="w-[180px] max-w-full font-mono"
                        disabled={saving}
                      >
                        <option value="off">off</option>
                        <option value="full">full</option>
                      </Select>
                    </SettingItem>

                    <SettingItem
                      label="Vim 模式 (general.vimMode)"
                      subtitle="开启后启用 Vim 风格编辑交互。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.vimMode)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ vimMode: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>
                  </div>
                </div>

                <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
                  <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                    <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                    功能开关
                  </h3>
                  <div className="divide-y divide-slate-100 dark:divide-slate-700">
                    <SettingItem
                      label="自动更新 (general.enableAutoUpdate)"
                      subtitle="控制 CLI 自更新。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.enableAutoUpdate)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ enableAutoUpdate: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="通知 (general.enableNotifications)"
                      subtitle="控制通知开关。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.enableNotifications)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ enableNotifications: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="重试抓取错误 (general.retryFetchErrors)"
                      subtitle="开启后自动重试 fetch 失败。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.retryFetchErrors)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ retryFetchErrors: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="使用统计 (privacy.usageStatisticsEnabled)"
                      subtitle="控制匿名使用统计上报。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.usageStatisticsEnabled)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ usageStatisticsEnabled: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>
                  </div>
                </div>

                <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
                  <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                    <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                    会话与认证
                  </h3>
                  <div className="divide-y divide-slate-100 dark:divide-slate-700">
                    <SettingItem
                      label="会话保留 (general.sessionRetention.enabled)"
                      subtitle="开启后保留历史会话。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.sessionRetentionEnabled)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ sessionRetentionEnabled: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="会话保留时长 (general.sessionRetention.maxAge)"
                      subtitle="例如：30d, 7d。留空删除配置。"
                    >
                      <Input
                        value={sessionRetentionMaxAgeText}
                        onChange={(e) => setSessionRetentionMaxAgeText(e.currentTarget.value)}
                        onBlur={() =>
                          void persistGeminiConfig({
                            sessionRetentionMaxAge: sessionRetentionMaxAgeText.trim(),
                          })
                        }
                        placeholder="例如：30d"
                        className="font-mono w-[180px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="计划模式模型路由 (general.plan.modelRouting)"
                      subtitle="自动在 Pro/Flash 模型间切换。"
                    >
                      <Switch
                        checked={boolOrDefault(geminiConfig.planModelRouting)}
                        onCheckedChange={(checked) =>
                          void persistGeminiConfig({ planModelRouting: checked })
                        }
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="认证类型 (security.auth.selectedType)"
                      subtitle="例如：gemini-api-key。留空删除配置。"
                    >
                      <Input
                        value={authTypeText}
                        onChange={(e) => setAuthTypeText(e.currentTarget.value)}
                        onBlur={() =>
                          void persistGeminiConfig({
                            securityAuthSelectedType: authTypeText.trim(),
                          })
                        }
                        placeholder="例如：gemini-api-key"
                        className="font-mono w-[220px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>
                  </div>
                </div>
              </>
            ) : (
              <div className="text-sm text-slate-500 dark:text-slate-400 text-center py-8">
                暂无配置，请尝试刷新
              </div>
            )}
          </div>
        )}

        {geminiInfo?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 dark:bg-rose-900/30 p-4 text-sm text-rose-600 dark:text-rose-400 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {geminiInfo.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
