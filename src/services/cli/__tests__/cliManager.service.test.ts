import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
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

describe("services/cli/cliManager", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("cli manager boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(cliManagerClaudeInfoGet()).rejects.toThrow(
      "IPC_NULL_RESULT: cli_manager_claude_info_get"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await cliManagerCodexInfoGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_codex_info_get");

    await cliManagerCodexConfigSet({ model: "gpt-5" });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_codex_config_set", {
      patch: { model: "gpt-5" },
    });

    await cliManagerCodexConfigTomlGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_codex_config_toml_get");

    await cliManagerCodexConfigTomlValidate('model = "gpt-5"');
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_codex_config_toml_validate", {
      toml: 'model = "gpt-5"',
    });

    await cliManagerCodexConfigTomlSet('model = "gpt-5"');
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_codex_config_toml_set", {
      toml: 'model = "gpt-5"',
    });

    await cliManagerClaudeEnvSet({ mcp_timeout_ms: 30_000, disable_error_reporting: true });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_claude_env_set", {
      mcpTimeoutMs: 30_000,
      disableErrorReporting: true,
    });

    await cliManagerClaudeSettingsGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_claude_settings_get");

    await cliManagerClaudeSettingsSet({ model: "claude-3" });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_manager_claude_settings_set", {
      patch: { model: "claude-3" },
    });
  });
});
