import { useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { gatewayEventNames } from "../constants/gatewayEvents";
import { gatewayKeys, providerLimitUsageKeys, usageKeys } from "../query/keys";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";
import { useCoalescedAsyncRefresh } from "./useCoalescedAsyncRefresh";

const CIRCUIT_INVALIDATE_THROTTLE_MS = 500;
const STATUS_INVALIDATE_THROTTLE_MS = 300;
const USAGE_INVALIDATE_THROTTLE_MS = 1000;

export function useGatewayQuerySync() {
  const queryClient = useQueryClient();

  const logInvalidationError = useCallback((source: string, error: unknown) => {
    logToConsole("warn", "网关查询缓存失效失败", {
      stage: "useGatewayQuerySync",
      source,
      error: String(error),
    });
    return null;
  }, []);

  const { schedule: scheduleInvalidateCircuits } = useCoalescedAsyncRefresh<void, unknown>({
    enabled: true,
    delayMs: CIRCUIT_INVALIDATE_THROTTLE_MS,
    task: () => queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() }),
    onError: (error) => logInvalidationError("circuit", error),
  });

  const { schedule: scheduleInvalidateStatus } = useCoalescedAsyncRefresh<void, unknown>({
    enabled: true,
    delayMs: STATUS_INVALIDATE_THROTTLE_MS,
    task: () => queryClient.invalidateQueries({ queryKey: gatewayKeys.status() }),
    onError: (error) => logInvalidationError("status", error),
  });

  const { schedule: scheduleInvalidateUsageDerived } = useCoalescedAsyncRefresh<void, unknown>({
    enabled: true,
    delayMs: USAGE_INVALIDATE_THROTTLE_MS,
    task: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: usageKeys.all }),
        queryClient.invalidateQueries({ queryKey: providerLimitUsageKeys.all }),
      ]);
    },
    onError: (error) => logInvalidationError("usage", error),
  });

  useEffect(() => {
    let cancelled = false;

    const circuitSub = subscribeGatewayEvent(gatewayEventNames.circuit, () => {
      if (cancelled) return;
      scheduleInvalidateCircuits();
    });
    const statusSub = subscribeGatewayEvent(gatewayEventNames.status, () => {
      if (cancelled) return;
      scheduleInvalidateStatus();
    });
    const requestSignalSub = subscribeGatewayEvent(gatewayEventNames.requestSignal, (payload) => {
      const requestSignal = normalizeGatewayRequestSignalEvent(payload);
      if (!requestSignal || !isRequestSignalComplete(requestSignal)) {
        return;
      }
      if (cancelled) return;
      scheduleInvalidateUsageDerived();
    });

    void Promise.allSettled([circuitSub.ready, statusSub.ready, requestSignalSub.ready]).then(
      (results) => {
        if (cancelled) return;

        const subscribeFailed = results.some((result) => result.status === "rejected");
        if (!subscribeFailed) return;

        circuitSub.unsubscribe();
        statusSub.unsubscribe();
        requestSignalSub.unsubscribe();

        const failedResult = results.find((result) => result.status === "rejected");
        logToConsole("warn", "网关查询同步监听初始化失败", {
          stage: "useGatewayQuerySync",
          error: String(failedResult?.status === "rejected" ? failedResult.reason : "unknown"),
        });
      }
    );

    return () => {
      cancelled = true;
      circuitSub.unsubscribe();
      statusSub.unsubscribe();
      requestSignalSub.unsubscribe();
    };
  }, [scheduleInvalidateCircuits, scheduleInvalidateStatus, scheduleInvalidateUsageDerived]);
}
