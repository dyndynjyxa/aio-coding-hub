import { useEffect, useMemo, useRef } from "react";
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  gatewayStart,
  gatewayStop,
  gatewayCircuitResetCli,
  gatewayCircuitResetProvider,
  gatewayCircuitStatus,
  GATEWAY_SESSIONS_DEFAULT_LIMIT,
  gatewaySessionsList,
  gatewayStatus,
  normalizeGatewaySessionsLimit,
  validateGatewayCliKey,
  type GatewayProviderCircuitStatus,
} from "../services/gateway/gateway";
import type { CliKey } from "../services/providers/providers";
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

export function useGatewayStartMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { preferredPort?: number | null }) =>
      gatewayStart(input.preferredPort ?? undefined),
    onSuccess: (status) => {
      if (!status) return;
      queryClient.setQueryData(gatewayKeys.status(), status);
    },
  });
}

export function useGatewayStopMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => gatewayStop(),
    onSuccess: (status) => {
      if (!status) return;
      queryClient.setQueryData(gatewayKeys.status(), status);
    },
  });
}

export function useGatewayCircuitStatusQuery(cliKey: CliKey) {
  const normalizedCliKey = validateGatewayCliKey(cliKey);

  return useQuery({
    queryKey: gatewayKeys.circuitStatus(normalizedCliKey),
    queryFn: () => gatewayCircuitStatus(normalizedCliKey),
    enabled: true,
    placeholderData: keepPreviousData,
  });
}

export function useGatewayCircuitByProviderId(cliKey: CliKey) {
  const normalizedCliKey = validateGatewayCliKey(cliKey);
  const query = useGatewayCircuitStatusQuery(normalizedCliKey);
  const byId = useMemo(() => summarizeGatewayCircuitRows(query.data).byProviderId, [query.data]);

  return { ...query, circuitByProviderId: byId };
}

export function useGatewaySessionsListQuery(
  limit?: number | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  const normalizedLimit = normalizeGatewaySessionsLimit(limit) ?? GATEWAY_SESSIONS_DEFAULT_LIMIT;

  return useQuery({
    queryKey: gatewayKeys.sessionsList(normalizedLimit),
    queryFn: () => gatewaySessionsList(normalizedLimit),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchIntervalInBackground: true,
  });
}

export function useGatewayCircuitResetProviderMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey?: CliKey | null; providerId: number }) => {
      if (input.cliKey) validateGatewayCliKey(input.cliKey);
      return gatewayCircuitResetProvider(input.providerId);
    },
    onSuccess: (_ok, input) => {
      if (input.cliKey) {
        queryClient.invalidateQueries({
          queryKey: gatewayKeys.circuitStatus(validateGatewayCliKey(input.cliKey)),
        });
        return;
      }
      queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
    },
  });
}

export function useGatewayCircuitResetCliMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey }) =>
      gatewayCircuitResetCli(validateGatewayCliKey(input.cliKey)),
    onSuccess: (_count, input) => {
      queryClient.invalidateQueries({
        queryKey: gatewayKeys.circuitStatus(validateGatewayCliKey(input.cliKey)),
      });
    },
  });
}

export function useGatewayCircuitAutoRefresh(
  cliKey: CliKey,
  summary: GatewayCircuitRowsSummary,
  options?: { enabled?: boolean }
) {
  const queryClient = useQueryClient();
  const timerRef = useRef<number | null>(null);
  const normalizedCliKey = validateGatewayCliKey(cliKey);
  const enabled = options?.enabled ?? true;

  useEffect(() => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    if (!enabled || !summary.hasUnavailable) {
      return;
    }

    const nowUnix = Math.floor(Date.now() / 1000);
    const nextAvailableUntil = summary.hasUnavailableWithoutUntil
      ? nowUnix
      : summary.earliestUnavailableUntil;
    if (nextAvailableUntil == null) {
      return;
    }

    const delayMs = Math.max(200, (nextAvailableUntil - nowUnix) * 1000 + 250);
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      void queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(normalizedCliKey) });
    }, delayMs);

    return () => {
      if (timerRef.current != null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [
    enabled,
    normalizedCliKey,
    queryClient,
    summary.earliestUnavailableUntil,
    summary.hasUnavailable,
    summary.hasUnavailableWithoutUntil,
  ]);
}
