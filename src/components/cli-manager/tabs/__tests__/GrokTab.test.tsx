import { fireEvent, render, screen } from "@testing-library/react";
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
    context_window: null,
    telemetry: null,
    supports_backend_search: null,
  },
  aio_preferences: null,
  effective_preferences: {
    model_id: "grok-4-fast",
    api_backend: "chat_completions",
    context_window: null,
    telemetry: null,
    supports_backend_search: null,
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
    context_window: null,
    telemetry: null,
    supports_backend_search: null,
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
    envConflicts: [],
    envConflictsLoading: false,
    envConflictsError: null,
    refreshGrok: vi.fn(),
    openGrokConfigDir: vi.fn(),
    setModelIdDraft: vi.fn(),
    setApiBackendDraft: vi.fn(),
    setContextWindowDraft: vi.fn(),
    setTelemetryDraft: vi.fn(),
    setSupportsBackendSearchDraft: vi.fn(),
    persistModelId: vi.fn(),
    persistApiBackend: vi.fn(),
    persistContextWindow: vi.fn(),
    persistTelemetry: vi.fn(),
    persistSupportsBackendSearch: vi.fn(),
    ...overrides,
  };
}

describe("components/cli-manager/tabs/GrokTab", () => {
  it("展示安装、现有配置、供应商和保守接管诊断，且不暴露更新或 WSL 操作", () => {
    render(<CliManagerGrokTab {...createProps()} />);

    expect(screen.getByRole("heading", { name: "Grok" })).toBeInTheDocument();
    expect(screen.getByText("已安装 0.2.93")).toBeInTheDocument();
    expect(screen.getByText("/usr/local/bin/grok")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.grok/config.toml")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "模型 ID (model_id)" })).toHaveValue("grok-4-fast");
    expect(screen.getByRole("radio", { name: "Chat Completions" })).toBeChecked();
    expect(screen.getByText("现有 Grok 配置")).toBeInTheDocument();
    expect(screen.getByText("grok-fast")).toBeInTheDocument();
    expect(screen.getByText("grok-summary")).toBeInTheDocument();
    expect(screen.getByText("grok-search")).toBeInTheDocument();
    expect(screen.getByText("未检测到企业策略文件")).toBeInTheDocument();

    expect(screen.queryByRole("button", { name: /安装/i })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /检查更新|执行更新|WSL/i })
    ).not.toBeInTheDocument();
  });

  it("CLI 未安装时仍可编辑模型，配置目录操作被禁用", () => {
    const persistModelId = vi.fn();
    const openGrokConfigDir = vi.fn();

    render(
      <CliManagerGrokTab
        {...createProps({
          grokAvailable: "unavailable",
          grokInfo: { ...DEFAULT_INFO, found: false, executable_path: null, version: null },
          persistModelId,
          openGrokConfigDir,
        })}
      />
    );

    expect(screen.getByText("未检测到")).toBeInTheDocument();

    expect(screen.getByTitle("打开当前生效目录")).toBeDisabled();
    expect(openGrokConfigDir).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /安装 Grok/i })).not.toBeInTheDocument();
  });

  it("配置无效时显示原始错误并阻止回退、保存和代理启用", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          grokConfig: null,
          grokConfigError: "GROK_CONFIG_INVALID: config.toml 第 7 行语法错误",
          preferencesDraft: {
            model_id: "",
            api_backend: "responses",
            context_window: null,
            telemetry: null,
            supports_backend_search: null,
          },
        })}
      />
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "GROK_CONFIG_INVALID: config.toml 第 7 行语法错误"
    );
    expect(screen.getByRole("textbox", { name: "模型 ID (model_id)" })).toHaveValue("");
    expect(screen.queryByDisplayValue("grok-build")).not.toBeInTheDocument();
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
          preferencesDraft: {
            model_id: "",
            api_backend: "responses",
            context_window: null,
            telemetry: null,
            supports_backend_search: null,
          },
        })}
      />
    );

    expect(screen.getByText("加载中...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "模型 ID (model_id)" })).toBeDisabled();
  });

  it("保存期间禁用刷新并展示 AIO 偏好来源", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          grokConfigSaving: true,
          grokConfig: {
            ...DEFAULT_CONFIG,
            preference_source: "aio_settings",
          },
        })}
      />
    );

    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    expect(screen.getByText("AIO 已保存偏好")).toBeInTheDocument();
  });

  it("配置文件尚未创建和 CLI 探测失败时正确展示", () => {
    render(
      <CliManagerGrokTab
        {...createProps({
          grokInfo: { ...DEFAULT_INFO, error: "version command failed" },
          grokConfig: { ...DEFAULT_CONFIG, file_exists: false },
        })}
      />
    );

    expect(screen.getByText("不存在（将自动创建）")).toBeInTheDocument();
    expect(screen.getByText("检测失败：version command failed")).toBeInTheDocument();
  });

  it("变更后立即自动保存（模型 onBlur、协议 onChange），并转发刷新与目录操作", () => {
    const persistModelId = vi.fn();
    const persistApiBackend = vi.fn();
    const refreshGrok = vi.fn();
    const openGrokConfigDir = vi.fn();

    function Harness() {
      const [preferencesDraft, setPreferencesDraft] = useState<GrokProxyPreferences>({
        model_id: "grok-4-fast",
        api_backend: "chat_completions",
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      });

      return (
        <CliManagerGrokTab
          {...createProps({
            preferencesDraft,
            refreshGrok,
            openGrokConfigDir,
            setModelIdDraft: (modelId) =>
              setPreferencesDraft((current) => ({ ...current, model_id: modelId })),
            setApiBackendDraft: (apiBackend) =>
              setPreferencesDraft((current) => ({ ...current, api_backend: apiBackend })),
            persistModelId: (m) => {
              persistModelId(m);
              setPreferencesDraft((current) => ({ ...current, model_id: m.trim() }));
              return Promise.resolve();
            },
            persistApiBackend: (b) => {
              persistApiBackend(b);
              setPreferencesDraft((current) => ({ ...current, api_backend: b }));
              return Promise.resolve();
            },
          })}
        />
      );
    }

    render(<Harness />);

    fireEvent.change(screen.getByRole("textbox", { name: "模型 ID (model_id)" }), {
      target: { value: "grok-4.1-fast" },
    });
    fireEvent.blur(screen.getByRole("textbox", { name: "模型 ID (model_id)" }));

    expect(persistModelId).toHaveBeenCalledWith("grok-4.1-fast");

    fireEvent.click(screen.getByRole("radio", { name: "Responses" }));
    expect(persistApiBackend).toHaveBeenCalledWith("responses");

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    fireEvent.click(screen.getByTitle("打开当前生效目录"));

    expect(refreshGrok).toHaveBeenCalledTimes(1);
    expect(openGrokConfigDir).toHaveBeenCalledTimes(1);
  });

  it("自动保存 context_window、遥测和服务端搜索设置", () => {
    const persistContextWindow = vi.fn();
    const persistTelemetry = vi.fn();
    const persistSupportsBackendSearch = vi.fn();

    function Harness() {
      const [preferencesDraft, setPreferencesDraft] = useState<GrokProxyPreferences>({
        model_id: "grok-4-fast",
        api_backend: "responses",
        context_window: null,
        telemetry: null,
        supports_backend_search: null,
      });

      return (
        <CliManagerGrokTab
          {...createProps({
            preferencesDraft,
            setContextWindowDraft: (contextWindow) =>
              setPreferencesDraft((current) => ({
                ...current,
                context_window: contextWindow,
              })),
            setTelemetryDraft: (telemetry) =>
              setPreferencesDraft((current) => ({ ...current, telemetry })),
            setSupportsBackendSearchDraft: (supportsBackendSearch) =>
              setPreferencesDraft((current) => ({
                ...current,
                supports_backend_search: supportsBackendSearch,
              })),
            persistContextWindow,
            persistTelemetry,
            persistSupportsBackendSearch,
          })}
        />
      );
    }

    render(<Harness />);

    const contextWindow = screen.getByRole("spinbutton", { name: "context_window" });
    fireEvent.change(contextWindow, { target: { value: "500000" } });
    fireEvent.blur(contextWindow);
    expect(persistContextWindow).toHaveBeenCalledWith(500_000);

    fireEvent.click(screen.getByRole("switch", { name: "关闭客户端遥测" }));
    expect(persistTelemetry).toHaveBeenCalledWith(false);

    fireEvent.click(screen.getByRole("switch", { name: "服务端搜索" }));
    expect(persistSupportsBackendSearch).toHaveBeenCalledWith(false);
  });

  it("展示接管、环境变量和企业策略诊断", () => {
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
        })}
      />
    );

    expect(screen.getAllByText("已接管")).toHaveLength(2);
    expect(screen.getByText("检测到 1 个企业策略文件")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.grok/requirements.toml")).toBeInTheDocument();
    expect(screen.getByText("检测到 1 个相关环境变量")).toBeInTheDocument();
  });
});
