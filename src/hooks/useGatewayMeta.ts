import { useMemo } from "react";
import { DEFAULT_GATEWAY_PORT } from "../constants/gateway";
import type { GatewayStatus } from "../services/gateway/gateway";
import { useGatewayStatusQuery } from "../query/gateway";
import { useSettingsQuery } from "../query/settings";

export type GatewayAvailability = "checking" | "available" | "unavailable";

export type GatewayMeta = {
  gatewayAvailable: GatewayAvailability;
  gateway: GatewayStatus | null;
  preferredPort: number;
};

export function useGatewayMeta(): GatewayMeta {
  const settingsQuery = useSettingsQuery();
  const gatewayStatusQuery = useGatewayStatusQuery();

  return useMemo(() => {
    const preferredPort = settingsQuery.data?.preferred_port ?? DEFAULT_GATEWAY_PORT;

    if (gatewayStatusQuery.isLoading) {
      return {
        gatewayAvailable: "checking",
        gateway: gatewayStatusQuery.data ?? null,
        preferredPort,
      };
    }

    if (gatewayStatusQuery.isError) {
      return {
        gatewayAvailable: "unavailable",
        gateway: null,
        preferredPort,
      };
    }

    const gateway = gatewayStatusQuery.data ?? null;
    return {
      gatewayAvailable: gateway ? "available" : "unavailable",
      gateway,
      preferredPort,
    };
  }, [
    gatewayStatusQuery.data,
    gatewayStatusQuery.isError,
    gatewayStatusQuery.isLoading,
    settingsQuery.data?.preferred_port,
  ]);
}
