import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { mergeSettingsState, resetMswState } from "../../test/msw/state";
import { HomePage } from "../HomePage";
import { logToConsole } from "../../services/consoleLog";
import { envConflictsCheck } from "../../services/envConflicts";
import { gatewayKeys } from "../../query/keys";
import {
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitStatusQuery,
  useGatewaySessionsListQuery,
} from "../../query/gateway";
import { useProvidersListQuery } from "../../query/providers";
import {
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
  useRequestLogsIncrementalPollQuery,
  useRequestLogsListAllQuery,
} from "../../query/requestLogs";
import {
  useSortModeActiveListQuery,
  useSortModeActiveSetMutation,
  useSortModesListQuery,
} from "../../query/sortModes";
import { useUsageHourlySeriesQuery } from "../../query/usage";
import { useProviderLimitUsageV1Query } from "../../query/providerLimitUsage";
import { useCliProxy } from "../../hooks/useCliProxy";
import { useHomeWorkspaceConfigs } from "../home/hooks/useHomeWorkspaceConfigs";
import { emitBackgroundTaskVisibilityTrigger } from "../../services/backgroundTasks";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../services/backgroundTasks", () => ({
  emitBackgroundTaskVisibilityTrigger: vi.fn(),
}));

vi.mock("../../components/home/HomeOverviewPanel", () => ({
  HomeOverviewPanel: ({
    sortModesLoading,
    onSetCliActiveMode,
    onSetCliProxyEnabled,
    onRefreshUsageHeatmap,
    onRefreshRequestLogs,
    onSelectLogId,
    devPreviewEnabled,
    showHomeHeatmap,
    showHomeUsage,
    openCircuits,
    onResetCircuitProvider,
  }: any) => (
    <div>
      <div>sort-loading:{String(sortModesLoading)}</div>
      <div>dev-preview:{String(devPreviewEnabled)}</div>
      <div>show-heatmap:{String(showHomeHeatmap)}</div>
      <div>show-usage:{String(showHomeUsage)}</div>
      <div>open-circuits:{openCircuits.length}</div>
      <button type="button" onClick={() => onResetCircuitProvider(1)}>
        reset-1
      </button>
      <button type="button" onClick={() => onResetCircuitProvider(2)}>
        reset-2
      </button>
      <button type="button" onClick={() => onResetCircuitProvider(3)}>
        reset-3
      </button>
      <button type="button" onClick={() => onSetCliActiveMode("claude", 1)}>
        request-switch-same
      </button>
      <button type="button" onClick={() => onSetCliActiveMode("claude", 2)}>
        request-switch-claude-2
      </button>
      <button type="button" onClick={() => onSetCliActiveMode("codex", 1)}>
        request-switch-codex-1
      </button>
      <button type="button" onClick={() => onRefreshUsageHeatmap()}>
        refresh-heatmap
      </button>
      <button type="button" onClick={() => onRefreshRequestLogs()}>
        refresh-logs
      </button>
      <button type="button" onClick={() => onSetCliProxyEnabled("codex", true)}>
        enable-cli-proxy-codex
      </button>
      <button type="button" onClick={() => onSetCliProxyEnabled("codex", true)}>
        repair-cli-proxy-codex
      </button>
      <button type="button" onClick={() => onSelectLogId(123)}>
        select-log
      </button>
    </div>
  ),
}));

vi.mock("../../components/home/HomeCostPanel", () => ({
  HomeCostPanel: () => <div>cost-panel</div>,
}));

vi.mock("../../components/home/RequestLogDetailDialog", () => ({
  RequestLogDetailDialog: ({ selectedLogId, selectedLogLoading, attemptLogsLoading }: any) => (
    <div>
      <div>selected:{String(selectedLogId)}</div>
      <div>selLoading:{String(selectedLogLoading)}</div>
      <div>attemptLoading:{String(attemptLogsLoading)}</div>
    </div>
  ),
}));

vi.mock("../../hooks/useWindowForeground", () => ({
  useWindowForeground: ({ enabled, onForeground }: any) => {
    if (enabled) onForeground();
  },
}));

vi.mock("../../services/traceStore", () => ({ useTraceStore: () => ({ traces: [] }) }));

vi.mock("../../hooks/useCliProxy", async () => {
  const actual =
    await vi.importActual<typeof import("../../hooks/useCliProxy")>("../../hooks/useCliProxy");
  return { ...actual, useCliProxy: vi.fn() };
});

vi.mock("../home/hooks/useHomeWorkspaceConfigs", () => ({
  useHomeWorkspaceConfigs: vi.fn(),
}));

vi.mock("../../services/envConflicts", async () => {
  const actual = await vi.importActual<typeof import("../../services/envConflicts")>(
    "../../services/envConflicts"
  );
  return { ...actual, envConflictsCheck: vi.fn() };
});

vi.mock("../../query/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../query/gateway")>("../../query/gateway");
  return {
    ...actual,
    useGatewayCircuitResetProviderMutation: vi.fn(),
    useGatewayCircuitStatusQuery: vi.fn(),
    useGatewaySessionsListQuery: vi.fn(),
  };
});

vi.mock("../../query/providers", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/providers")>("../../query/providers");
  return { ...actual, useProvidersListQuery: vi.fn() };
});

vi.mock("../../query/requestLogs", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/requestLogs")>("../../query/requestLogs");
  return {
    ...actual,
    useRequestLogsListAllQuery: vi.fn(),
    useRequestLogsIncrementalPollQuery: vi.fn(),
    useRequestLogDetailQuery: vi.fn(),
    useRequestAttemptLogsByTraceIdQuery: vi.fn(),
  };
});

vi.mock("../../query/sortModes", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/sortModes")>("../../query/sortModes");
  return {
    ...actual,
    useSortModesListQuery: vi.fn(),
    useSortModeActiveListQuery: vi.fn(),
    useSortModeActiveSetMutation: vi.fn(),
  };
});

vi.mock("../../query/usage", async () => {
  const actual = await vi.importActual<typeof import("../../query/usage")>("../../query/usage");
  return { ...actual, useUsageHourlySeriesQuery: vi.fn() };
});

vi.mock("../../query/providerLimitUsage", async () => {
  const actual = await vi.importActual<typeof import("../../query/providerLimitUsage")>(
    "../../query/providerLimitUsage"
  );
  return { ...actual, useProviderLimitUsageV1Query: vi.fn() };
});

function renderWithProviders(client: any, element: ReactElement) {
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

function mockHomePageBaseQueries() {
  vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
    mutateAsync: vi.fn(),
  } as any);
  vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({ data: null } as any);
  vi.mocked(useProvidersListQuery).mockReturnValue({ data: null } as any);

  vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
    data: null,
    isFetching: false,
    refetch: vi.fn(),
  } as any);
  vi.mocked(useGatewaySessionsListQuery).mockReturnValue({ data: null, isLoading: false } as any);
  vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
    data: [],
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
  } as any);
  vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
    data: 0,
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
  } as any);

  vi.mocked(useSortModesListQuery).mockReturnValue({ data: [], isLoading: false } as any);
  vi.mocked(useSortModeActiveListQuery).mockReturnValue({ data: [], isLoading: false } as any);
  vi.mocked(useSortModeActiveSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

  vi.mocked(useRequestLogDetailQuery).mockReturnValue({ data: null, isFetching: false } as any);
  vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
    data: [],
    isFetching: false,
  } as any);

  vi.mocked(useProviderLimitUsageV1Query).mockReturnValue({
    data: null,
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
  } as any);

  vi.mocked(useHomeWorkspaceConfigs).mockReturnValue([
    {
      cliKey: "claude",
      cliLabel: "Claude Code",
      workspaceId: 1,
      workspaceName: "默认",
      loading: false,
      items: [],
    },
    {
      cliKey: "codex",
      cliLabel: "Codex",
      workspaceId: 2,
      workspaceName: "Default",
      loading: false,
      items: [],
    },
    {
      cliKey: "gemini",
      cliLabel: "Gemini",
      workspaceId: 3,
      workspaceName: "工作区 2",
      loading: false,
      items: [],
    },
  ] as any);
}

describe("pages/HomePage", () => {
  beforeEach(() => {
    localStorage.removeItem("devPreview.enabled");
    resetMswState();
    vi.mocked(useProviderLimitUsageV1Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useHomeWorkspaceConfigs).mockReturnValue([
      {
        cliKey: "claude",
        cliLabel: "Claude Code",
        workspaceId: 1,
        workspaceName: "默认",
        loading: false,
        items: [],
      },
      {
        cliKey: "codex",
        cliLabel: "Codex",
        workspaceId: 2,
        workspaceName: "Default",
        loading: false,
        items: [],
      },
      {
        cliKey: "gemini",
        cliLabel: "Gemini",
        workspaceId: 3,
        workspaceName: "工作区 2",
        loading: false,
        items: [],
      },
    ] as any);
  });

  it("covers circuits auto refresh, reset provider, mode switching, and refetch flows", async () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date("2026-02-01T00:00:00Z"));
      setTauriRuntime();

      const client = createTestQueryClient();
      const invalidateSpy = vi.spyOn(client, "invalidateQueries");

      const resetMutation = { mutateAsync: vi.fn() };
      resetMutation.mutateAsync
        .mockResolvedValueOnce(true)
        .mockResolvedValueOnce(false)
        .mockRejectedValueOnce(new Error("reset boom"));
      vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue(resetMutation as any);

      const nowUnix = Math.floor(Date.now() / 1000);
      vi.mocked(useGatewayCircuitStatusQuery).mockImplementation((cliKey: any) => {
        if (cliKey === "claude") {
          return {
            data: [
              { provider_id: 1, state: "OPEN", open_until: nowUnix + 5, cooldown_until: null },
            ],
          } as any;
        }
        if (cliKey === "codex") {
          return {
            data: [
              { provider_id: 2, state: "CLOSED", open_until: null, cooldown_until: nowUnix + 10 },
            ],
          } as any;
        }
        return {
          data: [
            { provider_id: 3, state: "OPEN", open_until: nowUnix + 1, cooldown_until: nowUnix + 2 },
          ],
        } as any;
      });

      vi.mocked(useProvidersListQuery).mockImplementation((cliKey: any) => {
        if (cliKey === "claude") return { data: [{ id: 1, name: " P1 " }] } as any;
        if (cliKey === "codex") return { data: [{ id: 2, name: "" }] } as any;
        return { data: [{ id: 3, name: "P3" }] } as any;
      });

      vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
        data: [],
        isFetching: false,
        refetch: vi.fn().mockResolvedValue({ error: new Error("u") }),
      } as any);

      vi.mocked(useGatewaySessionsListQuery).mockReturnValue({
        data: [{ cli_key: "claude", session_id: "s1" }],
        isLoading: false,
      } as any);

      const requestLogsRefetch = vi.fn().mockResolvedValue({ error: new Error("r") });
      vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
        data: [],
        isLoading: false,
        isFetching: true,
        refetch: requestLogsRefetch,
      } as any);
      vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
        data: 0,
        isLoading: false,
        isFetching: false,
        refetch: vi.fn(),
      } as any);

      vi.mocked(useSortModesListQuery).mockReturnValue({
        data: [
          { id: 1, name: "M1" },
          { id: 2, name: "M2" },
        ],
        isLoading: false,
      } as any);

      vi.mocked(useSortModeActiveListQuery).mockReturnValue({
        data: [
          { cli_key: "claude", mode_id: 1 },
          { cli_key: "codex", mode_id: null },
        ],
        isLoading: false,
      } as any);

      const activeSetMutation = { mutateAsync: vi.fn() };
      activeSetMutation.mutateAsync
        .mockResolvedValueOnce({ cli_key: "codex", mode_id: 1 })
        .mockResolvedValueOnce(null);
      vi.mocked(useSortModeActiveSetMutation).mockReturnValue(activeSetMutation as any);

      vi.mocked(useRequestLogDetailQuery).mockReturnValue({
        data: { trace_id: "t1" },
        isFetching: true,
      } as any);
      vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
        data: [],
        isFetching: true,
      } as any);

      vi.mocked(useCliProxy).mockReturnValue({
        enabled: false,
        appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
        toggling: false,
        setCliProxyEnabled: vi.fn(),
      } as any);

      renderWithProviders(client, <HomePage />);

      // open circuits derived from mocked circuits
      expect(screen.getByText("open-circuits:3")).toBeInTheDocument();

      // auto refresh timer should invalidate circuits after earliest open_until
      vi.advanceTimersByTime(2250);
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuits() });

      // reset provider success / fail / error
      fireEvent.click(screen.getByRole("button", { name: "reset-1" }));
      await Promise.resolve();
      expect(resetMutation.mutateAsync).toHaveBeenCalledWith({ providerId: 1 });
      expect((toast as any).success).toHaveBeenCalledWith("已解除熔断");

      fireEvent.click(screen.getByRole("button", { name: "reset-2" }));
      await Promise.resolve();
      expect(resetMutation.mutateAsync).toHaveBeenCalledWith({ providerId: 2 });
      expect((toast as any).error).toHaveBeenCalledWith("解除熔断失败");

      fireEvent.click(screen.getByRole("button", { name: "reset-3" }));
      await Promise.resolve();
      expect(resetMutation.mutateAsync).toHaveBeenCalledWith({ providerId: 3 });
      expect(logToConsole).toHaveBeenCalledWith("error", "解除熔断失败", {
        providerId: 3,
        error: "Error: reset boom",
      });

      // refresh callbacks (toasts on error)
      fireEvent.click(screen.getByRole("button", { name: "refresh-heatmap" }));
      await Promise.resolve();
      expect(toast).toHaveBeenCalledWith("刷新用量失败：请查看控制台日志");
      fireEvent.click(screen.getByRole("button", { name: "refresh-logs" }));
      await Promise.resolve();
      expect(toast).toHaveBeenCalledWith("读取使用记录失败：请查看控制台日志");

      // same switch is ignored
      fireEvent.click(screen.getByRole("button", { name: "request-switch-same" }));
      expect(activeSetMutation.mutateAsync).not.toHaveBeenCalledWith({
        cliKey: "claude",
        modeId: 1,
      });

      // switch codex directly -> activated toast branch
      fireEvent.click(screen.getByRole("button", { name: "request-switch-codex-1" }));
      await Promise.resolve();
      expect(activeSetMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "codex", modeId: 1 });
      expect(toast).toHaveBeenCalledWith("已激活：M1");

      // switch claude with active sessions -> confirmation dialog
      fireEvent.click(screen.getByRole("button", { name: "request-switch-claude-2" }));
      const dialog = within(screen.getByRole("dialog"));
      fireEvent.click(dialog.getByRole("button", { name: "确认切换" }));
      await Promise.resolve();
      expect(activeSetMutation.mutateAsync).toHaveBeenCalledWith({ cliKey: "claude", modeId: 2 });
      fireEvent.click(screen.getByRole("tab", { name: "花费" }));
      expect(screen.getByRole("tab", { name: "花费" })).toHaveAttribute("aria-selected", "true");
      fireEvent.click(screen.getByRole("tab", { name: "概览" }));
      await Promise.resolve();
      expect(requestLogsRefetch).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not count HALF_OPEN rows as open circuits", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();

    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation((cliKey: any) => {
      if (cliKey === "claude") {
        return {
          data: [
            {
              provider_id: 1,
              state: "HALF_OPEN",
              open_until: null,
              cooldown_until: null,
            },
          ],
        } as any;
      }
      return { data: [] } as any;
    });

    vi.mocked(useProvidersListQuery).mockImplementation((cliKey: any) => {
      if (cliKey === "claude") return { data: [{ id: 1, name: "P1" }] } as any;
      return { data: [] } as any;
    });

    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    expect(screen.getByText("open-circuits:0")).toBeInTheDocument();
  });

  it("prompts env conflicts before enabling CLI proxy", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();
    vi.mocked(envConflictsCheck).mockResolvedValue([
      { var_name: "OPENAI_API_KEY", source_type: "system", source_path: "Process Environment" },
    ]);

    const setCliProxyEnabled = vi.fn();
    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: false, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled,
    } as any);

    renderWithProviders(client, <HomePage />);

    fireEvent.click(screen.getByRole("button", { name: "enable-cli-proxy-codex" }));

    await waitFor(() => expect(envConflictsCheck).toHaveBeenCalledWith("codex"));

    const dialog = await screen.findByRole("dialog");
    expect(setCliProxyEnabled).not.toHaveBeenCalled();
    expect(within(dialog).getByText("OPENAI_API_KEY")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: "继续启用" }));
    expect(setCliProxyEnabled).toHaveBeenCalledWith("codex", true);
  });

  it("emits home overview visible trigger on mount and when returning to overview tab", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();
    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    await waitFor(() =>
      expect(emitBackgroundTaskVisibilityTrigger).toHaveBeenCalledWith("home-overview-visible")
    );

    vi.mocked(emitBackgroundTaskVisibilityTrigger).mockClear();
    fireEvent.click(screen.getByRole("tab", { name: "花费" }));
    fireEvent.click(screen.getByRole("tab", { name: "概览" }));

    await waitFor(() =>
      expect(emitBackgroundTaskVisibilityTrigger).toHaveBeenCalledWith("home-overview-visible")
    );
  });

  it("enables CLI proxy directly when no env conflicts are found", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();
    vi.mocked(envConflictsCheck).mockResolvedValue([]);

    const setCliProxyEnabled = vi.fn();
    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: false, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled,
    } as any);

    renderWithProviders(client, <HomePage />);

    fireEvent.click(screen.getByRole("button", { name: "enable-cli-proxy-codex" }));
    await waitFor(() => expect(setCliProxyEnabled).toHaveBeenCalledWith("codex", true));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("covers null-data branches and the 'more' tab", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({ data: null } as any);
    vi.mocked(useProvidersListQuery).mockReturnValue({ data: null } as any);

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: null,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useGatewaySessionsListQuery).mockReturnValue({ data: null, isLoading: true } as any);
    vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
      data: null,
      isLoading: true,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
      data: 0,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useSortModesListQuery).mockReturnValue({ data: null, isLoading: true } as any);
    vi.mocked(useSortModeActiveListQuery).mockReturnValue({ data: null, isLoading: true } as any);
    vi.mocked(useSortModeActiveSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    vi.mocked(useRequestLogDetailQuery).mockReturnValue({ data: null, isFetching: false } as any);
    vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
      data: null,
      isFetching: false,
    } as any);

    vi.mocked(useCliProxy).mockReturnValue({
      enabled: false,
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: false,
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    expect(screen.getByText("open-circuits:0")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "更多" }));
    expect(screen.getByText("更多功能开发中…")).toBeInTheDocument();
  });

  it("toggles the unified dev preview entry and passes it to overview", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();
    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    expect(screen.getByText("dev-preview:false")).toBeInTheDocument();

    const enableButton = screen.getByRole("button", { name: "Dev开启预览数据" });
    fireEvent.click(enableButton);

    expect(screen.getByRole("button", { name: "Dev关闭预览数据" })).toBeInTheDocument();
    expect(screen.getByText("dev-preview:true")).toBeInTheDocument();
  });

  it("passes homepage heatmap and usage switches to overview", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    mockHomePageBaseQueries();
    vi.mocked(useCliProxy).mockReturnValue({
      enabled: { claude: false, codex: false, gemini: false },
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: { claude: false, codex: false, gemini: false },
      setCliProxyEnabled: vi.fn(),
    } as any);

    mergeSettingsState({ show_home_heatmap: false, show_home_usage: true });

    renderWithProviders(client, <HomePage />);

    await waitFor(() => {
      expect(screen.getByText("show-heatmap:false")).toBeInTheDocument();
      expect(screen.getByText("show-usage:true")).toBeInTheDocument();
    });
  });

  it("covers pending switch dialog cancel/onOpenChange and auto refresh when open_until is null", async () => {
    vi.useFakeTimers();
    setTauriRuntime();

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation((cliKey: any) => {
      if (cliKey === "claude") return { data: null } as any;
      if (cliKey === "codex") {
        return {
          data: [{ provider_id: 9, state: "OPEN", open_until: null, cooldown_until: null }],
        } as any;
      }
      return { data: [] } as any;
    });
    vi.mocked(useProvidersListQuery).mockReturnValue({ data: null } as any);

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useGatewaySessionsListQuery).mockReturnValue({
      data: [{ cli_key: "claude", session_id: "s1" }],
      isLoading: false,
    } as any);

    vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
      data: 0,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useSortModesListQuery).mockReturnValue({
      data: [{ id: 1, name: "M1" }],
      isLoading: false,
    } as any);
    vi.mocked(useSortModeActiveListQuery).mockReturnValue({
      data: [{ cli_key: "claude", mode_id: 1 }],
      isLoading: false,
    } as any);
    vi.mocked(useSortModeActiveSetMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    vi.mocked(useRequestLogDetailQuery).mockReturnValue({ data: null, isFetching: false } as any);
    vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    vi.mocked(useCliProxy).mockReturnValue({
      enabled: false,
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: false,
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    // rows with open_until=null should fall back to 30s auto refresh
    expect(screen.getByText("open-circuits:1")).toBeInTheDocument();
    vi.advanceTimersByTime(30_000);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuits() });

    vi.useRealTimers();

    // open pending dialog and cancel via button
    fireEvent.click(screen.getByRole("button", { name: "request-switch-claude-2" }));
    const dialog = within(screen.getByRole("dialog"));
    fireEvent.click(dialog.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // open again and close by overlay (onOpenChange path)
    fireEvent.click(screen.getByRole("button", { name: "request-switch-claude-2" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("covers switchingCliKey guard when another switch is in-flight", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();

    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({ data: [] } as any);
    vi.mocked(useProvidersListQuery).mockReturnValue({ data: [] } as any);

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useGatewaySessionsListQuery).mockReturnValue({ data: [], isLoading: false } as any);
    vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
      data: 0,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useSortModesListQuery).mockReturnValue({
      data: [
        { id: 1, name: "M1" },
        { id: 2, name: "M2" },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSortModeActiveListQuery).mockReturnValue({
      data: [
        { cli_key: "claude", mode_id: 1 },
        { cli_key: "codex", mode_id: null },
      ],
      isLoading: false,
    } as any);

    let resolveActiveSet: (v: any) => void = () => {
      throw new Error("resolveActiveSet not set");
    };
    const activeSetMutation = {
      mutateAsync: vi.fn().mockImplementationOnce(
        () =>
          new Promise<any>((resolve) => {
            resolveActiveSet = resolve;
          })
      ),
    };
    vi.mocked(useSortModeActiveSetMutation).mockReturnValue(activeSetMutation as any);

    vi.mocked(useRequestLogDetailQuery).mockReturnValue({ data: null, isFetching: false } as any);
    vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    vi.mocked(useCliProxy).mockReturnValue({
      enabled: false,
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: false,
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    // start switching codex and keep promise pending
    fireEvent.click(screen.getByRole("button", { name: "request-switch-codex-1" }));
    await Promise.resolve();

    // switchingCliKey != null => setCliActiveMode early returns for other cli
    fireEvent.click(screen.getByRole("button", { name: "request-switch-claude-2" }));
    expect(activeSetMutation.mutateAsync).toHaveBeenCalledTimes(1);

    resolveActiveSet({ cli_key: "codex", mode_id: null });
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已切回：Default"));
  });

  it("covers setCliActiveMode fallback label and catch branches", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();

    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockReturnValue({ data: [] } as any);
    vi.mocked(useProvidersListQuery).mockReturnValue({ data: [] } as any);

    vi.mocked(useUsageHourlySeriesQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useGatewaySessionsListQuery).mockReturnValue({ data: [], isLoading: false } as any);
    vi.mocked(useRequestLogsListAllQuery).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useRequestLogsIncrementalPollQuery).mockReturnValue({
      data: 0,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useSortModesListQuery).mockReturnValue({
      data: [
        { id: 1, name: "M1" },
        { id: 2, name: "M2" },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSortModeActiveListQuery).mockReturnValue({
      data: [
        { cli_key: "claude", mode_id: 1 },
        { cli_key: "codex", mode_id: null },
      ],
      isLoading: false,
    } as any);

    const activeSetMutation = { mutateAsync: vi.fn() };
    activeSetMutation.mutateAsync
      .mockResolvedValueOnce({ cli_key: "codex", mode_id: 999 })
      .mockRejectedValueOnce(new Error("boom"));
    vi.mocked(useSortModeActiveSetMutation).mockReturnValue(activeSetMutation as any);

    vi.mocked(useRequestLogDetailQuery).mockReturnValue({ data: null, isFetching: false } as any);
    vi.mocked(useRequestAttemptLogsByTraceIdQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    vi.mocked(useCliProxy).mockReturnValue({
      enabled: false,
      appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
      toggling: false,
      setCliProxyEnabled: vi.fn(),
    } as any);

    renderWithProviders(client, <HomePage />);

    fireEvent.click(screen.getByRole("button", { name: "request-switch-codex-1" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("已激活：#999"));

    fireEvent.click(screen.getByRole("button", { name: "request-switch-codex-1" }));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("切换排序模板失败：Error: boom"));
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "切换排序模板失败",
      expect.objectContaining({ cli: "codex", mode_id: 1 })
    );
  });
});
