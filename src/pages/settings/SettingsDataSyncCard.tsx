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
  lastModelPricesSyncError: string | null;
  lastModelPricesSyncReport: ModelPricesSyncReport | null;
  lastModelPricesSyncTime: number | null;
  openModelPriceAliasesDialog: () => void;
  todayRequestsAvailable: AvailableStatus;
  todayRequestsTotal: number | null;
  syncingModelPrices: boolean;
  syncModelPrices: () => Promise<void>;
}) {
  const syncFailed = lastModelPricesSyncReport?.status === "failed";
  const syncStatus = lastModelPricesSyncError
    ? "同步失败"
    : lastModelPricesSyncReport
      ? syncFailed
        ? "同步失败"
        : lastModelPricesSyncReport.status === "not_modified"
          ? "无变更"
          : `+${lastModelPricesSyncReport.inserted} / ~${lastModelPricesSyncReport.updated} · 共 ${lastModelPricesSyncReport.total} 条`
      : "未同步";
  const syncTimeLabel = lastModelPricesSyncError || syncFailed ? "尝试" : "更新";

  return (
    <Card>
      <div className="mb-4 font-semibold text-foreground">数据与同步</div>
      <div className="divide-y divide-line-subtle">
        <SettingsRow label="模型定价">
          <span
            className={
              lastModelPricesSyncError || syncFailed
                ? "text-xs text-rose-600"
                : "text-xs text-muted-foreground"
            }
          >
            {syncStatus}
          </span>
          {lastModelPricesSyncTime ? (
            <span className="text-xs text-muted-foreground">
              {formatRelativeTime(lastModelPricesSyncTime)} · {syncTimeLabel}
            </span>
          ) : null}
        </SettingsRow>
        <SettingsRow label="定价匹配">
          <span className="text-xs text-muted-foreground">prefix / wildcard / exact</span>
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
          <span className="font-mono text-sm text-foreground">
            {todayRequestsAvailable === "checking"
              ? "加载中…"
              : todayRequestsAvailable === "unavailable"
                ? "—"
                : String(todayRequestsTotal ?? 0)}
          </span>
        </SettingsRow>
        <SettingsRow label="同步定价">
          <Button
            onClick={() => void syncModelPrices()}
            variant="secondary"
            size="sm"
            disabled={syncingModelPrices}
          >
            {syncingModelPrices ? "同步中" : "同步"}
          </Button>
        </SettingsRow>
      </div>
    </Card>
  );
}
