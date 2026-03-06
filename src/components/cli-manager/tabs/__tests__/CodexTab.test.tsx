import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CliManagerCodexTab } from "../CodexTab";

function createCodexInfo(overrides: Partial<any> = {}) {
  return {
    found: true,
    version: "0.0.0",
    executable_path: "/bin/codex",
    resolved_via: "PATH",
    shell: "/bin/zsh",
    error: null,
    ...overrides,
  };
}

function createCodexConfig(overrides: Partial<any> = {}) {
  return {
    config_dir: "/home/user/.codex",
    config_path: "/home/user/.codex/config.toml",
    can_open_config_dir: true,
    exists: true,
    model: "gpt-5-codex",
    approval_policy: "on-request",
    sandbox_mode: "workspace-write",
    sandbox_workspace_write_network_access: null,
    model_reasoning_effort: "medium",
    plan_mode_reasoning_effort: null,
    web_search: "cached",
    model_context_window: null,
    model_auto_compact_token_limit: null,
    service_tier: null,
    features_shell_snapshot: false,
    features_unified_exec: false,
    features_shell_tool: false,
    features_exec_policy: false,
    features_apply_patch_freeform: false,
    features_remote_compaction: false,
    features_fast_mode: false,
    features_remote_models: false,
    features_multi_agent: false,
    ...overrides,
  };
}

describe("components/cli-manager/tabs/CodexTab", () => {
  it("handles sandbox confirm flow and toggles", () => {
    const persistCodexConfig = vi.fn();
    const refreshCodex = vi.fn();
    const openCodexConfigDir = vi.fn();

    const confirmSpy = vi
      .spyOn(window, "confirm")
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig()}
        codexConfigToml={{
          config_path: "/home/user/.codex/config.toml",
          exists: true,
          toml: 'approval_policy = "on-request"\\n',
        }}
        refreshCodex={refreshCodex}
        openCodexConfigDir={openCodexConfigDir}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(true)}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(refreshCodex).toHaveBeenCalled();

    // Select danger-full-access but cancel.
    const sandboxItem = screen.getByText("沙箱模式 (sandbox_mode)").parentElement?.parentElement;
    expect(sandboxItem).toBeTruthy();
    const sandboxSelect = within(sandboxItem as HTMLElement).getByRole("combobox");
    fireEvent.change(sandboxSelect, { target: { value: "danger-full-access" } });
    expect(confirmSpy).toHaveBeenCalled();
    expect(persistCodexConfig).not.toHaveBeenCalledWith(
      expect.objectContaining({ sandbox_mode: "danger-full-access" })
    );

    // Confirm selection.
    fireEvent.change(sandboxSelect, { target: { value: "danger-full-access" } });
    expect(persistCodexConfig).toHaveBeenCalledWith({ sandbox_mode: "danger-full-access" });

    // Toggle the linked fast mode switch.
    const fastModeItem = screen.getByText("fast_mode").parentElement?.parentElement;
    expect(fastModeItem).toBeTruthy();
    fireEvent.click(within(fastModeItem as HTMLElement).getByRole("switch"));
    expect(persistCodexConfig).toHaveBeenCalledWith({
      features_fast_mode: true,
      service_tier: "fast",
    });

    // Toggle a normal feature switch.
    const remoteModelsItem = screen.getByText("remote_models").parentElement?.parentElement;
    expect(remoteModelsItem).toBeTruthy();
    fireEvent.click(within(remoteModelsItem as HTMLElement).getByRole("switch"));
    expect(persistCodexConfig).toHaveBeenCalledWith({ features_remote_models: true });

    // Radio group
    fireEvent.click(screen.getByRole("radio", { name: "禁用 (disabled)" }));
    expect(persistCodexConfig).toHaveBeenCalledWith({ web_search: "disabled" });

    // Model input blur persists trimmed value and clears gpt-5.4-only linked keys.
    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("textbox");
    fireEvent.change(modelInput, { target: { value: "  gpt-5-codex  " } });
    fireEvent.blur(modelInput);
    expect(persistCodexConfig).toHaveBeenCalledWith({
      model: "gpt-5-codex",
      model_context_window: null,
      model_auto_compact_token_limit: null,
    });

    // Approval policy select persists.
    const approvalItem =
      screen.getByText("审批策略 (approval_policy)").parentElement?.parentElement;
    expect(approvalItem).toBeTruthy();
    const approvalSelect = within(approvalItem as HTMLElement).getByRole("combobox");
    fireEvent.change(approvalSelect, { target: { value: "never" } });
    expect(persistCodexConfig).toHaveBeenCalledWith({ approval_policy: "never" });

    // Exercise remaining toggle handlers for function/branch coverage.
    for (const sw of screen.getAllByRole("switch")) fireEvent.click(sw);

    confirmSpy.mockRestore();
  });

  it("renders unavailable state", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="unavailable"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={null}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );
    expect(screen.getByText("仅在 Tauri Desktop 环境可用")).toBeInTheDocument();
  });

  it("treats service_tier=fast as enabled fast mode", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({ service_tier: "fast", features_fast_mode: false })}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const fastModeItem = screen.getByText("fast_mode").parentElement?.parentElement;
    expect(fastModeItem).toBeTruthy();
    expect(within(fastModeItem as HTMLElement).getByRole("switch")).toHaveAttribute(
      "data-state",
      "checked"
    );
  });

  it("shows gpt-5.4 linked settings and persists their defaults", () => {
    const persistCodexConfig = vi.fn();

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({ model: "gpt-5.4" })}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(screen.getByText("model_context_window")).toBeInTheDocument();
    expect(screen.getByText("model_auto_compact_token_limit")).toBeInTheDocument();

    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("textbox");
    fireEvent.blur(modelInput);

    expect(persistCodexConfig).toHaveBeenCalledWith({
      model: "gpt-5.4",
      model_context_window: 1000000,
      model_auto_compact_token_limit: 900000,
    });
  });
});
