import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import type {
  GrokConfigState,
  GrokProxyPreferences,
  SimpleCliInfo,
} from "../../../../services/cli/cliManager";
import { CliManagerGrokTab, type CliManagerGrokTabProps } from "../GrokTab";

const DEFAULT_INFO: SimpleCliInfo = {
  found: true,
  executable_path: "/usr/local/bin/grok",
  version: "0.2.93",
  error: null,
  shell: "zsh",
  resolved_via: "path",
};

const DEFAULT_CONFIG: GrokConfigState = {
  config_path: "/Users/test/.grok/config.toml",
  file_exists: true,
  preferences: {
    model_id: "grok-4-fast",
    api_backend: "chat_completions",
  },
  aio_preferences: null,
  effective_preferences: {
    model_id: "grok-4-fast",
    api_backend: "chat_completions",
  },
  preference_source: "existing_config",
  default_profile: "grok-fast",
  session_summary_profile: "grok-summary",
  web_search_profile: "grok-search",
  image_description_profile: null,
  policy_files: [],
};

function createProps(overrides: Partial<CliManagerGrokTabProps> = {}): CliManagerGrokTabProps {
  const preferencesDraft: GrokProxyPreferences = {
    model_id: "grok-4-fast",
    api_backend: "chat_completions",
  };

  return {
    grokAvailable: "available",
    grokLoading: false,
    grokInfo: DEFAULT_INFO,
    grokConfigLoading: false,
    grokConfigSaving: false,
    grokConfig: DEFAULT_CONFIG,
    grokConfigError: null,
    preferencesDraft,
    providers: [
      { id: 1, enabled: true },
      { id: 2, enabled: true },
      { id: 3, enabled: false },
    ] as CliManagerGrokTabProps["providers"],
    providersLoading: false,
    envConflicts: [],
    envConflictsLoading: false,
    envConflictsError: null,
    cliProxyLoading: false,
    cliProxyAvailable: true,
    cliProxyEnabled: false,
    cliProxyAppliedToCurrentGateway: null,
    cliProxyToggling: false,
    pendingCliProxyEnablePrompt: null,
    refreshGrok: vi.fn(),
    openGrokConfigDir: vi.fn(),
    setModelIdDraft: vi.fn(),
    setApiBackendDraft: vi.fn(),
    persistPreferences: vi.fn(),
    requestCliProxyEnabledSwitch: vi.fn(),
    clearPendingCliProxyEnablePrompt: vi.fn(),
    confirmPendingCliProxyEnable: vi.fn(),
    ...overrides,
  };
}

describe("components/cli-manager/tabs/GrokTab", () => {
  it("展示安装、现有配置、供应商和保守接管诊断，且不暴露更新或 WSL 操作", () => {
    render(<CliManagerGrokTab {...createProps()} />);

    expect(screen.getByRole("heading", { name: "Grok CLI" })).toBeInTheDocument();
    expect(screen.getByText("已安装 0.2.93")).toBeInTheDocument();
    expect(screen.getByText("/usr/local/bin/grok")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.grok/config.toml")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "模型 ID" })).toHaveValue("grok-4-fast");
    expect(screen.getByRole("radio", { name: "Chat Completions" })).toBeChecked();
    expect(screen.getByText("现有 Grok 配置")).toBeInTheDocument();
    expect(screen.getByText("2 个已启用 / 3 个供应商")).toBeInTheDocument();
    expect(screen.getByText("grok-fast")).toBeInTheDocument();
    expect(screen.getByText("grok-summary")).toBeInTheDocument();
    expect(screen.getByText("grok-search")).toBeInTheDocument();
    expect(screen.getByText("可能绕过网关")).toBeInTheDocument();
    expect(screen.getByText("未检测到企业策略文件")).toBeInTheDocument();

    expect(screen.queryByRole("button", { name: /安装/i })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /检查更新|执行更新|WSL/i })
    ).not.toBeInTheDocument();
  });

  it("CLI 未安装时仍允许保存偏好，但禁用代理和配置目录操作", () => {
    const persistPreferences = vi.fn();
    const openGrokConfigDir = vi.fn();
    const requestCliProxyEnabledSwitch = vi.fn();

    render(
      <CliManagerGrokTab
        {...createProps({
          grokAvailable: "unavailable",
          grokInfo: { ...DEFAULT_INFO, found: false, executable_path: null, version: null },
          persistPreferences,
          openGrokConfigDir,
          requestCliProxyEnabledSwitch,
        })}
      />
    );

    expect(screen.getByText("未检测到")).toBeInTheDocument();

    const saveButton = screen.getByRole("button", { name: "保存偏好" });
    expect(saveButton).toBeEnabled();
    fireEvent.click(saveButton);
    expect(persistPreferences).toHaveBeenCalledTimes(1);

    expect(screen.getByRole("button", { name: "打开 Grok 配置目录" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Grok CLI 代理" })).toBeDisabled();
    expect(openGrokConfigDir).not.toHaveBeenCalled();
    expect(requestCliProxyEnabledSwitch).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /安装 Grok/i })).not.toBeInTheDocument();
  });

  it("配置无效时显示原始错误并阻止回退、保存和代理启用", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          grokConfig: null,
          grokConfigError: "GROK_CONFIG_INVALID: config.toml 第 7 行语法错误",
          preferencesDraft: { model_id: "", api_backend: "responses" },
        })}
      />
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "GROK_CONFIG_INVALID: config.toml 第 7 行语法错误"
    );
    expect(screen.getByRole("textbox", { name: "模型 ID" })).toHaveValue("");
    expect(screen.queryByDisplayValue("grok-build")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存偏好" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Grok CLI 代理" })).toBeDisabled();
  });

  it("加载期间显示检测状态并锁定所有写操作", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          grokAvailable: "checking",
          grokLoading: true,
          grokInfo: null,
          grokConfigLoading: true,
          grokConfig: null,
          preferencesDraft: { model_id: "", api_backend: "responses" },
        })}
      />
    );

    expect(screen.getByText("检测中...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新 Grok 状态" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "模型 ID" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "保存偏好" })).toBeDisabled();
    expect(screen.getByRole("switch", { name: "Grok CLI 代理" })).toBeDisabled();
  });

  it("区分供应商加载、配置文件尚未创建和 CLI 探测失败", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          providers: null,
          providersLoading: true,
          grokInfo: { ...DEFAULT_INFO, error: "version command failed" },
          grokConfig: { ...DEFAULT_CONFIG, file_exists: false },
        })}
      />
    );

    expect(screen.getByText("正在读取供应商...")).toBeInTheDocument();
    expect(screen.queryByText("0 个已启用 / 0 个供应商")).not.toBeInTheDocument();
    expect(screen.getByText("配置文件尚未创建")).toBeInTheDocument();
    expect(screen.getByText("检测失败：version command failed")).toBeInTheDocument();
  });

  it("只在用户显式保存时提交当前模型和协议，并转发刷新、目录与代理操作", () => {
    const persistPreferences = vi.fn();
    const refreshGrok = vi.fn();
    const openGrokConfigDir = vi.fn();
    const requestCliProxyEnabledSwitch = vi.fn();

    function Harness() {
      const [preferencesDraft, setPreferencesDraft] = useState<GrokProxyPreferences>({
        model_id: "grok-4-fast",
        api_backend: "chat_completions",
      });

      return (
        <CliManagerGrokTab
          {...createProps({
            preferencesDraft,
            refreshGrok,
            openGrokConfigDir,
            requestCliProxyEnabledSwitch,
            setModelIdDraft: (modelId) =>
              setPreferencesDraft((current) => ({ ...current, model_id: modelId })),
            setApiBackendDraft: (apiBackend) =>
              setPreferencesDraft((current) => ({ ...current, api_backend: apiBackend })),
            persistPreferences: () => persistPreferences(preferencesDraft),
          })}
        />
      );
    }

    render(<Harness />);

    fireEvent.change(screen.getByRole("textbox", { name: "模型 ID" }), {
      target: { value: "grok-4.1-fast" },
    });
    fireEvent.click(screen.getByRole("radio", { name: "Responses" }));

    expect(persistPreferences).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "保存偏好" }));
    expect(persistPreferences).toHaveBeenCalledWith({
      model_id: "grok-4.1-fast",
      api_backend: "responses",
    });

    fireEvent.click(screen.getByRole("button", { name: "刷新 Grok 状态" }));
    fireEvent.click(screen.getByRole("button", { name: "打开 Grok 配置目录" }));
    fireEvent.click(screen.getByRole("switch", { name: "Grok CLI 代理" }));

    expect(refreshGrok).toHaveBeenCalledTimes(1);
    expect(openGrokConfigDir).toHaveBeenCalledTimes(1);
    expect(requestCliProxyEnabledSwitch).toHaveBeenCalledWith(true);
  });

  it("展示接管、环境变量和企业策略诊断，并通过共享弹窗确认代理冲突", () => {
    const clearPendingCliProxyEnablePrompt = vi.fn();
    const confirmPendingCliProxyEnable = vi.fn();
    const conflict = {
      var_name: "XAI_API_KEY",
      source_type: "system" as const,
      source_path: "Process Environment",
    };

    render(
      <CliManagerGrokTab
        {...createProps({
          grokConfig: {
            ...DEFAULT_CONFIG,
            default_profile: "aio",
            session_summary_profile: "aio",
            policy_files: [
              {
                kind: "requirements_user",
                path: "/Users/test/.grok/requirements.toml",
                exists: true,
              },
            ],
          },
          envConflicts: [conflict],
          cliProxyEnabled: true,
          cliProxyAppliedToCurrentGateway: true,
          pendingCliProxyEnablePrompt: { cliKey: "grok", conflicts: [conflict] },
          clearPendingCliProxyEnablePrompt,
          confirmPendingCliProxyEnable,
        })}
      />
    );

    expect(screen.getAllByText("已接管")).toHaveLength(2);
    expect(screen.getByText("检测到 1 个企业策略文件")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.grok/requirements.toml")).toBeInTheDocument();
    expect(screen.getByText("检测到 1 个相关环境变量")).toBeInTheDocument();

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("XAI_API_KEY")).toBeInTheDocument();
    expect(within(dialog).getByText("Process Environment")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: "继续启用" }));
    expect(confirmPendingCliProxyEnable).toHaveBeenCalledTimes(1);

    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(clearPendingCliProxyEnablePrompt).toHaveBeenCalledTimes(1);
  });
});
