import { useCallback, useEffect, useMemo } from "react";
import { gatewayEventNames } from "../constants/gatewayEvents";
import {
  useRequestLogsIncrementalRefreshMutation,
  useRequestLogsListAllQuery,
} from "../query/requestLogs";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";
import { useCoalescedAsyncRefresh } from "./useCoalescedAsyncRefresh";
import { useDocumentVisibility } from "./useDocumentVisibility";
import { useWindowForeground } from "./useWindowForeground";

type UseRequestLogsFeedOptions = {
  limit: number;
  enabled?: boolean;
  liveUpdatesEnabled?: boolean;
  liveUpdateIntervalMs?: number | false;
  refreshOnForeground?: boolean;
  foregroundThrottleMs?: number;
};

function resolveSignalRefreshWindowMs(input: number | false | undefined) {
  if (input === false) return 400;
  if (!Number.isFinite(input) || input == null) return 400;
  return Math.max(200, Math.min(2_000, Math.trunc(input)));
}

export function useRequestLogsFeed({
  limit,
  enabled = true,
  liveUpdatesEnabled = false,
  liveUpdateIntervalMs = false,
  refreshOnForeground = false,
  foregroundThrottleMs = 1000,
}: UseRequestLogsFeedOptions) {
  const foregroundActive = useDocumentVisibility();
  const requestLogsQuery = useRequestLogsListAllQuery(limit, { enabled });
  const incrementalRefreshMutation = useRequestLogsIncrementalRefreshMutation(limit);
  const liveRefreshEnabled = enabled && liveUpdatesEnabled && foregroundActive;
  const liveRefreshWindowMs = resolveSignalRefreshWindowMs(liveUpdateIntervalMs);
  const { schedule: scheduleLiveRefresh } = useCoalescedAsyncRefresh<void, unknown>({
    enabled: liveRefreshEnabled,
    delayMs: liveRefreshWindowMs,
    task: () => incrementalRefreshMutation.mutateAsync(),
    onError: (error) => {
      logToConsole("warn", "增量刷新请求记录失败", { limit, error: String(error) });
      return null;
    },
  });

  const refreshRequestLogs = useCallback(() => {
    return requestLogsQuery.refetch();
  }, [requestLogsQuery]);

  const refreshForForeground = useCallback(() => {
    if (!enabled) {
      return;
    }

    if (liveUpdatesEnabled) {
      scheduleLiveRefresh();
      return;
    }

    void requestLogsQuery.refetch();
  }, [enabled, liveUpdatesEnabled, requestLogsQuery, scheduleLiveRefresh]);

  useWindowForeground({
    enabled: enabled && refreshOnForeground,
    throttleMs: foregroundThrottleMs,
    onForeground: refreshForForeground,
  });

  useEffect(() => {
    if (!liveRefreshEnabled) {
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

      scheduleLiveRefresh();
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
      logToConsole("warn", "请求记录实时监听初始化失败", {
        stage: "useRequestLogsFeed",
        error: String(failedResult?.status === "rejected" ? failedResult.reason : "unknown"),
      });
    });

    return () => {
      cancelled = true;
      requestSignalSub.unsubscribe();
    };
  }, [liveRefreshEnabled, scheduleLiveRefresh]);

  const requestLogs = useMemo(() => requestLogsQuery.data ?? [], [requestLogsQuery.data]);
  const requestLogsLoading = requestLogsQuery.isLoading;
  const requestLogsRefreshing =
    (requestLogsQuery.isFetching && !requestLogsQuery.isLoading) ||
    incrementalRefreshMutation.isPending;
  const requestLogsAvailable: boolean | null = requestLogsQuery.isLoading
    ? null
    : requestLogsQuery.data != null;

  return {
    requestLogs,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsAvailable,
    refreshRequestLogs,
  };
}
