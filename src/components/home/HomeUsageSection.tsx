// Usage:
// - Render in `HomeOverviewPanel` as the top row showing usage heatmap + token chart.

import { useMemo } from "react";
import type { UsageHourlyRow } from "../../services/usage";
import { Card } from "../../ui/Card";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { dayKeyFromLocalDate } from "../../utils/dateKeys";
import { UsageHeatmap15d } from "../UsageHeatmap15d";
import { UsageTokensChart } from "../UsageTokensChart";

export type HomeUsageSectionProps = {
  showHeatmap: boolean;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;
};

export function HomeUsageSection({
  showHeatmap,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
}: HomeUsageSectionProps) {
  const todayTokens = useMemo(() => {
    const todayKey = dayKeyFromLocalDate(new Date());
    return usageHeatmapRows.reduce((sum, row) => {
      if (row.day !== todayKey) return sum;
      return sum + (Number(row.total_tokens) || 0);
    }, 0);
  }, [usageHeatmapRows]);

  return (
    <div className="grid h-full flex-1 grid-cols-1 gap-4 md:grid-cols-12 md:items-stretch md:gap-5">
      {showHeatmap ? (
        <Card className="min-w-0 h-full md:col-span-7 flex flex-col" padding="sm">
          <div className="text-sm font-medium text-slate-600 dark:text-slate-400 mb-2">热力图</div>
          {usageHeatmapLoading && usageHeatmapRows.length === 0 ? (
            <div className="text-sm text-slate-400">加载中…</div>
          ) : (
            <div className="flex-1">
              <UsageHeatmap15d
                rows={usageHeatmapRows}
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
        {usageHeatmapLoading && usageHeatmapRows.length === 0 ? (
          <div className="text-sm text-slate-400">加载中…</div>
        ) : (
          <div className="h-[160px] flex-1">
            <UsageTokensChart rows={usageHeatmapRows} days={15} className="h-full" />
          </div>
        )}
      </Card>
    </div>
  );
}
