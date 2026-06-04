import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
} from "../../services/usage/cost";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { useCostAnalyticsV1Query, type CostFilters } from "../cost";
import { costKeys } from "../keys";

vi.mock("../../services/usage/cost", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/cost")>(
    "../../services/usage/cost"
  );
  return {
    ...actual,
    costSummaryV1: vi.fn(),
    costTrendV1: vi.fn(),
    costBreakdownProviderV1: vi.fn(),
    costBreakdownModelV1: vi.fn(),
    costScatterCliProviderModelV1: vi.fn(),
    costTopRequestsV1: vi.fn(),
  };
});

describe("query/cost", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("aggregates cost analytics when all services return data", async () => {
    setTauriRuntime();

    const summary = {
      requests_total: 10,
      requests_success: 9,
      requests_failed: 1,
      cost_covered_success: 9,
      total_cost_usd: 1.23,
      avg_cost_usd_per_covered_success: 0.12,
    };
    const trend = [
      {
        day: "2026-01-31",
        hour: null,
        cost_usd: 1.23,
        requests_success: 9,
        cost_covered_success: 9,
      },
    ];
    const providers = [
      {
        cli_key: "claude" as const,
        provider_id: 1,
        provider_name: "P1",
        requests_success: 9,
        cost_covered_success: 9,
        cost_usd: 1.23,
      },
    ];
    const models = [{ model: "m1", requests_success: 9, cost_covered_success: 9, cost_usd: 1.23 }];
    const scatter = [
      {
        cli_key: "claude" as const,
        provider_name: "P1",
        model: "m1",
        requests_success: 9,
        total_cost_usd: 1.23,
        total_duration_ms: 1234,
      },
    ];
    const top = [
      {
        log_id: 1,
        trace_id: "t1",
        cli_key: "claude" as const,
        method: "GET",
        path: "/v1/test",
        requested_model: null,
        provider_id: 1,
        provider_name: "P1",
        duration_ms: 100,
        ttfb_ms: null,
        cost_usd: 0.1,
        cost_multiplier: 1,
        created_at: 0,
      },
    ];

    vi.mocked(costSummaryV1).mockResolvedValue(summary);
    vi.mocked(costTrendV1).mockResolvedValue(trend);
    vi.mocked(costBreakdownProviderV1).mockResolvedValue(providers);
    vi.mocked(costBreakdownModelV1).mockResolvedValue(models);
    vi.mocked(costScatterCliProviderModelV1).mockResolvedValue(scatter);
    vi.mocked(costTopRequestsV1).mockResolvedValue(top);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () =>
        useCostAnalyticsV1Query("daily", {
          startTs: null,
          endTs: null,
          cliKey: "claude",
          providerId: null,
          model: null,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.data).not.toBeUndefined();
    });

    expect(result.current.data).toEqual({
      summary,
      trend,
      providers,
      models,
      scatter,
      topRequests: top,
    });
  });

  it("normalizes cost filters before cache key and service calls", async () => {
    setTauriRuntime();

    const summary = {
      requests_total: 0,
      requests_success: 0,
      requests_failed: 0,
      cost_covered_success: 0,
      total_cost_usd: 0,
      avg_cost_usd_per_covered_success: null,
    };

    vi.mocked(costSummaryV1).mockResolvedValue(summary);
    vi.mocked(costTrendV1).mockResolvedValue([]);
    vi.mocked(costBreakdownProviderV1).mockResolvedValue([]);
    vi.mocked(costBreakdownModelV1).mockResolvedValue([]);
    vi.mocked(costScatterCliProviderModelV1).mockResolvedValue([]);
    vi.mocked(costTopRequestsV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const filters = {
      startTs: 1,
      endTs: 2,
      cliKey: " claude ",
      providerId: 3,
      model: " m1 ",
    } as unknown as CostFilters;
    const normalizedFilters = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
    } satisfies CostFilters;

    const { result } = renderHook(() => useCostAnalyticsV1Query("custom", filters), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(costSummaryV1).toHaveBeenCalledWith("custom", normalizedFilters);
    expect(costTrendV1).toHaveBeenCalledWith("custom", normalizedFilters);
    expect(costBreakdownProviderV1).toHaveBeenCalledWith("custom", {
      ...normalizedFilters,
      limit: 120,
    });
    expect(costBreakdownModelV1).toHaveBeenCalledWith("custom", {
      ...normalizedFilters,
      limit: 120,
    });
    expect(costScatterCliProviderModelV1).toHaveBeenCalledWith("custom", {
      ...normalizedFilters,
      limit: 500,
    });
    expect(costTopRequestsV1).toHaveBeenCalledWith("custom", {
      ...normalizedFilters,
      limit: 50,
    });
    expect(client.getQueryState(costKeys.analyticsV1("custom", normalizedFilters))).toBeTruthy();
    expect(client.getQueryState(costKeys.analyticsV1("custom", filters))).toBeUndefined();
  });

  it("rejects invalid cost filters before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(
        () =>
          useCostAnalyticsV1Query("daily", {
            startTs: null,
            endTs: null,
            cliKey: "opencode",
            providerId: null,
            model: null,
          } as unknown as CostFilters),
        { wrapper }
      )
    ).toThrow("SEC_INVALID_INPUT");

    expect(costSummaryV1).not.toHaveBeenCalled();
    expect(costTrendV1).not.toHaveBeenCalled();
    expect(costBreakdownProviderV1).not.toHaveBeenCalled();
    expect(costBreakdownModelV1).not.toHaveBeenCalled();
    expect(costScatterCliProviderModelV1).not.toHaveBeenCalled();
    expect(costTopRequestsV1).not.toHaveBeenCalled();
  });

  it("useCostAnalyticsV1Query enters error state when underlying call rejects", async () => {
    setTauriRuntime();

    vi.mocked(costSummaryV1).mockRejectedValue(new Error("cost summary boom"));
    vi.mocked(costTrendV1).mockResolvedValue([]);
    vi.mocked(costBreakdownProviderV1).mockResolvedValue([]);
    vi.mocked(costBreakdownModelV1).mockResolvedValue([]);
    vi.mocked(costScatterCliProviderModelV1).mockResolvedValue([]);
    vi.mocked(costTopRequestsV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () =>
        useCostAnalyticsV1Query("daily", {
          startTs: null,
          endTs: null,
          cliKey: "claude",
          providerId: null,
          model: null,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("does not call cost services when disabled", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useCostAnalyticsV1Query(
          "daily",
          {
            startTs: null,
            endTs: null,
            cliKey: "claude",
            providerId: null,
            model: null,
          },
          { enabled: false }
        ),
      { wrapper }
    );

    expect(costSummaryV1).not.toHaveBeenCalled();
    expect(costTrendV1).not.toHaveBeenCalled();
    expect(costBreakdownProviderV1).not.toHaveBeenCalled();
    expect(costBreakdownModelV1).not.toHaveBeenCalled();
    expect(costScatterCliProviderModelV1).not.toHaveBeenCalled();
    expect(costTopRequestsV1).not.toHaveBeenCalled();
  });
});
