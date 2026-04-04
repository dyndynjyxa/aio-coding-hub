import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type { GatewayAvailability } from "../../hooks/useGatewayMeta";
import { gatewayKeys } from "../../query/keys";
import { useTheme } from "../../hooks/useTheme";
import { logToConsole } from "../../services/consoleLog";
import { gatewayStart, gatewayStop, type GatewayStatus } from "../../services/gateway";
import type { HomeUsagePeriod } from "../../services/settings";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { SettingsRow } from "../../ui/SettingsRow";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { CliPriorityOrderEditor } from "./CliPriorityOrderEditor";
import { HomeOverviewTabOrderEditor } from "./HomeOverviewTabOrderEditor";
import type { NoticePermissionStatus } from "./useSystemNotification";
import type { CliKey } from "../../services/providers";

type PersistKey = "preferred_port" | "log_retention_days";
type BooleanPersistKey =
  | "show_home_heatmap"
  | "show_home_usage"
  | "auto_start"
  | "start_minimized"
  | "tray_enabled";
type SettingsPersistPatch = Partial<{
  show_home_heatmap: boolean;
  show_home_usage: boolean;
  home_usage_period: HomeUsagePeriod;
  cli_priority_order: CliKey[];
  auto_start: boolean;
  start_minimized: boolean;
  tray_enabled: boolean;
}>;

const HOME_USAGE_PERIOD_OPTIONS: Array<{ value: HomeUsagePeriod; label: string }> = [
  { value: "last7", label: "最近7天" },
  { value: "last15", label: "最近15天" },
  { value: "last30", label: "最近30天" },
  { value: "month", label: "当月" },
];

export type SettingsMainColumnProps = {
  gateway: GatewayStatus | null;
  gatewayAvailable: GatewayAvailability;

  settingsReady: boolean;
  settingsReadErrorMessage: string | null;
  settingsWriteBlocked: boolean;
  settingsSaving: boolean;

  port: number;
  setPort: (next: number) => void;
  commitNumberField: (options: {
    key: PersistKey;
    next: number;
    min: number;
    max: number;
    invalidMessage: string;
  }) => void;

  showHomeHeatmap: boolean;
  setShowHomeHeatmap: (next: boolean) => void;
  showHomeUsage: boolean;
  setShowHomeUsage: (next: boolean) => void;
  homeUsagePeriod: HomeUsagePeriod;
  setHomeUsagePeriod: (next: HomeUsagePeriod) => void;
  cliPriorityOrder: CliKey[];
  setCliPriorityOrder: (next: CliKey[]) => void;
  autoStart: boolean;
  setAutoStart: (next: boolean) => void;
  startMinimized: boolean;
  setStartMinimized: (next: boolean) => void;
  trayEnabled: boolean;
  setTrayEnabled: (next: boolean) => void;
  logRetentionDays: number;
  setLogRetentionDays: (next: number) => void;
  requestPersist: (patch: SettingsPersistPatch) => void;

  noticePermissionStatus: NoticePermissionStatus;
  requestingNoticePermission: boolean;
  sendingNoticeTest: boolean;
  requestSystemNotificationPermission: () => Promise<void>;
  sendSystemNotificationTest: () => Promise<void>;
};

function blurOnEnter(e: ReactKeyboardEvent<HTMLInputElement>) {
  if (e.key === "Enter") e.currentTarget.blur();
}

export function SettingsMainColumn({
  gateway,
  gatewayAvailable,
  settingsReady,
  settingsReadErrorMessage,
  settingsWriteBlocked,
  settingsSaving,
  port,
  setPort,
  showHomeHeatmap,
  setShowHomeHeatmap,
  showHomeUsage,
  setShowHomeUsage,
  homeUsagePeriod,
  setHomeUsagePeriod,
  cliPriorityOrder,
  setCliPriorityOrder,
  commitNumberField,
  autoStart,
  setAutoStart,
  startMinimized,
  setStartMinimized,
  trayEnabled,
  setTrayEnabled,
  logRetentionDays,
  setLogRetentionDays,
  requestPersist,
  noticePermissionStatus,
  requestingNoticePermission,
  sendingNoticeTest,
  requestSystemNotificationPermission,
  sendSystemNotificationTest,
}: SettingsMainColumnProps) {
  const { theme, setTheme } = useTheme();
  const queryClient = useQueryClient();
  const settingsInputsDisabled = !settingsReady || settingsWriteBlocked || settingsSaving;
  const gatewayRestartDisabled =
    gatewayAvailable !== "available" || settingsWriteBlocked || settingsSaving;
  const gatewayStopDisabled =
    gatewayAvailable !== "available" || !gateway?.running || settingsSaving;

  return (
    <div className="space-y-6 lg:col-span-8">
      {settingsReadErrorMessage ? (
        <div
          role="alert"
          className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800"
        >
          {settingsReadErrorMessage}
        </div>
      ) : null}
      {/* 网关服务 */}
      <Card>
        <div className="mb-4 flex items-center justify-between border-b border-slate-100 dark:border-slate-700 pb-4">
          <div className="font-semibold text-slate-900 dark:text-slate-100">网关服务</div>
          <span
            className={cn(
              "rounded-full px-2.5 py-0.5 text-xs font-medium",
              gatewayAvailable === "checking" || gatewayAvailable === "unavailable"
                ? "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400"
                : gateway?.running
                  ? "bg-emerald-50 text-emerald-700"
                  : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400"
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
                  if (!Number.isFinite(desiredPort) || desiredPort < 1024 || desiredPort > 65535) {
                    toast("端口号必须为 1024-65535");
                    return;
                  }

                  if (gateway?.running) {
                    const stopped = await gatewayStop();
                    if (!stopped) {
                      toast("重启失败：无法停止网关");
                      return;
                    }
                    queryClient.setQueryData(gatewayKeys.status(), stopped);
                  }

                  const status = await gatewayStart(desiredPort);
                  if (!status) {
                    toast("启动失败：当前环境不可用或 command 未注册");
                    return;
                  }
                  queryClient.setQueryData(gatewayKeys.status(), status);
                  logToConsole("info", "启动本地网关", {
                    port: status.port,
                    base_url: status.base_url,
                  });
                  toast(gateway?.running ? "本地网关已重启" : "本地网关已启动");
                }}
                variant={gateway?.running ? "secondary" : "primary"}
                size="sm"
                disabled={gatewayRestartDisabled}
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
                  queryClient.setQueryData(gatewayKeys.status(), status);
                  logToConsole("info", "停止本地网关");
                  toast("本地网关已停止");
                }}
                variant="secondary"
                size="sm"
                disabled={gatewayStopDisabled}
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
              disabled={settingsInputsDisabled}
            />
          </SettingsRow>
        </div>
      </Card>

      {/* 参数配置 */}
      <Card>
        <div className="mb-4 border-b border-slate-100 dark:border-slate-700 pb-4">
          <div className="font-semibold text-slate-900 dark:text-slate-100">参数配置</div>
        </div>

        <div className="space-y-8">
          <div className="grid gap-6 xl:grid-cols-2 xl:items-start">
            {/* 系统设置 */}
            <div className="rounded-xl border border-slate-200/80 bg-slate-50/40 p-4 dark:border-slate-700 dark:bg-slate-800/30">
              <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                系统设置
              </h3>
              <div className="space-y-1">
                {(
                  [
                    {
                      label: "开机自启",
                      key: "auto_start" as const,
                      checked: autoStart,
                      setter: setAutoStart,
                      disabled: settingsInputsDisabled,
                    },
                    {
                      label: "静默启动",
                      key: "start_minimized" as const,
                      checked: startMinimized,
                      setter: setStartMinimized,
                      disabled: settingsInputsDisabled || !autoStart,
                    },
                    {
                      label: "托盘常驻",
                      key: "tray_enabled" as const,
                      checked: trayEnabled,
                      setter: setTrayEnabled,
                      disabled: settingsInputsDisabled,
                    },
                  ] satisfies {
                    label: string;
                    key: BooleanPersistKey;
                    checked: boolean;
                    setter: (v: boolean) => void;
                    disabled: boolean;
                  }[]
                ).map(({ label, key, checked, setter, disabled }) => (
                  <SettingsRow key={key} label={label}>
                    <Switch
                      checked={checked}
                      onCheckedChange={(next) => {
                        setter(next);
                        requestPersist({ [key]: next } as SettingsPersistPatch);
                      }}
                      disabled={disabled}
                    />
                  </SettingsRow>
                ))}
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
                      className="h-8 w-16 text-xs"
                      min={1}
                      max={3650}
                      disabled={settingsInputsDisabled}
                    />
                    <span className="text-sm text-slate-500 dark:text-slate-400">天</span>
                  </div>
                </SettingsRow>
              </div>
            </div>

            {/* 系统通知 */}
            <div className="rounded-xl border border-slate-200/80 bg-slate-50/40 p-4 dark:border-slate-700 dark:bg-slate-800/30">
              <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
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
                          ? "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400"
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

          {/* UI 偏好 */}
          <div className="rounded-xl border border-slate-200/80 bg-slate-50/40 p-4 dark:border-slate-700 dark:bg-slate-800/30">
            <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
              UI 偏好
            </h3>
            <div className="space-y-1">
              <SettingsRow label="主题">
                <div className="flex items-center gap-1 rounded-lg bg-slate-100 p-0.5 dark:bg-slate-800">
                  <button
                    type="button"
                    className={cn(
                      "flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs transition",
                      theme === "light"
                        ? "bg-white text-slate-900 shadow-sm dark:bg-slate-600 dark:text-slate-100"
                        : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                    )}
                    onClick={() => setTheme("light")}
                  >
                    Light
                  </button>
                  <button
                    type="button"
                    className={cn(
                      "flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs transition",
                      theme === "dark"
                        ? "bg-white text-slate-900 shadow-sm dark:bg-slate-600 dark:text-slate-100"
                        : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                    )}
                    onClick={() => setTheme("dark")}
                  >
                    Dark
                  </button>
                  <button
                    type="button"
                    className={cn(
                      "flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs transition",
                      theme === "system"
                        ? "bg-white text-slate-900 shadow-sm dark:bg-slate-600 dark:text-slate-100"
                        : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                    )}
                    onClick={() => setTheme("system")}
                  >
                    System
                  </button>
                </div>
              </SettingsRow>
              <SettingsRow label="首页用量范围">
                <div className="flex flex-wrap items-center gap-1 rounded-lg bg-slate-100 p-0.5 dark:bg-slate-800">
                  {HOME_USAGE_PERIOD_OPTIONS.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      className={cn(
                        "flex items-center justify-center rounded-md px-2.5 py-1.5 text-xs transition",
                        homeUsagePeriod === option.value
                          ? "bg-white text-slate-900 shadow-sm dark:bg-slate-600 dark:text-slate-100"
                          : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                      )}
                      onClick={() => {
                        setHomeUsagePeriod(option.value);
                        requestPersist({ home_usage_period: option.value });
                      }}
                      disabled={settingsInputsDisabled}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </SettingsRow>
              <SettingsRow label="首页概览排序">
                <div className="w-full max-w-md">
                  <HomeOverviewTabOrderEditor />
                </div>
              </SettingsRow>
              <SettingsRow label="CLI 优先顺序">
                <div className="w-full max-w-md">
                  <CliPriorityOrderEditor
                    order={cliPriorityOrder}
                    onChange={(nextOrder) => {
                      setCliPriorityOrder(nextOrder);
                      requestPersist({ cli_priority_order: nextOrder });
                    }}
                  />
                </div>
              </SettingsRow>
              {(
                [
                  {
                    label: "显示首页热力图",
                    key: "show_home_heatmap" as const,
                    checked: showHomeHeatmap,
                    setter: setShowHomeHeatmap,
                    disabled: settingsInputsDisabled,
                  },
                  {
                    label: "显示首页用量统计",
                    key: "show_home_usage" as const,
                    checked: showHomeUsage,
                    setter: setShowHomeUsage,
                    disabled: settingsInputsDisabled,
                  },
                ] satisfies {
                  label: string;
                  key: BooleanPersistKey;
                  checked: boolean;
                  setter: (v: boolean) => void;
                  disabled: boolean;
                }[]
              ).map(({ label, key, checked, setter, disabled }) => (
                <SettingsRow key={key} label={label}>
                  <Switch
                    checked={checked}
                    onCheckedChange={(next) => {
                      setter(next);
                      requestPersist({ [key]: next } as SettingsPersistPatch);
                    }}
                    disabled={disabled}
                  />
                </SettingsRow>
              ))}
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
