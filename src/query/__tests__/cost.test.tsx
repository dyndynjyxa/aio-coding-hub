import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
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
import { useCostAnalyticsV1Query } from "../cost";

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

  it("returns null when any underlying call returns null", async () => {
    setTauriRuntime();

    vi.mocked(costSummaryV1).mockResolvedValue(null);
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
      expect(result.current.isFetched).toBe(true);
    });

    expect(result.current.data).toBeNull();
  });
});
