import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT,
  REQUEST_LOGS_DEFAULT_LIMIT,
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAfterIdAll,
  requestLogsListAll,
  normalizeRequestAttemptLogsLimit,
  normalizeRequestLogTraceIdOrNull,
  normalizeRequestLogsLimit,
  type RequestLogSummary,
} from "../services/gateway/requestLogs";
import {
  isPersistedRequestLogInProgress,
  requestLogCreatedAtMs,
} from "../services/gateway/requestLogState";
import { requestLogsKeys } from "./keys";

type RequestLogsIncrementalRefreshResult = {
  mode: "full" | "incremental";
  items: RequestLogSummary[];
};

export const REQUEST_LOG_DETAIL_STALE_TIME_MS = 0;
export const REQUEST_LOG_DETAIL_GC_TIME_MS = 60 * 1000;

function isRequestLogsQueryEnabled(enabled: boolean | undefined) {
  return enabled ?? true;
}

function sortRequestLogsDesc(a: RequestLogSummary, b: RequestLogSummary) {
  const aTsMs = requestLogCreatedAtMs(a);
  const bTsMs = requestLogCreatedAtMs(b);
  if (aTsMs !== bTsMs) return bTsMs - aTsMs;
  return b.id - a.id;
}

function computeRequestLogsCursorId(rows: RequestLogSummary[]) {
  let maxId = 0;
  for (const row of rows) {
    if (Number.isFinite(row.id) && row.id > maxId) maxId = row.id;
  }
  return maxId;
}

function shouldUseFullRefresh(prev: RequestLogSummary[] | null | undefined) {
  if (!prev?.length) return true;
  return prev.some(isPersistedRequestLogInProgress);
}

function mergeRequestLogs(prev: RequestLogSummary[], incoming: RequestLogSummary[], limit: number) {
  const byId = new Map<number, RequestLogSummary>();
  for (const row of incoming) byId.set(row.id, row);
  for (const row of prev) {
    if (!byId.has(row.id)) byId.set(row.id, row);
  }
  const merged = Array.from(byId.values());
  merged.sort(sortRequestLogsDesc);
  return merged.slice(0, limit);
}

function capRequestLogs(rows: RequestLogSummary[], limit: number) {
  return rows.slice().sort(sortRequestLogsDesc).slice(0, limit);
}

export function useRequestLogsListAllQuery(
  limit?: number | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);
  const normalizedLimit = normalizeRequestLogsLimit(limit) ?? REQUEST_LOGS_DEFAULT_LIMIT;

  return useQuery<RequestLogSummary[]>({
    queryKey: requestLogsKeys.listAll(normalizedLimit),
    queryFn: async () => {
      const rows = await requestLogsListAll(normalizedLimit);
      return capRequestLogs(rows, normalizedLimit);
    },
    enabled,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useRequestLogsIncrementalRefreshMutation(limit?: number | null) {
  const queryClient = useQueryClient();
  const normalizedLimit = normalizeRequestLogsLimit(limit) ?? REQUEST_LOGS_DEFAULT_LIMIT;

  return useMutation<RequestLogsIncrementalRefreshResult>({
    mutationFn: async () => {
      const prev = queryClient.getQueryData<RequestLogSummary[] | null>(
        requestLogsKeys.listAll(normalizedLimit)
      );
      const cursorId = prev?.length ? computeRequestLogsCursorId(prev) : 0;
      const useFullRefresh = shouldUseFullRefresh(prev);

      if (useFullRefresh) {
        const items = await requestLogsListAll(normalizedLimit);
        return { mode: "full" as const, items: capRequestLogs(items, normalizedLimit) };
      }

      const items = await requestLogsListAfterIdAll(cursorId, normalizedLimit);
      return { mode: "incremental" as const, items: capRequestLogs(items, normalizedLimit) };
    },
    onSuccess: (result) => {
      if (!result) return;

      if (result.mode === "full") {
        queryClient.setQueryData(requestLogsKeys.listAll(normalizedLimit), result.items);
        return;
      }

      if (result.items.length === 0) return;

      queryClient.setQueryData<RequestLogSummary[]>(
        requestLogsKeys.listAll(normalizedLimit),
        (cur) => mergeRequestLogs(cur ?? [], result.items, normalizedLimit)
      );
    },
  });
}

export function useRequestLogDetailQuery(logId: number | null) {
  return useQuery({
    queryKey: requestLogsKeys.detail(logId),
    queryFn: () => {
      if (logId == null) return null;
      return requestLogGet(logId);
    },
    enabled: logId != null,
    placeholderData: keepPreviousData,
    staleTime: REQUEST_LOG_DETAIL_STALE_TIME_MS,
    gcTime: REQUEST_LOG_DETAIL_GC_TIME_MS,
  });
}

export function useRequestAttemptLogsByTraceIdQuery(traceId: string | null, limit?: number | null) {
  const normalizedTraceId = normalizeRequestLogTraceIdOrNull(traceId);
  const normalizedLimit =
    normalizeRequestAttemptLogsLimit(limit) ?? REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT;

  return useQuery({
    queryKey: requestLogsKeys.attemptsByTrace(normalizedTraceId, normalizedLimit),
    queryFn: () => {
      if (!normalizedTraceId) return null;
      return requestAttemptLogsByTraceId(normalizedTraceId, normalizedLimit);
    },
    enabled: Boolean(normalizedTraceId),
    placeholderData: keepPreviousData,
    staleTime: REQUEST_LOG_DETAIL_STALE_TIME_MS,
    gcTime: REQUEST_LOG_DETAIL_GC_TIME_MS,
  });
}
