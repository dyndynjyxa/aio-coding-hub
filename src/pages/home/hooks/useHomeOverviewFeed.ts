import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useWindowForeground } from "../../../hooks/useWindowForeground";
import { useRequestLogsFeed } from "../../../hooks/useRequestLogsFeed";
import { useProviderLimitUsageV1Query } from "../../../query/providerLimitUsage";
import { useUsageHourlySeriesQuery } from "../../../query/usage";
import { emitBackgroundTaskVisibilityTrigger } from "../../../services/backgroundTasks";
import { backgroundTaskVisibilityTriggers } from "../../../constants/backgroundTaskContracts";

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
  const overviewForegroundPollingEnabled = overviewActive && foregroundActive;

  const usageHeatmapQuery = useUsageHourlySeriesQuery(homeUsageWindowDays, {
    enabled: overviewActive && overviewUsageSeriesEnabled,
  });
  const providerLimitQuery = useProviderLimitUsageV1Query(null, {
    enabled: providerLimitEnabled && overviewForegroundPollingEnabled,
    refetchIntervalMs: providerLimitEnabled && overviewForegroundPollingEnabled ? 30000 : false,
  });
  const requestLogsFeed = useRequestLogsFeed({
    limit: 50,
    enabled: overviewActive,
    liveUpdatesEnabled: overviewActive,
    liveUpdateIntervalMs: 1000,
    refreshOnForeground: overviewActive,
  });

  const refetchUsageHeatmapSilently = useCallback(async () => {
    if (!shouldRefetchOverviewUsageSeries) return null;
    return usageHeatmapQuery.refetch();
  }, [shouldRefetchOverviewUsageSeries, usageHeatmapQuery]);

  const refetchProviderLimitSilently = useCallback(async () => {
    if (!providerLimitEnabled || !overviewForegroundPollingEnabled) return null;
    return providerLimitQuery.refetch();
  }, [overviewForegroundPollingEnabled, providerLimitEnabled, providerLimitQuery]);

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

  const refreshRequestLogs = useCallback(() => {
    void refetchRequestLogsSilently().then((res) => {
      if (res?.error) toast("读取使用记录失败：请查看控制台日志");
    });
  }, [refetchRequestLogsSilently]);

  useEffect(() => {
    const wasOverviewActive = previousOverviewActiveRef.current;
    previousOverviewActiveRef.current = overviewActive;

    if (!overviewActive) return;

    emitBackgroundTaskVisibilityTrigger(backgroundTaskVisibilityTriggers.homeOverviewVisible);
    if (wasOverviewActive) return;

    void refetchUsageHeatmapSilently();
    void refetchProviderLimitSilently();
  }, [overviewActive, refetchProviderLimitSilently, refetchUsageHeatmapSilently]);

  useWindowForeground({
    enabled: overviewActive,
    throttleMs: 1000,
    onForeground: () => {
      emitBackgroundTaskVisibilityTrigger(backgroundTaskVisibilityTriggers.homeOverviewVisible);
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
    refreshRequestLogs,
  };
}
