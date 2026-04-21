import { useCallback, useEffect, useMemo, useRef } from "react";
import { gatewayEventNames } from "../constants/gatewayEvents";
import { useRequestLogsIncrementalRefreshMutation, useRequestLogsListAllQuery } from "../query/requestLogs";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import type { GatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";
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
  const liveRefreshTimerRef = useRef<number | null>(null);
  const liveRefreshQueuedRef = useRef(false);
  const liveRefreshInFlightRef = useRef(false);
  const liveRefreshEnabledRef = useRef(liveRefreshEnabled);

  const clearQueuedLiveRefresh = useCallback(() => {
    if (liveRefreshTimerRef.current != null) {
      window.clearTimeout(liveRefreshTimerRef.current);
      liveRefreshTimerRef.current = null;
    }
    liveRefreshQueuedRef.current = false;
  }, []);

  const flushLiveRefresh = useCallback(() => {
    if (!liveRefreshEnabledRef.current) {
      clearQueuedLiveRefresh();
      return;
    }

    if (liveRefreshInFlightRef.current) {
      liveRefreshQueuedRef.current = true;
      return;
    }

    liveRefreshQueuedRef.current = false;
    liveRefreshInFlightRef.current = true;

    void incrementalRefreshMutation
      .mutateAsync()
      .catch((error) => {
        logToConsole("warn", "增量刷新请求记录失败", { limit, error: String(error) });
      })
      .finally(() => {
        liveRefreshInFlightRef.current = false;
        if (!liveRefreshQueuedRef.current || !liveRefreshEnabledRef.current) {
          liveRefreshQueuedRef.current = false;
          return;
        }
        liveRefreshQueuedRef.current = false;
        flushLiveRefresh();
      });
  }, [clearQueuedLiveRefresh, incrementalRefreshMutation, limit]);

  const scheduleLiveRefresh = useCallback(() => {
    if (!liveRefreshEnabled) {
      return;
    }
    if (liveRefreshTimerRef.current != null) {
      liveRefreshQueuedRef.current = true;
      return;
    }

    liveRefreshTimerRef.current = window.setTimeout(() => {
      liveRefreshTimerRef.current = null;
      flushLiveRefresh();
    }, liveRefreshWindowMs);
  }, [flushLiveRefresh, liveRefreshEnabled, liveRefreshWindowMs]);

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
    const requestSignalSub = subscribeGatewayEvent<GatewayRequestSignalEvent>(
      gatewayEventNames.requestSignal,
      (payload) => {
        if (cancelled || !payload) {
          return;
        }

        if (!isRequestSignalComplete(payload)) {
          return;
        }

        scheduleLiveRefresh();
      }
    );

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

  useEffect(() => {
    liveRefreshEnabledRef.current = liveRefreshEnabled;
    if (!liveRefreshEnabled) {
      clearQueuedLiveRefresh();
    }
  }, [clearQueuedLiveRefresh, liveRefreshEnabled]);

  useEffect(() => {
    return () => {
      clearQueuedLiveRefresh();
      liveRefreshInFlightRef.current = false;
    };
  }, [clearQueuedLiveRefresh]);

  const requestLogs = useMemo(() => requestLogsQuery.data ?? [], [requestLogsQuery.data]);
  const requestLogsLoading = requestLogsQuery.isLoading;
  const requestLogsRefreshing =
    (requestLogsQuery.isFetching && !requestLogsQuery.isLoading) || incrementalRefreshMutation.isPending;
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
