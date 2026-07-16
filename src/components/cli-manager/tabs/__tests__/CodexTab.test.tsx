import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { cliManagerCodexConfigTomlValidate } from "../../../../services/cli/cliManager";
import { CliManagerCodexTab } from "../CodexTab";
import { createTestAppSettings } from "../../../../test/fixtures/settings";

vi.mock("../../../../utils/platform", () => ({
  isWindowsRuntime: () => true,
}));

vi.mock("../../../../ui/CodeEditor", () => ({
  CodeEditor: ({ value, onChange, readOnly }: any) => (
    <textarea
      aria-label="mock-code-editor"
      value={value}
      readOnly={readOnly}
      onChange={(e) => onChange?.(e.currentTarget.value)}
    />
  ),
}));

vi.mock("../../../../services/cli/cliManager", async () => {
  const actual = await vi.importActual<typeof import("../../../../services/cli/cliManager")>(
    "../../../../services/cli/cliManager"
  );
  return {
    ...actual,
    cliManagerCodexConfigTomlValidate: vi.fn().mockResolvedValue({
      ok: true,
      error: null,
    }),
  };
});
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
    user_home_default_dir: "C:\\Users\\MyPC\\.codex",
    user_home_default_path: "C:\\Users\\MyPC\\.codex\\config.toml",
    follow_codex_home_dir: "C:\\Users\\MyPC\\.codex",
    follow_codex_home_path: "C:\\Users\\MyPC\\.codex\\config.toml",
    can_open_config_dir: true,
    exists: true,
    model: "gpt-5-codex",
    approval_policy: "on-request",
    sandbox_mode: "workspace-write",
    sandbox_workspace_write_network_access: null,
    model_reasoning_effort: "medium",
    plan_mode_reasoning_effort: null,
    web_search: "cached",
    personality: null,
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
    features_responses_websockets_v2: false,
    features_multi_agent: false,
    ...overrides,
  };
}

function createAppSettings(overrides: Parameters<typeof createTestAppSettings>[0] = {}) {
  return createTestAppSettings({
    codex_home_mode: "user_home_default",
    codex_home_override: "",
    ...overrides,
  });
}

function createCodexModel(overrides: Partial<any> = {}) {
  return {
    id: "gpt-5.6-sol-id",
    model: "gpt-5.6-sol",
    display_name: "GPT-5.6 Sol",
    hidden: false,
    is_default: false,
    supported_reasoning_efforts: [
      { reasoning_effort: "low", description: null },
      { reasoning_effort: "medium", description: null },
      { reasoning_effort: "high", description: null },
      { reasoning_effort: "xhigh", description: null },
      { reasoning_effort: "max", description: "Maximum reasoning depth" },
      { reasoning_effort: "ultra", description: "Automatic task delegation" },
    ],
    default_reasoning_effort: "medium",
    ...overrides,
  };
}

function createCodexModelCatalog(models = [createCodexModel()]) {
  return {
    status: "ready" as const,
    issue: null,
    snapshot: {
      config_path: "/home/user/.codex/config.toml",
      executable_path: "/bin/codex",
      cli_version: "0.0.0",
    },
    models,
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

    const websocketItem = screen.getByText("responses_websockets_v2").parentElement?.parentElement;
    expect(websocketItem).toBeTruthy();
    fireEvent.click(within(websocketItem as HTMLElement).getByRole("switch"));
    expect(persistCodexConfig).toHaveBeenCalledWith({
      features_responses_websockets_v2: true,
    });

    // Radio group
    fireEvent.click(screen.getByRole("radio", { name: "禁用 (disabled)" }));
    expect(persistCodexConfig).toHaveBeenCalledWith({ web_search: "disabled" });

    const personalityItem = screen.getByText("输出风格 (personality)").parentElement?.parentElement;
    expect(personalityItem).toBeTruthy();
    fireEvent.click(
      within(personalityItem as HTMLElement).getByRole("radio", { name: "友好 (friendly)" })
    );
    expect(persistCodexConfig).toHaveBeenCalledWith({ personality: "friendly" });

    fireEvent.click(
      within(personalityItem as HTMLElement).getByRole("radio", {
        name: "默认 / 删除配置 (none)",
      })
    );
    expect(persistCodexConfig).toHaveBeenCalledWith({ personality: "" });

    // Model input blur persists a changed trimmed value and clears model-linked keys.
    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("combobox", {
      name: "默认模型 (model)",
    });
    fireEvent.change(modelInput, { target: { value: "  gpt-5.6-sol  " } });
    fireEvent.blur(modelInput);
    expect(persistCodexConfig).toHaveBeenCalledWith({
      model: "gpt-5.6-sol",
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

  it("toggles Codex OAuth compatible proxy mode from app settings", () => {
    const persistCodexOauthCompatibleProxyMode = vi.fn().mockResolvedValue(true);

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
        appSettings={createAppSettings({ codex_oauth_compatible_proxy_mode: false })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(true)}
        persistCodexOauthCompatibleProxyMode={persistCodexOauthCompatibleProxyMode}
      />
    );

    expect(screen.getByText("OAuth 兼容代理模式")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "切换 Codex OAuth 兼容代理模式" }));

    expect(persistCodexOauthCompatibleProxyMode).toHaveBeenCalledWith(true);
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
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("keeps a loaded Codex config editable when the CLI is unavailable", () => {
    const persistCodexConfig = vi.fn();

    render(
      <CliManagerCodexTab
        codexAvailable="unavailable"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo({
          found: false,
          executable_path: null,
          version: null,
        })}
        codexConfig={createCodexConfig()}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const contextItem = screen.getByText("model_context_window").parentElement?.parentElement;
    expect(contextItem).toBeTruthy();
    const contextInput = within(contextItem as HTMLElement).getByRole("spinbutton");
    expect(contextInput).toBeEnabled();
    fireEvent.change(contextInput, { target: { value: "1000000" } });
    fireEvent.blur(contextInput);
    expect(persistCodexConfig).toHaveBeenCalledWith({ model_context_window: 1_000_000 });
  });

  it("disables open config dir and shows hint when CODEX_HOME is overridden", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "/custom/codex",
          config_path: "/custom/codex/config.toml",
          can_open_config_dir: false,
        })}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(
      screen.getByText("受权限限制，无法自动打开该目录；请手动打开该路径。")
    ).toBeInTheDocument();
    const openBtn = screen.getByTitle("受权限限制，无法自动打开该目录");
    expect(openBtn).toBeDisabled();
  });

  it("saves a custom codex home override and normalizes config.toml input", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(true);

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
        codexConfigToml={null}
        appSettings={createAppSettings()}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("radio", { name: "手动指定目录" }));
    const customCard = (await screen.findByText("自定义 .codex 目录")).closest("div");
    expect(customCard).toBeTruthy();
    const input = within(customCard as HTMLElement).getByRole("textbox");
    fireEvent.change(input, { target: { value: "D:\\Work\\Codex\\config.toml" } });
    fireEvent.blur(input);

    expect(persistCodexHomeSettings).toHaveBeenCalledWith("custom", "D:\\Work\\Codex");
    expect(
      screen.getByText(
        "保存后将使用 D:\\Work\\Codex\\config.toml。支持普通 Windows 路径、UNC 路径，也可以点“选择目录”。"
      )
    ).toBeInTheDocument();
  });

  it("shows validation for invalid custom codex home input", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(true);

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
        codexConfigToml={null}
        appSettings={createAppSettings()}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("radio", { name: "手动指定目录" }));
    const customCard = (await screen.findByText("自定义 .codex 目录")).closest("div");
    expect(customCard).toBeTruthy();
    const input = within(customCard as HTMLElement).getByRole("textbox");
    fireEvent.change(input, { target: { value: "https://example.com/config.toml" } });
    fireEvent.blur(input);

    expect(persistCodexHomeSettings).not.toHaveBeenCalled();
    expect(screen.getByText("这里填写的是本地目录路径，不要包含协议头。")).toBeInTheDocument();
  });

  it("uses directory picker to switch into custom mode and persist", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(true);
    const pickCodexHomeDirectory = vi.fn().mockResolvedValue("D:\\Users\\MyPC\\.codex");

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "C:\\Users\\MyPC\\.codex",
          config_path: "C:\\Users\\MyPC\\.codex\\config.toml",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings()}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={pickCodexHomeDirectory}
      />
    );

    expect(screen.queryByRole("button", { name: "选择目录" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("手动指定目录"));
    fireEvent.click(await screen.findByRole("button", { name: "选择目录" }));

    expect(pickCodexHomeDirectory).toHaveBeenCalledWith("C:\\Users\\MyPC\\.codex");
    expect(await screen.findByDisplayValue("D:\\Users\\MyPC\\.codex")).toBeInTheDocument();
    expect(persistCodexHomeSettings).toHaveBeenCalledWith("custom", "D:\\Users\\MyPC\\.codex");
  });

  it("switches to follow CODEX_HOME mode and disables manual selection", () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(true);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          follow_codex_home_dir: "D:\\Workspace\\.codex",
          follow_codex_home_path: "D:\\Workspace\\.codex\\config.toml",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings({ codex_home_mode: "user_home_default" })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("radio", { name: "跟随环境变量 $CODEX_HOME" }));

    expect(persistCodexHomeSettings).toHaveBeenCalledWith("follow_codex_home", "");
    expect(screen.queryByRole("button", { name: "选择目录" })).not.toBeInTheDocument();
    expect(
      screen.getByText("当前为跟随模式，手动目录选择器已收起；现在会使用 D:\\Workspace\\.codex。")
    ).toBeInTheDocument();
    expect(
      screen.getAllByText("当前路径跟随 $CODEX_HOME 解析；后续会随环境变量变化。").length
    ).toBeGreaterThan(0);
  });

  it("rolls back mode change when saving codex home settings fails", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(false);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          follow_codex_home_dir: "D:\\Workspace\\.codex",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings({ codex_home_mode: "user_home_default" })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("radio", { name: "跟随环境变量 $CODEX_HOME" }));

    expect(persistCodexHomeSettings).toHaveBeenCalledWith("follow_codex_home", "");
    await screen.findByText(
      "当前为默认模式，手动目录选择器已收起；固定使用 C:\\Users\\MyPC\\.codex。"
    );
    expect(screen.getByRole("radio", { name: "固定到 Windows 用户目录" })).toBeChecked();
  });

  it("rolls back reset when saving the default codex home fails", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(false);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "D:\\Work\\Saved\\.codex",
          config_path: "D:\\Work\\Saved\\.codex\\config.toml",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings({
          codex_home_mode: "custom",
          codex_home_override: "D:\\Work\\Saved\\.codex",
        })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "恢复默认" }));

    expect(persistCodexHomeSettings).toHaveBeenCalledWith("user_home_default", "");
    expect(await screen.findByDisplayValue("D:\\Work\\Saved\\.codex")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "手动指定目录" })).toBeChecked();
  });

  it("rolls back the picked custom codex home when saving fails", async () => {
    const persistCodexHomeSettings = vi.fn().mockResolvedValue(false);
    const pickCodexHomeDirectory = vi.fn().mockResolvedValue("D:\\Users\\MyPC\\.codex");

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
        codexConfigToml={null}
        appSettings={createAppSettings({
          codex_home_mode: "custom",
          codex_home_override: "D:\\Work\\Saved\\.codex",
        })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
        persistCodexHomeSettings={persistCodexHomeSettings}
        pickCodexHomeDirectory={pickCodexHomeDirectory}
      />
    );

    fireEvent.click(await screen.findByRole("button", { name: "选择目录" }));

    await waitFor(() =>
      expect(pickCodexHomeDirectory).toHaveBeenCalledWith("D:\\Work\\Saved\\.codex")
    );
    await waitFor(() =>
      expect(persistCodexHomeSettings).toHaveBeenCalledWith("custom", "D:\\Users\\MyPC\\.codex")
    );
    expect(await screen.findByDisplayValue("D:\\Work\\Saved\\.codex")).toBeInTheDocument();
  });

  it("labels the active directory card clearly in default mode", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "C:\\Users\\MyPC\\.codex",
          config_path: "C:\\Users\\MyPC\\.codex\\config.toml",
          follow_codex_home_dir: "D:\\Workspace\\.codex",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings({ codex_home_mode: "user_home_default" })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(screen.getByText("当前 .codex 目录")).toBeInTheDocument();
    expect(
      screen.getAllByText("当前固定使用 Windows 用户目录下的 .codex。").length
    ).toBeGreaterThan(0);
    expect(
      screen.getByText("当前为默认模式，手动目录选择器已收起；固定使用 C:\\Users\\MyPC\\.codex。")
    ).toBeInTheDocument();
    expect(screen.queryByText("CODEX_HOME")).not.toBeInTheDocument();
    expect(screen.queryByPlaceholderText("例如：D:\\Users\\you\\.codex")).not.toBeInTheDocument();
  });

  it("shows follow mode as same-as-default when both resolve to the same path", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          user_home_default_dir: "C:\\Users\\MyPC\\.codex",
          follow_codex_home_dir: "C:\\Users\\MyPC\\.codex",
        })}
        codexConfigToml={null}
        appSettings={createAppSettings({ codex_home_mode: "user_home_default" })}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(
      screen.getByRole("radio", {
        name: "跟随环境变量 $CODEX_HOME（当前路径与固定目录一致）",
      })
    ).toBeInTheDocument();
    expect(screen.getByText("当前路径相同，但后续会随 $CODEX_HOME 变化。")).toBeInTheDocument();
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

  it("defaults personality to none when config is unset", () => {
    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({ personality: null })}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const personalityItem = screen.getByText("输出风格 (personality)").parentElement?.parentElement;
    expect(personalityItem).toBeTruthy();
    expect(
      within(personalityItem as HTMLElement).getByRole("radio", {
        name: "默认 / 删除配置 (none)",
      })
    ).toBeChecked();
  });

  it("always shows model token overrides and skips unchanged model blur", () => {
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
        codexConfig={createCodexConfig({ model: "gpt-5.6-sol", features_multi_agent: null })}
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
    const modelInput = within(modelItem as HTMLElement).getByRole("combobox", {
      name: "默认模型 (model)",
    });
    fireEvent.blur(modelInput);

    expect(persistCodexConfig).not.toHaveBeenCalled();
  });

  it("persists null for model token overrides when input is zero or cleared", () => {
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
        codexConfig={createCodexConfig({
          model: "gpt-5.6-sol",
          model_context_window: 1_000_000,
          model_auto_compact_token_limit: 900_000,
        })}
        codexConfigToml={null}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const contextItem = screen.getByText("model_context_window").parentElement?.parentElement;
    expect(contextItem).toBeTruthy();
    const contextInput = within(contextItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(contextInput, { target: { value: "0" } });
    fireEvent.blur(contextInput);
    expect(persistCodexConfig).toHaveBeenCalledWith({ model_context_window: null });

    const compactItem = screen.getByText("model_auto_compact_token_limit").parentElement
      ?.parentElement;
    expect(compactItem).toBeTruthy();
    const compactInput = within(compactItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(compactInput, { target: { value: "" } });
    fireEvent.blur(compactInput);
    expect(persistCodexConfig).toHaveBeenCalledWith({
      model_auto_compact_token_limit: null,
    });
  });

  it("uses catalog efforts for normal mode and keeps max/ultra out of plan mode", () => {
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
        codexConfig={createCodexConfig({ model: "gpt-5.6-sol", features_multi_agent: null })}
        codexConfigToml={null}
        codexModelCatalog={createCodexModelCatalog()}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const reasoningGroup = screen.getByRole("radiogroup", {
      name: "推理强度 (model_reasoning_effort)",
    });
    expect(reasoningGroup).toHaveAccessibleDescription(
      "调整推理强度（仅对支持的模型/Responses API 生效）。值越高通常越稳健但更慢。"
    );

    const maxOption = within(reasoningGroup).getByRole("radio", {
      name: "最大深度 (max)",
    });
    expect(maxOption).toHaveAccessibleDescription("最大单任务推理深度，可能增加延迟和用量。");

    const ultraOption = within(reasoningGroup).getByRole("radio", {
      name: "自动委派 (ultra)",
    });
    expect(ultraOption).toHaveAccessibleName("自动委派 (ultra)");
    expect(ultraOption).toHaveAccessibleDescription(
      "会自动委派子智能体并行处理任务，增加并发和额外用量。"
    );
    expect(screen.getByText(/当前未设置，使用 Codex 默认行为/)).toBeInTheDocument();
    fireEvent.click(ultraOption);
    expect(persistCodexConfig).toHaveBeenCalledWith({ model_reasoning_effort: "ultra" });

    const planItem = screen.getByText("计划模式推理强度 (plan_mode_reasoning_effort)").parentElement
      ?.parentElement;
    expect(planItem).toBeTruthy();
    expect(
      within(planItem as HTMLElement).queryByRole("radio", { name: /最大深度 \(max\)/ })
    ).toBeNull();
    expect(
      within(planItem as HTMLElement).queryByRole("radio", { name: /自动委派 \(ultra\)/ })
    ).toBeNull();
  });

  it("keeps the reasoning control editable when the catalog query fails", () => {
    const refreshCodex = vi.fn().mockResolvedValue(undefined);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexModelCatalogError
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig()}
        codexConfigToml={null}
        refreshCodex={refreshCodex}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(screen.getByText("读取模型能力失败，当前推理选项仅供编辑。")).toBeInTheDocument();
    const reasoningItem = screen.getByText("推理强度 (model_reasoning_effort)").parentElement
      ?.parentElement;
    expect(reasoningItem).toBeTruthy();
    expect(
      within(reasoningItem as HTMLElement).getByRole("radio", { name: "低 (low)" })
    ).toBeEnabled();
    fireEvent.click(
      within(reasoningItem as HTMLElement).getByRole("button", { name: "重试能力目录" })
    );
    expect(refreshCodex).toHaveBeenCalledTimes(1);
  });

  it("downgrades an incompatible max or ultra effort in the model switch patch", async () => {
    const persistCodexConfig = vi
      .fn()
      .mockResolvedValue(
        createCodexConfig({ model: "gpt-5.6-luna", model_reasoning_effort: "max" })
      );
    const catalog = createCodexModelCatalog([
      createCodexModel(),
      createCodexModel({
        id: "gpt-5.6-luna-id",
        model: "gpt-5.6-luna",
        supported_reasoning_efforts: [
          { reasoning_effort: "low", description: null },
          { reasoning_effort: "high", description: null },
          { reasoning_effort: "max", description: null },
        ],
      }),
    ]);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({ model: "gpt-5.6-sol", model_reasoning_effort: "ultra" })}
        codexConfigToml={null}
        codexModelCatalog={catalog}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("combobox", {
      name: "默认模型 (model)",
    });
    fireEvent.change(modelInput, { target: { value: "gpt-5.6-luna" } });
    expect(screen.getByRole("radio", { name: /自动委派 \(ultra\)/ })).toBeInTheDocument();
    expect(persistCodexConfig).not.toHaveBeenCalled();
    fireEvent.blur(modelInput);

    await waitFor(() =>
      expect(persistCodexConfig).toHaveBeenCalledWith({
        model: "gpt-5.6-luna",
        model_context_window: null,
        model_auto_compact_token_limit: null,
        model_reasoning_effort: "max",
      })
    );
  });

  it("keeps the saved configuration unchanged and reports model migration failure", async () => {
    const persistCodexConfig = vi.fn().mockResolvedValue(null);
    const catalog = createCodexModelCatalog([
      createCodexModel(),
      createCodexModel({
        id: "gpt-5.6-luna-id",
        model: "gpt-5.6-luna",
        supported_reasoning_efforts: [
          { reasoning_effort: "low", description: null },
          { reasoning_effort: "max", description: null },
        ],
      }),
    ]);

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          model: "gpt-5.6-sol",
          model_reasoning_effort: "ultra",
          model_context_window: 1_000_000,
          model_auto_compact_token_limit: 900_000,
        })}
        codexConfigToml={null}
        codexModelCatalog={catalog}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={persistCodexConfig}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    const modelItem = screen.getByText("默认模型 (model)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("combobox", {
      name: "默认模型 (model)",
    });
    fireEvent.change(modelInput, { target: { value: "gpt-5.6-luna" } });
    fireEvent.blur(modelInput);

    await waitFor(() =>
      expect(screen.getByText("模型保存失败，未清除覆盖或调整推理强度。")).toBeInTheDocument()
    );
    expect(screen.getByDisplayValue("1000000")).toBeInTheDocument();
    expect(screen.getByDisplayValue("900000")).toBeInTheDocument();
  });

  it("resets toml draft when codex config path changes", async () => {
    vi.mocked(cliManagerCodexConfigTomlValidate).mockResolvedValue({
      ok: true,
      error: null,
    });

    const { rerender } = render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "C:\\Users\\MyPC\\.codex",
          config_path: "C:\\Users\\MyPC\\.codex\\config.toml",
        })}
        codexConfigToml={{
          config_path: "C:\\Users\\MyPC\\.codex\\config.toml",
          exists: true,
          toml: 'model = "gpt-5"\n',
        }}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(true)}
      />
    );

    fireEvent.click(screen.getByText("高级配置（config.toml）"));
    fireEvent.click(await screen.findByRole("button", { name: "编辑" }));
    await screen.findByRole("button", { name: "取消" });
    fireEvent.change(await screen.findByLabelText("mock-code-editor"), {
      target: { value: 'model = "dirty-old"\n' },
    });

    expect(screen.getByLabelText("mock-code-editor")).toHaveValue('model = "dirty-old"\n');
    expect(screen.getByRole("button", { name: "取消" })).toBeInTheDocument();

    rerender(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({
          config_dir: "D:\\Work\\.codex",
          config_path: "D:\\Work\\.codex\\config.toml",
        })}
        codexConfigToml={{
          config_path: "D:\\Work\\.codex\\config.toml",
          exists: true,
          toml: 'model = "gpt-5.4"\n',
        }}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(true)}
      />
    );

    expect(screen.getByLabelText("mock-code-editor")).toHaveValue('model = "gpt-5.4"\n');
    expect(screen.getByRole("button", { name: "编辑" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "取消" })).not.toBeInTheDocument();
  });

  it("validates, cancels, reloads, and saves raw config.toml edits", async () => {
    const persistCodexConfigToml = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    vi.mocked(cliManagerCodexConfigTomlValidate)
      .mockResolvedValueOnce({ ok: true, error: null })
      .mockResolvedValueOnce({
        ok: false,
        error: { message: "invalid toml", line: 2, column: 3 },
      })
      .mockResolvedValueOnce({ ok: true, error: null })
      // 每次点击「保存」都会先同步校验一次；两次保存各消耗一次 mock
      .mockResolvedValueOnce({ ok: true, error: null })
      .mockResolvedValueOnce({ ok: true, error: null });

    render(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo()}
        codexConfig={createCodexConfig({ config_path: null })}
        codexConfigToml={{
          config_path: "/home/user/.codex/config.toml",
          exists: true,
          toml: 'model = "gpt-5"\n',
        }}
        refreshCodex={vi.fn()}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={persistCodexConfigToml}
      />
    );

    fireEvent.click(screen.getByText("高级配置（config.toml）"));
    const reloadButton = await screen.findByRole("button", { name: "重新加载" });
    expect(
      screen.getByText((_, element) => element?.textContent === "/home/user/.codex/config.toml")
    ).toBeInTheDocument();

    fireEvent.click(reloadButton);
    expect(screen.getByLabelText("mock-code-editor")).toHaveValue('model = "gpt-5"\n');

    fireEvent.click(screen.getByRole("button", { name: "编辑" }));
    await waitFor(() => expect(cliManagerCodexConfigTomlValidate).toHaveBeenCalled());

    fireEvent.change(screen.getByLabelText("mock-code-editor"), {
      target: { value: "bad = [" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    expect(await screen.findByText("TOML 校验失败")).toBeInTheDocument();
    expect(screen.getByText("invalid toml")).toBeInTheDocument();
    expect(screen.getByText("(line 2, column 3)")).toBeInTheDocument();
    expect(persistCodexConfigToml).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("mock-code-editor"), {
      target: { value: 'model = "gpt-5.4"\n' },
    });
    await waitFor(
      () => {
        expect(cliManagerCodexConfigTomlValidate).toHaveBeenCalledWith('model = "gpt-5.4"\n');
      },
      { timeout: 1200 }
    );
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => {
      expect(persistCodexConfigToml).toHaveBeenCalledWith('model = "gpt-5.4"\n');
    });
    expect(screen.getByRole("button", { name: "取消" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(persistCodexConfigToml).toHaveBeenCalledTimes(2));
    expect(await screen.findByRole("button", { name: "编辑" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "编辑" }));
    fireEvent.change(screen.getByLabelText("mock-code-editor"), {
      target: { value: 'model = "discarded"\n' },
    });
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.getByLabelText("mock-code-editor")).toHaveValue('model = "gpt-5"\n');
  });

  it("renders loading, missing config, fallback info, and detection error states", async () => {
    const refreshCodex = vi.fn().mockResolvedValue(undefined);

    const { rerender } = render(
      <CliManagerCodexTab
        codexAvailable="checking"
        codexLoading={true}
        codexConfigLoading={true}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={null}
        codexConfig={null}
        codexConfigToml={null}
        refreshCodex={refreshCodex}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(screen.getByText("加载中...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    expect(screen.getByText("暂无配置，请尝试刷新")).toBeInTheDocument();

    rerender(
      <CliManagerCodexTab
        codexAvailable="available"
        codexLoading={false}
        codexConfigLoading={false}
        codexConfigSaving={false}
        codexConfigTomlLoading={false}
        codexConfigTomlSaving={false}
        codexInfo={createCodexInfo({
          found: false,
          version: null,
          executable_path: null,
          resolved_via: null,
          shell: null,
          error: "codex boom",
        })}
        codexConfig={createCodexConfig({
          exists: false,
          executable_path: undefined,
          resolved_via: undefined,
          config_dir: "",
          config_path: "",
          user_home_default_dir: "",
          follow_codex_home_dir: "",
          approval_policy: null,
          sandbox_mode: null,
          model: null,
          model_reasoning_effort: null,
          plan_mode_reasoning_effort: null,
          web_search: null,
          personality: "  ",
        })}
        codexConfigToml={null}
        refreshCodex={refreshCodex}
        openCodexConfigDir={vi.fn()}
        persistCodexConfig={vi.fn()}
        persistCodexConfigToml={vi.fn().mockResolvedValue(false)}
      />
    );

    expect(screen.getByText("未检测到")).toBeInTheDocument();
    expect(screen.getByText("不存在（将自动创建）")).toBeInTheDocument();
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    expect(screen.getByText("检测失败：")).toBeInTheDocument();
    expect(screen.getByText("codex boom")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => expect(refreshCodex).toHaveBeenCalled());
  });
});
