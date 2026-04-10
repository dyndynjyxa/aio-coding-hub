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
import type { UsageHourlyRow } from "../services/usage/usage";
import { useTheme } from "../hooks/useTheme";
import { cn } from "../utils/cn";
import { buildRecentDayKeys } from "../utils/dateKeys";
import { formatTokensMillions, computeNiceYAxis, toDateLabel } from "../utils/chartHelpers";
import {
  CHART_COLORS,
  getAxisStyle,
  getGridLineStyle,
  getTooltipStyle,
  getAxisLineStroke,
  getCursorStroke,
  CHART_ANIMATION,
} from "./charts/chartTheme";

type ChartDataPoint = {
  label: string;
  tokens: number;
};

export function buildUsageTokensXAxisTicks(labels: string[]) {
  if (labels.length <= 7) return labels;

  const interval = Math.max(1, Math.ceil((labels.length - 1) / 6));
  const ticks = labels.filter((_, i) => i % interval === 0);
  const last = labels[labels.length - 1];

  if (last && ticks[ticks.length - 1] !== last) {
    ticks.push(last);
  }

  return ticks;
}

export function UsageTokensChart({
  rows,
  days = 15,
  className,
}: {
  rows: UsageHourlyRow[];
  days?: number;
  className?: string;
}) {
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  const axisStyle = useMemo(() => getAxisStyle(isDark), [isDark]);
  const gridLineStyle = useMemo(() => getGridLineStyle(isDark), [isDark]);
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);
  const axisLineStroke = getAxisLineStroke(isDark);
  const cursorStroke = getCursorStroke(isDark);

  const dayKeys = useMemo(() => buildRecentDayKeys(days), [days]);

  const tokensByDay = useMemo(() => {
    const map = new Map<string, number>();
    for (const row of rows) {
      const day = row.day;
      if (!day) continue;
      const prev = map.get(day) ?? 0;
      const next = prev + (Number(row.total_tokens) || 0);
      map.set(day, next);
    }
    return map;
  }, [rows]);

  const chartData = useMemo<ChartDataPoint[]>(() => {
    return dayKeys.map((day) => ({
      label: toDateLabel(day),
      tokens: tokensByDay.get(day) ?? 0,
    }));
  }, [dayKeys, tokensByDay]);

  const yAxisConfig = useMemo(() => {
    const maxY = Math.max(0, ...chartData.map((d) => d.tokens));
    return computeNiceYAxis(maxY, 5);
  }, [chartData]);

  const tickValues = useMemo(() => {
    const ticks: number[] = [];
    for (let v = 0; v <= yAxisConfig.max; v += yAxisConfig.interval) {
      ticks.push(v);
    }
    return ticks;
  }, [yAxisConfig]);

  const xAxisTicks = useMemo(() => {
    return buildUsageTokensXAxisTicks(chartData.map((d) => d.label));
  }, [chartData]);

  return (
    <div className={cn("h-full w-full", className)}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ left: 0, right: 16, top: 8, bottom: 0 }}>
          <defs>
            <linearGradient id="tokenAreaGradient" x1="0" y1="0" x2="0" y2="1">
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
            domain={[0, yAxisConfig.max]}
            ticks={tickValues}
            axisLine={false}
            tickLine={false}
            tick={{ ...axisStyle }}
            tickFormatter={formatTokensMillions}
            width={45}
          />
          <Tooltip
            contentStyle={tooltipStyle}
            labelStyle={{ fontWeight: 600, marginBottom: 4 }}
            formatter={(value: number) => [formatTokensMillions(value), "Tokens"]}
            cursor={{ stroke: cursorStroke, strokeWidth: 1 }}
          />
          <Area
            type="monotone"
            dataKey="tokens"
            stroke={CHART_COLORS.primary}
            strokeWidth={3}
            fill="url(#tokenAreaGradient)"
            animationDuration={CHART_ANIMATION.animationDuration}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
