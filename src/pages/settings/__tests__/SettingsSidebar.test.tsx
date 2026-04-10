import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { createTestQueryClient } from "../../../test/utils/reactQuery";
import { SettingsSidebar } from "../SettingsSidebar";
import {
  useModelPricesSyncBasellmMutation,
  useModelPricesTotalCountQuery,
} from "../../../query/modelPrices";
import { useUsageSummaryQuery } from "../../../query/usage";
import { useDbDiskUsageQuery, useRequestLogsClearAllMutation } from "../../../query/dataManagement";
import { appDataDirGet, appDataReset, appExit } from "../../../services/app/dataManagement";
import { runBackgroundTask } from "../../../services/backgroundTasks";
import { logToConsole } from "../../../services/consoleLog";
import {
  tauriDialogOpen,
  tauriOpenPath,
  tauriOpenUrl,
  tauriReadTextFile,
} from "../../../test/mocks/tauri";
import { notifyModelPricesUpdated } from "../../../services/usage/modelPrices";
import { modelPricesKeys } from "../../../query/keys";

const devPreviewRef = vi.hoisted(() => ({
  current: { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() } as any,
}));

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../hooks/useDevPreviewData", () => ({
  useDevPreviewData: () => devPreviewRef.current,
}));
vi.mock("../../../services/backgroundTasks", async () => {
  const actual = await vi.importActual<typeof import("../../../services/backgroundTasks")>(
    "../../../services/backgroundTasks"
  );
  return {
    ...actual,
    runBackgroundTask: vi.fn(),
  };
});

vi.mock("../../../services/app/dataManagement", async () => {
  const actual = await vi.importActual<typeof import("../../../services/app/dataManagement")>(
    "../../../services/app/dataManagement"
  );
  return {
    ...actual,
    appDataDirGet: vi.fn(),
    appDataReset: vi.fn(),
    appExit: vi.fn(),
  };
});

vi.mock("../../../query/modelPrices", async () => {
  const actual = await vi.importActual<typeof import("../../../query/modelPrices")>(
    "../../../query/modelPrices"
  );
  return {
    ...actual,
    useModelPricesTotalCountQuery: vi.fn(),
    useModelPricesSyncBasellmMutation: vi.fn(),
  };
});

vi.mock("../../../query/usage", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/usage")>("../../../query/usage");
  return { ...actual, useUsageSummaryQuery: vi.fn() };
});

vi.mock("../../../query/dataManagement", async () => {
  const actual = await vi.importActual<typeof import("../../../query/dataManagement")>(
    "../../../query/dataManagement"
  );
  return {
    ...actual,
    useDbDiskUsageQuery: vi.fn(),
    useRequestLogsClearAllMutation: vi.fn(),
  };
});

vi.mock("../SettingsAboutCard", () => ({
  SettingsAboutCard: ({ about, checkUpdate, checkingUpdate }: any) => (
    <div>
      <div>about:{about?.run_mode ?? "none"}</div>
      <div>checking:{String(checkingUpdate)}</div>
      <button type="button" onClick={() => checkUpdate()}>
        check-update
      </button>
    </div>
  ),
}));

vi.mock("../SettingsDataManagementCard", () => ({
  SettingsDataManagementCard: ({
    openAppDataDir,
    refreshDbDiskUsage,
    openClearRequestLogsDialog,
    openResetAllDialog,
    onImportConfig,
  }: any) => (
    <div>
      <button type="button" onClick={() => openAppDataDir()}>
        open-data-dir
      </button>
      <button type="button" onClick={() => refreshDbDiskUsage()}>
        refresh-db
      </button>
      <button type="button" onClick={() => openClearRequestLogsDialog()}>
        open-clear-logs
      </button>
      <button type="button" onClick={() => openResetAllDialog()}>
        open-reset-all
      </button>
      <button type="button" onClick={() => onImportConfig()}>
        import-config
      </button>
    </div>
  ),
}));

vi.mock("../SettingsDataSyncCard", () => ({
  SettingsDataSyncCard: ({ syncModelPrices, openModelPriceAliasesDialog }: any) => (
    <div>
      <button type="button" onClick={() => syncModelPrices(false)}>
        sync-model-prices
      </button>
      <button type="button" onClick={() => syncModelPrices(true)}>
        sync-model-prices-force
      </button>
      <button type="button" onClick={() => openModelPriceAliasesDialog()}>
        open-aliases
      </button>
    </div>
  ),
}));

vi.mock("../SettingsDialogs", () => ({
  SettingsDialogs: ({
    clearRequestLogsDialogOpen,
    resetAllDialogOpen,
    configImportDialogOpen,
    clearRequestLogs,
    resetAllData,
  }: any) => (
    <div>
      <div>clearOpen:{String(clearRequestLogsDialogOpen)}</div>
      <div>resetOpen:{String(resetAllDialogOpen)}</div>
      <div>configImportOpen:{String(configImportDialogOpen)}</div>
      <button type="button" onClick={() => clearRequestLogs()}>
        confirm-clear-logs
      </button>
      <button type="button" onClick={() => resetAllData()}>
        confirm-reset-all
      </button>
    </div>
  ),
}));

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  const invalidateQueries = vi.spyOn(client, "invalidateQueries");
  return {
    client,
    invalidateQueries,
    ...render(
      <QueryClientProvider client={client}>
        <MemoryRouter>{element}</MemoryRouter>
      </QueryClientProvider>
    ),
  };
}

function createUpdateMeta(overrides: Partial<any> = {}) {
  return {
    about: null,
    updateCandidate: null,
    checkingUpdate: false,
    dialogOpen: false,
    installingUpdate: false,
    installError: null,
    installTotalBytes: null,
    installDownloadedBytes: 0,
    ...overrides,
  };
}

function createConfigImportBundle(overrides: Record<string, unknown> = {}) {
  return {
    schema_version: 2,
    exported_at: "2026-03-29T00:00:00.000Z",
    app_version: "0.0.0-test",
    settings: "{}",
    providers: [],
    sort_modes: [],
    sort_mode_active: {},
    workspaces: [],
    mcp_servers: [],
    skill_repos: [],
    installed_skills: [],
    local_skills: [],
    ...overrides,
  };
}

function mockSidebarQueries() {
  vi.mocked(useModelPricesTotalCountQuery).mockReturnValue({ data: 1, isLoading: false } as any);
  vi.mocked(useModelPricesSyncBasellmMutation).mockReturnValue({
    isPending: false,
    mutateAsync: vi.fn(),
  } as any);
  vi.mocked(useUsageSummaryQuery).mockReturnValue({
    data: { requests_total: 1 },
    isLoading: false,
  } as any);
  vi.mocked(useDbDiskUsageQuery).mockReturnValue({
    data: null,
    isLoading: false,
    refetch: vi.fn(),
  } as any);
  vi.mocked(useRequestLogsClearAllMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
}

describe("pages/settings/SettingsSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    devPreviewRef.current = { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() };
  });

  it("handles update checks (no about, portable, normal)", async () => {
    vi.mocked(useModelPricesTotalCountQuery).mockReturnValue({ data: 3, isLoading: false } as any);
    vi.mocked(useModelPricesSyncBasellmMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useUsageSummaryQuery).mockReturnValue({
      data: { requests_total: 1 },
      isLoading: false,
    } as any);
    vi.mocked(useDbDiskUsageQuery).mockReturnValue({
      data: null,
      isLoading: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useRequestLogsClearAllMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);

    const { rerender } = renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "check-update" }));

    vi.mocked(tauriOpenUrl).mockResolvedValueOnce(undefined as any);
    rerender(
      <QueryClientProvider client={createTestQueryClient()}>
        <MemoryRouter>
          <SettingsSidebar updateMeta={createUpdateMeta({ about: { run_mode: "portable" } })} />
        </MemoryRouter>
      </QueryClientProvider>
    );

    fireEvent.click(screen.getByRole("button", { name: "check-update" }));
    expect(toast).toHaveBeenCalledWith("portable 模式请手动下载");
    await waitFor(() => expect(tauriOpenUrl).toHaveBeenCalled());

    rerender(
      <QueryClientProvider client={createTestQueryClient()}>
        <MemoryRouter>
          <SettingsSidebar updateMeta={createUpdateMeta({ about: { run_mode: "desktop" } })} />
        </MemoryRouter>
      </QueryClientProvider>
    );

    fireEvent.click(screen.getByRole("button", { name: "check-update" }));
    expect(runBackgroundTask).toHaveBeenCalledWith("app-update-check", { trigger: "manual" });
  });

  it("runs local update preview even when about.run_mode is portable", async () => {
    mockSidebarQueries();
    devPreviewRef.current = { enabled: true, setEnabled: vi.fn(), toggle: vi.fn() };

    renderWithProviders(
      <SettingsSidebar updateMeta={createUpdateMeta({ about: { run_mode: "portable" } })} />
    );

    fireEvent.click(screen.getByRole("button", { name: "check-update" }));

    await waitFor(() => {
      expect(runBackgroundTask).toHaveBeenCalledWith("app-update-check", { trigger: "manual" });
    });
    expect(toast).not.toHaveBeenCalledWith("portable 模式请手动下载");
  });

  it("handles data management, model price sync, and subscription invalidation", async () => {
    vi.useFakeTimers();
    vi.mocked(useModelPricesTotalCountQuery).mockReturnValue({ data: 0, isLoading: false } as any);

    const syncMutation = { isPending: false, mutateAsync: vi.fn() };
    syncMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({
        status: "not_modified",
        inserted: 0,
        updated: 0,
        skipped: 0,
        total: 0,
      })
      .mockResolvedValueOnce({ status: "updated", inserted: 1, updated: 2, skipped: 3, total: 6 })
      .mockRejectedValueOnce(new Error("sync boom"));
    vi.mocked(useModelPricesSyncBasellmMutation).mockReturnValue(syncMutation as any);

    vi.mocked(useUsageSummaryQuery).mockReturnValue({ data: null, isLoading: false } as any);

    const refetchDb = vi.fn().mockResolvedValue({ data: {} });
    vi.mocked(useDbDiskUsageQuery).mockReturnValue({
      data: { total_bytes: 123 },
      isLoading: false,
      refetch: refetchDb,
    } as any);

    const clearMutation = { mutateAsync: vi.fn() };
    clearMutation.mutateAsync
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ request_logs_deleted: 1, request_attempt_logs_deleted: 2 })
      .mockRejectedValueOnce(new Error("clear boom"));
    vi.mocked(useRequestLogsClearAllMutation).mockReturnValue(clearMutation as any);

    vi.mocked(appDataDirGet).mockResolvedValueOnce(null).mockResolvedValueOnce("/tmp/app-data");
    vi.mocked(tauriOpenPath)
      .mockRejectedValueOnce(new Error("open boom"))
      .mockResolvedValueOnce(undefined as any);

    vi.mocked(appDataReset)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(true)
      .mockRejectedValueOnce(new Error("reset boom"));
    vi.mocked(appExit).mockResolvedValue(true as any);

    const client = createTestQueryClient();
    const invalidateQueries = vi.spyOn(client, "invalidateQueries");

    render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <SettingsSidebar updateMeta={createUpdateMeta({ about: { run_mode: "desktop" } })} />
        </MemoryRouter>
      </QueryClientProvider>
    );

    // open app data dir: null -> no-op, then openPath error branch
    fireEvent.click(screen.getByRole("button", { name: "open-data-dir" }));
    await act(async () => {
      await Promise.resolve();
    });

    fireEvent.click(screen.getByRole("button", { name: "open-data-dir" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(logToConsole).toHaveBeenCalledWith("error", "打开数据目录失败", {
      error: "Error: open boom",
    });

    // refresh db usage
    fireEvent.click(screen.getByRole("button", { name: "refresh-db" }));
    expect(refetchDb).toHaveBeenCalled();

    // clear request logs: open dialog flag then confirm (null -> toast; then ok; then error)
    fireEvent.click(screen.getByRole("button", { name: "open-clear-logs" }));
    expect(screen.getByText("clearOpen:true")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm-clear-logs" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(clearMutation.mutateAsync).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "confirm-clear-logs" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(clearMutation.mutateAsync).toHaveBeenCalledTimes(2);
    expect(toast).toHaveBeenCalledWith(
      "已清理请求日志：request_logs 1 条；legacy request_attempt_logs 2 条"
    );

    fireEvent.click(screen.getByRole("button", { name: "confirm-clear-logs" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(clearMutation.mutateAsync).toHaveBeenCalledTimes(3);
    expect(toast).toHaveBeenCalledWith("清理请求日志失败：请稍后重试");

    // reset all: null -> toast; ok -> schedules exit; error -> toast
    fireEvent.click(screen.getByRole("button", { name: "open-reset-all" }));
    expect(screen.getByText("resetOpen:true")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm-reset-all" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(appDataReset).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "confirm-reset-all" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(appDataReset).toHaveBeenCalledTimes(2);
    expect(toast).toHaveBeenCalledWith("已清理全部信息：应用即将退出，请重新打开");
    vi.advanceTimersByTime(1000);
    await act(async () => {
      await Promise.resolve();
    });
    expect(appExit).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "confirm-reset-all" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(appDataReset).toHaveBeenCalledTimes(3);
    expect(toast).toHaveBeenCalledWith("清理全部信息失败：请稍后重试");

    // model prices sync: null / not_modified / updated / error
    fireEvent.click(screen.getByRole("button", { name: "sync-model-prices" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(syncMutation.mutateAsync).toHaveBeenCalledWith({ force: false });

    fireEvent.click(screen.getByRole("button", { name: "sync-model-prices" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(syncMutation.mutateAsync).toHaveBeenCalledTimes(2);
    expect(toast).toHaveBeenCalledWith("模型定价已是最新（无变更）");

    fireEvent.click(screen.getByRole("button", { name: "sync-model-prices-force" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(syncMutation.mutateAsync).toHaveBeenCalledWith({ force: true });
    expect(toast).toHaveBeenCalledWith("同步完成：新增 1，更新 2，跳过 3");

    fireEvent.click(screen.getByRole("button", { name: "sync-model-prices-force" }));
    await act(async () => {
      await Promise.resolve();
    });
    expect(syncMutation.mutateAsync).toHaveBeenCalledTimes(4);
    expect(toast).toHaveBeenCalledWith("同步模型定价失败：请稍后重试");

    // subscription invalidation
    notifyModelPricesUpdated();
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: modelPricesKeys.all });
    vi.useRealTimers();
  });

  it("rejects invalid config import JSON before opening confirm dialog", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/invalid-config.json");
    vi.mocked(tauriReadTextFile).mockResolvedValueOnce(
      JSON.stringify({
        schema_version: 1,
        providers: {},
        sort_modes: [],
        workspaces: [],
        mcp_servers: [],
        skill_repos: [],
      }) as any
    );

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriReadTextFile).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("无效的配置文件格式");
    expect(screen.getByText("clearOpen:false")).toBeInTheDocument();
    expect(screen.getByText("resetOpen:false")).toBeInTheDocument();
    expect(screen.getByText("configImportOpen:false")).toBeInTheDocument();
  });

  it("accepts legacy v1 config import without skill payload arrays", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/legacy-config.json");
    vi.mocked(tauriReadTextFile).mockResolvedValueOnce(
      JSON.stringify(
        createConfigImportBundle({
          schema_version: 1,
          installed_skills: undefined,
          local_skills: undefined,
        })
      ) as any
    );

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriReadTextFile).toHaveBeenCalled());
    expect(toast).not.toHaveBeenCalledWith("无效的配置文件格式");
    expect(screen.getByText("configImportOpen:true")).toBeInTheDocument();
  });

  it("rejects v2 config import missing skill payload arrays", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/v2-missing-skills.json");
    vi.mocked(tauriReadTextFile).mockResolvedValueOnce(
      JSON.stringify(
        createConfigImportBundle({
          installed_skills: undefined,
          local_skills: undefined,
        })
      ) as any
    );

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriReadTextFile).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("无效的配置文件格式");
    expect(screen.getByText("configImportOpen:false")).toBeInTheDocument();
  });

  it("rejects config import missing sort_mode_active", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/missing-sort-mode-active.json");
    vi.mocked(tauriReadTextFile).mockResolvedValueOnce(
      JSON.stringify(createConfigImportBundle({ sort_mode_active: undefined })) as any
    );

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriReadTextFile).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("无效的配置文件格式");
    expect(screen.getByText("configImportOpen:false")).toBeInTheDocument();
  });
});
