import type { AppAboutInfo } from "../../services/appAbout";
import type { DbDiskUsage } from "../../services/dataManagement";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { SettingsRow } from "../../ui/SettingsRow";
import { formatBytes } from "../../utils/formatters";

type AvailableStatus = "checking" | "available" | "unavailable";

export function SettingsDataManagementCard({
  about,
  dbDiskUsageAvailable,
  dbDiskUsage,
  refreshDbDiskUsage,
  openAppDataDir,
  openClearRequestLogsDialog,
  openResetAllDialog,
  onExportConfig,
  onImportConfig,
  exportingConfig,
}: {
  about: AppAboutInfo | null;
  dbDiskUsageAvailable: AvailableStatus;
  dbDiskUsage: DbDiskUsage | null;
  refreshDbDiskUsage: () => Promise<void>;
  openAppDataDir: () => Promise<void>;
  openClearRequestLogsDialog: () => void;
  openResetAllDialog: () => void;
  onExportConfig: () => Promise<void>;
  onImportConfig: () => void;
  exportingConfig: boolean;
}) {
  return (
    <Card>
      <div className="mb-4 flex items-center justify-between gap-2">
        <div className="font-semibold text-slate-900 dark:text-slate-100">数据管理</div>
        <Button
          onClick={() => void openAppDataDir()}
          variant="secondary"
          size="sm"
          disabled={!about}
        >
          打开数据/日志目录
        </Button>
      </div>
      <div className="divide-y divide-slate-100 dark:divide-slate-700">
        <SettingsRow label="数据磁盘占用">
          <span className="font-mono text-sm text-slate-900 dark:text-slate-100">
            {dbDiskUsageAvailable === "checking"
              ? "加载中…"
              : dbDiskUsageAvailable === "unavailable"
                ? "—"
                : formatBytes(dbDiskUsage?.total_bytes ?? 0)}
          </span>
          <Button
            onClick={() => refreshDbDiskUsage().catch(() => {})}
            variant="secondary"
            size="sm"
            disabled={!about || dbDiskUsageAvailable === "checking"}
          >
            刷新
          </Button>
        </SettingsRow>
        <SettingsRow label="清理请求日志">
          <span className="text-xs text-slate-500 dark:text-slate-400">不可撤销</span>
          <Button
            onClick={openClearRequestLogsDialog}
            variant="warning"
            size="sm"
            disabled={!about}
          >
            清理
          </Button>
        </SettingsRow>
        <SettingsRow label="清理全部信息">
          <span className="text-xs text-rose-700">不可撤销</span>
          <Button onClick={openResetAllDialog} variant="danger" size="sm" disabled={!about}>
            清理
          </Button>
        </SettingsRow>
        <div className="flex flex-col gap-3 py-3 sm:flex-row sm:items-start sm:justify-between sm:gap-6">
          <div className="min-w-0 sm:flex-1 sm:pr-2">
            <div className="text-sm text-slate-700 dark:text-slate-300">导出配置</div>
            <div className="mt-1 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
              导出所有供应商、工作区、提示词、MCP 服务器等配置
            </div>
            <div className="mt-2 inline-flex max-w-full self-start items-center justify-start whitespace-nowrap text-left rounded-full bg-amber-50 px-2.5 py-1 text-[11px] text-amber-800 dark:bg-amber-500/10 dark:text-amber-300 sm:px-3 sm:text-xs">
              包含 API Key 等敏感信息，请妥善保管
            </div>
          </div>
          <div className="flex justify-end sm:flex-none sm:self-start">
            <Button
              onClick={() => void onExportConfig()}
              variant="secondary"
              size="sm"
              className="whitespace-nowrap"
              disabled={!about || exportingConfig}
            >
              {exportingConfig ? "导出中…" : "导出配置"}
            </Button>
          </div>
        </div>
        <SettingsRow label="导入配置" subtitle="从导出文件恢复所有配置（将覆盖当前配置）">
          <Button onClick={onImportConfig} variant="warning" size="sm" disabled={!about}>
            导入配置
          </Button>
        </SettingsRow>
      </div>
    </Card>
  );
}
