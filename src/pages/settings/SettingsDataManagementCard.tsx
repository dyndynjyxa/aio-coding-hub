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
        <SettingsRow label="导出配置" subtitle="导出所有供应商、工作区、提示词、MCP 服务器等配置">
          <span className="text-xs text-amber-700 dark:text-amber-400">
            ⚠️ 导出文件包含 API Key 等敏感信息，请妥善保管
          </span>
          <Button
            onClick={() => void onExportConfig()}
            variant="secondary"
            size="sm"
            disabled={!about || exportingConfig}
          >
            {exportingConfig ? "导出中…" : "导出配置"}
          </Button>
        </SettingsRow>
        <SettingsRow label="导入配置" subtitle="从导出文件恢复所有配置（将覆盖当前配置）">
          <Button onClick={onImportConfig} variant="warning" size="sm" disabled={!about}>
            导入配置
          </Button>
        </SettingsRow>
      </div>
    </Card>
  );
}
