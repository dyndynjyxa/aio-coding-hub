import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { tauriDialogOpen, tauriOpenPath } from "../../test/mocks/tauri";
import type { AppSettings } from "../../services/settings/settings";
import { createTestAppSettings } from "../../test/fixtures/settings";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { CliManagerPage } from "../CliManagerPage";
import { logToConsole } from "../../services/consoleLog";
import {
  useSettingsCircuitBreakerNoticeSetMutation,
  useSettingsCodexSessionIdCompletionSetMutation,
  useSettingsGatewayRectifierSetMutation,
  useSettingsQuery,
  useSettingsSetMutation,
} from "../../query/settings";
import {
  useCliManagerClaudeInfoQuery,
  useCliManagerClaudeSettingsQuery,
  useCliManagerClaudeSettingsSetMutation,
  useCliManagerCodexConfigQuery,
  useCliManagerCodexConfigSetMutation,
  useCliManagerCodexConfigTomlQuery,
  useCliManagerCodexConfigTomlSetMutation,
  useCliManagerCodexInfoQuery,
  useCliManagerGeminiConfigQuery,
  useCliManagerGeminiConfigSetMutation,
  useCliManagerGeminiInfoQuery,
} from "../../query/cliManager";
import { useProvidersListQuery } from "../../query/providers";
import { cliProxyKeys } from "../../query/keys";
import { cliProxyRebindCodexHome, cliProxyStatusAll } from "../../services/cli/cliProxy";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../services/cli/cliProxy", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliProxy")>(
    "../../services/cli/cliProxy"
  );
  return {
    ...actual,
    cliProxyStatusAll: vi.fn(),
    cliProxyRebindCodexHome: vi.fn(),
  };
});

vi.mock("../../components/cli-manager/tabs/GeneralTab", () => ({
  CliManagerGeneralTab: ({
    onPersistRectifier,
    onPersistCircuitBreakerNotice,
    onPersistCodexSessionIdCompletion,
    onPersistCacheAnomalyMonitor,
    onPersistCommonSettings,
    blurOnEnter,
  }: any) => (
    <div>
      <input aria-label="enter-blur" onKeyDown={blurOnEnter} />
      <button type="button" onClick={() => onPersistRectifier({ enable_response_fixer: false })}>
        persist-rectifier
      </button>
      <button type="button" onClick={() => onPersistCircuitBreakerNotice(true)}>
        persist-circuit-notice
      </button>
      <button type="button" onClick={() => onPersistCircuitBreakerNotice(false)}>
        disable-circuit-notice
      </button>
      <button type="button" onClick={() => onPersistCodexSessionIdCompletion(false)}>
        persist-codex-completion
      </button>
      <button type="button" onClick={() => onPersistCodexSessionIdCompletion(true)}>
        enable-codex-completion
      </button>
      <button type="button" onClick={() => onPersistCacheAnomalyMonitor(true)}>
        enable-cache-monitor
      </button>
      <button type="button" onClick={() => onPersistCacheAnomalyMonitor(false)}>
        disable-cache-monitor
      </button>
      <button
        type="button"
        onClick={() => onPersistCommonSettings({ provider_cooldown_seconds: 99 })}
      >
        persist-common
      </button>
    </div>
  ),
}));

vi.mock("../../components/cli-manager/tabs/ClaudeTab", () => ({
  CliManagerClaudeTab: ({ refreshClaude, openClaudeConfigDir, persistClaudeSettings }: any) => (
    <div>
      <div>claude-tab</div>
      <button type="button" onClick={() => refreshClaude()}>
        refresh-claude
      </button>
      <button type="button" onClick={() => openClaudeConfigDir()}>
        open-claude-dir
      </button>
      <button type="button" onClick={() => persistClaudeSettings({ foo: "bar" })}>
        save-claude
      </button>
    </div>
  ),
}));

vi.mock("../../components/cli-manager/tabs/CodexTab", () => ({
  CliManagerCodexTab: ({
    refreshCodex,
    openCodexConfigDir,
    persistCodexConfig,
    persistCodexHomeSettings,
    pickCodexHomeDirectory,
  }: any) => (
    <div>
      <div>codex-tab</div>
      <button type="button" onClick={() => refreshCodex()}>
        refresh-codex
      </button>
      <button type="button" onClick={() => openCodexConfigDir()}>
        open-codex-dir
      </button>
      <button type="button" onClick={() => pickCodexHomeDirectory?.()}>
        pick-codex-dir
      </button>
      <button type="button" onClick={() => persistCodexConfig({ foo: "bar" })}>
        save-codex
      </button>
      <button
        type="button"
        onClick={() => persistCodexHomeSettings?.("custom", "D:\\Work\\CodexHome")}
      >
        save-codex-home
      </button>
    </div>
  ),
}));

vi.mock("../../components/cli-manager/tabs/GeminiTab", () => ({
  CliManagerGeminiTab: ({ refreshGeminiInfo, persistGeminiConfig }: any) => (
    <div>
      <div>gemini-tab</div>
      <button type="button" onClick={() => refreshGeminiInfo()}>
        refresh-gemini
      </button>
      <button type="button" onClick={() => persistGeminiConfig?.({ modelName: "gemini-2.5-pro" })}>
        save-gemini
      </button>
    </div>
  ),
}));

vi.mock("../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/settings")>("../../query/settings");
  return {
    ...actual,
    useSettingsQuery: vi.fn(),
    useSettingsGatewayRectifierSetMutation: vi.fn(),
    useSettingsCircuitBreakerNoticeSetMutation: vi.fn(),
    useSettingsCodexSessionIdCompletionSetMutation: vi.fn(),
    useSettingsSetMutation: vi.fn(),
  };
});

vi.mock("../../query/cliManager", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/cliManager")>("../../query/cliManager");
  return {
    ...actual,
    useCliManagerClaudeInfoQuery: vi.fn(),
    useCliManagerClaudeSettingsQuery: vi.fn(),
    useCliManagerClaudeSettingsSetMutation: vi.fn(),
    useCliManagerCodexInfoQuery: vi.fn(),
    useCliManagerCodexConfigQuery: vi.fn(),
    useCliManagerCodexConfigSetMutation: vi.fn(),
    useCliManagerCodexConfigTomlQuery: vi.fn(),
    useCliManagerCodexConfigTomlSetMutation: vi.fn(),
    useCliManagerGeminiConfigQuery: vi.fn(),
    useCliManagerGeminiConfigSetMutation: vi.fn(),
    useCliManagerGeminiInfoQuery: vi.fn(),
  };
});

vi.mock("../../query/providers", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/providers")>("../../query/providers");
  return {
    ...actual,
    useProvidersListQuery: vi.fn(),
  };
});

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  const view = render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
  return { ...view, client };
}

function createAppSettings(
  overrides: Partial<{ [K in keyof AppSettings]: AppSettings[K] | undefined }> = {}
) {
  return {
    ...createTestAppSettings(),
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();

  vi.mocked(useCliManagerGeminiConfigQuery).mockReturnValue({
    data: null,
    isFetching: false,
    refetch: vi.fn(),
  } as any);

  vi.mocked(useCliManagerGeminiConfigSetMutation).mockReturnValue({
    isPending: false,
    mutateAsync: vi.fn(),
  } as any);

  vi.mocked(useProvidersListQuery).mockReturnValue({
    data: null,
    isFetching: false,
    refetch: vi.fn(),
  } as any);
});

describe("pages/CliManagerPage", () => {
  it("drives general tab persistence and handles tauri unavailable/errors", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);

    const rectifierMutation = { isPending: false, mutateAsync: vi.fn() };
    rectifierMutation.mutateAsync
      .mockResolvedValueOnce(createAppSettings({ enable_response_fixer: false }))
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error("rectifier boom"));
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue(rectifierMutation as any);

    const noticeMutation = { isPending: false, mutateAsync: vi.fn() };
    noticeMutation.mutateAsync
      .mockResolvedValueOnce(createAppSettings({ enable_circuit_breaker_notice: true }))
      .mockResolvedValueOnce(null);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue(noticeMutation as any);

    const completionMutation = { isPending: false, mutateAsync: vi.fn() };
    completionMutation.mutateAsync
      .mockResolvedValueOnce(createAppSettings({ enable_codex_session_id_completion: false }))
      .mockResolvedValueOnce(null);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue(
      completionMutation as any
    );

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync
      .mockResolvedValueOnce(createAppSettings({ provider_cooldown_seconds: 99 }))
      .mockRejectedValueOnce(new Error("common boom"));
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    // CLI manager queries are disabled until tab is selected; provide stable placeholders.
    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<CliManagerPage />);

    fireEvent.keyDown(screen.getByLabelText("enter-blur"), { key: "Enter" });

    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));
    await waitFor(() => expect(rectifierMutation.mutateAsync).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));
    await waitFor(() => expect(rectifierMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith("error", "更新网关整流配置失败", {
        error: "Error: rectifier boom",
      })
    );

    fireEvent.click(screen.getByRole("button", { name: "persist-circuit-notice" }));
    await waitFor(() => expect(noticeMutation.mutateAsync).toHaveBeenCalledWith(true));
    expect(toast).toHaveBeenCalledWith("已开启熔断通知");

    fireEvent.click(screen.getByRole("button", { name: "persist-circuit-notice" }));
    await waitFor(() => expect(noticeMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "persist-codex-completion" }));
    await waitFor(() => expect(completionMutation.mutateAsync).toHaveBeenCalledWith(false));
    expect(toast).toHaveBeenCalledWith("已关闭 Codex Session ID 补全");

    fireEvent.click(screen.getByRole("button", { name: "persist-codex-completion" }));
    await waitFor(() => expect(completionMutation.mutateAsync).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "persist-common" }));
    await waitFor(() => expect(commonMutation.mutateAsync).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("已保存");

    fireEvent.click(screen.getByRole("button", { name: "persist-common" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新通用网关参数失败：请稍后重试"));
  });

  it("blocks app settings writes when settings query is readonly", () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
      isError: true,
      error: new Error("boom"),
    } as any);

    const rectifierMutation = { isPending: false, mutateAsync: vi.fn() };
    const noticeMutation = { isPending: false, mutateAsync: vi.fn() };
    const completionMutation = { isPending: false, mutateAsync: vi.fn() };
    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue(rectifierMutation as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue(noticeMutation as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue(
      completionMutation as any
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<CliManagerPage />);

    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-circuit-notice" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-codex-completion" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-common" }));

    expect(rectifierMutation.mutateAsync).not.toHaveBeenCalled();
    expect(noticeMutation.mutateAsync).not.toHaveBeenCalled();
    expect(completionMutation.mutateAsync).not.toHaveBeenCalled();
    expect(commonMutation.mutateAsync).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(
      "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。"
    );
  });

  it("drives claude/codex/gemini tab actions and handles open dir edge cases", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const claudeInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const claudeSettingsRefetch = vi.fn().mockResolvedValue({ data: {} });

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: { config_dir: "/claude", found: true },
      isFetching: false,
      refetch: claudeInfoRefetch,
    } as any);

    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: { config_dir: "/claude-settings" },
      isFetching: false,
      refetch: claudeSettingsRefetch,
    } as any);

    const claudeSetMutation = { isPending: false, mutateAsync: vi.fn() };
    claudeSetMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ ok: true })
      .mockRejectedValueOnce(new Error("claude boom"));
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue(claudeSetMutation as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);

    let codexCanOpen = false;
    vi.mocked(useCliManagerCodexConfigQuery).mockImplementation(() => {
      return {
        data: {
          config_dir: "/codex",
          can_open_config_dir: codexCanOpen,
        },
        isFetching: false,
        refetch: codexConfigRefetch,
      } as any;
    });

    const codexSetMutation = { isPending: false, mutateAsync: vi.fn() };
    codexSetMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ ok: true })
      .mockRejectedValueOnce(new Error("CODEX_NO_PERM: denied"));
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue(codexSetMutation as any);

    const codexConfigTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexConfigTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const geminiInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: geminiInfoRefetch,
    } as any);

    renderWithProviders(<CliManagerPage />);

    // Claude tab
    fireEvent.click(screen.getByRole("tab", { name: "Claude Code" }));
    expect(await screen.findByText("claude-tab")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "refresh-claude" }));
    await waitFor(() => expect(claudeInfoRefetch).toHaveBeenCalled());

    vi.mocked(tauriOpenPath).mockRejectedValueOnce(new Error("open claude boom"));
    fireEvent.click(screen.getByRole("button", { name: "open-claude-dir" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("打开目录失败：请查看控制台日志"));

    // persist claude settings: null -> toast; ok -> toast; error -> toast
    fireEvent.click(screen.getByRole("button", { name: "save-claude" }));
    await waitFor(() => expect(claudeSetMutation.mutateAsync).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "save-claude" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已更新 Claude Code 配置"));
    fireEvent.click(screen.getByRole("button", { name: "save-claude" }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("更新 Claude Code 配置失败：请稍后重试")
    );

    // Codex tab permission denied -> toast
    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    expect(await screen.findByText("codex-tab")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "open-codex-dir" }));
    expect(toast).toHaveBeenCalledWith("受权限限制，无法自动打开该目录");

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/codex-picked");
    fireEvent.click(screen.getByRole("button", { name: "pick-codex-dir" }));
    await waitFor(() =>
      expect(tauriDialogOpen).toHaveBeenCalledWith(
        expect.objectContaining({
          directory: true,
          multiple: false,
          title: "选择 Codex .codex 目录",
          defaultPath: "/codex",
        })
      )
    );

    // enable open dir and retry (error branch)
    codexCanOpen = true;
    fireEvent.click(screen.getByRole("tab", { name: "通用" }));
    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");

    vi.mocked(tauriOpenPath).mockRejectedValueOnce(new Error("open codex boom"));
    fireEvent.click(screen.getByRole("button", { name: "open-codex-dir" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("打开目录失败：请查看控制台日志"));

    // persist codex config: null -> toast; ok -> toast; error -> toast formatted
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    await waitFor(() => expect(codexSetMutation.mutateAsync).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已更新 Codex 配置"));
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("更新 Codex 配置失败（code CODEX_NO_PERM）：denied")
    );

    // Gemini tab refresh
    fireEvent.click(screen.getByRole("tab", { name: "Gemini" }));
    expect(await screen.findByText("gemini-tab")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "refresh-gemini" }));
    await waitFor(() => expect(geminiInfoRefetch).toHaveBeenCalled());
  });

  it("skips persisting when mutations are pending", async () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);

    const rectifierMutation = { isPending: true, mutateAsync: vi.fn() };
    const noticeMutation = { isPending: true, mutateAsync: vi.fn() };
    const completionMutation = { isPending: true, mutateAsync: vi.fn() };
    const commonMutation = { isPending: true, mutateAsync: vi.fn() };

    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue(rectifierMutation as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue(noticeMutation as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue(
      completionMutation as any
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: { config_dir: "/claude", found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: { config_dir: "/claude-settings" },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    const claudeSetMutation = { isPending: true, mutateAsync: vi.fn() };
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue(claudeSetMutation as any);

    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    const codexSetMutation = { isPending: true, mutateAsync: vi.fn() };
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue(codexSetMutation as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<CliManagerPage />);

    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-circuit-notice" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-codex-completion" }));
    fireEvent.click(screen.getByRole("button", { name: "persist-common" }));

    expect(rectifierMutation.mutateAsync).not.toHaveBeenCalled();
    expect(noticeMutation.mutateAsync).not.toHaveBeenCalled();
    expect(completionMutation.mutateAsync).not.toHaveBeenCalled();
    expect(commonMutation.mutateAsync).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("tab", { name: "Claude Code" }));
    await screen.findByText("claude-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-claude" }));
    expect(claudeSetMutation.mutateAsync).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    expect(codexSetMutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("skips persisting when settings are loading/unavailable and CLIs are checking", async () => {
    vi.mocked(toast).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({ data: null, isLoading: true } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const claudeSetMutation = { isPending: false, mutateAsync: vi.fn() };
    const codexSetMutation = { isPending: false, mutateAsync: vi.fn() };

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: true,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue(claudeSetMutation as any);

    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: null,
      isFetching: true,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue(codexSetMutation as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: true,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<CliManagerPage />);

    // rectifierAvailable is \"checking\" => no-op
    fireEvent.click(screen.getByRole("button", { name: "persist-rectifier" }));

    fireEvent.click(screen.getByRole("tab", { name: "Claude Code" }));
    await screen.findByText("claude-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-claude" }));
    expect(claudeSetMutation.mutateAsync).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    expect(codexSetMutation.mutateAsync).not.toHaveBeenCalled();
  });

  it("covers toggle branches, null returns, and open directory fallbacks/success paths", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(logToConsole).mockClear();
    vi.mocked(tauriOpenPath).mockClear();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings({
        enable_circuit_breaker_notice: undefined,
        enable_codex_session_id_completion: undefined,
      }),
      isLoading: false,
    } as any);

    const noticeMutation = { isPending: false, mutateAsync: vi.fn() };
    noticeMutation.mutateAsync
      .mockResolvedValueOnce(createAppSettings({ enable_circuit_breaker_notice: undefined }) as any)
      .mockRejectedValueOnce(new Error("notice boom"));
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue(noticeMutation as any);

    const completionMutation = { isPending: false, mutateAsync: vi.fn() };
    completionMutation.mutateAsync
      .mockResolvedValueOnce(
        createAppSettings({ enable_codex_session_id_completion: undefined }) as any
      )
      .mockRejectedValueOnce(new Error("completion boom"));
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue(
      completionMutation as any
    );

    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValueOnce(null);
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: { config_dir: null, found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: { config_dir: "/claude-settings" },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    const codexSetMutation = { isPending: false, mutateAsync: vi.fn() };
    codexSetMutation.mutateAsync.mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue(codexSetMutation as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(tauriOpenPath).mockResolvedValue(true as any);

    renderWithProviders(<CliManagerPage />);

    // circuit notice: enable then disable (catch)
    fireEvent.click(screen.getByRole("button", { name: "persist-circuit-notice" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已开启熔断通知"));
    fireEvent.click(screen.getByRole("button", { name: "disable-circuit-notice" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新熔断通知配置失败：请稍后重试"));

    // codex completion: enable then disable (catch)
    fireEvent.click(screen.getByRole("button", { name: "enable-codex-completion" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已开启 Codex Session ID 补全"));
    fireEvent.click(screen.getByRole("button", { name: "persist-codex-completion" }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("更新 Codex Session ID 补全配置失败：请稍后重试")
    );

    // common settings: null -> tauri-only toast
    fireEvent.click(screen.getByRole("button", { name: "persist-common" }));
    await waitFor(() => expect(commonMutation.mutateAsync).toHaveBeenCalledTimes(1));

    // open dirs success + claude dir fallback (claudeInfo.config_dir=null -> use claudeSettings.config_dir)
    fireEvent.click(screen.getByRole("tab", { name: "Claude Code" }));
    await screen.findByText("claude-tab");
    fireEvent.click(screen.getByRole("button", { name: "open-claude-dir" }));
    await waitFor(() => expect(tauriOpenPath).toHaveBeenCalledWith("/claude-settings"));

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");
    fireEvent.click(screen.getByRole("button", { name: "open-codex-dir" }));
    await waitFor(() => expect(tauriOpenPath).toHaveBeenCalledWith("/codex"));

    // persist codex config: error -> formatted toast branch (no known code)
    fireEvent.click(screen.getByRole("button", { name: "save-codex" }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(expect.stringContaining("更新 Codex 配置失败"))
    );
  });

  it("rebinds enabled codex proxy and invalidates status cache after saving codex_home", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(cliProxyStatusAll).mockReset();
    vi.mocked(cliProxyRebindCodexHome).mockReset();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValue(
      createAppSettings({
        codex_home_mode: "custom",
        codex_home_override: "D:\\Work\\CodexHome",
      })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: codexConfigRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(cliProxyStatusAll).mockResolvedValue([
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:40002",
        applied_to_current_gateway: true,
      },
    ] as any);
    vi.mocked(cliProxyRebindCodexHome).mockResolvedValue({
      cli_key: "codex",
      enabled: true,
      ok: true,
      trace_id: "trace-1",
      error_code: null,
      base_origin: "http://127.0.0.1:40002",
      message: "已重绑 Codex 目录并写入当前网关配置",
    } as any);

    const { client } = renderWithProviders(<CliManagerPage />);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");

    fireEvent.click(screen.getByRole("button", { name: "save-codex-home" }));

    await waitFor(() =>
      expect(commonMutation.mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          codexHomeMode: "custom",
          codexHomeOverride: "D:\\Work\\CodexHome",
        })
      )
    );
    await waitFor(() => expect(cliProxyStatusAll).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(cliProxyRebindCodexHome).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(codexConfigRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(codexTomlRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(codexInfoRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(client.getQueryState(cliProxyKeys.statusAll())?.isInvalidated).toBe(true)
    );
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith(
        "info",
        "Codex Home 已切换，已触发 Codex 代理重绑",
        expect.objectContaining({
          trace_id: "trace-1",
          base_origin: "http://127.0.0.1:40002",
        })
      )
    );
  });

  it("does not rebind codex proxy when codex proxy is disabled after saving codex_home", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(cliProxyStatusAll).mockReset();
    vi.mocked(cliProxyRebindCodexHome).mockReset();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValue(
      createAppSettings({
        codex_home_mode: "custom",
        codex_home_override: "D:\\Work\\CodexHome",
      })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: codexConfigRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(cliProxyStatusAll).mockResolvedValue([
      {
        cli_key: "codex",
        enabled: false,
        base_origin: "http://127.0.0.1:40002",
        applied_to_current_gateway: false,
      },
    ] as any);
    const { client } = renderWithProviders(<CliManagerPage />);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");

    fireEvent.click(screen.getByRole("button", { name: "save-codex-home" }));

    await waitFor(() =>
      expect(commonMutation.mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          codexHomeMode: "custom",
          codexHomeOverride: "D:\\Work\\CodexHome",
        })
      )
    );
    await waitFor(() => expect(cliProxyStatusAll).toHaveBeenCalledTimes(1));
    expect(cliProxyRebindCodexHome).not.toHaveBeenCalled();
    await waitFor(() => expect(codexConfigRefetch).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(codexTomlRefetch).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(codexInfoRefetch).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(client.getQueryState(cliProxyKeys.statusAll())?.isInvalidated).toBe(true)
    );
  });

  it("still attempts codex proxy rebind when cli proxy status read fails after saving codex_home", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(cliProxyStatusAll).mockReset();
    vi.mocked(cliProxyRebindCodexHome).mockReset();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValue(
      createAppSettings({
        codex_home_mode: "custom",
        codex_home_override: "D:\\Work\\CodexHome",
      })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: codexConfigRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(cliProxyStatusAll).mockRejectedValue(new Error("status failed"));
    vi.mocked(cliProxyRebindCodexHome).mockResolvedValue({
      cli_key: "codex",
      enabled: true,
      ok: true,
      trace_id: "trace-fallback",
      error_code: null,
      base_origin: "http://127.0.0.1:40002",
      message: "已重绑 Codex 目录并写入当前网关配置",
    } as any);

    const { client } = renderWithProviders(<CliManagerPage />);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");

    fireEvent.click(screen.getByRole("button", { name: "save-codex-home" }));

    await waitFor(() =>
      expect(commonMutation.mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          codexHomeMode: "custom",
          codexHomeOverride: "D:\\Work\\CodexHome",
        })
      )
    );
    await waitFor(() => expect(cliProxyRebindCodexHome).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "读取 CLI 代理状态失败，将直接尝试重绑 Codex 代理",
        expect.objectContaining({ error: expect.stringContaining("status failed") })
      )
    );
    await waitFor(() => expect(codexConfigRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(codexTomlRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(codexInfoRefetch).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(client.getQueryState(cliProxyKeys.statusAll())?.isInvalidated).toBe(true)
    );
  });

  it("toasts and invalidates status cache when codex proxy rebind returns non-ok", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(cliProxyStatusAll).mockReset();
    vi.mocked(cliProxyRebindCodexHome).mockReset();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValue(
      createAppSettings({
        codex_home_mode: "custom",
        codex_home_override: "D:\\Work\\CodexHome",
      })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: codexConfigRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(cliProxyStatusAll).mockResolvedValue([
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:40002",
        applied_to_current_gateway: false,
      },
    ] as any);
    vi.mocked(cliProxyRebindCodexHome).mockResolvedValue({
      cli_key: "codex",
      enabled: true,
      ok: false,
      trace_id: "trace-non-ok",
      error_code: "CLI_PROXY_REBIND_APPLY_FAILED",
      base_origin: "http://127.0.0.1:40002",
      message: "rebind failed",
    } as any);

    const { client } = renderWithProviders(<CliManagerPage />);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-codex-home" }));

    await waitFor(() => expect(cliProxyRebindCodexHome).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        "Codex 目录已切换，但代理重绑失败；可稍后在首页点击“修复”重试"
      )
    );
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "Codex Home 已切换，但 Codex 代理重绑失败",
        expect.objectContaining({
          error_code: "CLI_PROXY_REBIND_APPLY_FAILED",
          message: "rebind failed",
        })
      )
    );
    await waitFor(() =>
      expect(client.getQueryState(cliProxyKeys.statusAll())?.isInvalidated).toBe(true)
    );
  });

  it("toasts and invalidates status cache when codex proxy rebind throws", async () => {
    vi.mocked(toast).mockClear();
    vi.mocked(cliProxyStatusAll).mockReset();
    vi.mocked(cliProxyRebindCodexHome).mockReset();

    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createAppSettings(),
      isLoading: false,
    } as any);
    vi.mocked(useSettingsGatewayRectifierSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCircuitBreakerNoticeSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useSettingsCodexSessionIdCompletionSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const commonMutation = { isPending: false, mutateAsync: vi.fn() };
    commonMutation.mutateAsync.mockResolvedValue(
      createAppSettings({
        codex_home_mode: "custom",
        codex_home_override: "D:\\Work\\CodexHome",
      })
    );
    vi.mocked(useSettingsSetMutation).mockReturnValue(commonMutation as any);

    vi.mocked(useCliManagerClaudeInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useCliManagerClaudeSettingsSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);

    const codexInfoRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexConfigRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexTomlRefetch = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useCliManagerCodexInfoQuery).mockReturnValue({
      data: { found: true },
      isFetching: false,
      refetch: codexInfoRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigQuery).mockReturnValue({
      data: { config_dir: "/codex", can_open_config_dir: true },
      isFetching: false,
      refetch: codexConfigRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlQuery).mockReturnValue({
      data: { config_path: "/codex/config.toml", exists: true, toml: "" },
      isFetching: false,
      refetch: codexTomlRefetch,
    } as any);
    vi.mocked(useCliManagerCodexConfigTomlSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useCliManagerGeminiInfoQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(cliProxyStatusAll).mockResolvedValue([
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:40002",
        applied_to_current_gateway: false,
      },
    ] as any);
    vi.mocked(cliProxyRebindCodexHome).mockRejectedValue(new Error("rebind boom"));

    const { client } = renderWithProviders(<CliManagerPage />);
    client.setQueryData(cliProxyKeys.statusAll(), [
      {
        cli_key: "codex",
        enabled: true,
        base_origin: "http://127.0.0.1:37123",
        applied_to_current_gateway: false,
      },
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    await screen.findByText("codex-tab");
    fireEvent.click(screen.getByRole("button", { name: "save-codex-home" }));

    await waitFor(() => expect(cliProxyRebindCodexHome).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        "Codex 目录已切换，但代理重绑失败；可稍后在首页点击“修复”重试"
      )
    );
    await waitFor(() =>
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "Codex Home 已切换，但 Codex 代理重绑失败",
        expect.objectContaining({
          error: "Error: rebind boom",
        })
      )
    );
    await waitFor(() =>
      expect(client.getQueryState(cliProxyKeys.statusAll())?.isInvalidated).toBe(true)
    );
  });
});
