import { useEffect } from "react";
import { gatewayEventNames } from "../constants/gatewayEvents";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";
import { useCoalescedAsyncRefresh } from "./useCoalescedAsyncRefresh";

type UseRequestLogDetailSignalRefreshOptions = {
  traceId: string | null;
  enabled?: boolean;
  delayMs?: number;
  refresh: (traceId: string) => Promise<unknown>;
};

export function useRequestLogDetailSignalRefresh({
  traceId,
  enabled = true,
  delayMs = 400,
  refresh,
}: UseRequestLogDetailSignalRefreshOptions) {
  const normalizedTraceId = traceId?.trim() || null;
  const refreshEnabled = enabled && normalizedTraceId != null;
  const { clearQueued, schedule: scheduleRefresh } = useCoalescedAsyncRefresh<string, unknown>({
    enabled: refreshEnabled,
    delayMs,
    task: refresh,
    onError: (error, sourceTraceId) => {
      logToConsole("warn", "请求记录详情刷新失败", {
        traceId: sourceTraceId,
        error: String(error),
      });
      return null;
    },
  });

  useEffect(() => {
    if (!refreshEnabled || !normalizedTraceId) {
      return;
    }

    let cancelled = false;
    const requestSignalSub = subscribeGatewayEvent(gatewayEventNames.requestSignal, (payload) => {
      const requestSignal = normalizeGatewayRequestSignalEvent(payload);
      if (cancelled || !requestSignal) {
        return;
      }

      if (!isRequestSignalComplete(requestSignal) || requestSignal.trace_id !== normalizedTraceId) {
        return;
      }

      scheduleRefresh(requestSignal.trace_id);
    });

    void requestSignalSub.ready.catch((error) => {
      if (cancelled) {
        return;
      }

      requestSignalSub.unsubscribe();
      logToConsole("warn", "请求记录详情实时监听初始化失败", {
        traceId: normalizedTraceId,
        error: String(error),
      });
    });

    return () => {
      cancelled = true;
      clearQueued();
      requestSignalSub.unsubscribe();
    };
  }, [clearQueued, normalizedTraceId, refreshEnabled, scheduleRefresh]);
}
