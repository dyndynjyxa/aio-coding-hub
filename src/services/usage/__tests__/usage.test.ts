import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
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
    vi.mocked(commands.usageSummary).mockResolvedValueOnce(null as any);

    await expect(usageSummary("today")).rejects.toThrow("IPC_NULL_RESULT: usage_summary");
  });

  it("passes normalized args to generated commands", async () => {
    vi.mocked(commands.usageSummary).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.usageLeaderboardProvider).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.usageLeaderboardDay).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.usageHourlySeries).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.usageSummaryV2).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.usageLeaderboardV2).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.usageProviderCacheRateTrendV1).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });

    await usageSummary("today");
    await usageSummary("last7", { cliKey: "claude" });

    await usageLeaderboardProvider("today");
    await usageLeaderboardProvider("today", { cliKey: "codex", limit: 10 });

    await usageLeaderboardDay("today");
    await usageLeaderboardDay("today", { cliKey: "gemini", limit: 20 });

    await usageHourlySeries(15);

    await usageSummaryV2("custom");
    await usageSummaryV2("custom", { startTs: 1, endTs: 2, cliKey: "gemini", providerId: 7 });

    await usageLeaderboardV2("provider", "custom");
    await usageLeaderboardV2("provider", "custom", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 9,
      limit: null,
    });

    await usageProviderCacheRateTrendV1("daily", {
      startTs: 1,
      endTs: 2,
      cliKey: "claude",
      providerId: 11,
      limit: 20,
    });

    expect(commands.usageSummary).toHaveBeenNthCalledWith(1, "today", null);
    expect(commands.usageSummary).toHaveBeenNthCalledWith(2, "last7", "claude");
    expect(commands.usageLeaderboardProvider).toHaveBeenNthCalledWith(1, "today", null, null);
    expect(commands.usageLeaderboardProvider).toHaveBeenNthCalledWith(2, "today", "codex", 10);
    expect(commands.usageLeaderboardDay).toHaveBeenNthCalledWith(1, "today", null, null);
    expect(commands.usageLeaderboardDay).toHaveBeenNthCalledWith(2, "today", "gemini", 20);
    expect(commands.usageHourlySeries).toHaveBeenCalledWith(15);
    expect(commands.usageSummaryV2).toHaveBeenNthCalledWith(1, {
      period: "custom",
      startTs: null,
      endTs: null,
      cliKey: null,
      providerId: null,
    });
    expect(commands.usageSummaryV2).toHaveBeenNthCalledWith(2, {
      period: "custom",
      startTs: 1,
      endTs: 2,
      cliKey: "gemini",
      providerId: 7,
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
      },
      null
    );
    expect(commands.usageProviderCacheRateTrendV1).toHaveBeenCalledWith(
      {
        period: "daily",
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 11,
      },
      20
    );
  });
});
