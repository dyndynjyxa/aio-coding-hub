import { useDevPreviewData } from "../../hooks/useDevPreviewData";
import type { UpdateMeta } from "../../hooks/useUpdateMeta";
import { useConfigExportMutation, useConfigImportMutation } from "../../query/configMigrate";
import { useDbDiskUsageQuery, useRequestLogsClearAllMutation } from "../../query/dataManagement";
import { useModelPricesSyncBasellmMutation, useModelPricesTotalCountQuery } from "../../query/modelPrices";
import { useUsageSummaryQuery } from "../../query/usage";
import { resolveAvailableStatus } from "./settingsSidebarModel";
import { useSettingsSidebarController } from "./useSettingsSidebarController";

export function useSettingsSidebar(updateMeta: UpdateMeta) {
  const about = updateMeta.about;
  const devPreview = useDevPreviewData();

  const modelPricesCountQuery = useModelPricesTotalCountQuery();
  const modelPricesSyncMutation = useModelPricesSyncBasellmMutation();
  const configExportMutation = useConfigExportMutation();
  const configImportMutation = useConfigImportMutation();
  const todaySummaryQuery = useUsageSummaryQuery("today", { cliKey: null });
  const dbDiskUsageQuery = useDbDiskUsageQuery();
  const clearRequestLogsMutation = useRequestLogsClearAllMutation();

  const controller = useSettingsSidebarController({
    updateMeta,
    devPreviewEnabled: devPreview.enabled,
    refreshDbDiskUsage: dbDiskUsageQuery.refetch,
    clearRequestLogsMutation: {
      isPending: clearRequestLogsMutation.isPending,
      mutateAsync: clearRequestLogsMutation.mutateAsync,
    },
    configExportMutation: {
      isPending: configExportMutation.isPending,
      mutateAsync: configExportMutation.mutateAsync,
    },
    configImportMutation: {
      isPending: configImportMutation.isPending,
      mutateAsync: configImportMutation.mutateAsync,
    },
    modelPricesSyncMutation: {
      isPending: modelPricesSyncMutation.isPending,
      mutateAsync: modelPricesSyncMutation.mutateAsync,
    },
  });

  const modelPricesCount = modelPricesCountQuery.data ?? null;
  const todayRequestsTotal = todaySummaryQuery.data?.requests_total ?? null;
  const dbDiskUsage = dbDiskUsageQuery.data ?? null;

  return {
    aboutCardProps: {
      about,
      checkingUpdate: updateMeta.checkingUpdate,
      checkUpdate: controller.checkUpdate,
    },
    dataManagementCardProps: {
      about,
      dbDiskUsageAvailable: resolveAvailableStatus(dbDiskUsage, dbDiskUsageQuery.isLoading),
      dbDiskUsage,
      refreshDbDiskUsage: controller.refreshDbDiskUsage,
      openAppDataDir: controller.openAppDataDir,
      openClearRequestLogsDialog: controller.openClearRequestLogsDialog,
      openResetAllDialog: controller.openResetAllDialog,
      onExportConfig: controller.exportConfig,
      onImportConfig: controller.openConfigImport,
      exportingConfig: controller.exportingConfig,
    },
    dataSyncCardProps: {
      about,
      modelPricesAvailable: resolveAvailableStatus(
        modelPricesCount,
        modelPricesCountQuery.isLoading
      ),
      modelPricesCount,
      lastModelPricesSyncError: controller.lastModelPricesSyncError,
      lastModelPricesSyncReport: controller.lastModelPricesSyncReport,
      lastModelPricesSyncTime: controller.lastModelPricesSyncTime,
      openModelPriceAliasesDialog: controller.openModelPriceAliasesDialog,
      todayRequestsAvailable: resolveAvailableStatus(
        todaySummaryQuery.data ?? null,
        todaySummaryQuery.isLoading
      ),
      todayRequestsTotal,
      syncingModelPrices: controller.syncingModelPrices,
      syncModelPrices: controller.syncModelPrices,
    },
    dialogsProps: controller.dialogs,
  };
}
