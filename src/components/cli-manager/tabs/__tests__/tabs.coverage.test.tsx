import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CliManagerClaudeTab } from "../ClaudeTab";
import { CliManagerCodexTab } from "../CodexTab";
import { CliManagerGeminiTab } from "../GeminiTab";

vi.mock("../../CliVersionBadge", () => ({
  CliVersionBadge: ({ cliKey }: { cliKey: string }) => <div>version-badge-{cliKey}</div>,
}));

vi.mock("../ClaudeOAuthCard", () => ({
  ClaudeOAuthCard: () => <div>claude-oauth-card</div>,
}));

describe("cli-manager tabs (coverage)", () => {
  it("renders ClaudeTab (available)", () => {
    render(
      <CliManagerClaudeTab
        claudeAvailable="available"
        claudeLoading={false}
        claudeInfo={{
          found: true,
          executable_path: "/usr/bin/claude",
          version: "1.0.0",
          error: null,
          shell: "zsh",
          resolved_via: "PATH",
          config_dir: "/tmp/.claude",
          settings_path: "/tmp/.claude/settings.json",
          mcp_timeout_ms: null,
          disable_error_reporting: false,
        }}
        claudeSettingsLoading={false}
        claudeSettingsSaving={false}
        claudeSettings={{
          config_dir: "/tmp/.claude",
          settings_path: "/tmp/.claude/settings.json",
          exists: true,
          model: "claude-3-opus",
          output_style: null,
          language: "zh",
          always_thinking_enabled: false,
          show_turn_duration: false,
          spinner_tips_enabled: true,
          terminal_progress_bar_enabled: true,
          respect_gitignore: true,
          disable_git_participant: false,
          permissions_allow: ["ReadFile"],
          permissions_ask: [],
          permissions_deny: ["WriteFile"],
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
        }}
        providers={null}
        refreshClaude={vi.fn()}
        openClaudeConfigDir={vi.fn()}
        persistClaudeSettings={vi.fn()}
      />
    );

    expect(screen.getByText("settings.json")).toBeInTheDocument();
  });

  it("renders CodexTab (available)", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={{
          found: true,
          executable_path: "/usr/bin/codex",
          version: "0.0.0",
          error: null,
          shell: "zsh",
          resolved_via: "PATH",
        }}
        codexConfig={{
          config_dir: "/tmp/.codex",
          config_path: "/tmp/.codex/config.toml",
          user_home_default_dir: "C:\\Users\\MyPC\\.codex",
          user_home_default_path: "C:\\Users\\MyPC\\.codex\\config.toml",
          follow_codex_home_dir: "C:\\Users\\MyPC\\.codex",
          follow_codex_home_path: "C:\\Users\\MyPC\\.codex\\config.toml",
          can_open_config_dir: true,
          exists: true,
          model: "gpt-5.4",
          approval_policy: "never",
          sandbox_mode: "workspace-write",
          model_reasoning_effort: "medium",
          plan_mode_reasoning_effort: "high",
          web_search: "cached",
          personality: "pragmatic",
          model_context_window: 1000000,
          model_auto_compact_token_limit: 900000,
          service_tier: "fast",
          sandbox_workspace_write_network_access: false,
          features_unified_exec: true,
          features_shell_snapshot: true,
          features_apply_patch_freeform: true,
          features_shell_tool: true,
          features_exec_policy: true,
          features_remote_compaction: true,
          features_fast_mode: true,
          features_responses_websockets_v2: true,
          features_multi_agent: true,
        }}
        codexConfigToml={{
          config_path: "/tmp/.codex/config.toml",
          exists: true,
          toml: 'approval_policy = "never"\\n',
        }}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(true)}
      />
    );

    expect(screen.getAllByText("config.toml").length).toBeGreaterThan(0);
  });

  it("renders GeminiTab", () => {
    render(
      <CliManagerGeminiTab
        geminiAvailable="unavailable"
        geminiLoading={false}
        geminiConfigLoading={false}
        geminiConfigSaving={false}
        geminiInfo={null}
        geminiConfig={null}
        refreshGeminiInfo={vi.fn()}
        persistGeminiConfig={vi.fn()}
      />
    );

    expect(screen.getByRole("heading", { level: 2, name: "Gemini" })).toBeInTheDocument();
  });
});
