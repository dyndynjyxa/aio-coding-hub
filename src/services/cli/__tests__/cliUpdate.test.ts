import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { cliCheckLatestVersion, cliUpdateCli } from "../cliUpdate";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      cliCheckLatestVersion: vi.fn(),
      cliUpdate: vi.fn(),
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

describe("services/cli/cliUpdate", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.cliCheckLatestVersion).mockRejectedValueOnce(new Error("version check boom"));

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
    vi.mocked(commands.cliCheckLatestVersion).mockResolvedValueOnce({
      status: "ok",
      data: null as any,
    });

    await expect(cliCheckLatestVersion("codex")).rejects.toThrow(
      "IPC_NULL_RESULT: cli_check_latest_version"
    );
  });

  it("invokes cli update commands with expected parameters", async () => {
    vi.mocked(commands.cliCheckLatestVersion).mockResolvedValueOnce({
      status: "ok",
      data: {
        cliKey: "claude",
        npmPackage: "@anthropic-ai/claude-code",
        installedVersion: "1.0.0",
        latestVersion: "1.1.0",
        updateAvailable: true,
        error: null,
      },
    });
    vi.mocked(commands.cliUpdate).mockResolvedValueOnce({
      status: "ok",
      data: {
        cliKey: "codex",
        success: true,
        output: "ok",
        error: null,
      },
    });

    await cliCheckLatestVersion("claude");
    expect(commands.cliCheckLatestVersion).toHaveBeenCalledWith("claude");

    await cliUpdateCli("codex");
    expect(commands.cliUpdate).toHaveBeenCalledWith("codex");
  });
});
