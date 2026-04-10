import { describe, expect, it, vi } from "vitest";
import {
  claudeValidationHistoryClearProvider,
  claudeValidationHistoryList,
} from "../claudeModelValidationHistory";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";

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

describe("services/claude/claudeModelValidationHistory", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("history boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(claudeValidationHistoryList({ provider_id: 1, limit: 50 })).rejects.toThrow(
      "IPC_NULL_RESULT: claude_validation_history_list"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue(true as any);

    await claudeValidationHistoryList({ provider_id: 1, limit: 50 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("claude_validation_history_list", {
      providerId: 1,
      limit: 50,
    });

    await claudeValidationHistoryClearProvider({ provider_id: 1 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("claude_validation_history_clear_provider", {
      providerId: 1,
    });
  });
});
