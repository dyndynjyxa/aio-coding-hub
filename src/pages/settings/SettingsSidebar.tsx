import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
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
import { configExport, configImport, type ConfigBundle } from "../../services/configMigrate";
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

type AvailableStatus = "checking" | "available" | "unavailable";

export type SettingsSidebarProps = {
  updateMeta: UpdateMeta;
};

function isStringRecord(value: unknown): value is Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }

  return Object.values(value).every((entry) => typeof entry === "string");
}

function isConfigBundleShape(value: unknown): value is ConfigBundle {
  if (!value || typeof value !== "object") {
    return false;
  }

  const bundle = value as Record<string, unknown>;
  if (
    typeof bundle.schema_version !== "number" ||
    typeof bundle.exported_at !== "string" ||
    typeof bundle.app_version !== "string" ||
    typeof bundle.settings !== "string" ||
    !Array.isArray(bundle.providers) ||
    !Array.isArray(bundle.sort_modes) ||
    !isStringRecord(bundle.sort_mode_active) ||
    !Array.isArray(bundle.workspaces) ||
    !Array.isArray(bundle.mcp_servers) ||
    !Array.isArray(bundle.skill_repos)
  ) {
    return false;
  }

  if (bundle.schema_version >= 2) {
    return Array.isArray(bundle.installed_skills) && Array.isArray(bundle.local_skills);
  }

  return (
    (bundle.installed_skills === undefined || Array.isArray(bundle.installed_skills)) &&
    (bundle.local_skills === undefined || Array.isArray(bundle.local_skills))
  );
}

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
  const [exportingConfig, setExportingConfig] = useState(false);
  const [configImportDialogOpen, setConfigImportDialogOpen] = useState(false);
  const [importingConfig, setImportingConfig] = useState(false);
  const [pendingConfigBundle, setPendingConfigBundle] = useState<ConfigBundle | null>(null);

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

  async function exportConfig() {
    if (exportingConfig) return;
    setExportingConfig(true);

    try {
      const bundle = await configExport();
      if (!bundle) {
        return;
      }

      const target = await saveDialog({
        title: "导出配置",
        defaultPath: "aio-coding-hub-config-export.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (!target) {
        return;
      }

      const filePath = Array.isArray(target) ? (target[0] ?? null) : target;
      if (!filePath) {
        return;
      }

      await writeTextFile(filePath, JSON.stringify(bundle, null, 2));
      toast("配置已导出");
    } catch (err) {
      logToConsole("error", "导出配置失败", { error: String(err) });
      toast(`导出配置失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setExportingConfig(false);
    }
  }

  async function openConfigImport() {
    try {
      const selected = await openDialog({
        multiple: false,
        title: "选择配置文件",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (!selected) {
        return;
      }

      const filePath = Array.isArray(selected) ? (selected[0] ?? null) : selected;
      if (!filePath) {
        return;
      }

      const raw = await readTextFile(filePath);
      const parsed = JSON.parse(raw);
      if (!isConfigBundleShape(parsed)) {
        toast("无效的配置文件格式");
        return;
      }

      setPendingConfigBundle(parsed);
      setConfigImportDialogOpen(true);
    } catch (err) {
      logToConsole("error", "读取配置导入文件失败", { error: String(err) });
      toast("读取配置导入文件失败：请检查 JSON 文件格式");
    }
  }

  async function confirmConfigImport() {
    if (importingConfig || !pendingConfigBundle) return;
    setImportingConfig(true);

    try {
      const result = await configImport(pendingConfigBundle);
      if (!result) {
        return;
      }

      await queryClient.invalidateQueries();
      setConfigImportDialogOpen(false);
      setPendingConfigBundle(null);
      toast(
        `配置导入完成：供应商 ${result.providers_imported}，排序模式 ${result.sort_modes_imported}，工作区 ${result.workspaces_imported}，提示词 ${result.prompts_imported}，MCP ${result.mcp_servers_imported}，技能仓库 ${result.skill_repos_imported}，通用技能 ${result.installed_skills_imported}，本机技能 ${result.local_skills_imported}`
      );
    } catch (err) {
      logToConsole("error", "导入配置失败", { error: String(err) });
      toast("导入配置失败：请稍后重试");
    } finally {
      setImportingConfig(false);
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
        <SettingsAboutCard
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
          onExportConfig={exportConfig}
          onImportConfig={() => void openConfigImport()}
          exportingConfig={exportingConfig}
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
        configImportDialogOpen={configImportDialogOpen}
        setConfigImportDialogOpen={(open) => {
          setConfigImportDialogOpen(open);
          if (!open) {
            setPendingConfigBundle(null);
          }
        }}
        importingConfig={importingConfig}
        setImportingConfig={setImportingConfig}
        pendingConfigBundle={pendingConfigBundle}
        confirmConfigImport={confirmConfigImport}
      />
    </>
  );
}
