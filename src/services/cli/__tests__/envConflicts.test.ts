import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { envConflictsCheck } from "../envConflicts";
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

describe("services/cli/envConflicts", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("env conflicts boom"));

    await expect(envConflictsCheck("codex")).rejects.toThrow("env conflicts boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "检查环境变量冲突失败",
      expect.objectContaining({
        cmd: "env_conflicts_check",
        error: expect.stringContaining("env conflicts boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(envConflictsCheck("codex")).rejects.toThrow(
      "IPC_NULL_RESULT: env_conflicts_check"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue([] as any);

    await envConflictsCheck("codex");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("env_conflicts_check", {
      cliKey: "codex",
    });
  });
});
