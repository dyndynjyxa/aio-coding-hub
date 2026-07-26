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
import { normalizeCircuitState } from "../services/gateway/circuitState";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { gatewayKeys } from "./keys";

export type CircuitDisplayState = "healthy" | "open" | "cooldown" | "half_open";

export type GatewayCircuitDerivedState = {
  displayState: CircuitDisplayState;
  isOpen: boolean;
  isUnavailable: boolean;
  unavailableUntil: number | null;
};

export type GatewayCircuitDerivedRow = GatewayCircuitDerivedState & {
  row: GatewayProviderCircuitStatus;
};

export type GatewayCircuitRowsSummary = {
  byProviderId: Record<number, GatewayProviderCircuitStatus>;
  /** 全部非健康行（open / cooldown / half_open），供展示层按四态渲染。 */
  attentionRows: GatewayCircuitDerivedRow[];
  /** 仅 open / cooldown 行（半开不算不可用），存量语义不变。 */
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
  const state = normalizeCircuitState(row?.state);
  const isOpen = state === "OPEN";
  const cooldownUntil = normalizeGatewayCircuitUnix(row?.cooldown_until);
  const openUntil = isOpen ? normalizeGatewayCircuitUnix(row?.open_until) : null;
  const unavailableUntil =
    openUntil == null
      ? cooldownUntil
      : cooldownUntil == null
        ? openUntil
        : Math.max(openUntil, cooldownUntil);

  // 判定优先级：open > cooldown > half_open > healthy。
  // cooldown 沿用现状“非空即算”（依赖后端轮询清除过期值），不引入前端时钟比对。
  // HALF_OPEN 表示已允许试探请求，不计入“当前熔断/不可用”（isUnavailable 存量语义不变）。
  const displayState: CircuitDisplayState = isOpen
    ? "open"
    : cooldownUntil != null
      ? "cooldown"
      : state === "HALF_OPEN"
        ? "half_open"
        : "healthy";

  return {
    displayState,
    isOpen,
    isUnavailable: displayState === "open" || displayState === "cooldown",
    unavailableUntil,
  };
}

export function summarizeGatewayCircuitRows(
  rows: readonly GatewayProviderCircuitStatus[] | null | undefined
): GatewayCircuitRowsSummary {
  const byProviderId: Record<number, GatewayProviderCircuitStatus> = {};
  const attentionRows: GatewayCircuitDerivedRow[] = [];
  const unavailableRows: GatewayCircuitDerivedRow[] = [];
  let earliestUnavailableUntil: number | null = null;
  let hasUnavailableWithoutUntil = false;

  for (const row of rows ?? []) {
    byProviderId[row.provider_id] = row;

    const derived = getGatewayCircuitDerivedState(row);
    if (derived.displayState === "healthy") continue;

    const derivedRow = { row, ...derived };
    attentionRows.push(derivedRow);

    if (!derived.isUnavailable) continue;

    unavailableRows.push(derivedRow);

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
    attentionRows,
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
  // Polling pauses while the window is hidden (same semantic as the backend's
  // event gating) but keeps running when merely unfocused — hence the
  // visibility gate on refetchInterval instead of refetchIntervalInBackground.
  const documentVisible = useDocumentVisibility();

  return useQuery({
    queryKey: gatewayKeys.status(),
    queryFn: () => gatewayStatus(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: documentVisible ? (options?.refetchIntervalMs ?? false) : false,
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
  // See useGatewayStatusQuery: poll only while the window is visible.
  const documentVisible = useDocumentVisibility();

  return useQuery({
    queryKey: gatewayKeys.sessionsList(normalizedLimit),
    queryFn: () => gatewaySessionsList(normalizedLimit),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: documentVisible ? (options?.refetchIntervalMs ?? false) : false,
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
