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
      refreshActiveRequests: vi.fn().mockResolvedValue({ error: null }),
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

  it("passes active snapshots and their refresh action to the home freshness owner", async () => {
    const refreshActiveRequests = vi.fn().mockResolvedValue({ error: null });
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [],
      activeRequests: [{ trace_id: "trace-active" }],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshActiveRequests,
      refreshRequestLogs: vi.fn().mockResolvedValue({ error: null }),
    } as any);

    renderHook(() =>
      useHomeOverviewFeed({
        overviewActive: true,
        foregroundActive: true,
        overviewUsageSeriesEnabled: true,
        shouldRefetchOverviewUsageSeries: true,
        homeUsageWindowDays: 7,
      })
    );

    expect(useHomeFreshnessOwner).toHaveBeenCalledWith(
      expect.objectContaining({
        requestActivityPending: true,
        onRefreshActiveRequests: expect.any(Function),
      })
    );

    const ownerCalls = vi.mocked(useHomeFreshnessOwner).mock.calls;
    const ownerOptions = ownerCalls[ownerCalls.length - 1]?.[0];
    await act(async () => {
      await ownerOptions?.onRefreshActiveRequests();
    });
    expect(refreshActiveRequests).toHaveBeenCalledTimes(1);
  });

  it("keeps the watchdog pending while a recent log row still lacks a terminal state", () => {
    // 终态落库是异步批量写：complete 信号触发的拉取可能读到仍无终态的占位行。
    // 此时注册表已空，watchdog 必须继续轮询直到读到终态。
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [
        {
          id: 1,
          trace_id: "trace-stale",
          is_interrupted: true,
          created_at_ms: Date.now() - 5_000,
          created_at: Math.floor((Date.now() - 5_000) / 1000),
        },
      ],
      activeRequests: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: vi.fn().mockResolvedValue({ error: null }),
    } as any);

    renderHook(() =>
      useHomeOverviewFeed({
        overviewActive: true,
        foregroundActive: true,
        overviewUsageSeriesEnabled: true,
        shouldRefetchOverviewUsageSeries: true,
        homeUsageWindowDays: 7,
      })
    );

    expect(useHomeFreshnessOwner).toHaveBeenCalledWith(
      expect.objectContaining({
        requestActivityPending: true,
      })
    );
  });

  it("stops the watchdog for unresolved rows outside the watch window and for terminal rows", () => {
    vi.mocked(useRequestLogsFeed).mockReturnValue({
      requestLogs: [
        {
          id: 1,
          trace_id: "trace-lost",
          is_interrupted: true,
          created_at_ms: Date.now() - 11 * 60 * 1000,
          created_at: Math.floor((Date.now() - 11 * 60 * 1000) / 1000),
        },
        {
          id: 2,
          trace_id: "trace-done",
          is_interrupted: false,
          created_at_ms: Date.now() - 1_000,
          created_at: Math.floor((Date.now() - 1_000) / 1000),
        },
      ],
      activeRequests: [],
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsAvailable: true,
      refreshRequestLogs: vi.fn().mockResolvedValue({ error: null }),
    } as any);

    renderHook(() =>
      useHomeOverviewFeed({
        overviewActive: true,
        foregroundActive: true,
        overviewUsageSeriesEnabled: true,
        shouldRefetchOverviewUsageSeries: true,
        homeUsageWindowDays: 7,
      })
    );

    expect(useHomeFreshnessOwner).toHaveBeenCalledWith(
      expect.objectContaining({
        requestActivityPending: false,
      })
    );
  });
});
