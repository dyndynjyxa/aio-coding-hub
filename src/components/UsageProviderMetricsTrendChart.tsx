import { Suspense, useMemo, type ReactNode } from "react";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "./charts/lazyRecharts";
import type { CustomDateRangeApplied } from "../hooks/useCustomDateRange";
import type { UsagePeriod, UsageProviderMetricsTrendRowV1 } from "../services/usage/usage";
import { useTheme } from "../hooks/useTheme";
import { cn } from "../utils/cn";
import {
  buildDayKeysInRangeInclusive,
  buildMonthKeysFromRows,
  buildMonthToTodayDayKeys,
  buildRecentDayKeys,
  toMmDd,
} from "../utils/dateKeys";
import { formatDurationMs, formatInteger, formatTokensPerSecond } from "../utils/formatters";
import {
  pickPaletteColor,
  getAxisStyle,
  getGridLineStyle,
  getTooltipStyle,
  getLegendStyle,
  getAxisLineStroke,
  CHART_ANIMATION,
} from "./charts/chartTheme";

export type UsageTrendMetric = "duration" | "ttfb" | "rate";

type MetricConfig = {
  label: string;
  pickValue: (row: UsageProviderMetricsTrendRowV1) => number | null;
  format: (value: number) => string;
};

const METRIC_CONFIG: Record<UsageTrendMetric, MetricConfig> = {
  duration: {
    label: "平均耗时",
    pickValue: (row) => (row.avg_duration_ms == null ? null : Number(row.avg_duration_ms)),
    format: (v) => formatDurationMs(v),
  },
  ttfb: {
    label: "平均首字",
    pickValue: (row) => (row.avg_ttfb_ms == null ? null : Number(row.avg_ttfb_ms)),
    format: (v) => formatDurationMs(v),
  },
  rate: {
    label: "平均速率",
    pickValue: (row) =>
      row.avg_output_tokens_per_second == null ? null : Number(row.avg_output_tokens_per_second),
    format: (v) => formatTokensPerSecond(v),
  },
};

type ChartDataPoint = {
  label: string;
  [provider: string]: string | number | PointMeta | undefined;
};

type PointMeta = {
  requestsSuccess: number;
};

type TooltipItem = PointMeta & {
  name: string;
  color: string;
  value: number;
};

type ChartTooltipPayloadEntry = {
  dataKey?: string | number;
  payload?: unknown;
  value?: unknown;
  name?: unknown;
  color?: string;
};

type ChartTooltipProps = {
  active?: boolean;
  payload?: ChartTooltipPayloadEntry[];
  label?: ReactNode;
};

function MetricsTooltip({
  active,
  payload,
  label,
  isDark,
  tooltipStyle,
  format,
}: ChartTooltipProps & {
  isDark: boolean;
  tooltipStyle: ReturnType<typeof getTooltipStyle>;
  format: (value: number) => string;
}) {
  if (!active || !payload || payload.length === 0) return null;

  const items: TooltipItem[] = payload
    .map((entry) => {
      const providerKey = String(entry.dataKey ?? "");
      if (!providerKey) return null;
      const meta = (entry.payload as ChartDataPoint | undefined)?.[`${providerKey}_meta`] as
        | PointMeta
        | undefined;
      const value = entry.value;
      if (value == null || !Number.isFinite(value as number) || !meta) return null;

      return {
        name: entry.name as string,
        color: entry.color ?? "",
        value: value as number,
        ...meta,
      };
    })
    .filter((v): v is TooltipItem => v != null)
    .sort((a, b) => b.requestsSuccess - a.requestsSuccess);

  if (items.length === 0) return null;

  const MAX_ITEMS = 12;
  const sliced = items.slice(0, MAX_ITEMS);
  const hidden = items.length - sliced.length;

  return (
    <div
      style={{
        backgroundColor: tooltipStyle.backgroundColor,
        border: tooltipStyle.border,
        borderRadius: tooltipStyle.borderRadius,
        boxShadow: tooltipStyle.boxShadow,
        padding: tooltipStyle.padding,
        color: tooltipStyle.color,
        minWidth: 200,
      }}
    >
      <div style={{ marginBottom: 6, fontWeight: 600 }}>{label}</div>
      <div style={{ marginBottom: 6, color: isDark ? "#94a3b8" : "#64748b" }}>
        供应商: {items.length}
      </div>
      {sliced.map((item: TooltipItem) => (
        <div key={item.name}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span
              style={{
                display: "inline-block",
                width: 8,
                height: 8,
                borderRadius: 999,
                background: item.color,
              }}
            />
            <span
              style={{
                flex: 1,
                minWidth: 0,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {item.name}
            </span>
            <span
              style={{ fontVariantNumeric: "tabular-nums", color: isDark ? "#e2e8f0" : "#0f172a" }}
            >
              {format(item.value)}
            </span>
          </div>
          <div
            style={{
              margin: "2px 0 8px 16px",
              color: isDark ? "#94a3b8" : "#64748b",
              fontSize: 12,
            }}
          >
            ok {formatInteger(item.requestsSuccess)}
          </div>
        </div>
      ))}
      {hidden > 0 && (
        <div style={{ marginTop: 4, color: isDark ? "#94a3b8" : "#64748b" }}>
          ... +{hidden}（可通过 legend 过滤）
        </div>
      )}
    </div>
  );
}

function niceCeil(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 1;
  const exp = Math.floor(Math.log10(value));
  const base = Math.pow(10, exp);
  const frac = value / base;
  const niceFrac = frac <= 1 ? 1 : frac <= 2 ? 2 : frac <= 5 ? 5 : 10;
  return niceFrac * base;
}

type ProviderSeries = {
  key: string;
  name: string;
  color: string;
  totalRequests: number;
};

export function UsageProviderMetricsTrendChart({
  rows,
  period,
  metric,
  customApplied,
  className,
}: {
  rows: UsageProviderMetricsTrendRowV1[];
  period: UsagePeriod;
  metric: UsageTrendMetric;
  customApplied: CustomDateRangeApplied | null;
  className?: string;
}) {
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  const axisStyle = useMemo(() => getAxisStyle(isDark), [isDark]);
  const gridLineStyle = useMemo(() => getGridLineStyle(isDark), [isDark]);
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);
  const legendStyle = useMemo(() => getLegendStyle(isDark), [isDark]);
  const axisLineStroke = getAxisLineStroke(isDark);
  const config = METRIC_CONFIG[metric];

  const { xLabels, chartData, providers, yMax } = useMemo(() => {
    const isHourly = period === "daily";
    const isAllTime = period === "allTime";

    const xKeys = (() => {
      if (isHourly) return Array.from({ length: 24 }).map((_, h) => String(h).padStart(2, "0"));
      if (isAllTime) return buildMonthKeysFromRows(rows);
      if (period === "weekly") return buildRecentDayKeys(7);
      if (period === "monthly") return buildMonthToTodayDayKeys();
      if (period === "custom" && customApplied) {
        return buildDayKeysInRangeInclusive(customApplied.startDate, customApplied.endDate);
      }
      return [];
    })();

    const xLabels = isHourly || isAllTime ? xKeys : xKeys.map(toMmDd);

    const byProvider = new Map<
      string,
      { name: string; totalRequests: number; points: Map<string, UsageProviderMetricsTrendRowV1> }
    >();

    for (const row of rows) {
      const key = row.key;
      if (!key) continue;
      const provider = byProvider.get(key) ?? {
        name: row.name || row.key,
        totalRequests: 0,
        points: new Map(),
      };

      const xKey = (() => {
        if (isHourly) {
          const h = row.hour == null ? NaN : Number(row.hour);
          if (!Number.isFinite(h)) return null;
          return String(h).padStart(2, "0");
        }
        return row.day || null;
      })();
      if (!xKey) continue;

      provider.name = row.name || provider.name;
      provider.totalRequests += Number(row.requests_success) || 0;
      provider.points.set(xKey, row);
      byProvider.set(key, provider);
    }

    const providers: ProviderSeries[] = Array.from(byProvider.entries())
      .map(([key, value]) => ({ key, ...value }))
      .sort((a, b) => b.totalRequests - a.totalRequests)
      .map((provider, idx) => ({
        key: provider.key,
        name: provider.name,
        color: pickPaletteColor(idx),
        totalRequests: provider.totalRequests,
      }));

    let globalMax = Number.NEGATIVE_INFINITY;

    const chartData: ChartDataPoint[] = xLabels.map((label, xIndex) => {
      const xKey = xKeys[xIndex]!;
      const point: ChartDataPoint = { label };

      providers.forEach((provider) => {
        const row = byProvider.get(provider.key)?.points.get(xKey);
        if (!row) return;

        const value = config.pickValue(row);
        if (value == null || !Number.isFinite(value)) return;

        globalMax = Math.max(globalMax, value);
        point[provider.key] = value;
        point[`${provider.key}_meta`] = {
          requestsSuccess: Number(row.requests_success) || 0,
        };
      });

      return point;
    });

    const yMax = Number.isFinite(globalMax) && globalMax > 0 ? niceCeil(globalMax) : 1;

    return { xLabels, chartData, providers, yMax };
  }, [config, customApplied, period, rows]);

  const xAxisTicks = useMemo(() => {
    const interval = period === "daily" ? 2 : 3;
    return xLabels.filter((_, i) => i % interval === 0);
  }, [xLabels, period]);

  const lineWidth = providers.length > 25 ? 1.5 : 2;

  return (
    <div className={cn("h-full w-full", className)}>
      <Suspense fallback={<div className="h-full w-full" />}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData} margin={{ left: 0, right: 16, top: 56, bottom: 0 }}>
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
              domain={[0, yMax]}
              axisLine={false}
              tickLine={false}
              tick={{ ...axisStyle }}
              tickFormatter={(v: number) => config.format(v)}
              width={56}
            />
            <Tooltip
              content={
                <MetricsTooltip
                  isDark={isDark}
                  tooltipStyle={tooltipStyle}
                  format={config.format}
                />
              }
            />
            <Legend
              wrapperStyle={{
                paddingTop: 8,
                fontSize: legendStyle.fontSize,
                color: legendStyle.color,
              }}
            />
            {providers.map((provider) => (
              <Line
                key={provider.key}
                type="monotone"
                dataKey={provider.key}
                name={provider.name}
                stroke={provider.color}
                strokeWidth={lineWidth}
                dot={false}
                animationDuration={CHART_ANIMATION.animationDuration}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </Suspense>
    </div>
  );
}
