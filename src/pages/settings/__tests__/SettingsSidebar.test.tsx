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
import {
  useConfigExportMutation,
  useConfigImportMutation,
} from "../../../query/configMigrate";
import { useUsageSummaryQuery } from "../../../query/usage";
import { useDbDiskUsageQuery, useRequestLogsClearAllMutation } from "../../../query/dataManagement";
import { appDataDirGet, appDataReset, appExit } from "../../../services/app/dataManagement";
import { runBackgroundTask } from "../../../services/backgroundTasks";
import { logToConsole } from "../../../services/consoleLog";
import { tauriDialogOpen, tauriOpenPath, tauriOpenUrl } from "../../../test/mocks/tauri";
import { notifyModelPricesUpdated } from "../../../services/usage/modelPrices";
import { modelPricesKeys } from "../../../query/keys";

const devPreviewRef = vi.hoisted(() => ({
  current: { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() } as any,
}));
const configImportMutationRef = vi.hoisted(() => ({
  current: { isPending: false, mutateAsync: vi.fn() } as any,
}));
const configExportMutationRef = vi.hoisted(() => ({
  current: { isPending: false, mutateAsync: vi.fn() } as any,
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
vi.mock("../../../query/configMigrate", async () => {
  const actual = await vi.importActual<typeof import("../../../query/configMigrate")>(
    "../../../query/configMigrate"
  );
  return {
    ...actual,
    useConfigExportMutation: vi.fn(() => configExportMutationRef.current),
    useConfigImportMutation: vi.fn(() => configImportMutationRef.current),
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
    clearRequestLogs,
    resetAll,
    configImport,
  }: any) => (
    <div>
      <div>clearOpen:{String(clearRequestLogs.open)}</div>
      <div>resetOpen:{String(resetAll.open)}</div>
      <div>configImportOpen:{String(configImport.open)}</div>
      <div>configImportPath:{configImport.pendingFilePath ?? "none"}</div>
      <button type="button" onClick={() => clearRequestLogs.confirm()}>
        confirm-clear-logs
      </button>
      <button type="button" onClick={() => resetAll.confirm()}>
        confirm-reset-all
      </button>
      <button type="button" onClick={() => configImport.confirm()}>
        confirm-config-import
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
  vi.mocked(useConfigExportMutation).mockReturnValue(configExportMutationRef.current);
  vi.mocked(useConfigImportMutation).mockReturnValue(configImportMutationRef.current);
}

describe("pages/settings/SettingsSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    devPreviewRef.current = { enabled: false, setEnabled: vi.fn(), toggle: vi.fn() };
    configExportMutationRef.current = { isPending: false, mutateAsync: vi.fn() };
    configImportMutationRef.current = { isPending: false, mutateAsync: vi.fn() };
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

    vi.mocked(appDataDirGet).mockResolvedValueOnce(null as any).mockResolvedValueOnce("/tmp/app-data");
    vi.mocked(tauriOpenPath)
      .mockRejectedValueOnce(new Error("open boom"))
      .mockResolvedValueOnce(undefined as any);

    vi.mocked(appDataReset)
      .mockResolvedValueOnce(null as any)
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

  it("opens config import confirm dialog after selecting a file", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/import-config.json");

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriDialogOpen).toHaveBeenCalled());
    expect(screen.getByText("clearOpen:false")).toBeInTheDocument();
    expect(screen.getByText("resetOpen:false")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("configImportOpen:true")).toBeInTheDocument());
    expect(screen.getByText("configImportPath:/tmp/import-config.json")).toBeInTheDocument();
    expect(configImportMutationRef.current.mutateAsync).not.toHaveBeenCalled();
  });

  it("keeps config import dialog closed when file selection is cancelled", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce(null as any);

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));

    await waitFor(() => expect(tauriDialogOpen).toHaveBeenCalled());
    expect(screen.getByText("configImportOpen:false")).toBeInTheDocument();
    expect(screen.getByText("configImportPath:none")).toBeInTheDocument();
    expect(configImportMutationRef.current.mutateAsync).not.toHaveBeenCalled();
  });

  it("confirms config import through the shared mutation and closes dialog on success", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/shared-config.json");
    configImportMutationRef.current.mutateAsync.mockResolvedValueOnce({
      providers_imported: 2,
      sort_modes_imported: 1,
      workspaces_imported: 3,
      prompts_imported: 4,
      mcp_servers_imported: 5,
      skill_repos_imported: 6,
      installed_skills_imported: 7,
      local_skills_imported: 8,
    });

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));
    await waitFor(() => expect(screen.getByText("configImportOpen:true")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "confirm-config-import" }));

    await waitFor(() => {
      expect(configImportMutationRef.current.mutateAsync).toHaveBeenCalledWith({
        filePath: "/tmp/shared-config.json",
      });
    });
    expect(toast).toHaveBeenCalledWith(
      "配置导入完成：供应商 2，排序模式 1，工作区 3，提示词 4，MCP 5，技能仓库 6，通用技能 7，本机技能 8"
    );
    expect(screen.getByText("configImportOpen:false")).toBeInTheDocument();
    expect(screen.getByText("configImportPath:none")).toBeInTheDocument();
  });

  it("keeps config import dialog open when the shared mutation fails", async () => {
    mockSidebarQueries();

    vi.mocked(tauriDialogOpen).mockResolvedValueOnce("/tmp/broken-config.json");
    configImportMutationRef.current.mutateAsync.mockRejectedValueOnce(new Error("invalid config"));

    renderWithProviders(<SettingsSidebar updateMeta={createUpdateMeta()} />);

    fireEvent.click(screen.getByRole("button", { name: "import-config" }));
    await waitFor(() => expect(screen.getByText("configImportOpen:true")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "confirm-config-import" }));

    await waitFor(() => {
      expect(configImportMutationRef.current.mutateAsync).toHaveBeenCalledWith({
        filePath: "/tmp/broken-config.json",
      });
    });
    expect(toast).toHaveBeenCalledWith("导入配置失败：请稍后重试");
    expect(screen.getByText("configImportOpen:true")).toBeInTheDocument();
    expect(screen.getByText("configImportPath:/tmp/broken-config.json")).toBeInTheDocument();
  });
});
