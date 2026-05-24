import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { envConflictsCheck, normalizeEnvConflictCliKey } from "../envConflicts";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      envConflictsCheck: vi.fn(),
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

describe("services/cli/envConflicts", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.envConflictsCheck).mockRejectedValueOnce(new Error("env conflicts boom"));

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

  it("returns null fallback when generated command returns null", async () => {
    vi.mocked(commands.envConflictsCheck).mockResolvedValueOnce(null as any);

    await expect(envConflictsCheck("codex")).resolves.toBeNull();
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.envConflictsCheck).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });

    await envConflictsCheck(" CODEX " as any);
    expect(commands.envConflictsCheck).toHaveBeenCalledWith("codex");
  });

  it("normalizes env conflict result rows", async () => {
    vi.mocked(commands.envConflictsCheck).mockResolvedValueOnce({
      status: "ok",
      data: [
        {
          var_name: "  OPENAI_API_KEY  ",
          source_type: "file",
          source_path: "  /tmp/.zshrc:1  ",
        },
      ],
    });

    await expect(envConflictsCheck("codex")).resolves.toEqual([
      {
        var_name: "OPENAI_API_KEY",
        source_type: "file",
        source_path: "/tmp/.zshrc:1",
      },
    ]);
  });

  it("rejects invalid cli keys before generated commands", async () => {
    expect(normalizeEnvConflictCliKey(" gemini ")).toBe("gemini");
    expect(() => normalizeEnvConflictCliKey("npm")).toThrow("invalid cliKey=npm");

    await expect(envConflictsCheck("npm" as any)).rejects.toThrow("SEC_INVALID_INPUT");
    expect(commands.envConflictsCheck).not.toHaveBeenCalled();
  });

  it("rejects invalid env conflict result rows", async () => {
    vi.mocked(commands.envConflictsCheck).mockResolvedValueOnce({
      status: "ok",
      data: [
        {
          var_name: "OPENAI_API_KEY",
          source_type: "profile",
          source_path: "/tmp/.zshrc:1",
        },
      ],
    });

    await expect(envConflictsCheck("codex")).rejects.toThrow(
      "invalid env conflict source_type=profile"
    );
  });
});
