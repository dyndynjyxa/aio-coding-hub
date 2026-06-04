import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  COST_BACKFILL_MAX_ROWS,
  COST_BREAKDOWN_MAX_LIMIT,
  COST_LIMIT_MIN,
  COST_MODEL_FILTER_MAX_CHARS,
  COST_SCATTER_MAX_LIMIT,
  type CostBackfillReportV1,
  type CostModelBreakdownRowV1,
  type CostProviderBreakdownRowV1,
  type CostScatterCliProviderModelRowV1,
  type CostSummaryV1,
  type CostTopRequestRowV1,
  type CostTrendRowV1,
  costBackfillMissingV1,
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
  normalizeCostQueryInput,
  normalizeCostBackfillMaxRows,
  normalizeCostBreakdownLimit,
  normalizeCostScatterLimit,
  validateCostCliKey,
} from "../cost";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      costSummaryV1: vi.fn(),
      costTrendV1: vi.fn(),
      costBreakdownProviderV1: vi.fn(),
      costBreakdownModelV1: vi.fn(),
      costTopRequestsV1: vi.fn(),
      costScatterCliProviderModelV1: vi.fn(),
      costBackfillMissingV1: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

function makeCostSummary(overrides: Partial<CostSummaryV1> = {}): CostSummaryV1 {
  return {
    requests_total: 10,
    requests_success: 9,
    requests_failed: 1,
    cost_covered_success: 9,
    total_cost_usd: 1.23,
    avg_cost_usd_per_covered_success: 0.12,
    ...overrides,
  };
}

function makeCostTrendRow(overrides: Partial<CostTrendRowV1> = {}): CostTrendRowV1 {
  return {
    day: "2026-04-22",
    hour: null,
    cost_usd: 1.23,
    requests_success: 9,
    cost_covered_success: 9,
    ...overrides,
  };
}

function makeCostProviderBreakdownRow(
  overrides: Partial<CostProviderBreakdownRowV1> = {}
): CostProviderBreakdownRowV1 {
  return {
    cli_key: "claude",
    provider_id: 1,
    provider_name: "P1",
    requests_success: 9,
    cost_covered_success: 9,
    cost_usd: 1.23,
    ...overrides,
  };
}

function makeCostModelBreakdownRow(
  overrides: Partial<CostModelBreakdownRowV1> = {}
): CostModelBreakdownRowV1 {
  return {
    model: "m1",
    requests_success: 9,
    cost_covered_success: 9,
    cost_usd: 1.23,
    ...overrides,
  };
}

function makeCostScatterRow(
  overrides: Partial<CostScatterCliProviderModelRowV1> = {}
): CostScatterCliProviderModelRowV1 {
  return {
    cli_key: "claude",
    provider_name: "P1",
    model: "m1",
    requests_success: 9,
    total_cost_usd: 1.23,
    total_duration_ms: 1234,
    ...overrides,
  };
}

function makeCostTopRequestRow(overrides: Partial<CostTopRequestRowV1> = {}): CostTopRequestRowV1 {
  return {
    log_id: 1,
    trace_id: "trace-1",
    cli_key: "claude",
    method: "GET",
    path: "/v1/messages",
    requested_model: null,
    provider_id: 1,
    provider_name: "P1",
    duration_ms: 100,
    ttfb_ms: null,
    cost_usd: 0.12,
    cost_multiplier: 1,
    created_at: 1,
    ...overrides,
  };
}

function makeCostBackfillReport(
  overrides: Partial<CostBackfillReportV1> = {}
): CostBackfillReportV1 {
  return {
    scanned: 10,
    updated: 9,
    skipped_no_model: 0,
    skipped_no_usage: 0,
    skipped_no_price: 1,
    skipped_other: 0,
    capped: false,
    max_rows: 999,
    ...overrides,
  };
}

describe("services/usage/cost", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.costSummaryV1).mockRejectedValueOnce(new Error("cost boom"));

    await expect(costSummaryV1("daily")).rejects.toThrow("cost boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取花费汇总失败",
      expect.objectContaining({
        cmd: "cost_summary_v1",
        error: expect.stringContaining("cost boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.costSummaryV1).mockResolvedValueOnce(null as never);

    await expect(costSummaryV1("daily")).rejects.toThrow("IPC_NULL_RESULT: cost_summary_v1");
  });

  it("passes optional args and maps generated payloads", async () => {
    vi.mocked(commands.costSummaryV1).mockResolvedValue({ status: "ok", data: makeCostSummary() });
    vi.mocked(commands.costTrendV1).mockResolvedValue({
      status: "ok",
      data: [makeCostTrendRow()],
    });
    vi.mocked(commands.costBreakdownProviderV1).mockResolvedValue({
      status: "ok",
      data: [makeCostProviderBreakdownRow()],
    });
    vi.mocked(commands.costBreakdownModelV1).mockResolvedValue({
      status: "ok",
      data: [makeCostModelBreakdownRow()],
    });
    vi.mocked(commands.costTopRequestsV1).mockResolvedValue({
      status: "ok",
      data: [makeCostTopRequestRow()],
    });
    vi.mocked(commands.costScatterCliProviderModelV1).mockResolvedValue({
      status: "ok",
      data: [makeCostScatterRow()],
    });
    vi.mocked(commands.costBackfillMissingV1).mockResolvedValue({
      status: "ok",
      data: makeCostBackfillReport(),
    });

    // input omitted
    const dailySummary = await costSummaryV1("daily");
    const weeklyTrend = await costTrendV1("weekly");
    const monthlyProviders = await costBreakdownProviderV1("monthly");
    const allTimeModels = await costBreakdownModelV1("allTime");
    const customTopRequests = await costTopRequestsV1("custom");
    const dailyScatter = await costScatterCliProviderModelV1("daily");
    const dailyBackfill = await costBackfillMissingV1("daily");

    // input with values
    await costSummaryV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
    });
    await costTrendV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
    });
    await costBreakdownProviderV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
      limit: 10,
    });
    await costBreakdownModelV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
      limit: 10,
    });
    await costTopRequestsV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
      limit: 10,
    });
    await costScatterCliProviderModelV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
      limit: 10,
    });
    await costBackfillMissingV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
      maxRows: 999,
    });

    expect(dailySummary.total_cost_usd).toBe(1.23);
    expect(weeklyTrend[0]?.day).toBe("2026-04-22");
    expect(monthlyProviders[0]?.cli_key).toBe("claude");
    expect(allTimeModels[0]?.model).toBe("m1");
    expect(customTopRequests[0]?.cli_key).toBe("claude");
    expect(dailyScatter[0]?.cli_key).toBe("claude");
    expect(dailyBackfill.max_rows).toBe(999);

    expect(commands.costSummaryV1).toHaveBeenLastCalledWith(
      expect.objectContaining({
        period: "custom",
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        model: "m1",
      })
    );
    expect(commands.costBackfillMissingV1).toHaveBeenLastCalledWith(
      expect.objectContaining({
        period: "custom",
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        model: "m1",
      }),
      999
    );
  });

  it("normalizes cost filters before ipc", async () => {
    vi.mocked(commands.costSummaryV1).mockClear();
    vi.mocked(commands.costSummaryV1).mockResolvedValue({ status: "ok", data: makeCostSummary() });

    expect(validateCostCliKey(" claude ")).toBe("claude");
    expect(validateCostCliKey("   ")).toBeNull();
    expect(
      normalizeCostQueryInput({
        startTs: 1,
        endTs: 2,
        cliKey: " codex " as never,
        providerId: 3,
        model: " gpt-test ",
      })
    ).toEqual({
      startTs: 1,
      endTs: 2,
      cliKey: "codex",
      providerId: 3,
      model: "gpt-test",
    });
    expect(
      normalizeCostQueryInput({
        cliKey: " " as never,
        model: " ",
      })
    ).toEqual({
      startTs: null,
      endTs: null,
      cliKey: null,
      providerId: null,
      model: null,
    });

    await costSummaryV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: " claude " as never,
      providerId: 3,
      model: " m1 ",
    });

    expect(commands.costSummaryV1).toHaveBeenCalledWith({
      period: "custom",
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 3,
      model: "m1",
    });
  });

  it("rejects invalid cost filters before ipc", async () => {
    vi.mocked(commands.costSummaryV1).mockClear();

    await expect(costSummaryV1("daily", { cliKey: "opencode" as never })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(costSummaryV1("daily", { providerId: 0 })).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(costSummaryV1("daily", { startTs: Number.NaN })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(costSummaryV1("daily", { endTs: -1 })).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      costSummaryV1("daily", { model: "x".repeat(COST_MODEL_FILTER_MAX_CHARS + 1) })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.costSummaryV1).not.toHaveBeenCalled();
  });

  it("normalizes cost limits before ipc", async () => {
    vi.mocked(commands.costBreakdownProviderV1).mockClear();
    vi.mocked(commands.costBreakdownModelV1).mockClear();
    vi.mocked(commands.costTopRequestsV1).mockClear();
    vi.mocked(commands.costScatterCliProviderModelV1).mockClear();
    vi.mocked(commands.costBackfillMissingV1).mockClear();

    vi.mocked(commands.costBreakdownProviderV1).mockResolvedValue({
      status: "ok",
      data: [makeCostProviderBreakdownRow()],
    });
    vi.mocked(commands.costBreakdownModelV1).mockResolvedValue({
      status: "ok",
      data: [makeCostModelBreakdownRow()],
    });
    vi.mocked(commands.costTopRequestsV1).mockResolvedValue({
      status: "ok",
      data: [makeCostTopRequestRow()],
    });
    vi.mocked(commands.costScatterCliProviderModelV1).mockResolvedValue({
      status: "ok",
      data: [makeCostScatterRow()],
    });
    vi.mocked(commands.costBackfillMissingV1).mockResolvedValue({
      status: "ok",
      data: makeCostBackfillReport(),
    });

    expect(normalizeCostBreakdownLimit(null)).toBeNull();
    expect(normalizeCostBreakdownLimit(0)).toBe(COST_LIMIT_MIN);
    expect(normalizeCostBreakdownLimit(999)).toBe(COST_BREAKDOWN_MAX_LIMIT);
    expect(normalizeCostScatterLimit(99_999)).toBe(COST_SCATTER_MAX_LIMIT);
    expect(normalizeCostBackfillMaxRows(99_999)).toBe(COST_BACKFILL_MAX_ROWS);

    await costBreakdownProviderV1("daily", { limit: 0 });
    await costBreakdownModelV1("daily", { limit: 999 });
    await costTopRequestsV1("daily", { limit: 999 });
    await costScatterCliProviderModelV1("daily", { limit: 99_999 });
    await costBackfillMissingV1("daily", { maxRows: 99_999 });

    expect(commands.costBreakdownProviderV1).toHaveBeenCalledWith(
      expect.objectContaining({ period: "daily" }),
      COST_LIMIT_MIN
    );
    expect(commands.costBreakdownModelV1).toHaveBeenCalledWith(
      expect.objectContaining({ period: "daily" }),
      COST_BREAKDOWN_MAX_LIMIT
    );
    expect(commands.costTopRequestsV1).toHaveBeenCalledWith(
      expect.objectContaining({ period: "daily" }),
      COST_BREAKDOWN_MAX_LIMIT
    );
    expect(commands.costScatterCliProviderModelV1).toHaveBeenCalledWith(
      expect.objectContaining({ period: "daily" }),
      COST_SCATTER_MAX_LIMIT
    );
    expect(commands.costBackfillMissingV1).toHaveBeenCalledWith(
      expect.objectContaining({ period: "daily" }),
      COST_BACKFILL_MAX_ROWS
    );
  });

  it("rejects invalid cost limits before ipc", async () => {
    vi.mocked(commands.costBreakdownProviderV1).mockClear();
    vi.mocked(commands.costScatterCliProviderModelV1).mockClear();
    vi.mocked(commands.costBackfillMissingV1).mockClear();

    await expect(costBreakdownProviderV1("daily", { limit: Number.NaN })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(costScatterCliProviderModelV1("daily", { limit: 1.5 })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(
      costBackfillMissingV1("daily", { maxRows: Number.POSITIVE_INFINITY })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.costBreakdownProviderV1).not.toHaveBeenCalled();
    expect(commands.costScatterCliProviderModelV1).not.toHaveBeenCalled();
    expect(commands.costBackfillMissingV1).not.toHaveBeenCalled();
  });
});
