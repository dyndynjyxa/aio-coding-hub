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
    <div className="grid grid-cols-1 gap-4 md:grid-cols-10 md:items-stretch md:gap-6">
      <Card className="min-w-0 md:col-span-6" padding="sm">
        <div className="text-sm font-medium text-slate-600 mb-2">热力图</div>
        {usageHeatmapLoading && usageHeatmapRows.length === 0 ? (
          <div className="text-sm text-slate-400">加载中…</div>
        ) : (
          <UsageHeatmap15d
            rows={usageHeatmapRows}
            days={15}
            onRefresh={onRefreshUsageHeatmap}
            refreshing={usageHeatmapLoading}
          />
        )}
      </Card>

      <Card className="flex flex-col md:col-span-4" padding="sm">
        <div className="text-sm font-medium text-slate-600 mb-2">用量统计</div>
        {usageHeatmapLoading && usageHeatmapRows.length === 0 ? (
          <div className="text-sm text-slate-400">加载中…</div>
        ) : (
          <div className="min-h-0 flex-1">
            <UsageTokensChart rows={usageHeatmapRows} days={15} className="h-full" />
          </div>
        )}
      </Card>
    </div>
  );
}
