// Usage:
// - Render in `HomeOverviewPanel` as the top row showing usage heatmap + token chart.

import { useMemo } from "react";
import type { UsageHourlyRow } from "../../services/usage";
import { Card } from "../../ui/Card";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { buildRecentDayKeys, dayKeyFromLocalDate } from "../../utils/dateKeys";
import { UsageHeatmap15d } from "../UsageHeatmap15d";
import { UsageTokensChart } from "../UsageTokensChart";

export type HomeUsageSectionProps = {
  devPreviewEnabled?: boolean;
  showHeatmap: boolean;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;
};

function buildPreviewUsageRows(days = 15): UsageHourlyRow[] {
  const dayKeys = buildRecentDayKeys(days);
  const previewHours = [9, 14, 20] as const;
  const dayWave = [0.82, 1.18, 0.94, 1.36, 0.88, 1.24, 0.98] as const;
  const hourWave = [0.72, 1.12, 0.91] as const;

  return dayKeys.flatMap((day, index) => {
    const dayFactor = dayWave[index % dayWave.length];

    return previewHours.map((hour, hourIndex) => {
      const hourFactor = hourWave[hourIndex];
      const requestsBase = Math.max(1, Math.round(5 * dayFactor + hourIndex * 2));
      const totalTokens = Math.round(780_000 * dayFactor * hourFactor);
      const failed = hourIndex === 2 && index % 4 === 0 ? 1 : 0;

      return {
        day,
        hour,
        requests_total: requestsBase,
        requests_with_usage: requestsBase,
        requests_success: requestsBase - failed,
        requests_failed: failed,
        total_tokens: totalTokens,
      };
    });
  });
}

export function HomeUsageSection({
  devPreviewEnabled = false,
  showHeatmap,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
}: HomeUsageSectionProps) {
  const displayedUsageHeatmapRows = useMemo(
    () =>
      devPreviewEnabled && usageHeatmapRows.length === 0 && !usageHeatmapLoading
        ? buildPreviewUsageRows()
        : usageHeatmapRows,
    [devPreviewEnabled, usageHeatmapLoading, usageHeatmapRows]
  );
  const todayTokens = useMemo(() => {
    const todayKey = dayKeyFromLocalDate(new Date());
    return displayedUsageHeatmapRows.reduce((sum, row) => {
      if (row.day !== todayKey) return sum;
      return sum + (Number(row.total_tokens) || 0);
    }, 0);
  }, [displayedUsageHeatmapRows]);

  return (
    <div className="grid h-full flex-1 grid-cols-1 gap-4 md:grid-cols-12 md:items-stretch md:gap-5">
      {showHeatmap ? (
        <Card className="min-w-0 h-full md:col-span-7 flex flex-col" padding="sm">
          <div className="text-sm font-medium text-slate-600 dark:text-slate-400 mb-2">热力图</div>
          {usageHeatmapLoading && displayedUsageHeatmapRows.length === 0 ? (
            <div className="text-sm text-slate-400">加载中…</div>
          ) : (
            <div className="flex-1">
              <UsageHeatmap15d
                rows={displayedUsageHeatmapRows}
                days={15}
                onRefresh={onRefreshUsageHeatmap}
                refreshing={usageHeatmapLoading}
              />
            </div>
          )}
        </Card>
      ) : null}

      <Card
        className={`flex h-full min-h-[200px] flex-col ${showHeatmap ? "md:col-span-5" : "md:col-span-12"}`}
        padding="sm"
      >
        <div className="mb-2 flex items-start justify-between gap-3">
          <div className="text-sm font-medium text-slate-600 dark:text-slate-400">用量统计</div>
          <div className="shrink-0 text-right text-sm text-slate-500 dark:text-slate-400">
            <span className="mr-1.5 text-[11px] font-medium uppercase tracking-wide text-slate-400 dark:text-slate-500">
              今日用量
            </span>
            <span className="font-semibold text-slate-700 dark:text-slate-200">
              {formatTokensMillions(todayTokens)}
            </span>
          </div>
        </div>
        {usageHeatmapLoading && displayedUsageHeatmapRows.length === 0 ? (
          <div className="text-sm text-slate-400">加载中…</div>
        ) : (
          <div className="h-[160px] flex-1">
            <UsageTokensChart rows={displayedUsageHeatmapRows} days={15} className="h-full" />
          </div>
        )}
      </Card>
    </div>
  );
}
