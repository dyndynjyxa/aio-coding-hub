import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  type ClaudeEnvState,
  type ClaudeHooksState,
  type ClaudeSettingsState,
  type CodexConfigState,
  type CodexConfigTomlState,
  type CodexConfigTomlValidationResult,
  type SimpleCliInfo,
  cliManagerClaudeEnvSet,
  cliManagerClaudeHooksGet,
  cliManagerClaudeHooksSet,
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
      cliManagerClaudeHooksGet: vi.fn(),
      cliManagerClaudeHooksSet: vi.fn(),
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

function makeSimpleCliInfo(overrides: Partial<SimpleCliInfo> = {}): SimpleCliInfo {
  return {
    found: true,
    executable_path: "/usr/bin/codex",
    version: "0.0.0",
    error: null,
    shell: "zsh",
    resolved_via: "PATH",
    ...overrides,
  };
}

function makeCodexConfigState(overrides: Partial<CodexConfigState> = {}): CodexConfigState {
  return {
    config_dir: "/tmp/.codex",
    config_path: "/tmp/.codex/config.toml",
    user_home_default_dir: "/tmp/.codex",
    user_home_default_path: "/tmp/.codex/config.toml",
    follow_codex_home_dir: "/tmp/.codex",
    follow_codex_home_path: "/tmp/.codex/config.toml",
    can_open_config_dir: true,
    exists: true,
    model: "gpt-5",
    approval_policy: null,
    sandbox_mode: null,
    model_reasoning_effort: null,
    plan_mode_reasoning_effort: null,
    web_search: null,
    personality: null,
    model_context_window: null,
    model_auto_compact_token_limit: null,
    service_tier: null,
    sandbox_workspace_write_network_access: null,
    features_unified_exec: null,
    features_shell_snapshot: null,
    features_apply_patch_freeform: null,
    features_shell_tool: null,
    features_exec_policy: null,
    features_remote_compaction: null,
    features_fast_mode: null,
    features_responses_websockets_v2: null,
    features_multi_agent: null,
    ...overrides,
  };
}

function makeCodexConfigTomlState(
  overrides: Partial<CodexConfigTomlState> = {}
): CodexConfigTomlState {
  return {
    config_path: "/tmp/.codex/config.toml",
    exists: true,
    toml: "",
    ...overrides,
  };
}

function makeCodexConfigTomlValidationResult(
  overrides: Partial<CodexConfigTomlValidationResult> = {}
): CodexConfigTomlValidationResult {
  return {
    ok: true,
    error: null,
    ...overrides,
  };
}

function makeClaudeEnvState(overrides: Partial<ClaudeEnvState> = {}): ClaudeEnvState {
  return {
    config_dir: "/tmp/.claude",
    settings_path: "/tmp/.claude/settings.json",
    mcp_timeout_ms: null,
    disable_error_reporting: false,
    ...overrides,
  };
}

function makeClaudeHooksState(overrides: Partial<ClaudeHooksState> = {}): ClaudeHooksState {
  return {
    settings_path: "/tmp/.claude/settings.json",
    groups: [],
    ...overrides,
  };
}

function makeClaudeSettingsState(
  overrides: Partial<ClaudeSettingsState> = {}
): ClaudeSettingsState {
  return {
    config_dir: "/tmp/.claude",
    settings_path: "/tmp/.claude/settings.json",
    exists: true,
    model: null,
    output_style: null,
    language: null,
    always_thinking_enabled: null,
    show_turn_duration: null,
    spinner_tips_enabled: null,
    terminal_progress_bar_enabled: null,
    respect_gitignore: null,
    disable_git_participant: false,
    permissions_allow: [],
    permissions_ask: [],
    permissions_deny: [],
    env_mcp_timeout_ms: null,
    env_mcp_tool_timeout_ms: null,
    env_experimental_agent_teams: false,
    env_claude_code_auto_compact_window: null,
    env_disable_background_tasks: false,
    env_disable_terminal_title: false,
    env_claude_bash_no_login: false,
    env_claude_code_attribution_header: false,
    env_claude_code_blocking_limit_override: null,
    env_claude_code_max_output_tokens: null,
    env_enable_experimental_mcp_cli: false,
    env_enable_tool_search: false,
    env_max_mcp_output_tokens: null,
    env_claude_code_disable_nonessential_traffic: false,
    env_claude_code_disable_1m_context: false,
    env_claude_code_proxy_resolves_hosts: false,
    env_claude_code_skip_prompt_history: false,
    ...overrides,
  };
}

describe("services/cli/cliManager", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.cliManagerClaudeInfoGet).mockRejectedValueOnce(
      new Error("cli manager boom")
    );

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
    vi.mocked(commands.cliManagerClaudeInfoGet).mockResolvedValueOnce(null as never);

    await expect(cliManagerClaudeInfoGet()).rejects.toThrow(
      "IPC_NULL_RESULT: cli_manager_claude_info_get"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.cliManagerCodexInfoGet).mockResolvedValue({
      status: "ok",
      data: makeSimpleCliInfo(),
    });
    vi.mocked(commands.cliManagerCodexConfigSet).mockResolvedValue({
      status: "ok",
      data: makeCodexConfigState(),
    });
    vi.mocked(commands.cliManagerCodexConfigTomlGet).mockResolvedValue({
      status: "ok",
      data: makeCodexConfigTomlState(),
    });
    vi.mocked(commands.cliManagerCodexConfigTomlValidate).mockResolvedValue({
      status: "ok",
      data: makeCodexConfigTomlValidationResult(),
    });
    vi.mocked(commands.cliManagerCodexConfigTomlSet).mockResolvedValue({
      status: "ok",
      data: makeCodexConfigState(),
    });
    vi.mocked(commands.cliManagerClaudeEnvSet).mockResolvedValue({
      status: "ok",
      data: makeClaudeEnvState(),
    });
    vi.mocked(commands.cliManagerClaudeHooksGet).mockResolvedValue({
      status: "ok",
      data: makeClaudeHooksState(),
    });
    vi.mocked(commands.cliManagerClaudeHooksSet).mockResolvedValue({
      status: "ok",
      data: makeClaudeHooksState(),
    });
    vi.mocked(commands.cliManagerClaudeSettingsGet).mockResolvedValue({
      status: "ok",
      data: makeClaudeSettingsState(),
    });
    vi.mocked(commands.cliManagerClaudeSettingsSet).mockResolvedValue({
      status: "ok",
      data: makeClaudeSettingsState(),
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

    await cliManagerClaudeEnvSet({ mcpTimeoutMs: 30_000, disableErrorReporting: true });
    expect(commands.cliManagerClaudeEnvSet).toHaveBeenCalledWith(30_000, true);

    await cliManagerClaudeHooksGet();
    expect(commands.cliManagerClaudeHooksGet).toHaveBeenCalledWith();

    const hooksInput = {
      groups: [
        {
          event: "PreToolUse",
          matcher: "",
          hooks: [{ hook_type: "command", command: "echo ok", timeout: null }],
        },
      ],
    };
    await cliManagerClaudeHooksSet(hooksInput);
    expect(commands.cliManagerClaudeHooksSet).toHaveBeenCalledWith(hooksInput);

    await cliManagerClaudeSettingsGet();
    expect(commands.cliManagerClaudeSettingsGet).toHaveBeenCalledWith();

    await cliManagerClaudeSettingsSet({ model: "claude-3" });
    expect(commands.cliManagerClaudeSettingsSet).toHaveBeenCalledWith(
      expect.objectContaining({ model: "claude-3" })
    );
  });
});
