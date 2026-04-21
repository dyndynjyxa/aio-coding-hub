import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import type { UpdateMeta } from "../../hooks/useUpdateMeta";
import { AIO_RELEASES_URL } from "../../constants/urls";
import { runBackgroundTask } from "../../services/backgroundTasks";
import type { ConfigImportResult } from "../../services/app/configMigrate";
import { appDataDirGet, appDataReset, appExit } from "../../services/app/dataManagement";
import type { ClearRequestLogsResult } from "../../services/app/dataManagement";
import {
  openDesktopSinglePath,
  saveDesktopFilePath,
} from "../../services/desktop/dialog";
import { openDesktopPath, openDesktopUrl } from "../../services/desktop/opener";
import {
  getLastModelPricesSync,
  setLastModelPricesSync,
  type ModelPricesSyncReport,
} from "../../services/usage/modelPrices";
import { useModelPricesUpdatedSubscription } from "../../query/modelPrices";
import {
  presentConfigExported,
  presentConfigImported,
  presentModelPricesSynced,
  presentRequestLogsCleared,
  presentResetAllSuccess,
  presentSettingsSidebarFailure,
} from "./settingsSidebarFeedback";

type SettingsSidebarControllerInput = {
  updateMeta: UpdateMeta;
  devPreviewEnabled: boolean;
  refreshDbDiskUsage: () => Promise<unknown>;
  clearRequestLogsMutation: {
    isPending: boolean;
    mutateAsync: () => Promise<ClearRequestLogsResult | null>;
  };
  configExportMutation: {
    isPending: boolean;
    mutateAsync: (input: { filePath: string }) => Promise<boolean | null>;
  };
  configImportMutation: {
    isPending: boolean;
    mutateAsync: (input: { filePath: string }) => Promise<ConfigImportResult | null>;
  };
  modelPricesSyncMutation: {
    isPending: boolean;
    mutateAsync: (input: { force: boolean }) => Promise<ModelPricesSyncReport | null>;
  };
};

type DialogController = {
  open: boolean;
  setOpen: (open: boolean) => void;
};

type PendingDialogController = DialogController & {
  pending: boolean;
  confirm: () => Promise<void>;
};

type ConfigImportDialogController = PendingDialogController & {
  pendingFilePath: string | null;
};

export function useSettingsSidebarController(input: SettingsSidebarControllerInput) {
  const {
    updateMeta,
    devPreviewEnabled,
    refreshDbDiskUsage,
    clearRequestLogsMutation,
    configExportMutation,
    configImportMutation,
    modelPricesSyncMutation,
  } = input;
  const about = updateMeta.about;

  const [modelPriceAliasesDialogOpen, setModelPriceAliasesDialogOpen] = useState(false);
  const [clearRequestLogsDialogOpen, setClearRequestLogsDialogOpen] = useState(false);
  const [resetAllDialogOpen, setResetAllDialogOpen] = useState(false);
  const [configImportDialogOpen, setConfigImportDialogOpen] = useState(false);
  const [pendingConfigImportPath, setPendingConfigImportPath] = useState<string | null>(null);
  const [resettingAll, setResettingAll] = useState(false);
  const [lastModelPricesSyncState, setLastModelPricesSyncState] = useState(() => {
    const initialSync = getLastModelPricesSync();
    return {
      report: initialSync.report,
      syncedAt: initialSync.syncedAt,
      error: null as string | null,
    };
  });

  useModelPricesUpdatedSubscription(
    useCallback((snapshot: { report: ModelPricesSyncReport | null; syncedAt: number | null }) => {
      setLastModelPricesSyncState({
        report: snapshot.report,
        syncedAt: snapshot.syncedAt,
        error: null,
      });
    }, [])
  );

  const openUpdateLog = useCallback(async () => {
    try {
      await openDesktopUrl(AIO_RELEASES_URL);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "打开更新日志失败",
        toastMessage: "打开更新日志失败",
        error,
        meta: { url: AIO_RELEASES_URL },
      });
    }
  }, []);

  const checkUpdate = useCallback(async () => {
    try {
      if (!about) {
        return;
      }

      if (about.run_mode === "portable" && !devPreviewEnabled) {
        toast("portable 模式请手动下载");
        await openUpdateLog();
        return;
      }

      await runBackgroundTask("app-update-check", {
        trigger: "manual",
      });
    } catch {
      // noop: registered update task already owns failure feedback
    }
  }, [about, devPreviewEnabled, openUpdateLog]);

  const openAppDataDir = useCallback(async () => {
    try {
      const dir = await appDataDirGet();
      if (!dir) {
        return;
      }

      await openDesktopPath(dir);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "打开数据目录失败",
        toastMessage: "打开数据目录失败：请查看控制台日志",
        error,
      });
    }
  }, []);

  const refreshDbDiskUsageAction = useCallback(async () => {
    await refreshDbDiskUsage();
  }, [refreshDbDiskUsage]);

  const openClearRequestLogsDialog = useCallback(() => {
    setClearRequestLogsDialogOpen(true);
  }, []);

  const openResetAllDialog = useCallback(() => {
    setResetAllDialogOpen(true);
  }, []);

  const openModelPriceAliasesDialog = useCallback(() => {
    setModelPriceAliasesDialogOpen(true);
  }, []);

  const clearRequestLogs = useCallback(async () => {
    if (clearRequestLogsMutation.isPending) {
      return;
    }

    try {
      const result = await clearRequestLogsMutation.mutateAsync();
      if (!result) {
        return;
      }

      presentRequestLogsCleared(result);
      setClearRequestLogsDialogOpen(false);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "清理请求日志失败",
        toastMessage: "清理请求日志失败：请稍后重试",
        error,
      });
    }
  }, [clearRequestLogsMutation]);

  const resetAllData = useCallback(async () => {
    if (resettingAll) {
      return;
    }

    setResettingAll(true);

    try {
      const ok = await appDataReset();
      if (!ok) {
        return;
      }

      presentResetAllSuccess();
      setResetAllDialogOpen(false);

      window.setTimeout(() => {
        appExit().catch(() => {});
      }, 1000);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "清理全部信息失败",
        toastMessage: "清理全部信息失败：请稍后重试",
        error,
      });
    } finally {
      setResettingAll(false);
    }
  }, [resettingAll]);

  const exportConfig = useCallback(async () => {
    if (configExportMutation.isPending) {
      return;
    }

    try {
      const filePath = await saveDesktopFilePath({
        title: "导出配置",
        defaultPath: "aio-coding-hub-config-export.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (!filePath) {
        return;
      }

      const ok = await configExportMutation.mutateAsync({ filePath });
      if (!ok) {
        return;
      }

      presentConfigExported();
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "导出配置失败",
        toastMessage: `导出配置失败：${error instanceof Error ? error.message : String(error)}`,
        error,
      });
    }
  }, [configExportMutation]);

  const openConfigImport = useCallback(async () => {
    try {
      const filePath = await openDesktopSinglePath({
        multiple: false,
        title: "选择配置文件",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (!filePath) {
        return;
      }

      setPendingConfigImportPath(filePath);
      setConfigImportDialogOpen(true);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "选择配置导入文件失败",
        toastMessage: "选择配置导入文件失败：请稍后重试",
        error,
      });
    }
  }, []);

  const closeConfigImportDialog = useCallback((open: boolean) => {
    setConfigImportDialogOpen(open);
    if (!open) {
      setPendingConfigImportPath(null);
    }
  }, []);

  const confirmConfigImport = useCallback(async () => {
    if (configImportMutation.isPending || !pendingConfigImportPath) {
      return;
    }

    try {
      const result = await configImportMutation.mutateAsync({
        filePath: pendingConfigImportPath,
      });
      if (!result) {
        return;
      }

      setConfigImportDialogOpen(false);
      setPendingConfigImportPath(null);
      presentConfigImported(result);
    } catch (error) {
      presentSettingsSidebarFailure({
        logTitle: "导入配置失败",
        toastMessage: "导入配置失败：请稍后重试",
        error,
      });
    }
  }, [configImportMutation, pendingConfigImportPath]);

  const syncModelPrices = useCallback(
    async (force: boolean) => {
      if (modelPricesSyncMutation.isPending) {
        return;
      }

      setLastModelPricesSyncState((current) => ({
        ...current,
        error: null,
      }));

      try {
        const report = await modelPricesSyncMutation.mutateAsync({ force });
        if (!report) {
          return;
        }

        setLastModelPricesSync(report);
        setLastModelPricesSyncState({
          report,
          syncedAt: Date.now(),
          error: null,
        });
        presentModelPricesSynced(report);
      } catch (error) {
        presentSettingsSidebarFailure({
          logTitle: "同步模型定价失败",
          toastMessage: "同步模型定价失败：请稍后重试",
          error,
        });
        setLastModelPricesSyncState((current) => ({
          ...current,
          error: String(error),
        }));
      }
    },
    [modelPricesSyncMutation]
  );

  const dialogs = useMemo<{
    modelPriceAliases: DialogController;
    clearRequestLogs: PendingDialogController;
    resetAll: PendingDialogController;
    configImport: ConfigImportDialogController;
  }>(
    () => ({
      modelPriceAliases: {
        open: modelPriceAliasesDialogOpen,
        setOpen: setModelPriceAliasesDialogOpen,
      },
      clearRequestLogs: {
        open: clearRequestLogsDialogOpen,
        setOpen: setClearRequestLogsDialogOpen,
        pending: clearRequestLogsMutation.isPending,
        confirm: clearRequestLogs,
      },
      resetAll: {
        open: resetAllDialogOpen,
        setOpen: setResetAllDialogOpen,
        pending: resettingAll,
        confirm: resetAllData,
      },
      configImport: {
        open: configImportDialogOpen,
        setOpen: closeConfigImportDialog,
        pending: configImportMutation.isPending,
        confirm: confirmConfigImport,
        pendingFilePath: pendingConfigImportPath,
      },
    }),
    [
      clearRequestLogs,
      clearRequestLogsDialogOpen,
      clearRequestLogsMutation.isPending,
      closeConfigImportDialog,
      configImportDialogOpen,
      configImportMutation.isPending,
      confirmConfigImport,
      modelPriceAliasesDialogOpen,
      pendingConfigImportPath,
      resetAllData,
      resetAllDialogOpen,
      resettingAll,
    ]
  );

  return {
    checkUpdate,
    openAppDataDir,
    refreshDbDiskUsage: refreshDbDiskUsageAction,
    openClearRequestLogsDialog,
    openResetAllDialog,
    exportConfig,
    openConfigImport,
    syncModelPrices,
    openModelPriceAliasesDialog,
    lastModelPricesSyncReport: lastModelPricesSyncState.report,
    lastModelPricesSyncTime: lastModelPricesSyncState.syncedAt,
    lastModelPricesSyncError: lastModelPricesSyncState.error,
    syncingModelPrices: modelPricesSyncMutation.isPending,
    exportingConfig: configExportMutation.isPending,
    dialogs,
  };
}
