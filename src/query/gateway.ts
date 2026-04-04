import { useMemo } from "react";
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  gatewayCircuitResetCli,
  gatewayCircuitResetProvider,
  gatewayCircuitStatus,
  gatewaySessionsList,
  gatewayStatus,
  type GatewayProviderCircuitStatus,
} from "../services/gateway";
import type { CliKey } from "../services/providers";
import { gatewayKeys } from "./keys";

export type GatewayCircuitDerivedState = {
  isOpen: boolean;
  isUnavailable: boolean;
  unavailableUntil: number | null;
};

export type GatewayCircuitDerivedRow = GatewayCircuitDerivedState & {
  row: GatewayProviderCircuitStatus;
};

export type GatewayCircuitRowsSummary = {
  byProviderId: Record<number, GatewayProviderCircuitStatus>;
  unavailableRows: GatewayCircuitDerivedRow[];
  hasUnavailable: boolean;
  hasUnavailableWithoutUntil: boolean;
  earliestUnavailableUntil: number | null;
};

function normalizeGatewayCircuitUnix(value: number | null | undefined) {
  return value != null && Number.isFinite(value) ? value : null;
}

export function getGatewayCircuitDerivedState(
  row: GatewayProviderCircuitStatus | null | undefined
): GatewayCircuitDerivedState {
  // HALF_OPEN 表示已允许试探请求，不应继续作为“当前熔断/不可用”展示。
  const isOpen = row?.state === "OPEN";
  const cooldownUntil = normalizeGatewayCircuitUnix(row?.cooldown_until);
  const openUntil = row?.state === "OPEN" ? normalizeGatewayCircuitUnix(row?.open_until) : null;
  const unavailableUntil =
    openUntil == null
      ? cooldownUntil
      : cooldownUntil == null
        ? openUntil
        : Math.max(openUntil, cooldownUntil);

  return {
    isOpen,
    isUnavailable: isOpen || cooldownUntil != null,
    unavailableUntil,
  };
}

export function summarizeGatewayCircuitRows(
  rows: readonly GatewayProviderCircuitStatus[] | null | undefined
): GatewayCircuitRowsSummary {
  const byProviderId: Record<number, GatewayProviderCircuitStatus> = {};
  const unavailableRows: GatewayCircuitDerivedRow[] = [];
  let earliestUnavailableUntil: number | null = null;
  let hasUnavailableWithoutUntil = false;

  for (const row of rows ?? []) {
    byProviderId[row.provider_id] = row;

    const derived = getGatewayCircuitDerivedState(row);
    if (!derived.isUnavailable) continue;

    unavailableRows.push({ row, ...derived });

    if (derived.unavailableUntil == null) {
      hasUnavailableWithoutUntil = true;
      continue;
    }

    if (earliestUnavailableUntil == null || derived.unavailableUntil < earliestUnavailableUntil) {
      earliestUnavailableUntil = derived.unavailableUntil;
    }
  }

  return {
    byProviderId,
    unavailableRows,
    hasUnavailable: unavailableRows.length > 0,
    hasUnavailableWithoutUntil,
    earliestUnavailableUntil,
  };
}

export function useGatewayStatusQuery(options?: {
  enabled?: boolean;
  refetchIntervalMs?: number | false;
}) {
  return useQuery({
    queryKey: gatewayKeys.status(),
    queryFn: () => gatewayStatus(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchIntervalInBackground: true,
  });
}

export function useGatewayCircuitStatusQuery(cliKey: CliKey) {
  return useQuery({
    queryKey: gatewayKeys.circuitStatus(cliKey),
    queryFn: () => gatewayCircuitStatus(cliKey),
    enabled: true,
    placeholderData: keepPreviousData,
  });
}

export function useGatewayCircuitByProviderId(cliKey: CliKey) {
  const query = useGatewayCircuitStatusQuery(cliKey);
  const byId = useMemo(() => summarizeGatewayCircuitRows(query.data).byProviderId, [query.data]);

  return { ...query, circuitByProviderId: byId };
}

export function useGatewaySessionsListQuery(
  limit: number,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  return useQuery({
    queryKey: gatewayKeys.sessionsList(limit),
    queryFn: () => gatewaySessionsList(limit),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchIntervalInBackground: true,
  });
}

export function useGatewayCircuitResetProviderMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey?: CliKey | null; providerId: number }) =>
      gatewayCircuitResetProvider(input.providerId),
    onSuccess: (_ok, input) => {
      if (input.cliKey) {
        queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(input.cliKey) });
        return;
      }
      queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
    },
  });
}

export function useGatewayCircuitResetCliMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey }) => gatewayCircuitResetCli(input.cliKey),
    onSuccess: (_count, input) => {
      queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(input.cliKey) });
    },
  });
}
