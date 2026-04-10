import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { gatewayEventNames } from "../constants/gatewayEvents";
import { gatewayKeys, requestLogsKeys, usageKeys } from "../query/keys";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";

const CIRCUIT_INVALIDATE_THROTTLE_MS = 500;
const STATUS_INVALIDATE_THROTTLE_MS = 300;
const REQUEST_INVALIDATE_THROTTLE_MS = 1000;

export function useGatewayQuerySync() {
  const queryClient = useQueryClient();

  const circuitTimerRef = useRef<number | null>(null);
  const statusTimerRef = useRef<number | null>(null);
  const requestTimerRef = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    const invalidateCircuits = () => {
      queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
    };

    const invalidateStatus = () => {
      queryClient.invalidateQueries({ queryKey: gatewayKeys.status() });
    };

    const invalidateRequestDerived = () => {
      queryClient.invalidateQueries({ queryKey: requestLogsKeys.lists() });
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

    const scheduleInvalidateRequestDerived = () => {
      if (requestTimerRef.current != null) return;
      requestTimerRef.current = window.setTimeout(() => {
        requestTimerRef.current = null;
        if (cancelled) return;
        invalidateRequestDerived();
      }, REQUEST_INVALIDATE_THROTTLE_MS);
    };

    const circuitSub = subscribeGatewayEvent(gatewayEventNames.circuit, () => {
      if (cancelled) return;
      scheduleInvalidateCircuits();
    });
    const statusSub = subscribeGatewayEvent(gatewayEventNames.status, () => {
      if (cancelled) return;
      scheduleInvalidateStatus();
    });
    const requestSub = subscribeGatewayEvent(gatewayEventNames.request, () => {
      if (cancelled) return;
      scheduleInvalidateRequestDerived();
    });

    void Promise.allSettled([circuitSub.ready, statusSub.ready, requestSub.ready]).then(
      (results) => {
        if (cancelled) return;

        const subscribeFailed = results.some((result) => result.status === "rejected");
        if (!subscribeFailed) return;

        circuitSub.unsubscribe();
        statusSub.unsubscribe();
        requestSub.unsubscribe();

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
      requestSub.unsubscribe();

      if (circuitTimerRef.current != null) {
        window.clearTimeout(circuitTimerRef.current);
        circuitTimerRef.current = null;
      }
      if (statusTimerRef.current != null) {
        window.clearTimeout(statusTimerRef.current);
        statusTimerRef.current = null;
      }
      if (requestTimerRef.current != null) {
        window.clearTimeout(requestTimerRef.current);
        requestTimerRef.current = null;
      }
    };
  }, [queryClient]);
}
