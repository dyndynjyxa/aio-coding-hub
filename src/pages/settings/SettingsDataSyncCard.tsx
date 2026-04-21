import type { AppAboutInfo } from "../../services/app/appAbout";
import type { ModelPricesSyncReport } from "../../services/usage/modelPrices";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { SettingsRow } from "../../ui/SettingsRow";

type AvailableStatus = "checking" | "available" | "unavailable";

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return "刚刚";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  return `${days} 天前`;
}

export function SettingsDataSyncCard({
  about,
  modelPricesAvailable,
  modelPricesCount,
  lastModelPricesSyncError,
  lastModelPricesSyncReport,
  lastModelPricesSyncTime,
  openModelPriceAliasesDialog,
  todayRequestsAvailable,
  todayRequestsTotal,
  syncingModelPrices,
  syncModelPrices,
}: {
  about: AppAboutInfo | null;
  modelPricesAvailable: AvailableStatus;
  modelPricesCount: number | null;
  lastModelPricesSyncError: string | null;
  lastModelPricesSyncReport: ModelPricesSyncReport | null;
  lastModelPricesSyncTime: number | null;
  openModelPriceAliasesDialog: () => void;
  todayRequestsAvailable: AvailableStatus;
  todayRequestsTotal: number | null;
  syncingModelPrices: boolean;
  syncModelPrices: (force: boolean) => Promise<void>;
}) {
  return (
    <Card>
      <div className="mb-4 font-semibold text-slate-900 dark:text-slate-100">数据与同步</div>
      <div className="divide-y divide-slate-100 dark:divide-slate-700">
        <SettingsRow label="模型定价">
          <span className="font-mono text-sm text-slate-900 dark:text-slate-100">
            {modelPricesAvailable === "checking"
              ? "加载中…"
              : modelPricesAvailable === "unavailable"
                ? "—"
                : modelPricesCount === 0
                  ? "未同步"
                  : `${modelPricesCount} 条`}
          </span>
          {lastModelPricesSyncError ? (
            <span className="text-xs text-rose-600">失败</span>
          ) : lastModelPricesSyncReport ? (
            <span className="text-xs text-slate-500 dark:text-slate-400">
              {lastModelPricesSyncReport.status === "not_modified"
                ? "最新"
                : `+${lastModelPricesSyncReport.inserted} / ~${lastModelPricesSyncReport.updated}`}
            </span>
          ) : null}
          {lastModelPricesSyncTime ? (
            <span className="text-xs text-slate-400 dark:text-slate-500">
              {formatRelativeTime(lastModelPricesSyncTime)} 同步
            </span>
          ) : null}
        </SettingsRow>
        <SettingsRow label="定价匹配">
          <span className="text-xs text-slate-500 dark:text-slate-400">
            prefix / wildcard / exact
          </span>
          <Button
            onClick={openModelPriceAliasesDialog}
            variant="secondary"
            size="sm"
            disabled={!about}
          >
            配置
          </Button>
        </SettingsRow>
        <SettingsRow label="今日请求">
          <span className="font-mono text-sm text-slate-900 dark:text-slate-100">
            {todayRequestsAvailable === "checking"
              ? "加载中…"
              : todayRequestsAvailable === "unavailable"
                ? "—"
                : String(todayRequestsTotal ?? 0)}
          </span>
        </SettingsRow>
        <SettingsRow label="同步定价">
          <div className="flex gap-2">
            <Button
              onClick={() => syncModelPrices(false)}
              variant="secondary"
              size="sm"
              disabled={syncingModelPrices}
            >
              {syncingModelPrices ? "同步中" : "同步"}
            </Button>
            <Button
              onClick={() => syncModelPrices(true)}
              variant="secondary"
              size="sm"
              disabled={syncingModelPrices}
            >
              强制
            </Button>
          </div>
        </SettingsRow>
      </div>
    </Card>
  );
}
