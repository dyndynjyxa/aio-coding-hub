import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  type UsageDayDetailV1,
  type UsageFolderOptionV1,
  type UsageDayRow,
  type UsageHourlyRow,
  type UsageLeaderboardRow,
  type UsageProviderCacheRateTrendRowV1,
  type UsageProviderRow,
  type UsageSummary,
  usageDayDetailV1,
  usageFolderOptionsV1,
  usageHourlySeries,
  usageLeaderboardDay,
  usageLeaderboardProvider,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageSummary,
  usageSummaryV2,
} from "../usage";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      usageSummary: vi.fn(),
      usageLeaderboardProvider: vi.fn(),
      usageLeaderboardDay: vi.fn(),
      usageHourlySeries: vi.fn(),
      usageDayDetailV1: vi.fn(),
      usageFolderOptionsV1: vi.fn(),
      usageSummaryV2: vi.fn(),
      usageLeaderboardV2: vi.fn(),
      usageProviderCacheRateTrendV1: vi.fn(),
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

function makeUsageSummary(overrides: Partial<UsageSummary> = {}): UsageSummary {
  return {
    requests_total: 1,
    requests_with_usage: 1,
    requests_success: 1,
    requests_failed: 0,
    cost_covered_success: 1,
    avg_duration_ms: 120,
    avg_ttfb_ms: 30,
    avg_output_tokens_per_second: 10,
    input_tokens: 100,
    output_tokens: 200,
    io_total_tokens: 300,
    total_tokens: 300,
    cache_read_input_tokens: 0,
    cache_creation_input_tokens: 0,
    cache_creation_5m_input_tokens: 0,
    cache_creation_1h_input_tokens: 0,
    ...overrides,
  };
}

function makeUsageProviderRow(overrides: Partial<UsageProviderRow> = {}): UsageProviderRow {
  return {
    cli_key: "claude",
    provider_id: 1,
    provider_name: "P1",
    requests_total: 1,
    requests_success: 1,
    requests_failed: 0,
    avg_duration_ms: 120,
    avg_ttfb_ms: 30,
    avg_output_tokens_per_second: 10,
    input_tokens: 100,
    output_tokens: 200,
    total_tokens: 300,
    cache_read_input_tokens: 0,
    cache_creation_input_tokens: 0,
    cache_creation_5m_input_tokens: 0,
    cache_creation_1h_input_tokens: 0,
    ...overrides,
  };
}

function makeUsageDayRow(overrides: Partial<UsageDayRow> = {}): UsageDayRow {
  return {
    day: "2026-04-22",
    requests_total: 1,
    input_tokens: 100,
    output_tokens: 200,
    total_tokens: 300,
    cache_read_input_tokens: 0,
    cache_creation_input_tokens: 0,
    cache_creation_5m_input_tokens: 0,
    cache_creation_1h_input_tokens: 0,
    ...overrides,
  };
}

function makeUsageHourlyRow(overrides: Partial<UsageHourlyRow> = {}): UsageHourlyRow {
  return {
    day: "2026-04-22",
    hour: 13,
    requests_total: 1,
    requests_with_usage: 1,
    requests_success: 1,
    requests_failed: 0,
    total_tokens: 300,
    ...overrides,
  };
}

function makeUsageLeaderboardRow(
  overrides: Partial<UsageLeaderboardRow> = {}
): UsageLeaderboardRow {
  return {
    key: "provider:1",
    name: "P1",
    requests_total: 1,
    requests_success: 1,
    requests_failed: 0,
    total_tokens: 300,
    io_total_tokens: 300,
    input_tokens: 100,
    output_tokens: 200,
    cache_creation_input_tokens: 0,
    cache_read_input_tokens: 0,
    avg_duration_ms: 120,
    avg_ttfb_ms: 30,
    avg_output_tokens_per_second: 10,
    cost_usd: 1.23,
    ...overrides,
  };
}

function makeUsageProviderCacheRateTrendRow(
  overrides: Partial<UsageProviderCacheRateTrendRowV1> = {}
): UsageProviderCacheRateTrendRowV1 {
  return {
    day: "2026-04-22",
    hour: null,
    key: "provider:1",
    name: "P1",
    denom_tokens: 300,
    cache_read_input_tokens: 30,
    requests_success: 1,
    ...overrides,
  };
}

function makeUsageDayDetail(overrides: Partial<UsageDayDetailV1> = {}): UsageDayDetailV1 {
  return {
    day: "2026-04-22",
    folders: [
      {
        key: "/tmp/project",
        name: "project",
        folder_path: "/tmp/project",
        requests_total: 1,
        requests_success: 1,
        requests_failed: 0,
        total_tokens: 300,
        io_total_tokens: 300,
        input_tokens: 100,
        output_tokens: 200,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        avg_duration_ms: 120,
        avg_ttfb_ms: 30,
        avg_output_tokens_per_second: 10,
        cost_usd: 1.23,
      },
    ],
    hours: Array.from({ length: 24 }, (_, hour) => ({
      hour,
      requests_total: hour === 13 ? 1 : 0,
      total_tokens: hour === 13 ? 300 : 0,
      io_total_tokens: hour === 13 ? 300 : 0,
    })),
    ...overrides,
  };
}

function makeUsageFolderOption(overrides: Partial<UsageFolderOptionV1> = {}): UsageFolderOptionV1 {
  return {
    key: "/tmp/project",
    name: "project",
    folder_path: "/tmp/project",
    requests_total: 1,
    total_tokens: 300,
    ...overrides,
  };
}

describe("services/usage/usage", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.usageSummary).mockRejectedValueOnce(new Error("usage boom"));

    await expect(usageSummary("today")).rejects.toThrow("usage boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取用量汇总失败",
      expect.objectContaining({
        cmd: "usage_summary",
        error: expect.stringContaining("usage boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.usageSummary).mockResolvedValueOnce(null as never);

    await expect(usageSummary("today")).rejects.toThrow("IPC_NULL_RESULT: usage_summary");
  });

  it("passes normalized args and maps generated payloads", async () => {
    vi.mocked(commands.usageSummary).mockResolvedValue({ status: "ok", data: makeUsageSummary() });
    vi.mocked(commands.usageLeaderboardProvider).mockResolvedValue({
      status: "ok",
      data: [makeUsageProviderRow()],
    });
    vi.mocked(commands.usageLeaderboardDay).mockResolvedValue({
      status: "ok",
      data: [makeUsageDayRow()],
    });
    vi.mocked(commands.usageHourlySeries).mockResolvedValue({
      status: "ok",
      data: [makeUsageHourlyRow()],
    });
    vi.mocked(commands.usageDayDetailV1).mockResolvedValue({
      status: "ok",
      data: makeUsageDayDetail(),
    });
    vi.mocked(commands.usageFolderOptionsV1).mockResolvedValue({
      status: "ok",
      data: [makeUsageFolderOption()],
    });
    vi.mocked(commands.usageSummaryV2).mockResolvedValue({
      status: "ok",
      data: makeUsageSummary({ requests_total: 2 }),
    });
    vi.mocked(commands.usageLeaderboardV2).mockResolvedValue({
      status: "ok",
      data: [makeUsageLeaderboardRow()],
    });
    vi.mocked(commands.usageProviderCacheRateTrendV1).mockResolvedValue({
      status: "ok",
      data: [makeUsageProviderCacheRateTrendRow()],
    });

    const todaySummary = await usageSummary("today");
    const cliSummary = await usageSummary("last7", { cliKey: "claude" });

    const providerRows = await usageLeaderboardProvider("today");
    await usageLeaderboardProvider("today", { cliKey: "codex", limit: 10 });

    const dayRows = await usageLeaderboardDay("today");
    await usageLeaderboardDay("today", { cliKey: "gemini", limit: 20 });

    const hourlyRows = await usageHourlySeries(15);
    const dayDetail = await usageDayDetailV1({
      day: "2026-04-22",
      cliKey: null,
      providerId: null,
      folderLimit: 8,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    });

    const summaryV2 = await usageSummaryV2("custom");
    await usageSummaryV2("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "gemini",
      providerId: 7,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    });

    const leaderboardRows = await usageLeaderboardV2("provider", "custom");
    await usageLeaderboardV2("provider", "custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 9,
      limit: null,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    });
    const folderOptions = await usageFolderOptionsV1("custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 9,
      excludeCx2CcGatewayBridge: true,
    });

    const cacheRateRows = await usageProviderCacheRateTrendV1("daily", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 11,
      limit: 20,
      excludeCx2CcGatewayBridge: true,
    });

    expect(todaySummary.requests_total).toBe(1);
    expect(cliSummary.requests_success).toBe(1);
    expect(providerRows[0]?.cli_key).toBe("claude");
    expect(dayRows[0]?.day).toBe("2026-04-22");
    expect(hourlyRows[0]?.hour).toBe(13);
    expect(dayDetail.folders[0]?.name).toBe("project");
    expect(summaryV2.requests_total).toBe(2);
    expect(leaderboardRows[0]?.key).toBe("provider:1");
    expect(folderOptions[0]?.key).toBe("/tmp/project");
    expect(cacheRateRows[0]?.key).toBe("provider:1");

    expect(commands.usageSummary).toHaveBeenNthCalledWith(1, "today", null);
    expect(commands.usageSummary).toHaveBeenNthCalledWith(2, "last7", "claude");
    expect(commands.usageLeaderboardProvider).toHaveBeenNthCalledWith(1, "today", null, null);
    expect(commands.usageLeaderboardProvider).toHaveBeenNthCalledWith(2, "today", "codex", 10);
    expect(commands.usageLeaderboardDay).toHaveBeenNthCalledWith(1, "today", null, null);
    expect(commands.usageLeaderboardDay).toHaveBeenNthCalledWith(2, "today", "gemini", 20);
    expect(commands.usageHourlySeries).toHaveBeenCalledWith(15);
    expect(commands.usageDayDetailV1).toHaveBeenCalledWith({
      day: "2026-04-22",
      cliKey: null,
      providerId: null,
      folderLimit: 8,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    });
    expect(commands.usageSummaryV2).toHaveBeenNthCalledWith(1, {
      period: "custom",
      startTs: null,
      endTs: null,
      cliKey: null,
      providerId: null,
      folderKeys: null,
      excludeCx2CcGatewayBridge: null,
    });
    expect(commands.usageSummaryV2).toHaveBeenNthCalledWith(2, {
      period: "custom",
      startTs: 1,
      endTs: 2,
      cliKey: "gemini",
      providerId: 7,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    });
    expect(commands.usageLeaderboardV2).toHaveBeenNthCalledWith(
      1,
      "provider",
      {
        period: "custom",
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        folderKeys: null,
        excludeCx2CcGatewayBridge: null,
      },
      null
    );
    expect(commands.usageLeaderboardV2).toHaveBeenNthCalledWith(
      2,
      "provider",
      {
        period: "custom",
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 9,
        folderKeys: ["/tmp/project"],
        excludeCx2CcGatewayBridge: true,
      },
      null
    );
    expect(commands.usageFolderOptionsV1).toHaveBeenCalledWith({
      period: "custom",
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 9,
      folderKeys: null,
      excludeCx2CcGatewayBridge: true,
    });
    expect(commands.usageProviderCacheRateTrendV1).toHaveBeenCalledWith(
      {
        period: "daily",
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 11,
        folderKeys: null,
        excludeCx2CcGatewayBridge: true,
      },
      20
    );
  });
});
