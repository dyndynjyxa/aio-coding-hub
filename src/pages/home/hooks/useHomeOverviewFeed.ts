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
  showOverviewUsageSection: boolean;
  homeUsageWindowDays: number;
};

export function useHomeOverviewFeed({
  overviewActive,
  foregroundActive,
  showOverviewUsageSection,
  homeUsageWindowDays,
}: UseHomeOverviewFeedOptions) {
  const previousOverviewActiveRef = useRef(false);
  const overviewForegroundPollingEnabled = overviewActive && foregroundActive;

  const usageHeatmapQuery = useUsageHourlySeriesQuery(homeUsageWindowDays, {
    enabled: overviewActive && showOverviewUsageSection,
  });
  const providerLimitQuery = useProviderLimitUsageV1Query(null, {
    enabled: overviewForegroundPollingEnabled,
    refetchIntervalMs: overviewForegroundPollingEnabled ? 30000 : false,
  });
  const requestLogsFeed = useRequestLogsFeed({
    limit: 50,
    enabled: overviewActive,
    liveUpdatesEnabled: overviewActive,
    liveUpdateIntervalMs: 1000,
    refreshOnForeground: overviewActive,
  });

  const refetchUsageHeatmapSilently = useCallback(async () => {
    if (!showOverviewUsageSection) return null;
    return usageHeatmapQuery.refetch();
  }, [showOverviewUsageSection, usageHeatmapQuery]);

  const refetchProviderLimitSilently = useCallback(async () => {
    if (!overviewForegroundPollingEnabled) return null;
    return providerLimitQuery.refetch();
  }, [overviewForegroundPollingEnabled, providerLimitQuery]);

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
  }, [
    overviewActive,
    refetchProviderLimitSilently,
    refetchUsageHeatmapSilently,
  ]);

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
    usageHeatmapRows: usageHeatmapQuery.data ?? [],
    usageHeatmapLoading: usageHeatmapQuery.isFetching,
    providerLimitRows: providerLimitQuery.data ?? [],
    providerLimitLoading: providerLimitQuery.isLoading,
    providerLimitRefreshing: providerLimitQuery.isFetching && !providerLimitQuery.isLoading,
    providerLimitAvailable: providerLimitQuery.isLoading ? null : providerLimitQuery.data != null,
    requestLogs: requestLogsFeed.requestLogs,
    requestLogsLoading: requestLogsFeed.requestLogsLoading,
    requestLogsRefreshing: requestLogsFeed.requestLogsRefreshing,
    requestLogsAvailable: requestLogsFeed.requestLogsAvailable,
    refreshUsageHeatmap,
    refreshProviderLimit,
    refreshRequestLogs,
  };
}
