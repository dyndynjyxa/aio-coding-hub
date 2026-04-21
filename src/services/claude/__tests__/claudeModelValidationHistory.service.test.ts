import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import {
  claudeValidationHistoryClearProvider,
  claudeValidationHistoryList,
} from "../claudeModelValidationHistory";
import { logToConsole } from "../../consoleLog";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      claudeValidationHistoryList: vi.fn(),
      claudeValidationHistoryClearProvider: vi.fn(),
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

describe("services/claude/claudeModelValidationHistory", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.claudeValidationHistoryList).mockRejectedValueOnce(new Error("history boom"));

    await expect(claudeValidationHistoryList({ provider_id: 1, limit: 50 })).rejects.toThrow(
      "history boom"
    );

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取 Claude 模型验证历史失败",
      expect.objectContaining({
        cmd: "claude_validation_history_list",
        error: expect.stringContaining("history boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.claudeValidationHistoryList).mockResolvedValueOnce(null as any);

    await expect(claudeValidationHistoryList({ provider_id: 1, limit: 50 })).rejects.toThrow(
      "IPC_NULL_RESULT: claude_validation_history_list"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.claudeValidationHistoryList).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.claudeValidationHistoryClearProvider).mockResolvedValue({
      status: "ok",
      data: true,
    });

    await claudeValidationHistoryList({ provider_id: 1, limit: 50 });
    expect(commands.claudeValidationHistoryList).toHaveBeenCalledWith(1, 50);

    await claudeValidationHistoryClearProvider({ provider_id: 1 });
    expect(commands.claudeValidationHistoryClearProvider).toHaveBeenCalledWith(1);
  });
});
