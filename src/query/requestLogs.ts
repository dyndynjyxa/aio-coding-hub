import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT,
  REQUEST_LOGS_DEFAULT_LIMIT,
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAll,
  normalizeRequestAttemptLogsLimit,
  normalizeRequestLogTraceIdOrNull,
  normalizeRequestLogsLimit,
  type RequestLogSummary,
} from "../services/gateway/requestLogs";
import { activeRequestLogsSnapshot, type ActiveRequest } from "../services/gateway/activeRequests";
import { requestLogCreatedAtMs } from "../services/gateway/requestLogState";
import { logToConsole } from "../services/consoleLog";
import { requestLogsKeys } from "./keys";

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

export function useActiveRequestLogsSnapshotQuery(options?: { enabled?: boolean }) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);

  return useQuery<ActiveRequest[]>({
    queryKey: requestLogsKeys.activeSnapshot(),
    queryFn: async () => {
      try {
        return await activeRequestLogsSnapshot();
      } catch (error) {
        logToConsole("warn", "读取进行中请求快照失败", { error: String(error) });
        return [];
      }
    },
    enabled,
    placeholderData: keepPreviousData,
  });
}

export function useRequestLogsRefreshMutation(limit?: number | null) {
  const queryClient = useQueryClient();
  const normalizedLimit = normalizeRequestLogsLimit(limit) ?? REQUEST_LOGS_DEFAULT_LIMIT;

  return useMutation<RequestLogSummary[]>({
    mutationFn: async () => {
      const items = await requestLogsListAll(normalizedLimit);
      return capRequestLogs(items, normalizedLimit);
    },
    onSuccess: (items) => {
      queryClient.setQueryData(requestLogsKeys.listAll(normalizedLimit), items);
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
