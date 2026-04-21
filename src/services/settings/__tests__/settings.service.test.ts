import { describe, expect, it, vi } from "vitest";
import { settingsGet } from "../settings";
import { logToConsole } from "../../consoleLog";
import { commands } from "../../../generated/bindings";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      settingsGet: vi.fn(),
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

describe("services/settings (error semantics)", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.settingsGet).mockRejectedValueOnce(new Error("settings boom"));

    await expect(settingsGet()).rejects.toThrow("settings boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取设置失败",
      expect.objectContaining({
        cmd: "settings_get",
        error: expect.stringContaining("settings boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.settingsGet).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(settingsGet()).rejects.toThrow("IPC_NULL_RESULT: settings_get");
  });
});
