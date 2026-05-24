import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
  type QueryKey,
} from "@tanstack/react-query";
import {
  dbDiskUsageGet,
  isClearRequestLogsResult as isClearRequestLogsResultValue,
  requestLogsClearAll,
  type ClearRequestLogsResult,
  type DbDiskUsage,
} from "../services/app/dataManagement";
import type { GatewayStatus } from "../services/gateway/gateway";
import {
  cliProxyKeys,
  costKeys,
  dataManagementKeys,
  gatewayKeys,
  mcpKeys,
  modelPricesKeys,
  oauthLimitsKeys,
  promptsKeys,
  providerLimitUsageKeys,
  providersKeys,
  requestLogsKeys,
  settingsKeys,
  skillsKeys,
  sortModesKeys,
  usageKeys,
  workspacesKeys,
  wslKeys,
} from "./keys";

export const APP_DATA_RESET_STOPPED_GATEWAY_STATUS: GatewayStatus = {
  running: false,
  port: null,
  base_url: null,
  listen_addr: null,
};

const APP_DATA_RESET_EMPTY_DB_DISK_USAGE: DbDiskUsage = {
  db_bytes: 0,
  wal_bytes: 0,
  shm_bytes: 0,
  total_bytes: 0,
};

const APP_DATA_RESET_EMPTY_USAGE_SUMMARY = {
  requests_total: 0,
  requests_with_usage: 0,
  requests_success: 0,
  requests_failed: 0,
  cost_covered_success: 0,
  avg_duration_ms: null,
  avg_ttfb_ms: null,
  avg_output_tokens_per_second: null,
  input_tokens: 0,
  output_tokens: 0,
  io_total_tokens: 0,
  total_tokens: 0,
  cache_read_input_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_creation_5m_input_tokens: 0,
  cache_creation_1h_input_tokens: 0,
};

const APP_DATA_RESET_MODEL_PRICES_COUNT_QUERY_KEY = [...modelPricesKeys.all, "count"] as const;

const APP_DATA_RESET_REMOVED_QUERY_KEYS: readonly QueryKey[] = [
  oauthLimitsKeys.all,
  providersKeys.all,
  requestLogsKeys.all,
  sortModesKeys.all,
  usageKeys.all,
  costKeys.all,
  workspacesKeys.all,
  promptsKeys.all,
  mcpKeys.all,
  skillsKeys.all,
  settingsKeys.all,
  modelPricesKeys.all,
  dataManagementKeys.all,
  cliProxyKeys.all,
  wslKeys.all,
  providerLimitUsageKeys.all,
  gatewayKeys.all,
];

export async function resetAppDataQueryCaches(queryClient: QueryClient) {
  // Destructive reset must not invalidate/refetch; that can recreate SQLite files before exit.
  await Promise.all(
    APP_DATA_RESET_REMOVED_QUERY_KEYS.map((queryKey) =>
      queryClient.cancelQueries({ queryKey }, { revert: false })
    )
  );

  for (const queryKey of APP_DATA_RESET_REMOVED_QUERY_KEYS) {
    queryClient.removeQueries({ queryKey, type: "inactive" });
  }

  queryClient.setQueryData<GatewayStatus>(gatewayKeys.status(), {
    ...APP_DATA_RESET_STOPPED_GATEWAY_STATUS,
  });
  queryClient.setQueriesData<unknown[]>({ queryKey: gatewayKeys.sessions(), type: "active" }, []);
  queryClient.setQueriesData<unknown[]>({ queryKey: gatewayKeys.circuits(), type: "active" }, []);
  queryClient.setQueriesData<unknown[]>({ queryKey: requestLogsKeys.lists(), type: "active" }, []);
  queryClient.setQueriesData<DbDiskUsage>(
    { queryKey: dataManagementKeys.dbDiskUsage(), type: "active" },
    () => ({ ...APP_DATA_RESET_EMPTY_DB_DISK_USAGE })
  );
  queryClient.setQueriesData({ queryKey: settingsKeys.get(), type: "active" }, null);
  queryClient.setQueriesData<number>(
    { queryKey: APP_DATA_RESET_MODEL_PRICES_COUNT_QUERY_KEY, type: "active" },
    0
  );
  queryClient.setQueriesData<typeof APP_DATA_RESET_EMPTY_USAGE_SUMMARY>(
    { queryKey: usageKeys.summary("today", { cliKey: null }), type: "active" },
    () => ({ ...APP_DATA_RESET_EMPTY_USAGE_SUMMARY })
  );
}

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
  return isClearRequestLogsResultValue(result);
}

export function formatDbDiskUsageAvailable(usage: DbDiskUsage | null | undefined) {
  if (!usage) return null;
  if (!Number.isSafeInteger(usage.total_bytes) || usage.total_bytes < 0) return null;
  return usage.total_bytes;
}
