import { useCallback, useEffect } from "react";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { useCoalescedAsyncRefresh } from "../../../hooks/useCoalescedAsyncRefresh";
import { useWindowForeground } from "../../../hooks/useWindowForeground";
import { logToConsole } from "../../../services/consoleLog";
import { subscribeGatewayEvent } from "../../../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../../../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../../../services/gateway/requestLogState";

type RefreshSource =
  | "request_signal.complete"
  | "foreground"
  | "manual"
  | "request_activity.watchdog";

type ActiveRequestRefreshSource = "request_signal.start" | "request_signal.complete";

const ACTIVE_REQUEST_REFRESH_WINDOW_MS = 200;

type UseHomeFreshnessOwnerOptions = {
  overviewActive: boolean;
  foregroundActive: boolean;
  requestActivityPending?: boolean;
  requestLogsRefreshWindowMs?: number;
  requestActivityWatchdogIntervalMs?: number | false;
  foregroundThrottleMs?: number;
  onRefreshActiveRequests: () => Promise<unknown>;
  onRefreshRequestLogs: () => Promise<unknown>;
};

function resolveRequestLogsRefreshWindowMs(input: number | undefined) {
  if (!Number.isFinite(input) || input == null) return 1000;
  return Math.max(200, Math.min(2_000, Math.trunc(input)));
}

function resolveRequestActivityWatchdogIntervalMs(input: number | false | undefined) {
  if (input === false) return false;
  if (!Number.isFinite(input) || input == null) return 15_000;
  return Math.max(5_000, Math.min(60_000, Math.trunc(input)));
}

export function useHomeFreshnessOwner({
  overviewActive,
  foregroundActive,
  requestActivityPending = false,
  requestLogsRefreshWindowMs,
  requestActivityWatchdogIntervalMs,
  foregroundThrottleMs = 1000,
  onRefreshActiveRequests,
  onRefreshRequestLogs,
}: UseHomeFreshnessOwnerOptions) {
  // 事件驱动刷新（complete 信号 / 前台补拉 / 手动）只要求页面处于活跃路由：
  // 窗口在后台时信号照常触发拉取（低频、无轮询），回前台即是新数据。
  // watchdog 轮询仍仅在前台运行（active），避免后台周期性空转。
  const active = overviewActive && foregroundActive;
  const refreshWindowMs = resolveRequestLogsRefreshWindowMs(requestLogsRefreshWindowMs);
  const watchdogIntervalMs = resolveRequestActivityWatchdogIntervalMs(
    requestActivityWatchdogIntervalMs
  );
  const { schedule: scheduleActiveRequestsRefresh } = useCoalescedAsyncRefresh<
    ActiveRequestRefreshSource,
    unknown
  >({
    enabled: overviewActive,
    delayMs: ACTIVE_REQUEST_REFRESH_WINDOW_MS,
    task: () => onRefreshActiveRequests(),
    onError: (error, source) => {
      logToConsole("warn", "首页进行中请求快照刷新失败", {
        source,
        error: String(error),
      });
      return { error };
    },
  });
  const { flush: flushRequestLogs, schedule: scheduleRequestLogsRefresh } =
    useCoalescedAsyncRefresh<RefreshSource, unknown>({
      enabled: overviewActive,
      delayMs: refreshWindowMs,
      task: () => onRefreshRequestLogs(),
      onError: (error, source) => {
        logToConsole("warn", "首页请求记录刷新失败", {
          source,
          error: String(error),
        });
        return { error };
      },
    });

  const refreshRequestLogsNow = useCallback(() => {
    return flushRequestLogs("manual") ?? Promise.resolve(null);
  }, [flushRequestLogs]);

  useWindowForeground({
    enabled: overviewActive,
    throttleMs: foregroundThrottleMs,
    onForeground: () => {
      scheduleRequestLogsRefresh("foreground");
    },
  });

  useEffect(() => {
    if (!active || !requestActivityPending || watchdogIntervalMs === false) {
      return;
    }

    const intervalId = window.setInterval(() => {
      scheduleRequestLogsRefresh("request_activity.watchdog");
    }, watchdogIntervalMs);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [active, requestActivityPending, scheduleRequestLogsRefresh, watchdogIntervalMs]);

  useEffect(() => {
    if (!overviewActive) {
      return;
    }

    let cancelled = false;
    const requestSignalSub = subscribeGatewayEvent(gatewayEventNames.requestSignal, (payload) => {
      const requestSignal = normalizeGatewayRequestSignalEvent(payload);
      if (cancelled || !requestSignal) {
        return;
      }

      scheduleActiveRequestsRefresh(`request_signal.${requestSignal.phase}`);
      if (!isRequestSignalComplete(requestSignal)) {
        return;
      }

      scheduleRequestLogsRefresh("request_signal.complete");
    });

    void Promise.allSettled([requestSignalSub.ready]).then((results) => {
      if (cancelled) {
        return;
      }

      const subscribeFailed = results.some((result) => result.status === "rejected");
      if (!subscribeFailed) {
        return;
      }

      requestSignalSub.unsubscribe();
      const failedResult = results.find((result) => result.status === "rejected");
      logToConsole("warn", "首页请求记录实时监听初始化失败", {
        stage: "useHomeFreshnessOwner",
        error: String(failedResult?.status === "rejected" ? failedResult.reason : "unknown"),
      });
    });

    return () => {
      cancelled = true;
      requestSignalSub.unsubscribe();
    };
  }, [overviewActive, scheduleActiveRequestsRefresh, scheduleRequestLogsRefresh]);

  return {
    refreshRequestLogsNow,
  };
}
