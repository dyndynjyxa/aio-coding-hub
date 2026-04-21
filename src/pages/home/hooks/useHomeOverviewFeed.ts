import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useWindowForeground } from "../../../hooks/useWindowForeground";
import { useRequestLogsFeed } from "../../../hooks/useRequestLogsFeed";
import { useProviderLimitUsageV1Query } from "../../../query/providerLimitUsage";
import { useUsageHourlySeriesQuery } from "../../../query/usage";
import { emitBackgroundTaskVisibilityTrigger } from "../../../services/backgroundTasks";
import { backgroundTaskVisibilityTriggers } from "../../../constants/backgroundTaskContracts";
import { useHomeFreshnessOwner } from "./useHomeFreshnessOwner";

type UseHomeOverviewFeedOptions = {
  overviewActive: boolean;
  foregroundActive: boolean;
  overviewUsageSeriesEnabled: boolean;
  shouldRefetchOverviewUsageSeries: boolean;
  homeUsageWindowDays: number;
  providerLimitEnabled?: boolean;
};

export function useHomeOverviewFeed({
  overviewActive,
  foregroundActive,
  overviewUsageSeriesEnabled,
  shouldRefetchOverviewUsageSeries,
  homeUsageWindowDays,
  providerLimitEnabled = true,
}: UseHomeOverviewFeedOptions) {
  const previousOverviewActiveRef = useRef(false);
  const overviewForegroundActive = overviewActive && foregroundActive;

  const usageHeatmapQuery = useUsageHourlySeriesQuery(homeUsageWindowDays, {
    enabled: overviewActive && overviewUsageSeriesEnabled,
  });
  const providerLimitQuery = useProviderLimitUsageV1Query(null, {
    enabled: providerLimitEnabled && overviewForegroundActive,
    refetchIntervalMs: providerLimitEnabled && overviewForegroundActive ? 30000 : false,
  });
  const requestLogsFeed = useRequestLogsFeed({
    limit: 50,
    enabled: overviewActive,
  });

  const refetchUsageHeatmapSilently = useCallback(async () => {
    if (!shouldRefetchOverviewUsageSeries) return null;
    return usageHeatmapQuery.refetch();
  }, [shouldRefetchOverviewUsageSeries, usageHeatmapQuery]);

  const refetchProviderLimitSilently = useCallback(async () => {
    if (!providerLimitEnabled || !overviewForegroundActive) return null;
    return providerLimitQuery.refetch();
  }, [overviewForegroundActive, providerLimitEnabled, providerLimitQuery]);

  const refetchRequestLogsSilently = useCallback(async () => {
    return requestLogsFeed.refreshRequestLogs();
  }, [requestLogsFeed]);

  const refreshUsageHeatmap = useCallback(() => {
    void refetchUsageHeatmapSilently().then((res) => {
      if (res?.error) toast("刷新用量失败：请查看控制台日志");
    });
  }, [refetchUsageHeatmapSilently]);

  const refreshProviderLimit = useCallback(() => {
    void refetchProviderLimitSilently().then((res) => {
      if (res?.error) toast("读取供应商限额失败：请查看控制台日志");
    });
  }, [refetchProviderLimitSilently]);

  const { refreshRequestLogsNow } = useHomeFreshnessOwner({
    overviewActive,
    foregroundActive,
    requestLogsRefreshWindowMs: 1000,
    onRefreshRequestLogs: refetchRequestLogsSilently,
  });

  const refreshRequestLogsFromOwner = useCallback(() => {
    void refreshRequestLogsNow().then((res) => {
      if (res && typeof res === "object" && "error" in res && res.error) {
        toast("读取使用记录失败：请查看控制台日志");
      }
    });
  }, [refreshRequestLogsNow]);

  useEffect(() => {
    const wasOverviewActive = previousOverviewActiveRef.current;
    previousOverviewActiveRef.current = overviewActive;

    if (!overviewActive || wasOverviewActive) return;

    void emitBackgroundTaskVisibilityTrigger(backgroundTaskVisibilityTriggers.homeOverviewVisible);
  }, [overviewActive]);

  useWindowForeground({
    enabled: overviewActive,
    throttleMs: 1000,
    onForeground: () => {
      void emitBackgroundTaskVisibilityTrigger(
        backgroundTaskVisibilityTriggers.homeOverviewVisible
      );
      void refetchUsageHeatmapSilently();
      void refetchProviderLimitSilently();
    },
  });

  return {
    usageHeatmapRows: overviewUsageSeriesEnabled ? (usageHeatmapQuery.data ?? []) : [],
    usageHeatmapLoading: overviewUsageSeriesEnabled && usageHeatmapQuery.isFetching,
    providerLimitRows: providerLimitEnabled ? (providerLimitQuery.data ?? []) : [],
    providerLimitLoading: providerLimitEnabled && providerLimitQuery.isLoading,
    providerLimitRefreshing:
      providerLimitEnabled && providerLimitQuery.isFetching && !providerLimitQuery.isLoading,
    providerLimitAvailable: providerLimitEnabled
      ? providerLimitQuery.isLoading
        ? null
        : providerLimitQuery.data != null
      : null,
    requestLogs: requestLogsFeed.requestLogs,
    requestLogsLoading: requestLogsFeed.requestLogsLoading,
    requestLogsRefreshing: requestLogsFeed.requestLogsRefreshing,
    requestLogsAvailable: requestLogsFeed.requestLogsAvailable,
    refreshUsageHeatmap,
    refreshProviderLimit,
    refreshRequestLogs: refreshRequestLogsFromOwner,
  };
}
