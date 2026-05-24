import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import type { CostAnalyticsV1, CostFilters } from "../../../query/cost";

function makeFilters(overrides: Partial<CostFilters> = {}): CostFilters {
  return {
    startTs: null,
    endTs: null,
    cliKey: null,
    providerId: null,
    model: null,
    ...overrides,
  };
}

function makeEmptyAnalytics(overrides: Partial<CostAnalyticsV1> = {}): CostAnalyticsV1 {
  return {
    summary: {
      requests_total: 0,
      requests_success: 0,
      requests_failed: 0,
      cost_covered_success: 0,
      total_cost_usd: 0,
      avg_cost_usd_per_covered_success: null,
    },
    trend: [],
    providers: [],
    models: [],
    scatter: [],
    topRequests: [],
    ...overrides,
  };
}

async function loadModule() {
  vi.resetModules();
  return await import("../previewCostData");
}

describe("components/home/previewCostData", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-11T12:00:00"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("detects empty analytics across all short-circuit branches", async () => {
    const { isCostAnalyticsEmpty } = await loadModule();

    expect(isCostAnalyticsEmpty(null)).toBe(true);
    expect(isCostAnalyticsEmpty(undefined)).toBe(true);
    expect(isCostAnalyticsEmpty(makeEmptyAnalytics())).toBe(true);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({ summary: { ...makeEmptyAnalytics().summary, requests_total: 1 } })
      )
    ).toBe(false);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({
          trend: [
            {
              day: "2026-05-11",
              hour: null,
              cost_usd: 0,
              requests_success: 0,
              cost_covered_success: 0,
            },
          ],
        })
      )
    ).toBe(false);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({
          providers: [
            {
              cli_key: "claude",
              provider_id: 1,
              provider_name: "P1",
              requests_success: 1,
              cost_covered_success: 0,
              cost_usd: 0,
            },
          ],
        })
      )
    ).toBe(false);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({
          models: [{ model: "m", requests_success: 1, cost_covered_success: 0, cost_usd: 0 }],
        })
      )
    ).toBe(false);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({
          scatter: [
            {
              cli_key: "claude",
              provider_name: "P1",
              model: "m",
              requests_success: 1,
              total_cost_usd: 0,
              total_duration_ms: 0,
            },
          ],
        })
      )
    ).toBe(false);
    expect(
      isCostAnalyticsEmpty(
        makeEmptyAnalytics({
          topRequests: [
            {
              log_id: 1,
              trace_id: "t-1",
              cli_key: "claude",
              method: "POST",
              path: "/v1/messages",
              requested_model: "m",
              provider_id: 1,
              provider_name: "P1",
              duration_ms: 1,
              ttfb_ms: 1,
              cost_usd: 1,
              cost_multiplier: 1,
              created_at: 1,
            },
          ],
        })
      )
    ).toBe(false);
  });

  it("builds daily, weekly, monthly, all-time, and custom preview analytics", async () => {
    const { buildPreviewCostAnalytics, isCostAnalyticsEmpty } = await loadModule();
    const filters = makeFilters();

    const daily = buildPreviewCostAnalytics("daily", filters, null);
    expect(daily.summary).toMatchObject({
      requests_total: 6,
      requests_success: 5,
      requests_failed: 1,
      cost_covered_success: 4,
    });
    expect(daily.summary.total_cost_usd).toBeCloseTo(4.16);
    expect(daily.trend.map((row) => row.hour)).toEqual([9, 10, 11, 13, 15, 19]);
    expect(daily.providers).toHaveLength(4);
    expect(daily.models).toHaveLength(4);
    expect(daily.scatter).toHaveLength(4);
    expect(daily.topRequests.map((row) => row.cost_usd)).toEqual([1.84, 1.22, 0.66, 0.44]);

    const weekly = buildPreviewCostAnalytics("weekly", filters, null);
    expect(weekly.summary.requests_total).toBe(14);
    expect(weekly.trend).toHaveLength(7);

    const monthly = buildPreviewCostAnalytics("monthly", filters, null);
    expect(monthly.summary.requests_total).toBe(15);
    expect(monthly.trend).toHaveLength(8);

    const unknownPeriod = buildPreviewCostAnalytics("unknown" as never, filters, null);
    expect(unknownPeriod.summary.requests_total).toBe(15);

    const allTime = buildPreviewCostAnalytics("allTime", filters, null);
    expect(allTime.summary.requests_total).toBe(15);
    expect(allTime.topRequests).toHaveLength(13);

    const customMissing = buildPreviewCostAnalytics("custom", filters, null);
    expect(isCostAnalyticsEmpty(customMissing)).toBe(true);
    expect(customMissing.summary.requests_total).toBe(0);

    const customApplied = buildPreviewCostAnalytics("custom", filters, {
      startDate: "2026-05-10",
      endDate: "2026-05-11",
      startTs: Math.floor(new Date("2026-05-10T00:00:00").getTime() / 1000),
      endTs: Math.floor(new Date("2026-05-11T00:00:00").getTime() / 1000),
    });
    expect(customApplied.summary).toMatchObject({
      requests_total: 3,
      requests_success: 3,
      requests_failed: 0,
      cost_covered_success: 3,
    });
    expect(customApplied.summary.total_cost_usd).toBeCloseTo(4.21);
    expect(customApplied.trend).toHaveLength(1);
    expect(customApplied.trend[0]?.day).toBe("2026-05-10");

    const filtered = buildPreviewCostAnalytics(
      "allTime",
      {
        startTs: null,
        endTs: null,
        cliKey: "codex",
        providerId: 201,
        model: "gpt-5.4",
      },
      null
    );
    expect(filtered.summary).toMatchObject({
      requests_total: 3,
      requests_success: 3,
      requests_failed: 0,
      cost_covered_success: 2,
    });
    expect(filtered.summary.total_cost_usd).toBeCloseTo(2.7);
    expect(filtered.providers).toHaveLength(1);
    expect(filtered.models).toHaveLength(1);
    expect(filtered.scatter).toHaveLength(1);
    expect(filtered.topRequests.map((row) => row.cost_usd)).toEqual([1.48, 1.22]);
  });
});
