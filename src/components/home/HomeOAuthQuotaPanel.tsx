import { AlertTriangle, RefreshCw } from "lucide-react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { useNowUnix } from "../../hooks/useNowUnix";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import {
  hasHomeOAuthQuotaText,
  hasInsufficientHomeOAuthQuota,
  type HomeOAuthQuotaRow,
} from "./homeOAuthQuotaTypes";

export type HomeOAuthQuotaPanelContentProps = {
  rows: HomeOAuthQuotaRow[];
  hasProviders: boolean;
  hasRefreshed: boolean;
  refreshing: boolean;
  onRefresh?: () => void;
  onRefreshRow?: (providerId: number) => void;
};

type HomeOAuthQuotaPanelProps = HomeOAuthQuotaPanelContentProps & {
  onRefresh: () => void;
};

function formatCompactResetText(resetAt: number | null, nowUnix: number): string | null {
  if (resetAt == null) return null;
  const remaining = resetAt - nowUnix;
  if (remaining <= 0) return "已重置";
  const totalMinutes = Math.floor(remaining / 60);
  if (totalMinutes < 1) return "<1m";

  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;

  if (days > 0) {
    const dayPart = `${days}d`;
    const hourPart = hours > 0 ? `${hours}h` : "";
    const minutePart = minutes > 0 ? `${minutes}m` : "";
    return `${dayPart}${hourPart}${minutePart}`;
  }

  if (hours > 0) {
    return minutes > 0 ? `${hours}h${minutes}m` : `${hours}h`;
  }

  return `${minutes}m`;
}

function buildQuotaSegment(
  label: string,
  quotaText: string | null,
  resetText: string | null
): string | null {
  if (!quotaText) return null;
  return resetText ? `${label}: ${quotaText}·${resetText}` : `${label}: ${quotaText}`;
}

function OAuthQuotaProviderCard({
  row,
  onRefreshRow,
}: {
  row: HomeOAuthQuotaRow;
  onRefreshRow?: (providerId: number) => void;
}) {
  const shouldTrackNowUnix =
    row.state === "success" &&
    (row.limits?.limit_5h_reset_at != null || row.limits?.limit_weekly_reset_at != null);
  const nowUnix = useNowUnix(shouldTrackNowUnix);
  const shortLabel = row.limits?.limit_short_label || "短窗";
  const reset5h = formatCompactResetText(row.limits?.limit_5h_reset_at ?? null, nowUnix);
  const resetWeekly = formatCompactResetText(row.limits?.limit_weekly_reset_at ?? null, nowUnix);
  const quotaSummary = [
    buildQuotaSegment(shortLabel, row.limits?.limit_5h_text ?? null, reset5h),
    buildQuotaSegment("7d", row.limits?.limit_weekly_text ?? null, resetWeekly),
  ]
    .filter((segment): segment is string => Boolean(segment))
    .join(" / ");
  const showInsufficientQuota =
    row.state === "success" && hasInsufficientHomeOAuthQuota(row.limits);

  return (
    <div className="rounded-lg border border-border bg-white px-3 py-2.5 shadow-sm dark:border-border dark:bg-secondary">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2 text-xs text-secondary-foreground">
            <span
              className={cn(
                "shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium",
                cliBadgeTone(row.cliKey)
              )}
            >
              {cliShortLabel(row.cliKey)}
            </span>
            <span className="truncate font-medium">{row.providerName}</span>
          </div>

          <div className="flex items-center gap-1">
            {!row.enabled ? (
              <span className="rounded-full bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground dark:bg-secondary dark:text-muted-foreground">
                已禁用
              </span>
            ) : null}
            {row.state === "error" ? (
              <span className="rounded-full bg-rose-50 px-1.5 py-0.5 text-[10px] text-rose-700 dark:bg-rose-900/30 dark:text-rose-400">
                刷新失败
              </span>
            ) : null}
            {onRefreshRow ? (
              <button
                type="button"
                onClick={() => onRefreshRow(row.providerId)}
                disabled={row.state === "loading"}
                className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-secondary dark:hover:text-indigo-400"
                title={`刷新 ${row.providerName} OAuth 配额`}
                aria-label={`刷新 ${row.providerName} OAuth 配额`}
              >
                <RefreshCw
                  className={cn("h-3.5 w-3.5", row.state === "loading" && "animate-spin")}
                />
              </button>
            ) : null}
          </div>
        </div>

        {row.state === "loading" ? (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Spinner size="sm" />
            刷新中...
          </div>
        ) : row.state === "error" ? (
          <div className="flex items-start gap-2 text-xs text-rose-600 dark:text-rose-400">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <span>刷新失败，请重试</span>
          </div>
        ) : row.state === "idle" ? (
          <div className="text-xs text-muted-foreground">点击右上角刷新获取 OAuth 配额</div>
        ) : !hasHomeOAuthQuotaText(row.limits) ? (
          <div className="text-xs text-muted-foreground">暂无 OAuth 配额信息</div>
        ) : (
          <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
            <span className="min-w-0 truncate font-mono" title={quotaSummary}>
              {quotaSummary}
            </span>
            {showInsufficientQuota ? (
              <span className="shrink-0 rounded-full bg-amber-50 px-1.5 py-0.5 text-[10px] text-amber-700 dark:bg-amber-900/30 dark:text-amber-300">
                配额不足
              </span>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}

export function HomeOAuthQuotaPanelContent({
  rows,
  hasProviders,
  hasRefreshed,
  refreshing,
  onRefresh,
  onRefreshRow,
}: HomeOAuthQuotaPanelContentProps) {
  const showNoQuotaNotice =
    hasRefreshed &&
    rows.length > 0 &&
    rows.every((row) => row.state === "success" && !hasHomeOAuthQuotaText(row.limits));

  if (!hasProviders) {
    return <EmptyState title="当前没有 OAuth 供应商" />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-center justify-between shrink-0">
        <span className="text-xs text-muted-foreground">{rows.length} 个 OAuth 供应商</span>
        {onRefresh ? (
          <button
            type="button"
            onClick={onRefresh}
            disabled={refreshing}
            className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-secondary dark:hover:text-indigo-400"
            title="刷新 OAuth 配额"
            aria-label="刷新 OAuth 配额"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
          </button>
        ) : null}
      </div>

      <>
        {showNoQuotaNotice ? (
          <div className="rounded-lg border border-dashed border-border bg-secondary/80 px-3 py-2 text-xs text-muted-foreground dark:border-border dark:bg-secondary/40 dark:text-muted-foreground">
            当前暂无 OAuth 配额信息
          </div>
        ) : null}

        <div className="min-h-0 flex-1 space-y-2 overflow-auto pr-1 scrollbar-overlay">
          {rows.map((row) => (
            <OAuthQuotaProviderCard
              key={`${row.cliKey}:${row.providerId}`}
              row={row}
              onRefreshRow={onRefreshRow}
            />
          ))}
        </div>
      </>
    </div>
  );
}

export function HomeOAuthQuotaPanel({
  rows,
  hasProviders,
  hasRefreshed,
  refreshing,
  onRefresh,
  onRefreshRow,
}: HomeOAuthQuotaPanelProps) {
  return (
    <Card padding="sm" className="flex h-full flex-col">
      <div className="mb-3 text-sm font-semibold">OAuth 配额</div>
      <HomeOAuthQuotaPanelContent
        rows={rows}
        hasProviders={hasProviders}
        hasRefreshed={hasRefreshed}
        refreshing={refreshing}
        onRefresh={onRefresh}
        onRefreshRow={onRefreshRow}
      />
    </Card>
  );
}
