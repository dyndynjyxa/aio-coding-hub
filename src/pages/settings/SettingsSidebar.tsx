import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type { UpdateMeta } from "../../hooks/useUpdateMeta";
import { AIO_RELEASES_URL } from "../../constants/urls";
import { runBackgroundTask } from "../../services/backgroundTasks";
import { logToConsole } from "../../services/consoleLog";
import {
  getLastModelPricesSync,
  setLastModelPricesSync,
  subscribeModelPricesUpdated,
  type ModelPricesSyncReport,
} from "../../services/modelPrices";
import {
  useModelPricesSyncBasellmMutation,
  useModelPricesTotalCountQuery,
  isModelPricesSyncNotModified,
} from "../../query/modelPrices";
import { modelPricesKeys } from "../../query/keys";
import { useUsageSummaryQuery } from "../../query/usage";
import { appDataDirGet, appDataReset, appExit } from "../../services/dataManagement";
import { useDbDiskUsageQuery, useRequestLogsClearAllMutation } from "../../query/dataManagement";
import { SettingsAboutCard } from "./SettingsAboutCard";
import { SettingsDataManagementCard } from "./SettingsDataManagementCard";
import { SettingsDataSyncCard } from "./SettingsDataSyncCard";
import { SettingsDialogs } from "./SettingsDialogs";
import { SettingsUpdateCard } from "./SettingsUpdateCard";

type AvailableStatus = "checking" | "available" | "unavailable";

export type SettingsSidebarProps = {
  updateMeta: UpdateMeta;
};

export function SettingsSidebar({ updateMeta }: SettingsSidebarProps) {
  const about = updateMeta.about;

  const queryClient = useQueryClient();

  const modelPricesCountQuery = useModelPricesTotalCountQuery();
  const modelPricesSyncMutation = useModelPricesSyncBasellmMutation();

  const todaySummaryQuery = useUsageSummaryQuery("today", { cliKey: null });

  const dbDiskUsageQuery = useDbDiskUsageQuery();
  const clearRequestLogsMutation = useRequestLogsClearAllMutation();

  const initialSync = getLastModelPricesSync();
  const [lastModelPricesSyncReport, setLastModelPricesSyncReport] =
    useState<ModelPricesSyncReport | null>(initialSync.report);
  const [lastModelPricesSyncTime, setLastModelPricesSyncTime] = useState<number | null>(
    initialSync.syncedAt
  );
  const [lastModelPricesSyncError, setLastModelPricesSyncError] = useState<string | null>(null);
  const [modelPriceAliasesDialogOpen, setModelPriceAliasesDialogOpen] = useState(false);

  const syncingModelPrices = modelPricesSyncMutation.isPending;

  const modelPricesCount = modelPricesCountQuery.data ?? null;
  const modelPricesAvailable: AvailableStatus = modelPricesCountQuery.isLoading
    ? "checking"
    : modelPricesCount != null
      ? "available"
      : "unavailable";

  const todayRequestsTotal = todaySummaryQuery.data?.requests_total ?? null;
  const todayRequestsAvailable: AvailableStatus = todaySummaryQuery.isLoading
    ? "checking"
    : todaySummaryQuery.data
      ? "available"
      : "unavailable";

  const dbDiskUsage = dbDiskUsageQuery.data ?? null;
  const dbDiskUsageAvailable: AvailableStatus = dbDiskUsageQuery.isLoading
    ? "checking"
    : dbDiskUsage != null
      ? "available"
      : "unavailable";

  const [clearRequestLogsDialogOpen, setClearRequestLogsDialogOpen] = useState(false);
  const [clearingRequestLogs, setClearingRequestLogs] = useState(false);
  const [resetAllDialogOpen, setResetAllDialogOpen] = useState(false);
  const [resettingAll, setResettingAll] = useState(false);

  async function openUpdateLog() {
    const url = AIO_RELEASES_URL;

    try {
      await openUrl(url);
    } catch (err) {
      logToConsole("error", "打开更新日志失败", { error: String(err), url });
      toast("打开更新日志失败");
    }
  }

  async function checkUpdate() {
    try {
      if (!about) {
        return;
      }

      if (about.run_mode === "portable") {
        toast("portable 模式请手动下载");
        await openUpdateLog();
        return;
      }

      await runBackgroundTask("app-update-check", {
        trigger: "manual",
      });
    } catch {
      // noop: errors/toasts are handled in the registered update task
    }
  }

  async function openAppDataDir() {
    try {
      const dir = await appDataDirGet();
      if (!dir) {
        return;
      }
      await openPath(dir);
    } catch (err) {
      logToConsole("error", "打开数据目录失败", { error: String(err) });
      toast("打开数据目录失败：请查看控制台日志");
    }
  }

  const refreshDbDiskUsage = useCallback(async () => {
    await dbDiskUsageQuery.refetch();
  }, [dbDiskUsageQuery]);

  async function clearRequestLogs() {
    if (clearingRequestLogs) return;
    setClearingRequestLogs(true);

    try {
      const result = await clearRequestLogsMutation.mutateAsync();
      if (!result) {
        return;
      }

      toast(
        `已清理请求日志：request_logs ${result.request_logs_deleted} 条；legacy request_attempt_logs ${result.request_attempt_logs_deleted} 条`
      );
      logToConsole("info", "清理请求日志", result);
      setClearRequestLogsDialogOpen(false);
    } catch (err) {
      logToConsole("error", "清理请求日志失败", { error: String(err) });
      toast("清理请求日志失败：请稍后重试");
    } finally {
      setClearingRequestLogs(false);
    }
  }

  async function resetAllData() {
    if (resettingAll) return;
    setResettingAll(true);

    try {
      const ok = await appDataReset();
      if (!ok) {
        return;
      }

      logToConsole("info", "清理全部信息", { ok: true });
      toast("已清理全部信息：应用即将退出，请重新打开");
      setResetAllDialogOpen(false);

      window.setTimeout(() => {
        appExit().catch(() => {});
      }, 1000);
    } catch (err) {
      logToConsole("error", "清理全部信息失败", { error: String(err) });
      toast("清理全部信息失败：请稍后重试");
    } finally {
      setResettingAll(false);
    }
  }

  useEffect(() => {
    return subscribeModelPricesUpdated(() => {
      queryClient.invalidateQueries({ queryKey: modelPricesKeys.all });
      const latest = getLastModelPricesSync();
      setLastModelPricesSyncReport(latest.report);
      setLastModelPricesSyncTime(latest.syncedAt);
    });
  }, [queryClient]);

  async function syncModelPrices(force: boolean) {
    if (syncingModelPrices) return;
    setLastModelPricesSyncError(null);

    try {
      const report = await modelPricesSyncMutation.mutateAsync({ force });
      if (!report) {
        return;
      }

      setLastModelPricesSync(report);
      setLastModelPricesSyncReport(report);
      setLastModelPricesSyncTime(Date.now());

      if (isModelPricesSyncNotModified(report)) {
        toast("模型定价已是最新（无变更）");
        return;
      }

      toast(`同步完成：新增 ${report.inserted}，更新 ${report.updated}，跳过 ${report.skipped}`);
    } catch (err) {
      logToConsole("error", "同步模型定价失败", { error: String(err) });
      toast("同步模型定价失败：请稍后重试");
      setLastModelPricesSyncError(String(err));
    }
  }

  return (
    <>
      <div className="space-y-6 lg:col-span-4">
        <SettingsAboutCard about={about} />

        <SettingsUpdateCard
          about={about}
          checkingUpdate={updateMeta.checkingUpdate}
          checkUpdate={checkUpdate}
        />

        <SettingsDataManagementCard
          about={about}
          dbDiskUsageAvailable={dbDiskUsageAvailable}
          dbDiskUsage={dbDiskUsage}
          refreshDbDiskUsage={refreshDbDiskUsage}
          openAppDataDir={openAppDataDir}
          openClearRequestLogsDialog={() => setClearRequestLogsDialogOpen(true)}
          openResetAllDialog={() => setResetAllDialogOpen(true)}
        />

        <SettingsDataSyncCard
          about={about}
          modelPricesAvailable={modelPricesAvailable}
          modelPricesCount={modelPricesCount}
          lastModelPricesSyncError={lastModelPricesSyncError}
          lastModelPricesSyncReport={lastModelPricesSyncReport}
          lastModelPricesSyncTime={lastModelPricesSyncTime}
          openModelPriceAliasesDialog={() => setModelPriceAliasesDialogOpen(true)}
          todayRequestsAvailable={todayRequestsAvailable}
          todayRequestsTotal={todayRequestsTotal}
          syncingModelPrices={syncingModelPrices}
          syncModelPrices={syncModelPrices}
        />
      </div>

      <SettingsDialogs
        modelPriceAliasesDialogOpen={modelPriceAliasesDialogOpen}
        setModelPriceAliasesDialogOpen={setModelPriceAliasesDialogOpen}
        clearRequestLogsDialogOpen={clearRequestLogsDialogOpen}
        setClearRequestLogsDialogOpen={setClearRequestLogsDialogOpen}
        clearingRequestLogs={clearingRequestLogs}
        setClearingRequestLogs={setClearingRequestLogs}
        clearRequestLogs={clearRequestLogs}
        resetAllDialogOpen={resetAllDialogOpen}
        setResetAllDialogOpen={setResetAllDialogOpen}
        resettingAll={resettingAll}
        setResettingAll={setResettingAll}
        resetAllData={resetAllData}
      />
    </>
  );
}
