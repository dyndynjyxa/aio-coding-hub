import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  costBackfillMissingV1,
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
} from "../cost";

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

describe("services/usage/cost", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("cost boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(costSummaryV1("daily")).rejects.toThrow("IPC_NULL_RESULT: cost_summary_v1");
  });

  it("passes optional args and covers nullish branches", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    // input omitted
    await costSummaryV1("daily");
    await costTrendV1("weekly");
    await costBreakdownProviderV1("monthly");
    await costBreakdownModelV1("allTime");
    await costTopRequestsV1("custom");
    await costScatterCliProviderModelV1("daily");
    await costBackfillMissingV1("daily");

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

    expect(invokeTauriOrNull).toHaveBeenCalledWith(
      "cost_summary_v1",
      expect.objectContaining({
        params: expect.objectContaining({
          period: "custom",
          startTs: 1,
          endTs: 2,
          cliKey: "claude",
        }),
      })
    );
    expect(invokeTauriOrNull).toHaveBeenCalledWith(
      "cost_backfill_missing_v1",
      expect.objectContaining({ maxRows: 999 })
    );
  });
});
