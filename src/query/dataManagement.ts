import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  dbDiskUsageGet,
  requestLogsClearAll,
  type ClearRequestLogsResult,
  type DbDiskUsage,
} from "../services/app/dataManagement";
import { dataManagementKeys, requestLogsKeys } from "./keys";

export function useDbDiskUsageQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: dataManagementKeys.dbDiskUsage(),
    queryFn: () => dbDiskUsageGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useRequestLogsClearAllMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => requestLogsClearAll(),
    onSuccess: (result) => {
      if (!result) return;
      queryClient.invalidateQueries({ queryKey: dataManagementKeys.dbDiskUsage() });
      queryClient.invalidateQueries({ queryKey: requestLogsKeys.all });
    },
  });
}

export function isClearRequestLogsResult(result: ClearRequestLogsResult | null) {
  return (
    !!result &&
    Number.isFinite(result.request_logs_deleted) &&
    Number.isFinite(result.request_attempt_logs_deleted)
  );
}

export function formatDbDiskUsageAvailable(usage: DbDiskUsage | null | undefined) {
  if (!usage) return null;
  return usage.total_bytes;
}
