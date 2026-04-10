import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import { cliCheckLatestVersion, cliUpdateCli } from "../cliUpdate";

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

describe("services/cli/cliUpdate", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("version check boom"));

    await expect(cliCheckLatestVersion("claude")).rejects.toThrow("version check boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "检查版本失败",
      expect.objectContaining({
        cmd: "cli_check_latest_version",
        error: expect.stringContaining("version check boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(cliCheckLatestVersion("codex")).rejects.toThrow(
      "IPC_NULL_RESULT: cli_check_latest_version"
    );
  });

  it("invokes cli update commands with expected parameters", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await cliCheckLatestVersion("claude");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_check_latest_version", {
      cliKey: "claude",
    });

    await cliUpdateCli("codex");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_update", { cliKey: "codex" });
  });
});
