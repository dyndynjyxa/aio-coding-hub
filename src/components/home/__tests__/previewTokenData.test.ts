import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import type { UsageLeaderboardRow } from "../../../services/usage/usage";

async function loadModule() {
  vi.resetModules();
  return await import("../previewTokenData");
}

describe("components/home/previewTokenData", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-11T12:00:00"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("handles folder selection factors and scaling edge cases", async () => {
    const mod = await loadModule();

    expect(mod.previewFolderSelectionFactor(null)).toBe(1);
    expect(mod.previewFolderSelectionFactor([])).toBe(1);
    expect(mod.previewFolderSelectionFactor(["/Users/demo/aio-coding-hub"])).toBeCloseTo(0.52);
    expect(
      mod.previewFolderSelectionFactor([
        "/Users/demo/aio-coding-hub",
        "/Users/demo/workspace-alpha",
        "__unknown__",
      ])
    ).toBe(1);
    expect(mod.previewFolderSelectionFactor(["missing-folder"])).toBe(0);

    const baseRow = mod.PREVIEW_TOKEN_PROVIDER_ROWS[0];
    const nullCostRow = { ...baseRow, cost_usd: null } as UsageLeaderboardRow;

    const scaledZero = mod.scalePreviewTokenRows([nullCostRow], 0)[0];
    expect(scaledZero.requests_total).toBe(0);
    expect(scaledZero.requests_success).toBe(0);
    expect(scaledZero.requests_failed).toBe(0);
    expect(scaledZero.total_tokens).toBe(0);
    expect(scaledZero.total_duration_ms).toBe(0);
    expect(scaledZero.cost_usd).toBeNull();

    const scaledUp = mod.scalePreviewTokenRows([baseRow], 1.5)[0];
    expect(scaledUp.requests_total).toBe(27);
    expect(scaledUp.requests_failed).toBe(2);
    expect(scaledUp.requests_success).toBe(25);
    expect(scaledUp.total_tokens).toBe(73_800);
    expect(scaledUp.total_duration_ms).toBe(26_460);
    expect(scaledUp.cost_usd).toBeCloseTo(2.07);
  });

  it("builds summary and day detail preview rows", async () => {
    const mod = await loadModule();

    const emptySummary = mod.buildPreviewTokenSummary([]);
    expect(emptySummary.requests_total).toBe(0);
    expect(emptySummary.total_duration_ms).toBe(0);
    expect(emptySummary.avg_duration_ms).toBeNull();
    expect(emptySummary.avg_ttfb_ms).toBeNull();
    expect(emptySummary.avg_output_tokens_per_second).toBeNull();

    const weightedRows: UsageLeaderboardRow[] = [
      {
        ...mod.PREVIEW_TOKEN_PROVIDER_ROWS[0],
        requests_total: 1,
        requests_success: 1,
        requests_failed: 0,
        input_tokens: 10,
        output_tokens: 10,
        io_total_tokens: 20,
        total_tokens: 30,
        cache_creation_input_tokens: 4,
        cache_read_input_tokens: 6,
        total_duration_ms: 100,
        avg_duration_ms: null,
        avg_ttfb_ms: 100,
        avg_output_tokens_per_second: 50,
        cost_usd: null,
      },
      {
        ...mod.PREVIEW_TOKEN_PROVIDER_ROWS[1],
        requests_total: 2,
        requests_success: 2,
        requests_failed: 0,
        input_tokens: 20,
        output_tokens: 20,
        io_total_tokens: 40,
        total_tokens: 60,
        cache_creation_input_tokens: 8,
        cache_read_input_tokens: 12,
        total_duration_ms: 200,
        avg_duration_ms: 200,
        avg_ttfb_ms: null,
        avg_output_tokens_per_second: null,
        cost_usd: 2,
      },
    ];

    const summary = mod.buildPreviewTokenSummary(weightedRows);
    expect(summary.requests_total).toBe(3);
    expect(summary.requests_with_usage).toBe(3);
    expect(summary.requests_success).toBe(3);
    expect(summary.requests_failed).toBe(0);
    expect(summary.cost_covered_success).toBe(2);
    expect(summary.total_duration_ms).toBe(300);
    expect(summary.avg_duration_ms).toBeCloseTo(133.333, 2);
    expect(summary.avg_ttfb_ms).toBeCloseTo(33.333, 2);
    expect(summary.avg_output_tokens_per_second).toBeCloseTo(16.667, 2);
    expect(summary.cache_creation_5m_input_tokens).toBe(8);
    expect(summary.cache_creation_1h_input_tokens).toBe(4);

    const missingDay = mod.buildPreviewTokenDayDetail("missing-day", 1, null);
    expect(missingDay).toBeNull();

    const day = mod.PREVIEW_TOKEN_DAY_ROWS[0].key;
    const fullDetail = mod.buildPreviewTokenDayDetail(day, 1, null);
    expect(fullDetail).not.toBeNull();
    expect(fullDetail?.folders).toHaveLength(3);
    expect(fullDetail?.hours).toHaveLength(24);
    expect(fullDetail?.hours[0]?.requests_total).toBe(0);
    expect(fullDetail?.hours.some((hour) => hour.requests_total > 0)).toBe(true);

    const selectedDetail = mod.buildPreviewTokenDayDetail(day, 1, ["__unknown__"]);
    expect(selectedDetail?.folders).toHaveLength(1);
    expect(selectedDetail?.folders[0]?.key).toBe("__unknown__");
    expect(selectedDetail?.hours.some((hour) => hour.total_tokens > 0)).toBe(true);

    const originalDuration = mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_duration_ms;
    const originalTtfb = mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_ttfb_ms;
    const originalCost = mod.PREVIEW_TOKEN_DAY_ROWS[0].cost_usd;
    mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_duration_ms = null;
    mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_ttfb_ms = null;
    mod.PREVIEW_TOKEN_DAY_ROWS[0].cost_usd = null;
    try {
      const nullMetricsDetail = mod.buildPreviewTokenDayDetail(day, 1, null);
      expect(nullMetricsDetail?.folders[0]?.avg_duration_ms).toBeNull();
      expect(nullMetricsDetail?.folders[0]?.avg_ttfb_ms).toBeNull();
      expect(nullMetricsDetail?.folders[0]?.cost_usd).toBeNull();
    } finally {
      mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_duration_ms = originalDuration;
      mod.PREVIEW_TOKEN_DAY_ROWS[0].avg_ttfb_ms = originalTtfb;
      mod.PREVIEW_TOKEN_DAY_ROWS[0].cost_usd = originalCost;
    }
  });
});
