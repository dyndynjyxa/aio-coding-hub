// Usage:
// - Rendered in `HomeOverviewPanel` when the "供应商限额" tab is selected.
// - Displays providers with configured limits and their current usage.
// - Use `HomeProviderLimitPanelContent` for inline rendering without Card wrapper.

import { useMemo } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import type { ProviderLimitUsageRow } from "../../services/providers/providerLimitUsage";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { formatPercent, formatUsdRaw } from "../../utils/formatters";
import { AlertTriangle, RefreshCw } from "lucide-react";

/** Format a unix timestamp (seconds) to a short local date-time string like "1/27 14:00" */
function formatWindowTs(ts: number): string {
  const d = new Date(ts * 1000);
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Calculate window end timestamp based on window type */
function getWindowEndTs(startTs: number, windowType: string): number {
  switch (windowType) {
    case "5h":
      return startTs + 5 * 60 * 60;
    case "24h":
    case "Daily":
      return startTs + 24 * 60 * 60;
    case "Weekly":
      return startTs + 7 * 24 * 60 * 60;
    case "Monthly": {
      // Calculate first day of next month
      const d = new Date(startTs * 1000);
      const nextMonth = new Date(d.getFullYear(), d.getMonth() + 1, 1, 0, 0, 0);
      return Math.floor(nextMonth.getTime() / 1000);
    }
    default:
      return startTs;
  }
}

export type HomeProviderLimitPanelProps = {
  rows: ProviderLimitUsageRow[];
  loading: boolean;
  available: boolean | null;
  onRefresh: () => void;
  refreshing: boolean;
};

type LimitDisplay = {
  label: string;
  limit: number;
  usage: number;
  percent: number;
  warning: boolean;
  windowStartTs: number | null; // unix seconds, null for "Total"
};

function getLimitDisplays(row: ProviderLimitUsageRow): LimitDisplay[] {
  const displays: LimitDisplay[] = [];

  if (row.limit_5h_usd != null) {
    const percent = row.limit_5h_usd > 0 ? row.usage_5h_usd / row.limit_5h_usd : 0;
    displays.push({
      label: "5h",
      limit: row.limit_5h_usd,
      usage: row.usage_5h_usd,
      percent,
      warning: percent >= 0.8,
      windowStartTs: row.window_5h_start_ts,
    });
  }

  if (row.limit_daily_usd != null) {
    const percent = row.limit_daily_usd > 0 ? row.usage_daily_usd / row.limit_daily_usd : 0;
    const modeLabel = row.daily_reset_mode === "rolling" ? "24h" : "Daily";
    displays.push({
      label: modeLabel,
      limit: row.limit_daily_usd,
      usage: row.usage_daily_usd,
      percent,
      warning: percent >= 0.8,
      windowStartTs: row.window_daily_start_ts,
    });
  }

  if (row.limit_weekly_usd != null) {
    const percent = row.limit_weekly_usd > 0 ? row.usage_weekly_usd / row.limit_weekly_usd : 0;
    displays.push({
      label: "Weekly",
      limit: row.limit_weekly_usd,
      usage: row.usage_weekly_usd,
      percent,
      warning: percent >= 0.8,
      windowStartTs: row.window_weekly_start_ts,
    });
  }

  if (row.limit_monthly_usd != null) {
    const percent = row.limit_monthly_usd > 0 ? row.usage_monthly_usd / row.limit_monthly_usd : 0;
    displays.push({
      label: "Monthly",
      limit: row.limit_monthly_usd,
      usage: row.usage_monthly_usd,
      percent,
      warning: percent >= 0.8,
      windowStartTs: row.window_monthly_start_ts,
    });
  }

  if (row.limit_total_usd != null) {
    const percent = row.limit_total_usd > 0 ? row.usage_total_usd / row.limit_total_usd : 0;
    displays.push({
      label: "Total",
      limit: row.limit_total_usd,
      usage: row.usage_total_usd,
      percent,
      warning: percent >= 0.8,
      windowStartTs: null, // Total has no window start
    });
  }

  return displays;
}

function ProgressBar({ percent, warning }: { percent: number; warning: boolean }) {
  const width = Math.min(100, Math.max(0, percent * 100));
  return (
    <div className="h-1.5 w-full rounded-full bg-slate-200 dark:bg-slate-700 overflow-hidden">
      <div
        className={cn(
          "h-full rounded-full transition-all duration-300",
          warning ? "bg-amber-500" : "bg-indigo-500"
        )}
        style={{ width: `${width}%` }}
      />
    </div>
  );
}

function LimitItem({ display }: { display: LimitDisplay }) {
  const windowLabel =
    display.windowStartTs != null
      ? `${formatWindowTs(display.windowStartTs)} → ${formatWindowTs(getWindowEndTs(display.windowStartTs, display.label))}`
      : null;

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-[10px]">
        <div className="flex items-center gap-1.5">
          <span className="text-slate-500 dark:text-slate-400 font-medium">{display.label}</span>
          {windowLabel && (
            <span className="text-slate-400 dark:text-slate-500 font-mono">{windowLabel}</span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {display.warning && <AlertTriangle className="h-3 w-3 text-amber-500" />}
          <span className="font-mono text-slate-700 dark:text-slate-300">
            {formatUsdRaw(display.usage)} / {formatUsdRaw(display.limit)}
          </span>
          <span className="text-slate-400 dark:text-slate-500">
            ({formatPercent(display.percent, 0)})
          </span>
        </div>
      </div>
      <ProgressBar percent={display.percent} warning={display.warning} />
    </div>
  );
}

function ProviderCard({ row }: { row: ProviderLimitUsageRow }) {
  const limitDisplays = useMemo(() => getLimitDisplays(row), [row]);
  const hasWarning = limitDisplays.some((d) => d.warning);

  return (
    <div
      className={cn(
        "rounded-lg border bg-white dark:bg-slate-800 px-3 py-2.5 shadow-sm transition-all duration-200 hover:shadow-md",
        hasWarning
          ? "border-amber-200 hover:border-amber-300 dark:border-amber-700 dark:hover:border-amber-600"
          : "border-slate-200 hover:border-indigo-200 dark:border-slate-700 dark:hover:border-indigo-700"
      )}
    >
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-300">
            <span
              className={cn(
                "shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium",
                cliBadgeTone(row.cli_key)
              )}
            >
              {cliShortLabel(row.cli_key)}
            </span>
            <span className="truncate max-w-[180px] font-medium">{row.provider_name}</span>
          </div>
          <div className="flex items-center gap-1">
            {!row.enabled && (
              <span className="text-[10px] text-slate-400 dark:text-slate-500 bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 rounded">
                已禁用
              </span>
            )}
            {hasWarning && (
              <span className="text-[10px] text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/30 px-1.5 py-0.5 rounded">
                接近限额
              </span>
            )}
          </div>
        </div>

        <div className="flex flex-col gap-2">
          {limitDisplays.map((display) => (
            <LimitItem key={display.label} display={display} />
          ))}
        </div>
      </div>
    </div>
  );
}

export type HomeProviderLimitPanelContentProps = {
  rows: ProviderLimitUsageRow[];
  loading: boolean;
  available: boolean | null;
  onRefresh?: () => void;
  refreshing?: boolean;
};

/** Content-only version for embedding in external Card */
export function HomeProviderLimitPanelContent({
  rows,
  loading,
  available,
  onRefresh,
  refreshing = false,
}: HomeProviderLimitPanelContentProps) {
  const sortedRows = useMemo(() => {
    return rows.slice().sort((a, b) => {
      const cliCompare = a.cli_key.localeCompare(b.cli_key);
      if (cliCompare !== 0) return cliCompare;
      return a.provider_name.localeCompare(b.provider_name);
    });
  }, [rows]);

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
        <Spinner size="sm" />
        加载中...
      </div>
    );
  }

  if (available === false) {
    return <div className="text-sm text-slate-600 dark:text-slate-400">数据不可用</div>;
  }

  if (rows.length === 0) {
    return <EmptyState title="暂无配置限额的供应商" description="请在供应商编辑界面配置限额。" />;
  }

  return (
    <div className="flex flex-col h-full gap-2">
      <div className="flex items-center justify-between shrink-0">
        <span className="text-xs text-slate-400 dark:text-slate-500">{rows.length} 个供应商</span>
        {onRefresh && (
          <button
            type="button"
            onClick={onRefresh}
            disabled={refreshing}
            className="rounded-md p-1 text-slate-400 transition-colors hover:bg-slate-100 hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-slate-700 dark:hover:text-indigo-400"
            title="刷新供应商限额"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
          </button>
        )}
      </div>
      <div className="space-y-2 flex-1 min-h-0 overflow-auto pr-1 scrollbar-overlay">
        {sortedRows.map((row) => (
          <ProviderCard key={`${row.cli_key}:${row.provider_id}`} row={row} />
        ))}
      </div>
    </div>
  );
}

export function HomeProviderLimitPanel({
  rows,
  loading,
  available,
  onRefresh,
  refreshing,
}: HomeProviderLimitPanelProps) {
  const sortedRows = useMemo(() => {
    return rows.slice().sort((a, b) => {
      // Sort by CLI key first, then by provider name
      const cliCompare = a.cli_key.localeCompare(b.cli_key);
      if (cliCompare !== 0) return cliCompare;
      return a.provider_name.localeCompare(b.provider_name);
    });
  }, [rows]);

  return (
    <Card padding="sm" className="flex flex-col h-full">
      <div className="flex items-center justify-between gap-2 shrink-0">
        <div className="text-sm font-semibold">供应商限额</div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-slate-400 dark:text-slate-500">{rows.length} 个供应商</span>
          <button
            type="button"
            onClick={onRefresh}
            disabled={refreshing}
            className={cn(
              "flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-all",
              refreshing
                ? "bg-slate-100 dark:bg-slate-700 text-slate-400 dark:text-slate-500 cursor-not-allowed"
                : "bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-900/50"
            )}
          >
            <RefreshCw className={cn("h-3 w-3", refreshing && "animate-spin")} />
            刷新
          </button>
        </div>
      </div>

      {loading ? (
        <div className="mt-2 flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
          <Spinner size="sm" />
          加载中...
        </div>
      ) : available === false ? (
        <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">数据不可用</div>
      ) : rows.length === 0 ? (
        <div className="mt-2">
          <EmptyState title="暂无配置限额的供应商" description="请在供应商编辑界面配置限额。" />
        </div>
      ) : (
        <div className="mt-3 space-y-2 flex-1 min-h-0 overflow-auto pr-1 scrollbar-overlay">
          {sortedRows.map((row) => (
            <ProviderCard key={`${row.cli_key}:${row.provider_id}`} row={row} />
          ))}
        </div>
      )}
    </Card>
  );
}
