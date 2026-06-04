import { useCallback, useEffect, useRef } from "react";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { useCoalescedAsyncRefresh } from "../../../hooks/useCoalescedAsyncRefresh";
import { useWindowForeground } from "../../../hooks/useWindowForeground";
import { logToConsole } from "../../../services/consoleLog";
import { subscribeGatewayEvent } from "../../../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../../../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../../../services/gateway/requestLogState";

type RefreshSource = "request_signal.complete" | "foreground" | "manual";

type UseHomeFreshnessOwnerOptions = {
  overviewActive: boolean;
  foregroundActive: boolean;
  requestLogsRefreshWindowMs?: number;
  foregroundThrottleMs?: number;
  onRefreshRequestLogs: () => Promise<unknown>;
};

function resolveRequestLogsRefreshWindowMs(input: number | undefined) {
  if (!Number.isFinite(input) || input == null) return 1000;
  return Math.max(200, Math.min(2_000, Math.trunc(input)));
}

export function useHomeFreshnessOwner({
  overviewActive,
  foregroundActive,
  requestLogsRefreshWindowMs,
  foregroundThrottleMs = 1000,
  onRefreshRequestLogs,
}: UseHomeFreshnessOwnerOptions) {
  const active = overviewActive && foregroundActive;
  const refreshWindowMs = resolveRequestLogsRefreshWindowMs(requestLogsRefreshWindowMs);
  const previousActiveRef = useRef(active);
  const {
    clearQueued: clearQueuedRefresh,
    flush: flushRequestLogs,
    schedule: scheduleRequestLogsRefresh,
  } = useCoalescedAsyncRefresh<RefreshSource, unknown>({
    enabled: active,
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
    const wasActive = previousActiveRef.current;
    previousActiveRef.current = active;
    if (!active) {
      clearQueuedRefresh();
      return;
    }

    if (!wasActive) {
      scheduleRequestLogsRefresh("foreground");
    }
  }, [active, clearQueuedRefresh, scheduleRequestLogsRefresh]);

  useEffect(() => {
    if (!active) {
      return;
    }

    let cancelled = false;
    const requestSignalSub = subscribeGatewayEvent(gatewayEventNames.requestSignal, (payload) => {
      const requestSignal = normalizeGatewayRequestSignalEvent(payload);
      if (cancelled || !requestSignal) {
        return;
      }

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
  }, [active, scheduleRequestLogsRefresh]);

  return {
    refreshRequestLogsNow,
  };
}
