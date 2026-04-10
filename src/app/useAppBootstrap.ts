import { useEffect } from "react";
import { useAsyncListener } from "../hooks/useAsyncListener";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { useGatewayQuerySync } from "../hooks/useGatewayQuerySync";
import { useStartupTask } from "../hooks/useStartupTask";
import { cliProxyStatusAll } from "../services/cli/cliProxy";
import { logToConsole } from "../services/consoleLog";
import {
  registerBackgroundTask,
  setBackgroundTaskSchedulerForeground,
  startBackgroundTaskScheduler,
} from "../services/backgroundTasks";
import { listenAppHeartbeat } from "../services/app/appHeartbeat";
import { setCacheAnomalyMonitorEnabled } from "../services/gateway/cacheAnomalyMonitor";
import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { settingsGet } from "../services/settings/settings";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "../services/app/startup";
import {
  listenTaskCompleteNotifyEvents,
  setTaskCompleteNotifyEnabled,
} from "../services/notification/taskCompleteNotifyEvents";
import { queryClient } from "../query/queryClient";
import { cliProxyKeys, updaterKeys } from "../query/keys";
import { updateCheckNow } from "../hooks/useUpdateMeta";

export function useAppBootstrap() {
  useGatewayQuerySync();
  const documentVisible = useDocumentVisibility();

  useAsyncListener(listenAppHeartbeat, "listenAppHeartbeat", "应用心跳监听初始化失败");
  useAsyncListener(listenGatewayEvents, "listenGatewayEvents", "网关事件监听初始化失败");
  useAsyncListener(listenNoticeEvents, "listenNoticeEvents", "通知事件监听初始化失败");
  useAsyncListener(
    listenTaskCompleteNotifyEvents,
    "listenTaskCompleteNotifyEvents",
    "任务结束提醒监听初始化失败"
  );

  useEffect(() => {
    settingsGet()
      .then((settings) => {
        if (!settings) return;
        setCacheAnomalyMonitorEnabled(settings.enable_cache_anomaly_monitor ?? false);
        setTaskCompleteNotifyEnabled(settings.enable_task_complete_notify ?? true);
      })
      .catch((error) => {
        logToConsole("warn", "启动缓存异常监测开关同步失败", {
          stage: "startupSyncCacheAnomalyMonitorSwitch",
          error: String(error),
        });
      });
  }, []);

  useStartupTask(startupSyncModelPricesOnce, "startupSyncModelPricesOnce", "启动模型定价同步失败");
  useStartupTask(
    startupSyncDefaultPromptsFromFilesOncePerSession,
    "startupSyncDefaultPromptsFromFilesOncePerSession",
    "启动默认提示词同步失败"
  );

  useEffect(() => {
    const unregisterProxyTask = registerBackgroundTask({
      taskId: "cli-proxy-consistency",
      intervalMs: 15_000,
      runOnAppStart: true,
      foregroundOnly: true,
      visibilityTriggers: ["home-overview-visible"],
      run: async () => {
        await queryClient.fetchQuery({
          queryKey: cliProxyKeys.statusAll(),
          queryFn: () => cliProxyStatusAll(),
          staleTime: 0,
        });
      },
    });
    const unregisterUpdateTask = registerBackgroundTask({
      taskId: "app-update-check",
      intervalMs: 300_000,
      runOnAppStart: true,
      foregroundOnly: true,
      visibilityTriggers: [],
      run: async (context) => {
        const options =
          context.trigger === "manual"
            ? {
                silent: false,
                openDialogIfUpdate: true,
              }
            : {
                silent: true,
                openDialogIfUpdate: false,
              };
        await updateCheckNow(options);
        queryClient.invalidateQueries({ queryKey: updaterKeys.check() });
      },
    });

    startBackgroundTaskScheduler();

    return () => {
      unregisterProxyTask();
      unregisterUpdateTask();
    };
  }, []);

  useEffect(() => {
    setBackgroundTaskSchedulerForeground(documentVisible);
  }, [documentVisible]);
}
