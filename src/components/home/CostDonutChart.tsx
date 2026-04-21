// Usage:
// - Extracted from HomeCostPanel. Renders the donut charts for provider and model cost distribution.

import { useMemo } from "react";
import { ResponsiveContainer, PieChart, Pie, Cell, Label, Tooltip } from "recharts";
import { CHART_PALETTE } from "../../constants/colors";
import { Card } from "../../ui/Card";
import { formatUsd } from "../../utils/formatters";
import { getTooltipStyle } from "../charts/chartTheme";
import { CHART_ANIMATION } from "../charts/chartTheme";
import type { CostPeriod } from "../../services/usage/cost";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";

// Pie chart color palette — aligned with MULTI_SERIES_PALETTE via shared constants
const PIE_COLORS = [...CHART_PALETTE.slice(0, 7), "#64748b"];

export type DonutDataSet = {
  data: Array<{ name: string; value: number }>;
  total: number;
};

// Internal donut chart renderer
function DonutChart({
  data,
  total,
  isDark,
}: {
  data: Array<{ name: string; value: number }>;
  total: number;
  isDark: boolean;
}) {
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <PieChart>
        <Pie
          data={data}
          cx="50%"
          cy="50%"
          innerRadius="50%"
          outerRadius="75%"
          paddingAngle={2}
          dataKey="value"
          animationDuration={CHART_ANIMATION.animationDuration}
        >
          {data.map((_, index) => (
            <Cell
              key={`cell-${index}`}
              fill={PIE_COLORS[index % PIE_COLORS.length]}
              stroke={isDark ? "#1e293b" : "#fff"}
              strokeWidth={2}
            />
          ))}
          <Label
            value={formatUsd(total)}
            position="center"
            style={{
              fontSize: 14,
              fontWeight: 600,
              fill: isDark ? "#e2e8f0" : "#334155",
            }}
          />
        </Pie>
        <Tooltip
          contentStyle={tooltipStyle}
          formatter={(value: number, name: string) => [
            `${formatUsd(value)} (${((value / total) * 100).toFixed(1)}%)`,
            name,
          ]}
        />
      </PieChart>
    </ResponsiveContainer>
  );
}

// Exported wrapper that includes the Card, header, and loading/empty states
export function CostDonutCharts({
  providerData,
  modelData,
  period,
  loading,
  isDark,
  hasData,
  customApplied,
}: {
  providerData: DonutDataSet;
  modelData: DonutDataSet;
  period: CostPeriod;
  loading: boolean;
  isDark: boolean;
  hasData: boolean;
  customApplied: CustomDateRangeApplied | null;
}) {
  return (
    <Card padding="sm" className="flex flex-col min-h-[180px]" data-testid="home-cost-donut-charts">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">花费占比</div>
      </div>
      {loading ? (
        <div className="text-sm text-slate-400 dark:text-slate-500">加载中…</div>
      ) : hasData ? (
        <div className="grid grid-cols-2 gap-4 flex-1">
          <div className="flex flex-col">
            <div className="text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
              供应商
            </div>
            <div className="h-[140px]">
              <DonutChart data={providerData.data} total={providerData.total} isDark={isDark} />
            </div>
          </div>
          <div className="flex flex-col">
            <div className="text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">模型</div>
            <div className="h-[140px]">
              <DonutChart data={modelData.data} total={modelData.total} isDark={isDark} />
            </div>
          </div>
        </div>
      ) : (
        <div className="text-sm text-slate-600 dark:text-slate-400">
          {period === "custom" && !customApplied
            ? "自定义范围：请选择日期后点击「应用」。"
            : "暂无花费数据。"}
        </div>
      )}
    </Card>
  );
}
