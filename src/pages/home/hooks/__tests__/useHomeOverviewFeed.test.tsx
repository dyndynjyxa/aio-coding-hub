import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { backgroundTaskVisibilityTriggers } from "../../../../constants/backgroundTaskContracts";
import { useWindowForeground } from "../../../../hooks/useWindowForeground";
import { useProviderLimitUsageV1Query } from "../../../../query/providerLimitUsage";
import { useUsageHourlySeriesQuery } from "../../../../query/usage";
import { emitBackgroundTaskVisibilityTrigger } from "../../../../services/backgroundTasks";
import { useRequestLogsFeed } from "../../../../hooks/useRequestLogsFeed";
import { useHomeFreshnessOwner } from "../useHomeFreshnessOwner";
import { useHomeOverviewFeed } from "../useHomeOverviewFeed";

vi.mock("../../../../hooks/useWindowForeground", () => ({
  useWindowForeground: vi.fn(),
}));

vi.mock("../../../../hooks/useRequestLogsFeed", () => ({
  useRequestLogsFeed: vi.fn(),
}));

vi.mock("../useHomeFreshnessOwner", () => ({
  useHomeFreshnessOwner: vi.fn(),
}));

vi.mock("../../../../query/providerLimitUsage", () => ({
  useProviderLimitUsageV1Query: vi.fn(),
}));

vi.mock("../../../../query/usage", () => ({
  useUsageHourlySeriesQuery: vi.fn(),
}));

vi.mock("../../../../services/backgroundTasks", () => ({
  emitBackgroundTaskVisibilityTrigger: vi.fn(),
}));

describe("pages/home/hooks/useHomeOverviewFeed", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ error: null }),
    } as any);
    vi.mocked(useProviderLimitUsageV1Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: vi.fn().mockResolvedValue({ error: null }),
    } as any);
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: vi.fn().mockResolvedValue({ error: null }),
    } as any);
    vi.mocked(useHomeFreshnessOwner).mockReturnValue({
      refreshRequestLogsNow: vi.fn().mockResolvedValue(null),
    });
  });

  it("uses overview activation only for visibility trigger, not a second manual refetch path", () => {
    const usageRefetch = vi.fn().mockResolvedValue({ error: null });
    const providerRefetch = vi.fn().mockResolvedValue({ error: null });
    const requestLogsRefresh = vi.fn().mockResolvedValue({ error: null });

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: usageRefetch,
    } as any);
    vi.mocked(useProviderLimitUsageV1Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: providerRefetch,
    } as any);
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: requestLogsRefresh,
    } as any);

    const view = renderHook(
      (props: {
        overviewActive: boolean;
        foregroundActive: boolean;
        overviewUsageSeriesEnabled: boolean;
        shouldRefetchOverviewUsageSeries: boolean;
        homeUsageWindowDays: number;
      }) => useHomeOverviewFeed(props),
      {
        initialProps: {
          overviewActive: false,
          foregroundActive: true,
          overviewUsageSeriesEnabled: true,
          shouldRefetchOverviewUsageSeries: true,
          homeUsageWindowDays: 7,
        },
      }
    );

    view.rerender({
      overviewActive: true,
      foregroundActive: true,
      overviewUsageSeriesEnabled: true,
      shouldRefetchOverviewUsageSeries: true,
      homeUsageWindowDays: 7,
    });

    expect(emitBackgroundTaskVisibilityTrigger).toHaveBeenCalledWith(
      backgroundTaskVisibilityTriggers.homeOverviewVisible
    );
    expect(usageRefetch).not.toHaveBeenCalled();
    expect(providerRefetch).not.toHaveBeenCalled();
    expect(requestLogsRefresh).not.toHaveBeenCalled();
  });

  it("uses foreground callback only for overview catch-up and leaves request logs freshness to the owner", () => {
    const usageRefetch = vi.fn().mockResolvedValue({ error: null });
    const providerRefetch = vi.fn().mockResolvedValue({ error: null });
    const requestLogsRefresh = vi.fn().mockResolvedValue({ error: null });
    let foregroundArgs: { onForeground: () => void } | null = null;

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: usageRefetch,
    } as any);
    vi.mocked(useProviderLimitUsageV1Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: providerRefetch,
    } as any);
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: requestLogsRefresh,
    } as any);
    vi.mocked(useWindowForeground).mockImplementation((args: any) => {
      foregroundArgs = args;
    });

    renderHook(() =>
      useHomeOverviewFeed({
        overviewActive: true,
        foregroundActive: true,
        overviewUsageSeriesEnabled: true,
        shouldRefetchOverviewUsageSeries: true,
        homeUsageWindowDays: 7,
      })
    );

    vi.mocked(emitBackgroundTaskVisibilityTrigger).mockClear();

    act(() => {
      foregroundArgs?.onForeground();
    });

    expect(emitBackgroundTaskVisibilityTrigger).toHaveBeenCalledWith(
      backgroundTaskVisibilityTriggers.homeOverviewVisible
    );
    expect(usageRefetch).toHaveBeenCalledTimes(1);
    expect(providerRefetch).toHaveBeenCalledTimes(1);
    expect(requestLogsRefresh).not.toHaveBeenCalled();
  });

  it("delegates manual request logs refresh to home freshness owner", () => {
    const ownerRefresh = vi.fn().mockResolvedValue({ error: null });
    const requestLogsRefresh = vi.fn().mockResolvedValue({ error: null });

    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: requestLogsRefresh,
    } as any);
    vi.mocked(useHomeFreshnessOwner).mockReturnValue({
      refreshRequestLogsNow: ownerRefresh,
    });

    const { result } = renderHook(() =>
      useHomeOverviewFeed({
        overviewActive: true,
        foregroundActive: true,
        overviewUsageSeriesEnabled: true,
        shouldRefetchOverviewUsageSeries: true,
        homeUsageWindowDays: 7,
      })
    );

    act(() => {
      result.current.refreshRequestLogs();
    });

    expect(ownerRefresh).toHaveBeenCalledTimes(1);
    expect(requestLogsRefresh).not.toHaveBeenCalled();
  });
});
