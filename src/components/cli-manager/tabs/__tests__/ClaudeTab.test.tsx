import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { CliManagerClaudeTab } from "../ClaudeTab";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));

vi.mock("../../CliVersionBadge", () => ({
  CliVersionBadge: ({ cliKey }: { cliKey: string }) => <div>version-badge-{cliKey}</div>,
}));

vi.mock("../ClaudeOAuthCard", () => ({
  ClaudeOAuthCard: () => <div>claude-oauth-card</div>,
}));

function createClaudeInfo(overrides: Partial<any> = {}) {
  return {
    found: true,
    version: "0.0.0",
    executable_path: "/bin/claude",
    resolved_via: "PATH",
    shell: "/bin/zsh",
    config_dir: "/home/user/.claude",
    settings_path: "/home/user/.claude/settings.json",
    mcp_timeout_ms: 1000,
    disable_error_reporting: false,
    error: null,
    ...overrides,
  };
}

function createClaudeSettings(overrides: Partial<any> = {}) {
  return {
    exists: true,
    config_dir: "/home/user/.claude",
    settings_path: "/home/user/.claude/settings.json",
    model: "claude-sonnet",
    output_style: "Explanatory",
    language: "japanese",
    always_thinking_enabled: null,
    show_turn_duration: null,
    spinner_tips_enabled: null,
    terminal_progress_bar_enabled: null,
    respect_gitignore: null,
    disable_git_participant: false,
    permissions_allow: ["Read(./docs/**)"],
    permissions_ask: [],
    permissions_deny: [],
    env_mcp_timeout_ms: 1000,
    env_mcp_tool_timeout_ms: 2000,
    env_experimental_agent_teams: false,
    env_claude_code_auto_compact_window: 200000,
    env_claude_code_blocking_limit_override: 0,
    env_claude_code_max_output_tokens: 0,
    env_max_mcp_output_tokens: 25000,
    env_enable_experimental_mcp_cli: false,
    env_enable_tool_search: false,
    env_claude_code_attribution_header: false,
    env_disable_background_tasks: false,
    env_disable_terminal_title: false,
    env_claude_bash_no_login: false,
    env_claude_code_disable_nonessential_traffic: false,
    env_claude_code_disable_1m_context: false,
    env_claude_code_proxy_resolves_hosts: false,
    env_claude_code_skip_prompt_history: false,
    ...overrides,
  };
}

describe("components/cli-manager/tabs/ClaudeTab", () => {
  it("handles unavailable and empty settings states", () => {
    render(
      <CliManagerClaudeTab
        claudeAvailable="unavailable"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={null}
        providers={null}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={vi.fn()}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("does not render Claude OAuth card in the empty or loaded tab", () => {
    const { rerender } = render(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={null}
        providers={[]}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={vi.fn()}
      />
    );

    expect(screen.queryByText("claude-oauth-card")).not.toBeInTheDocument();

    rerender(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings()}
        providers={[]}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={vi.fn()}
      />
    );

    expect(screen.queryByText("claude-oauth-card")).not.toBeInTheDocument();
  });

  it("drives key form interactions and validations", () => {
    const refreshClaude = vi.fn();
    const openClaudeConfigDir = vi.fn();
    const persistClaudeSettings = vi.fn();

    const { rerender } = render(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings()}
        providers={null}
        refreshClaude={refreshClaude}
        openClaudeConfigDir={openClaudeConfigDir}
        persistClaudeSettings={persistClaudeSettings}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(refreshClaude).toHaveBeenCalled();

    fireEvent.click(screen.getByTitle("打开配置目录"));
    expect(openClaudeConfigDir).toHaveBeenCalled();

    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("textbox");
    fireEvent.change(modelInput, { target: { value: "  claude-opus  " } });
    fireEvent.blur(modelInput);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ model: "claude-opus" });

    const outputStyleItem = screen.getByText("输出风格 (outputStyle)").parentElement?.parentElement;
    expect(outputStyleItem).toBeTruthy();
    const outputStyleInput = within(outputStyleItem as HTMLElement).getByRole("textbox");
    fireEvent.change(outputStyleInput, { target: { value: "  Explanatory  " } });
    fireEvent.blur(outputStyleInput);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ output_style: "Explanatory" });

    const languageItem = screen.getByText("语言 (language)").parentElement?.parentElement;
    expect(languageItem).toBeTruthy();
    const languageInput = within(languageItem as HTMLElement).getByRole("textbox");
    fireEvent.change(languageInput, { target: { value: "  english  " } });
    fireEvent.blur(languageInput);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ language: "english" });

    const disableGitParticipantItem =
      screen.getByText("关闭 Claude Git 参与者").parentElement?.parentElement;
    expect(disableGitParticipantItem).toBeTruthy();
    fireEvent.click(within(disableGitParticipantItem as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({ disable_git_participant: true });

    // permissions.allow parses lines on blur
    const allowItem = screen.getByText("permissions.allow").parentElement?.parentElement;
    expect(allowItem).toBeTruthy();
    const allowTextarea = within(allowItem as HTMLElement).getByRole("textbox");
    fireEvent.change(allowTextarea, {
      target: { value: "Bash(git diff:*)\n\nRead(./docs/**)\n" },
    });
    fireEvent.blur(allowTextarea);
    expect(persistClaudeSettings).toHaveBeenCalledWith({
      permissions_allow: ["Bash(git diff:*)", "Read(./docs/**)"],
    });

    // Env timeout validation: too large => toast + revert.
    const timeoutItem = screen.getByText("MCP_TIMEOUT (ms)").parentElement?.parentElement;
    expect(timeoutItem).toBeTruthy();
    const timeoutInput = within(timeoutItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(timeoutInput, { target: { value: String(24 * 60 * 60 * 1000 * 2) } });
    fireEvent.blur(timeoutInput);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining("MCP_TIMEOUT 必须为"));

    // Experimental toggle: enable branch.
    const expItem = screen.getByText("ENABLE_EXPERIMENTAL_MCP_CLI").parentElement?.parentElement;
    expect(expItem).toBeTruthy();
    fireEvent.click(within(expItem as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({
      env_enable_experimental_mcp_cli: true,
      env_enable_tool_search: false,
    });

    // Tool search toggle: enable branch (mutually exclusive with experimental).
    const toolSearchItem = screen.getByText("ENABLE_TOOL_SEARCH").parentElement?.parentElement;
    expect(toolSearchItem).toBeTruthy();
    fireEvent.click(within(toolSearchItem as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({
      env_enable_tool_search: true,
      env_enable_experimental_mcp_cli: false,
    });

    // Rerender with experimental enabled to hit disable branch.
    rerender(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings({ env_enable_experimental_mcp_cli: true })}
        providers={null}
        refreshClaude={refreshClaude}
        openClaudeConfigDir={openClaudeConfigDir}
        persistClaudeSettings={persistClaudeSettings}
      />
    );
    const expItem2 = screen.getByText("ENABLE_EXPERIMENTAL_MCP_CLI").parentElement?.parentElement;
    expect(expItem2).toBeTruthy();
    fireEvent.click(within(expItem2 as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({ env_enable_experimental_mcp_cli: false });

    // Rerender with tool search enabled to hit disable branch.
    rerender(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo()}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings({ env_enable_tool_search: true })}
        providers={null}
        refreshClaude={refreshClaude}
        openClaudeConfigDir={openClaudeConfigDir}
        persistClaudeSettings={persistClaudeSettings}
      />
    );
    const toolSearchItem2 = screen.getByText("ENABLE_TOOL_SEARCH").parentElement?.parentElement;
    expect(toolSearchItem2).toBeTruthy();
    fireEvent.click(within(toolSearchItem2 as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({ env_enable_tool_search: false });

    // Exercise remaining toggle handlers for function/branch coverage.
    const switches = screen.getAllByRole("switch");
    for (const sw of switches) fireEvent.click(sw);
  });

  it("covers EnvU64 validation branches and permissions ask/deny persistence", () => {
    const persistClaudeSettings = vi.fn();

    render(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={createClaudeInfo({ found: false })}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings({
          env_claude_code_blocking_limit_override: 123,
          env_claude_code_max_output_tokens: 456,
          env_max_mcp_output_tokens: 25000,
          permissions_ask: [],
          permissions_deny: [],
        })}
        providers={null}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={persistClaudeSettings}
      />
    );

    // MAX_MCP_OUTPUT_TOKENS: negative -> toast + revert
    const maxMcpItem = screen.getByText("MAX_MCP_OUTPUT_TOKENS").parentElement?.parentElement;
    expect(maxMcpItem).toBeTruthy();
    const maxMcpInput = within(maxMcpItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(maxMcpInput, { target: { value: "-1" } });
    fireEvent.blur(maxMcpInput);
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("MAX_MCP_OUTPUT_TOKENS 必须为非负整数")
    );

    // CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE: too large -> toast + revert
    const blockingItem = screen.getByText("CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE").parentElement
      ?.parentElement;
    expect(blockingItem).toBeTruthy();
    const blockingInput = within(blockingItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(blockingInput, { target: { value: String(Number.MAX_SAFE_INTEGER + 1) } });
    fireEvent.blur(blockingInput);
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE 值过大")
    );

    // CLAUDE_CODE_MAX_OUTPUT_TOKENS: empty -> persist 0
    const maxOutItem = screen.getByText("CLAUDE_CODE_MAX_OUTPUT_TOKENS").parentElement
      ?.parentElement;
    expect(maxOutItem).toBeTruthy();
    const maxOutInput = within(maxOutItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(maxOutInput, { target: { value: "" } });
    fireEvent.blur(maxOutInput);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ env_claude_code_max_output_tokens: 0 });

    // CLAUDE_CODE_MAX_OUTPUT_TOKENS: negative -> toast + revert (covers revertMaxOutputTokensInput)
    fireEvent.change(maxOutInput, { target: { value: "-1" } });
    fireEvent.blur(maxOutInput);
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("CLAUDE_CODE_MAX_OUTPUT_TOKENS 必须为非负整数")
    );

    // CLAUDE_CODE_AUTO_COMPACT_WINDOW: empty -> persist 0
    const autoCompactItem = screen.getByText("CLAUDE_CODE_AUTO_COMPACT_WINDOW").parentElement
      ?.parentElement;
    expect(autoCompactItem).toBeTruthy();
    const autoCompactInput = within(autoCompactItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(autoCompactInput, { target: { value: "" } });
    fireEvent.blur(autoCompactInput);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ env_claude_code_auto_compact_window: 0 });

    // CLAUDE_CODE_DISABLE_1M_CONTEXT toggle
    const disable1mItem = screen.getByText("CLAUDE_CODE_DISABLE_1M_CONTEXT").parentElement
      ?.parentElement;
    expect(disable1mItem).toBeTruthy();
    fireEvent.click(within(disable1mItem as HTMLElement).getByRole("switch"));
    expect(persistClaudeSettings).toHaveBeenCalledWith({
      env_claude_code_disable_1m_context: true,
    });

    // permissions.ask
    const askItem = screen.getByText("permissions.ask").parentElement?.parentElement;
    expect(askItem).toBeTruthy();
    const askTextarea = within(askItem as HTMLElement).getByRole("textbox");
    fireEvent.change(askTextarea, { target: { value: "Bash(git push:*)\n\n" } });
    fireEvent.blur(askTextarea);
    expect(persistClaudeSettings).toHaveBeenCalledWith({ permissions_ask: ["Bash(git push:*)"] });

    // permissions.deny
    const denyItem = screen.getByText("permissions.deny").parentElement?.parentElement;
    expect(denyItem).toBeTruthy();
    const denyTextarea = within(denyItem as HTMLElement).getByRole("textbox");
    fireEvent.change(denyTextarea, { target: { value: "Read(./.env)\nBash(rm -rf:*)\n" } });
    fireEvent.blur(denyTextarea);
    expect(persistClaudeSettings).toHaveBeenCalledWith({
      permissions_deny: ["Read(./.env)", "Bash(rm -rf:*)"],
    });
  });

  it("covers checking state, placeholder fields, and error rendering", () => {
    const refreshClaude = vi.fn();
    const openClaudeConfigDir = vi.fn();

    render(
      <CliManagerClaudeTab
        claudeAvailable="checking"
        claudeLoading={true}
        claudeInfo={createClaudeInfo({
          found: false,
          resolved_via: null,
          shell: null,
          config_dir: null,
          settings_path: null,
          error: "boom",
        })}
        claudeSettingsLoading={true}
        claudeSettingsSaving={false}
        claudeSettings={createClaudeSettings({
          model: null,
          output_style: null,
          language: null,
          config_dir: null,
          settings_path: null,
          permissions_allow: null,
          permissions_ask: null,
          permissions_deny: null,
          env_mcp_timeout_ms: null,
          env_mcp_tool_timeout_ms: null,
          env_claude_code_auto_compact_window: null,
          env_claude_code_blocking_limit_override: null,
          env_claude_code_max_output_tokens: null,
          env_max_mcp_output_tokens: null,
          env_claude_code_disable_1m_context: false,
        })}
        providers={null}
        refreshClaude={refreshClaude}
        openClaudeConfigDir={openClaudeConfigDir}
        persistClaudeSettings={vi.fn()}
      />
    );

    expect(screen.getByText("加载中...")).toBeInTheDocument();
    const refreshButton = screen.getByRole("button", { name: "刷新" });
    expect(refreshButton).toBeDisabled();
    const refreshIcon = refreshButton.querySelector("svg");
    expect(refreshIcon?.getAttribute("class") ?? "").toContain("animate-spin");

    const openConfigButton = screen.getByTitle("打开配置目录");
    expect(openConfigButton).toBeDisabled();

    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    expect(screen.getByText(/检测失败：/)).toBeInTheDocument();
  });

  it("shows empty settings state when available and no settings are loaded", () => {
    render(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={null}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={null}
        providers={null}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={vi.fn()}
      />
    );

    expect(screen.getByText("暂无配置，请尝试刷新")).toBeInTheDocument();
    expect(screen.queryByText("配置目录")).not.toBeInTheDocument();
  });
});
