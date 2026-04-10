import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { settingsCodexSessionIdCompletionSet } from "../settingsCodexSessionIdCompletion";
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

describe("services/settings/settingsCodexSessionIdCompletion", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("codex session boom"));

    await expect(settingsCodexSessionIdCompletionSet(true)).rejects.toThrow("codex session boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存 Codex Session ID 补全设置失败",
      expect.objectContaining({
        cmd: "settings_codex_session_id_completion_set",
        error: expect.stringContaining("codex session boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(settingsCodexSessionIdCompletionSet(true)).rejects.toThrow(
      "IPC_NULL_RESULT: settings_codex_session_id_completion_set"
    );
  });
});
