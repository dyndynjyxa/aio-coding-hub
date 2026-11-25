import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CliManagerGeminiTab, type CliManagerGeminiTabProps } from "../GeminiTab";

vi.mock("../../CliVersionBadge", () => ({
  CliVersionBadge: ({ cliKey }: { cliKey: string }) => <div>version-badge-{cliKey}</div>,
}));

const MODELS = [
  { id: "gemini-4-neon-high", label: "Gemini 4 Neon (High)" },
  { id: "gemini-3.8-flash-low", label: "Gemini 3.8 Flash (Low)" },
];

function createInfo(overrides: Partial<CliManagerGeminiTabProps["geminiInfo"]> = {}) {
  return {
    found: true,
    version: "1.5.16",
    executable_path: "/home/user/.local/bin/agy",
    resolved_via: "login_shell",
    shell: "/bin/zsh",
    error: null,
    ...overrides,
  };
}

function createConfig(
  overrides: Partial<NonNullable<CliManagerGeminiTabProps["geminiConfig"]>> = {}
) {
  return {
    configDir: "/home/user/.gemini/antigravity-cli",
    configPath: "/home/user/.gemini/antigravity-cli/settings.json",
    exists: true,
    model: "Gemini 3.8 Flash (Low)",
    agentMode: null,
    toolPermission: "strict",
    artifactReviewPolicy: null,
    enableTerminalSandbox: null,
    allowNonWorkspaceAccess: true,
    colorScheme: null,
    altScreenMode: null,
    verbosity: null,
    editorMode: null,
    notifications: null,
    showTips: null,
    ...overrides,
  };
}

function renderTab(overrides: Partial<CliManagerGeminiTabProps> = {}) {
  const props: CliManagerGeminiTabProps = {
    geminiAvailable: "available",
    geminiLoading: false,
    geminiInfo: createInfo(),
    geminiConfigLoading: false,
    geminiConfigSaving: false,
    geminiConfig: createConfig(),
    geminiModels: MODELS,
    geminiModelsError: false,
    refreshGeminiInfo: vi.fn(),
    persistGeminiConfig: vi.fn(),
    ...overrides,
  };
  const view = render(<CliManagerGeminiTab {...props} />);
  return { ...view, props };
}

describe("components/cli-manager/tabs/GeminiTab", () => {
  it("renders installed agy, version badge for the gemini key, and refreshes", async () => {
    const refreshGeminiInfo = vi.fn().mockResolvedValue(undefined);
    renderTab({ refreshGeminiInfo });

    expect(screen.getByRole("heading", { level: 2, name: "Gemini" })).toBeInTheDocument();
    expect(screen.getByText("已安装 1.5.16")).toBeInTheDocument();
    expect(screen.getByText("version-badge-gemini")).toBeInTheDocument();
    expect(screen.getByText("/home/user/.local/bin/agy")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新状态" }));
    await waitFor(() => expect(refreshGeminiInfo).toHaveBeenCalledTimes(1));
  });

  it("selects models by label from agy models and can reset to CLI default", () => {
    const { props } = renderTab();
    const select = screen.getByRole("combobox", { name: "默认模型" });

    expect(select).toHaveValue("Gemini 3.8 Flash (Low)");
    fireEvent.change(select, { target: { value: "Gemini 4 Neon (High)" } });
    expect(props.persistGeminiConfig).toHaveBeenCalledWith({ model: "Gemini 4 Neon (High)" });

    fireEvent.change(select, { target: { value: "" } });
    expect(props.persistGeminiConfig).toHaveBeenLastCalledWith({ model: "" });
  });

  it("keeps a saved model that is missing from the catalog selectable", () => {
    renderTab({ geminiConfig: createConfig({ model: "Gemini 9 Custom" }) });
    const select = screen.getByRole("combobox", { name: "默认模型" });

    expect(select).toHaveValue("Gemini 9 Custom");
    expect(screen.getByRole("option", { name: "Gemini 9 Custom" })).toBeInTheDocument();
  });

  it("falls back to a free-text model input when the catalog fails to load", () => {
    const { props } = renderTab({ geminiModels: null, geminiModelsError: true });
    expect(screen.getByText(/读取模型列表失败/)).toBeInTheDocument();

    const input = screen.getByRole("textbox", { name: "默认模型" });
    fireEvent.blur(input);
    expect(props.persistGeminiConfig).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: "  Gemini 3.1 Pro (High) " } });
    fireEvent.blur(input);
    expect(props.persistGeminiConfig).toHaveBeenCalledWith({ model: "Gemini 3.1 Pro (High)" });
  });

  it("resets the model draft when the saved config changes", () => {
    const { rerender, props } = renderTab({ geminiModels: [] });
    const input = screen.getByRole("textbox", { name: "默认模型" });
    fireEvent.change(input, { target: { value: "dirty" } });
    expect(input).toHaveValue("dirty");

    rerender(
      <CliManagerGeminiTab
        {...props}
        geminiConfig={createConfig({ model: "Gemini 4 Neon (High)" })}
      />
    );
    expect(screen.getByRole("textbox", { name: "默认模型" })).toHaveValue("Gemini 4 Neon (High)");
  });

  it("persists enum and boolean settings with CLI defaults shown when unset", () => {
    const { props } = renderTab();

    const toolPermission = screen.getByRole("combobox", { name: "toolPermission" });
    expect(toolPermission).toHaveValue("strict");
    fireEvent.change(toolPermission, { target: { value: "request-review" } });
    expect(props.persistGeminiConfig).toHaveBeenCalledWith({ toolPermission: "request-review" });

    const agentMode = screen.getByRole("combobox", { name: "agentMode" });
    expect(agentMode).toHaveValue("");
    fireEvent.change(agentMode, { target: { value: "plan" } });
    expect(props.persistGeminiConfig).toHaveBeenCalledWith({ agentMode: "plan" });

    // showTips defaults to on in agy, the others to off.
    expect(screen.getByRole("switch", { name: "showTips" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "notifications" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "allowNonWorkspaceAccess" })).toBeChecked();
    fireEvent.click(screen.getByRole("switch", { name: "notifications" }));
    expect(props.persistGeminiConfig).toHaveBeenCalledWith({ notifications: true });
  });

  it("shows an unknown on-disk enum value instead of hiding it", () => {
    renderTab({ geminiConfig: createConfig({ verbosity: "future-level" }) });
    expect(screen.getByRole("combobox", { name: "verbosity" })).toHaveValue("future-level");
  });

  it("disables every control while saving", () => {
    const { props } = renderTab({ geminiConfigSaving: true });

    expect(screen.getByRole("combobox", { name: "默认模型" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "colorScheme" })).toBeDisabled();
    const sandbox = screen.getByRole("switch", { name: "enableTerminalSandbox" });
    expect(sandbox).toBeDisabled();
    fireEvent.click(sandbox);
    expect(props.persistGeminiConfig).not.toHaveBeenCalled();
  });

  it("renders checking, missing-binary, no-config, unavailable and error states", () => {
    const { rerender, props } = renderTab({
      geminiAvailable: "checking",
      geminiLoading: true,
      geminiInfo: null,
      geminiConfig: null,
    });
    expect(screen.getByText("检测中...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新状态" })).toBeDisabled();
    expect(screen.getByText("暂无信息，请尝试刷新")).toBeInTheDocument();

    rerender(
      <CliManagerGeminiTab
        {...props}
        geminiAvailable="available"
        geminiLoading={false}
        geminiInfo={createInfo({
          found: false,
          version: null,
          executable_path: null,
          error: "boom",
        })}
        geminiConfig={null}
      />
    );
    expect(screen.getByText("未检测到")).toBeInTheDocument();
    expect(screen.getByText(/antigravity\.google\/cli\/install\.sh/)).toBeInTheDocument();
    expect(screen.getByText("暂无配置，请尝试刷新")).toBeInTheDocument();
    expect(screen.getByText("boom")).toBeInTheDocument();

    rerender(
      <CliManagerGeminiTab
        {...props}
        geminiAvailable="unavailable"
        geminiLoading={false}
        geminiInfo={null}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });
});
