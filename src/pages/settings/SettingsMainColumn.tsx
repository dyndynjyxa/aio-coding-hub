import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type { GatewayAvailability } from "../../hooks/useGatewayMeta";
import { gatewayKeys } from "../../query/keys";
import { useTheme } from "../../hooks/useTheme";
import { logToConsole } from "../../services/consoleLog";
import { gatewayStart, gatewayStop, type GatewayStatus } from "../../services/gateway";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { SettingsRow } from "../../ui/SettingsRow";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import type { NoticePermissionStatus } from "./useSystemNotification";

type PersistKey = "preferred_port" | "log_retention_days";
type BooleanPersistKey = "show_home_heatmap" | "auto_start" | "start_minimized" | "tray_enabled";

export type SettingsMainColumnProps = {
  gateway: GatewayStatus | null;
  gatewayAvailable: GatewayAvailability;

  settingsReady: boolean;

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
  autoStart: boolean;
  setAutoStart: (next: boolean) => void;
  startMinimized: boolean;
  setStartMinimized: (next: boolean) => void;
  trayEnabled: boolean;
  setTrayEnabled: (next: boolean) => void;
  logRetentionDays: number;
  setLogRetentionDays: (next: number) => void;
  requestPersist: (patch: Partial<Record<BooleanPersistKey, boolean>>) => void;

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
  port,
  setPort,
  showHomeHeatmap,
  setShowHomeHeatmap,
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

  return (
    <div className="space-y-6 lg:col-span-8">
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
                  queryClient.setQueryData(gatewayKeys.status(), status);
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
        <div className="mb-4 border-b border-slate-100 dark:border-slate-700 pb-4">
          <div className="font-semibold text-slate-900 dark:text-slate-100">参数配置</div>
        </div>

        <div className="space-y-8">
          {/* 系统偏好 */}
          <div>
            <h3 className="mb-3 text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
              系统偏好
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
              {(
                [
                  {
                    label: "显示首页热力图",
                    key: "show_home_heatmap" as const,
                    checked: showHomeHeatmap,
                    setter: setShowHomeHeatmap,
                    disabled: !settingsReady,
                  },
                  {
                    label: "开机自启",
                    key: "auto_start" as const,
                    checked: autoStart,
                    setter: setAutoStart,
                    disabled: !settingsReady,
                  },
                  {
                    label: "静默启动",
                    key: "start_minimized" as const,
                    checked: startMinimized,
                    setter: setStartMinimized,
                    disabled: !settingsReady || !autoStart,
                  },
                  {
                    label: "托盘常驻",
                    key: "tray_enabled" as const,
                    checked: trayEnabled,
                    setter: setTrayEnabled,
                    disabled: !settingsReady,
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
                      const patch: Partial<Record<BooleanPersistKey, boolean>> = {};
                      patch[key] = next;
                      requestPersist(patch);
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
                    className="w-24"
                    min={1}
                    max={3650}
                    disabled={!settingsReady}
                  />
                  <span className="text-sm text-slate-500 dark:text-slate-400">天</span>
                </div>
              </SettingsRow>
            </div>
          </div>

          {/* 系统通知 */}
          <div>
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
      </Card>
    </div>
  );
}
