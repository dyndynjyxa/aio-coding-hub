import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  usageHourlySeries,
  usageLeaderboardDay,
  usageLeaderboardProvider,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageSummary,
  usageSummaryV2,
} from "../usage";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
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
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("usage boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(usageSummary("today")).rejects.toThrow("IPC_NULL_RESULT: usage_summary");
  });

  it("passes normalized args to invokeTauriOrNull", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

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

    expect(vi.mocked(invokeTauriOrNull).mock.calls).toEqual(
      expect.arrayContaining([
        ["usage_summary", { range: "today", cliKey: null }],
        ["usage_summary", { range: "last7", cliKey: "claude" }],
        ["usage_leaderboard_provider", { range: "today", cliKey: null, limit: undefined }],
        ["usage_leaderboard_provider", { range: "today", cliKey: "codex", limit: 10 }],
        ["usage_leaderboard_day", { range: "today", cliKey: null, limit: undefined }],
        ["usage_leaderboard_day", { range: "today", cliKey: "gemini", limit: 20 }],
        ["usage_hourly_series", { days: 15 }],
        [
          "usage_summary_v2",
          {
            params: {
              period: "custom",
              startTs: null,
              endTs: null,
              cliKey: null,
              providerId: null,
            },
          },
        ],
        [
          "usage_summary_v2",
          {
            params: {
              period: "custom",
              startTs: 1,
              endTs: 2,
              cliKey: "gemini",
              providerId: 7,
            },
          },
        ],
        [
          "usage_leaderboard_v2",
          {
            scope: "provider",
            params: {
              period: "custom",
              startTs: null,
              endTs: null,
              cliKey: null,
              providerId: null,
            },
            limit: undefined,
          },
        ],
        [
          "usage_leaderboard_v2",
          {
            scope: "provider",
            params: {
              period: "custom",
              startTs: 1,
              endTs: 2,
              cliKey: "claude",
              providerId: 9,
            },
            limit: null,
          },
        ],
        [
          "usage_provider_cache_rate_trend_v1",
          {
            params: {
              period: "daily",
              startTs: 1,
              endTs: 2,
              cliKey: "claude",
              providerId: 11,
            },
            limit: 20,
          },
        ],
      ])
    );
  });
});
