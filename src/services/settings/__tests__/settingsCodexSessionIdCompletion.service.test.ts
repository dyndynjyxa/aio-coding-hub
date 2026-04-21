import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { settingsCodexSessionIdCompletionSet } from "../settingsCodexSessionIdCompletion";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      settingsCodexSessionIdCompletionSet: vi.fn(),
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

describe("services/settings/settingsCodexSessionIdCompletion", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.settingsCodexSessionIdCompletionSet).mockRejectedValueOnce(
      new Error("codex session boom")
    );

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

  it("maps generated args and treats null as runtime error", async () => {
    vi.mocked(commands.settingsCodexSessionIdCompletionSet).mockResolvedValueOnce(null as any);
    await expect(settingsCodexSessionIdCompletionSet(true)).rejects.toThrow(
      "IPC_NULL_RESULT: settings_codex_session_id_completion_set"
    );

    vi.mocked(commands.settingsCodexSessionIdCompletionSet).mockResolvedValueOnce({
      status: "ok",
      data: { schema_version: 1 } as any,
    });
    await settingsCodexSessionIdCompletionSet(true);

    expect(commands.settingsCodexSessionIdCompletionSet).toHaveBeenCalledWith({
      enableCodexSessionIdCompletion: true,
    });
  });
});
