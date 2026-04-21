// Usage:
// - Extracted from HomeCostPanel. Renders the cost trend area chart with CLI filter buttons.

import { useMemo } from "react";
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { formatUsd } from "../../utils/formatters";
import {
  CHART_COLORS,
  getAxisStyle,
  getGridLineStyle,
  getTooltipStyle,
  getAxisLineStroke,
  getCursorStroke,
  CHART_ANIMATION,
} from "../charts/chartTheme";
import type { CostPeriod } from "../../services/usage/cost";
import { CLI_ITEMS, type CliFilter } from "./useCostFilters";

type TrendChartDataPoint = { label: string; cost: number };

// Internal area chart component
function TrendAreaChart({
  data,
  isHourly,
  isDark,
}: {
  data: TrendChartDataPoint[];
  isHourly: boolean;
  isDark: boolean;
}) {
  const axisStyle = useMemo(() => getAxisStyle(isDark), [isDark]);
  const gridLineStyle = useMemo(() => getGridLineStyle(isDark), [isDark]);
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);
  const axisLineStroke = getAxisLineStroke(isDark);
  const cursorStroke = getCursorStroke(isDark);

  const xAxisTicks = useMemo(() => {
    const interval = isHourly ? 4 : 3;
    return data.filter((_, i) => i % interval === 0).map((d) => d.label);
  }, [data, isHourly]);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <AreaChart data={data} margin={{ left: 0, right: 16, top: 8, bottom: 0 }}>
        <defs>
          <linearGradient id="costAreaGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={CHART_COLORS.primary} stopOpacity={0.25} />
            <stop offset="100%" stopColor={CHART_COLORS.primary} stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid
          vertical={false}
          stroke={gridLineStyle.stroke}
          strokeDasharray={gridLineStyle.strokeDasharray}
        />
        <XAxis
          dataKey="label"
          axisLine={{ stroke: axisLineStroke }}
          tickLine={false}
          tick={{ ...axisStyle }}
          ticks={xAxisTicks}
          interval="preserveStartEnd"
        />
        <YAxis
          axisLine={false}
          tickLine={false}
          tick={{ ...axisStyle }}
          tickFormatter={formatUsd}
          width={92}
        />
        <Tooltip
          contentStyle={tooltipStyle}
          labelStyle={{ fontWeight: 600, marginBottom: 4 }}
          formatter={(value: number) => [formatUsd(value), "Cost"]}
          cursor={{ stroke: cursorStroke, strokeWidth: 1 }}
        />
        <Area
          type="monotone"
          dataKey="cost"
          stroke={CHART_COLORS.primary}
          strokeWidth={3}
          fill="url(#costAreaGradient)"
          animationDuration={CHART_ANIMATION.animationDuration}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

// Exported wrapper that includes the Card, header, and loading/empty states
export function CostTrendChart({
  data,
  period,
  isDark,
  loading,
  fetching,
  hasData,
  cliKey,
  onCliKeyChange,
}: {
  data: TrendChartDataPoint[];
  period: CostPeriod;
  isDark: boolean;
  loading: boolean;
  fetching: boolean;
  hasData: boolean;
  cliKey: CliFilter;
  onCliKeyChange: (key: CliFilter) => void;
}) {
  return (
    <Card
      padding="sm"
      className="lg:col-span-6 flex flex-col min-h-[320px]"
      data-testid="home-cost-trend-chart"
    >
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            总花费趋势
          </span>
          <span className="text-xs text-slate-400 dark:text-slate-500">
            {period === "daily" ? "按小时" : "按天"}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            {CLI_ITEMS.map((item) => (
              <button
                key={item.key}
                type="button"
                onClick={() => onCliKeyChange(item.key)}
                disabled={fetching}
                className={cn(
                  "px-3 py-1 text-xs rounded-lg font-medium transition-all",
                  cliKey === item.key
                    ? "bg-indigo-500 text-white shadow-sm"
                    : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600"
                )}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>
      </div>
      {loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-400 dark:text-slate-500">
          <Spinner size="sm" />
          加载中…
        </div>
      ) : hasData ? (
        <div className="h-[280px] flex-1">
          <TrendAreaChart data={data} isHourly={period === "daily"} isDark={isDark} />
        </div>
      ) : (
        <EmptyState title="暂无可展示的数据。" />
      )}
    </Card>
  );
}
