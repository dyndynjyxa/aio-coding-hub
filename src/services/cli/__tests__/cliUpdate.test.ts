import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { cliCheckLatestVersion, cliUpdateCli, normalizeCliUpdateKey } from "../cliUpdate";

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
    vi.mocked(commands.cliCheckLatestVersion).mockRejectedValueOnce(
      new Error("version check boom")
    );

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

    await cliCheckLatestVersion(" Claude ");
    expect(commands.cliCheckLatestVersion).toHaveBeenCalledWith("claude");

    await cliUpdateCli(" CODEX ");
    expect(commands.cliUpdate).toHaveBeenCalledWith("codex");
  });

  it("normalizes cli update command results before returning them", async () => {
    vi.mocked(commands.cliCheckLatestVersion).mockResolvedValueOnce({
      status: "ok",
      data: {
        cliKey: " gemini ",
        npmPackage: "  @google/gemini-cli  ",
        installedVersion: "  1.0.0  ",
        latestVersion: "  1.1.0  ",
        updateAvailable: true,
        error: "   ",
      },
    });
    vi.mocked(commands.cliUpdate).mockResolvedValueOnce({
      status: "ok",
      data: {
        cliKey: " codex ",
        success: true,
        output: "  installed  ",
        error: "   ",
      },
    });

    await expect(cliCheckLatestVersion("gemini")).resolves.toEqual({
      cliKey: "gemini",
      npmPackage: "@google/gemini-cli",
      installedVersion: "1.0.0",
      latestVersion: "1.1.0",
      updateAvailable: true,
      error: null,
    });
    await expect(cliUpdateCli("codex")).resolves.toEqual({
      cliKey: "codex",
      success: true,
      output: "installed",
      error: null,
    });
  });

  it("rejects invalid cli keys before generated commands", async () => {
    expect(normalizeCliUpdateKey(" codex ")).toBe("codex");
    expect(() => normalizeCliUpdateKey("npm")).toThrow("invalid cliKey=npm");

    await expect(cliCheckLatestVersion("npm")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(cliUpdateCli("   ")).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.cliCheckLatestVersion).not.toHaveBeenCalled();
    expect(commands.cliUpdate).not.toHaveBeenCalled();
  });

  it("rejects invalid cli update command result shapes", async () => {
    vi.mocked(commands.cliCheckLatestVersion).mockResolvedValueOnce({
      status: "ok",
      data: {
        cliKey: "claude",
        npmPackage: "@anthropic-ai/claude-code",
        installedVersion: null,
        latestVersion: "1.1.0",
        updateAvailable: "yes" as any,
        error: null,
      },
    });

    await expect(cliCheckLatestVersion("claude")).rejects.toThrow(
      "cli_version_check.updateAvailable must be a boolean"
    );
  });
});
