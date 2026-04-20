import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  costBackfillMissingV1,
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
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
    vi.mocked(commands.costSummaryV1).mockResolvedValueOnce(null as any);

    await expect(costSummaryV1("daily")).rejects.toThrow("IPC_NULL_RESULT: cost_summary_v1");
  });

  it("passes optional args and covers nullish branches", async () => {
    vi.mocked(commands.costSummaryV1).mockResolvedValue({ status: "ok", data: {} as any });
    vi.mocked(commands.costTrendV1).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.costBreakdownProviderV1).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.costBreakdownModelV1).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.costTopRequestsV1).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.costScatterCliProviderModelV1).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.costBackfillMissingV1).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });

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
});
