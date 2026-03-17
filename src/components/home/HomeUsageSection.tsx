// Usage:
// - Render in `HomeOverviewPanel` as the top row showing usage heatmap + token chart.

import type { UsageHourlyRow } from "../../services/usage";
import { Card } from "../../ui/Card";
import { UsageHeatmap15d } from "../UsageHeatmap15d";
import { UsageTokensChart } from "../UsageTokensChart";

export type HomeUsageSectionProps = {
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;
};

export function HomeUsageSection({
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
}: HomeUsageSectionProps) {
  return (
    <div className="grid h-full flex-1 grid-cols-1 gap-4 md:grid-cols-12 md:items-stretch md:gap-5">
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

      <Card className="flex h-full min-h-[200px] flex-col md:col-span-5" padding="sm">
        <div className="text-sm font-medium text-slate-600 dark:text-slate-400 mb-2">用量统计</div>
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
