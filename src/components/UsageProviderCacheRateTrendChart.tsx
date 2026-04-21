import { useMemo } from "react";
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
  Legend,
  ReferenceArea,
  ReferenceLine,
} from "recharts";
import type { TooltipProps } from "recharts";
import type { ValueType, NameType } from "recharts/types/component/DefaultTooltipContent";
import type { CustomDateRangeApplied } from "../hooks/useCustomDateRange";
import type { UsagePeriod, UsageProviderCacheRateTrendRowV1 } from "../services/usage/usage";
import { useTheme } from "../hooks/useTheme";
import { cn } from "../utils/cn";
import { buildRecentDayKeys, dayKeyFromLocalDate } from "../utils/dateKeys";
import { parseYyyyMmDd } from "../utils/localDate";
import { formatInteger, formatPercent } from "../utils/formatters";
import {
  pickPaletteColor,
  getAxisStyle,
  getGridLineStyle,
  getTooltipStyle,
  getLegendStyle,
  getAxisLineStroke,
  CHART_ANIMATION,
  THRESHOLD_COLORS,
} from "./charts/chartTheme";

const WARN_THRESHOLD = 0.6;

type ChartDataPoint = {
  label: string;
  [provider: string]: string | number | PointMeta | undefined;
};

type PointMeta = {
  denomTokens: number;
  cacheReadTokens: number;
  requestsSuccess: number;
};

type TooltipItem = PointMeta & {
  name: string;
  color: string;
  value: number;
};

function toMmDd(dayKey: string) {
  const mmdd = dayKey.slice(5);
  return mmdd.replace("-", "/");
}

function buildDayKeysInRangeInclusive(startDay: string, endDay: string): string[] {
  const start = parseYyyyMmDd(startDay);
  const end = parseYyyyMmDd(endDay);
  if (!start || !end) return [];

  const startDate = new Date(start.year, start.month - 1, start.day, 0, 0, 0, 0);
  const endDate = new Date(end.year, end.month - 1, end.day, 0, 0, 0, 0);
  if (!Number.isFinite(startDate.getTime()) || !Number.isFinite(endDate.getTime())) return [];

  const out: string[] = [];
  const d = new Date(startDate);
  while (d.getTime() <= endDate.getTime()) {
    out.push(dayKeyFromLocalDate(d));
    d.setDate(d.getDate() + 1);
  }
  return out;
}

function buildMonthToTodayDayKeys(): string[] {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);
  if (!Number.isFinite(start.getTime()) || !Number.isFinite(now.getTime())) return [];

  const out: string[] = [];
  const d = new Date(start);
  while (d.getTime() <= now.getTime()) {
    out.push(dayKeyFromLocalDate(d));
    d.setDate(d.getDate() + 1);
  }
  return out;
}

function buildMonthKeysFromData(rows: UsageProviderCacheRateTrendRowV1[]): string[] {
  const set = new Set<string>();
  for (const row of rows) {
    if (!row.day) continue;
    if (/^\d{4}-\d{2}$/.test(row.day)) set.add(row.day);
  }
  return Array.from(set).sort();
}

type ProviderSeries = {
  key: string;
  name: string;
  color: string;
  totalDenomTokens: number;
};

export function UsageProviderCacheRateTrendChart({
  rows,
  period,
  customApplied,
  className,
}: {
  rows: UsageProviderCacheRateTrendRowV1[];
  period: UsagePeriod;
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

  const { xLabels, chartData, providers, warnRanges, yAxisRange } = useMemo(() => {
    const isHourly = period === "daily";
    const isAllTime = period === "allTime";

    const xKeys = (() => {
      if (isHourly) {
        return Array.from({ length: 24 }).map((_, h) => String(h).padStart(2, "0"));
      }
      if (isAllTime) {
        return buildMonthKeysFromData(rows);
      }
      if (period === "weekly") return buildRecentDayKeys(7);
      if (period === "monthly") return buildMonthToTodayDayKeys();
      if (period === "custom" && customApplied) {
        return buildDayKeysInRangeInclusive(customApplied.startDate, customApplied.endDate);
      }
      return [];
    })();

    const xLabels = (() => {
      if (isHourly) return xKeys;
      if (isAllTime) return xKeys;
      return xKeys.map(toMmDd);
    })();

    const byProvider = new Map<
      string,
      {
        name: string;
        totalDenomTokens: number;
        points: Map<string, UsageProviderCacheRateTrendRowV1>;
      }
    >();

    for (const row of rows) {
      const key = row.key;
      if (!key) continue;
      const provider = byProvider.get(key) ?? {
        name: row.name || row.key,
        totalDenomTokens: 0,
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
      provider.totalDenomTokens += Number(row.denom_tokens) || 0;
      provider.points.set(xKey, row);
      byProvider.set(key, provider);
    }

    const providers: ProviderSeries[] = Array.from(byProvider.entries())
      .map(([key, value]) => ({ key, ...value }))
      .sort((a, b) => b.totalDenomTokens - a.totalDenomTokens)
      .map((provider, idx) => ({
        key: provider.key,
        name: provider.name,
        color: pickPaletteColor(idx),
        totalDenomTokens: provider.totalDenomTokens,
      }));

    const warnAtX: boolean[] = Array.from({ length: xKeys.length }, () => false);
    let globalMin = Number.POSITIVE_INFINITY;
    let globalMax = Number.NEGATIVE_INFINITY;

    const chartData: ChartDataPoint[] = xLabels.map((label, xIndex) => {
      const xKey = xKeys[xIndex]!;
      const point: ChartDataPoint = { label };

      providers.forEach((provider) => {
        const providerData = byProvider.get(provider.key);
        const row = providerData?.points.get(xKey);
        if (!row) return;

        const denom = Number(row.denom_tokens) || 0;
        const read = Number(row.cache_read_input_tokens) || 0;
        const ok = Number(row.requests_success) || 0;
        if (!Number.isFinite(denom) || denom <= 0) return;

        const rateRaw = read / denom;
        if (!Number.isFinite(rateRaw)) return;

        const value = Math.max(0, Math.min(1, rateRaw));
        globalMin = Math.min(globalMin, value);
        globalMax = Math.max(globalMax, value);
        if (value < WARN_THRESHOLD) warnAtX[xIndex] = true;

        point[provider.key] = value;
        point[`${provider.key}_meta`] = {
          denomTokens: denom,
          cacheReadTokens: read,
          requestsSuccess: ok,
        };
      });

      return point;
    });

    // Compute Y-axis range
    const yAxisRange = (() => {
      if (!Number.isFinite(globalMin) || !Number.isFinite(globalMax)) {
        return { min: 0, max: 1, interval: 0.1 };
      }

      const span = Math.max(0.02, globalMax - globalMin);
      const pad = Math.min(0.15, span * 0.25 + 0.02);

      let min = Math.max(0, globalMin - pad);
      let max = Math.min(1, globalMax + pad);

      if (max - min < 0.08) {
        const mid = (min + max) / 2;
        min = Math.max(0, mid - 0.04);
        max = Math.min(1, mid + 0.04);
      }

      const nextSpan = max - min;
      const steps = [0.01, 0.02, 0.05, 0.1];
      const maxTicks = 10;
      const interval = steps.find((step) => Math.ceil(nextSpan / step) <= maxTicks) ?? 0.1;

      min = Math.floor(min / interval) * interval;
      max = Math.ceil(max / interval) * interval;

      min = Math.max(0, min);
      max = Math.min(1, max);

      if (max - min < interval) {
        max = Math.min(1, min + interval);
      }

      return {
        min: Math.round(min * 100) / 100,
        max: Math.round(max * 100) / 100,
        interval,
      };
    })();

    // Compute warning ranges
    const warnRanges: Array<{ x1: string; x2: string }> = [];
    let start: number | null = null;
    for (let i = 0; i < warnAtX.length; i += 1) {
      if (warnAtX[i]) {
        if (start == null) start = i;
        continue;
      }
      if (start != null) {
        warnRanges.push({ x1: xLabels[start]!, x2: xLabels[i - 1]! });
        start = null;
      }
    }
    if (start != null) {
      warnRanges.push({ x1: xLabels[start]!, x2: xLabels[warnAtX.length - 1]! });
    }

    return { xKeys, xLabels, chartData, providers, warnRanges, yAxisRange };
  }, [customApplied, period, rows]);

  const yAxisTicks = useMemo(() => {
    const ticks: number[] = [];
    for (let v = yAxisRange.min; v <= yAxisRange.max; v += yAxisRange.interval) {
      ticks.push(Math.round(v * 100) / 100);
    }
    return ticks;
  }, [yAxisRange]);

  const xAxisTicks = useMemo(() => {
    const isHourly = period === "daily";
    const interval = isHourly ? 2 : 3;
    return xLabels.filter((_, i) => i % interval === 0);
  }, [xLabels, period]);

  const CustomTooltip = ({ active, payload, label }: TooltipProps<ValueType, NameType>) => {
    if (!active || !payload || payload.length === 0) return null;

    const items: TooltipItem[] = payload
      .map((entry) => {
        const providerKey = entry.dataKey;
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
      .filter((v): v is TooltipItem => v != null);

    const warnItems = items
      .filter((item) => item.value < WARN_THRESHOLD)
      .sort((a, b) => a.value - b.value);
    const okItems = items
      .filter((item) => item.value >= WARN_THRESHOLD)
      .sort((a, b) => b.denomTokens - a.denomTokens);

    const MAX_ITEMS = 12;
    const renderItems = warnItems.length > 0 ? warnItems : okItems;
    const sliced = renderItems.slice(0, MAX_ITEMS);
    const hidden = renderItems.length - sliced.length;

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
        {warnItems.length > 0 ? (
          <div style={{ marginBottom: 6, color: "#b91c1c" }}>
            预警（&lt;60%）: {warnItems.length}
          </div>
        ) : (
          <div style={{ marginBottom: 6, color: isDark ? "#94a3b8" : "#64748b" }}>
            供应商: {items.length}
          </div>
        )}
        {sliced.map((item: TooltipItem, idx: number) => {
          const isWarn = item.value < WARN_THRESHOLD;
          const valueColor = isWarn ? "#b91c1c" : isDark ? "#e2e8f0" : "#0f172a";

          return (
            <div key={idx}>
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
                  style={{
                    fontVariantNumeric: "tabular-nums",
                    color: valueColor,
                  }}
                >
                  {formatPercent(item.value, 2)}
                </span>
              </div>
              <div
                style={{
                  margin: "2px 0 8px 16px",
                  color: isDark ? "#94a3b8" : "#64748b",
                  fontSize: 11,
                }}
              >
                denom {formatInteger(item.denomTokens)} · read {formatInteger(item.cacheReadTokens)}{" "}
                · ok {formatInteger(item.requestsSuccess)}
              </div>
            </div>
          );
        })}
        {hidden > 0 && (
          <div style={{ marginTop: 4, color: isDark ? "#94a3b8" : "#64748b" }}>
            ... +{hidden}（可通过 legend 过滤）
          </div>
        )}
      </div>
    );
  };

  const lineWidth = providers.length > 25 ? 1.5 : 2;

  return (
    <div className={cn("h-full w-full", className)}>
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
            domain={[yAxisRange.min, yAxisRange.max]}
            ticks={yAxisTicks}
            axisLine={false}
            tickLine={false}
            tick={{ ...axisStyle }}
            tickFormatter={(v: number) => formatPercent(v, 0)}
            width={45}
          />
          <Tooltip content={<CustomTooltip />} />
          <Legend
            wrapperStyle={{
              paddingTop: 8,
              fontSize: legendStyle.fontSize,
              color: legendStyle.color,
            }}
          />
          <ReferenceLine
            y={WARN_THRESHOLD}
            stroke={THRESHOLD_COLORS.warningLine}
            strokeDasharray="3 3"
            strokeWidth={1}
          />
          {warnRanges.map((range, idx) => (
            <ReferenceArea
              key={idx}
              x1={range.x1}
              x2={range.x2}
              fill={THRESHOLD_COLORS.warning}
              fillOpacity={1}
            />
          ))}
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
    </div>
  );
}
