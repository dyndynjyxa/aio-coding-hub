import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  cliManagerClaudeEnvSet,
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexConfigTomlValidate,
  cliManagerCodexInfoGet,
} from "../cliManager";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      cliManagerClaudeInfoGet: vi.fn(),
      cliManagerCodexInfoGet: vi.fn(),
      cliManagerCodexConfigSet: vi.fn(),
      cliManagerCodexConfigTomlGet: vi.fn(),
      cliManagerCodexConfigTomlValidate: vi.fn(),
      cliManagerCodexConfigTomlSet: vi.fn(),
      cliManagerClaudeEnvSet: vi.fn(),
      cliManagerClaudeSettingsGet: vi.fn(),
      cliManagerClaudeSettingsSet: vi.fn(),
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

describe("services/cli/cliManager", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.cliManagerClaudeInfoGet).mockRejectedValueOnce(new Error("cli manager boom"));

    await expect(cliManagerClaudeInfoGet()).rejects.toThrow("cli manager boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "获取 Claude CLI 信息失败",
      expect.objectContaining({
        cmd: "cli_manager_claude_info_get",
        error: expect.stringContaining("cli manager boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.cliManagerClaudeInfoGet).mockResolvedValueOnce(null as any);

    await expect(cliManagerClaudeInfoGet()).rejects.toThrow(
      "IPC_NULL_RESULT: cli_manager_claude_info_get"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.cliManagerCodexInfoGet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerCodexConfigSet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerCodexConfigTomlGet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerCodexConfigTomlValidate).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerCodexConfigTomlSet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerClaudeEnvSet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerClaudeSettingsGet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });
    vi.mocked(commands.cliManagerClaudeSettingsSet).mockResolvedValue({
      status: "ok",
      data: {} as any,
    });

    await cliManagerCodexInfoGet();
    expect(commands.cliManagerCodexInfoGet).toHaveBeenCalledWith();

    await cliManagerCodexConfigSet({ model: "gpt-5" });
    expect(commands.cliManagerCodexConfigSet).toHaveBeenCalledWith(
      expect.objectContaining({ model: "gpt-5" })
    );

    await cliManagerCodexConfigTomlGet();
    expect(commands.cliManagerCodexConfigTomlGet).toHaveBeenCalledWith();

    await cliManagerCodexConfigTomlValidate('model = "gpt-5"');
    expect(commands.cliManagerCodexConfigTomlValidate).toHaveBeenCalledWith('model = "gpt-5"');

    await cliManagerCodexConfigTomlSet('model = "gpt-5"');
    expect(commands.cliManagerCodexConfigTomlSet).toHaveBeenCalledWith('model = "gpt-5"');

    await cliManagerClaudeEnvSet({ mcp_timeout_ms: 30_000, disable_error_reporting: true });
    expect(commands.cliManagerClaudeEnvSet).toHaveBeenCalledWith(30_000, true);

    await cliManagerClaudeSettingsGet();
    expect(commands.cliManagerClaudeSettingsGet).toHaveBeenCalledWith();

    await cliManagerClaudeSettingsSet({ model: "claude-3" });
    expect(commands.cliManagerClaudeSettingsSet).toHaveBeenCalledWith(
      expect.objectContaining({ model: "claude-3" })
    );
  });
});
