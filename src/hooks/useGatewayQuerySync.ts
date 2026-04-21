import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { gatewayEventNames } from "../constants/gatewayEvents";
import { gatewayKeys, usageKeys } from "../query/keys";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import type { GatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";

const CIRCUIT_INVALIDATE_THROTTLE_MS = 500;
const STATUS_INVALIDATE_THROTTLE_MS = 300;
const USAGE_INVALIDATE_THROTTLE_MS = 1000;

export function useGatewayQuerySync() {
  const queryClient = useQueryClient();

  const circuitTimerRef = useRef<number | null>(null);
  const statusTimerRef = useRef<number | null>(null);
  const usageTimerRef = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    const invalidateCircuits = () => {
      queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
    };

    const invalidateStatus = () => {
      queryClient.invalidateQueries({ queryKey: gatewayKeys.status() });
    };

    const invalidateUsageDerived = () => {
      queryClient.invalidateQueries({ queryKey: usageKeys.all });
    };

    const scheduleInvalidateCircuits = () => {
      if (circuitTimerRef.current != null) return;
      circuitTimerRef.current = window.setTimeout(() => {
        circuitTimerRef.current = null;
        if (cancelled) return;
        invalidateCircuits();
      }, CIRCUIT_INVALIDATE_THROTTLE_MS);
    };

    const scheduleInvalidateStatus = () => {
      if (statusTimerRef.current != null) return;
      statusTimerRef.current = window.setTimeout(() => {
        statusTimerRef.current = null;
        if (cancelled) return;
        invalidateStatus();
      }, STATUS_INVALIDATE_THROTTLE_MS);
    };

    const scheduleInvalidateUsageDerived = () => {
      if (usageTimerRef.current != null) return;
      usageTimerRef.current = window.setTimeout(() => {
        usageTimerRef.current = null;
        if (cancelled) return;
        invalidateUsageDerived();
      }, USAGE_INVALIDATE_THROTTLE_MS);
    };

    const circuitSub = subscribeGatewayEvent(gatewayEventNames.circuit, () => {
      if (cancelled) return;
      scheduleInvalidateCircuits();
    });
    const statusSub = subscribeGatewayEvent(gatewayEventNames.status, () => {
      if (cancelled) return;
      scheduleInvalidateStatus();
    });
    const requestSignalSub = subscribeGatewayEvent<GatewayRequestSignalEvent>(
      gatewayEventNames.requestSignal,
      (payload) => {
        if (!payload || !isRequestSignalComplete(payload)) {
          return;
        }
        if (cancelled) return;
        scheduleInvalidateUsageDerived();
      }
    );

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

      if (circuitTimerRef.current != null) {
        window.clearTimeout(circuitTimerRef.current);
        circuitTimerRef.current = null;
      }
      if (statusTimerRef.current != null) {
        window.clearTimeout(statusTimerRef.current);
        statusTimerRef.current = null;
      }
      if (usageTimerRef.current != null) {
        window.clearTimeout(usageTimerRef.current);
        usageTimerRef.current = null;
      }
    };
  }, [queryClient]);
}
