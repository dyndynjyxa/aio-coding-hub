// Usage:
// - Extracted from HomeCostPanel. Renders the cost vs duration scatter chart with CLI legend.

import { useMemo } from "react";
import {
  ResponsiveContainer,
  ScatterChart,
  Scatter,
  XAxis,
  YAxis,
  ZAxis,
  Tooltip,
  CartesianGrid,
  LabelList,
} from "recharts";
import type { TooltipProps } from "recharts";
import type { ValueType, NameType } from "recharts/types/component/DefaultTooltipContent";
import { cliShortLabel } from "../../constants/clis";
import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import {
  formatDurationMs,
  formatDurationMsShort,
  formatInteger,
  formatUsd,
} from "../../utils/formatters";
import {
  CHART_COLORS,
  getAxisStyle,
  getTooltipStyle,
  getAxisLineStroke,
  CHART_ANIMATION,
} from "../charts/chartTheme";
import type { CliKey } from "../../services/providers/providers";
import { CLI_ITEMS, type CliFilter, type ScatterPoint } from "./useCostFilters";

// Scatter chart colors by CLI
const SCATTER_COLORS: Record<CliKey, string> = {
  claude: CHART_COLORS.primary,
  codex: CHART_COLORS.secondary,
  gemini: CHART_COLORS.success,
};

// Internal scatter chart renderer
function ScatterChartInner({ data, isDark }: { data: ScatterPoint[]; isDark: boolean }) {
  const axisStyle = useMemo(() => getAxisStyle(isDark), [isDark]);
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);
  const axisLineStroke = getAxisLineStroke(isDark);

  const byCliData = useMemo(() => {
    const grouped: Record<CliKey, ScatterPoint[]> = {
      claude: [],
      codex: [],
      gemini: [],
    };
    for (const point of data) {
      grouped[point.cli]?.push(point);
    }
    return grouped;
  }, [data]);

  const CustomTooltip = ({ active, payload }: TooltipProps<ValueType, NameType>) => {
    if (!active || !payload || payload.length === 0) return null;

    const point = payload[0]?.payload as ScatterPoint | undefined;
    if (!point) return null;

    const meta = point.meta;
    const cliLabel = cliShortLabel(meta.cli_key);
    const providerRaw = meta.provider_name?.trim() ? meta.provider_name.trim() : "Unknown";
    const modelRaw = meta.model?.trim() ? meta.model.trim() : "Unknown";
    const providerText = providerRaw === "Unknown" ? "未知" : providerRaw;
    const modelText = modelRaw === "Unknown" ? "未知" : modelRaw;
    const requests = Number.isFinite(meta.requests_success)
      ? Math.max(0, meta.requests_success)
      : 0;
    const avgCostUsd = requests > 0 ? meta.total_cost_usd / requests : null;
    const avgDurationMs = requests > 0 ? meta.total_duration_ms / requests : null;

    return (
      <div style={{ ...tooltipStyle, minWidth: 200 }}>
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
          {cliLabel} · {providerText} · {modelText}
        </div>
        <div style={{ fontSize: 11, color: isDark ? "#94a3b8" : "#64748b" }}>
          总成本：{formatUsd(meta.total_cost_usd)}
        </div>
        <div style={{ fontSize: 11, color: isDark ? "#94a3b8" : "#64748b" }}>
          总耗时：{formatDurationMs(meta.total_duration_ms)}
        </div>
        <div style={{ fontSize: 11, color: isDark ? "#94a3b8" : "#64748b" }}>
          请求数：{formatInteger(requests)}
        </div>
        <div style={{ fontSize: 11, color: isDark ? "#cbd5e1" : "#94a3b8" }}>
          {avgCostUsd == null
            ? "均值：—"
            : `均值：${formatUsd(avgCostUsd)} / ${formatDurationMs(avgDurationMs ?? 0)}`}
        </div>
      </div>
    );
  };

  const cliOrder: CliKey[] = ["claude", "codex", "gemini"];
  const activeClis = cliOrder.filter((cli) => byCliData[cli].length > 0);

  return (
    <ResponsiveContainer width="100%" height="100%">
      <ScatterChart margin={{ left: 0, right: 16, top: 8, bottom: 4 }}>
        <CartesianGrid
          stroke={isDark ? "rgba(100, 150, 255, 0.1)" : "rgba(15,23,42,0.08)"}
          strokeDasharray="3 3"
        />
        <XAxis
          type="number"
          dataKey="x"
          name="Cost"
          axisLine={{ stroke: axisLineStroke }}
          tickLine={false}
          tick={{ ...axisStyle }}
          tickFormatter={formatUsd}
        />
        <YAxis
          type="number"
          dataKey="y"
          name="Duration"
          axisLine={false}
          tickLine={false}
          tick={{ ...axisStyle }}
          tickFormatter={formatDurationMsShort}
          width={56}
        />
        <ZAxis type="number" dataKey="z" range={[60, 400]} />
        <Tooltip content={<CustomTooltip />} />
        {activeClis.map((cli) => (
          <Scatter
            key={cli}
            name={cliShortLabel(cli)}
            data={byCliData[cli]}
            fill={SCATTER_COLORS[cli]}
            fillOpacity={0.85}
            animationDuration={CHART_ANIMATION.animationDuration}
          >
            <LabelList
              dataKey="shortLabel"
              position="right"
              offset={6}
              style={{ fontSize: 9, fill: isDark ? "#cbd5e1" : "#94a3b8", fontWeight: 500 }}
            />
          </Scatter>
        ))}
      </ScatterChart>
    </ResponsiveContainer>
  );
}

// Exported wrapper that includes the Card, header, legend, and loading/empty states
export function CostScatterChartCard({
  scatterChartData,
  scatterRows,
  isDark,
  loading,
  fetching,
  scatterCliFilter,
  onScatterCliFilterChange,
}: {
  scatterChartData: { data: ScatterPoint[]; activeClis: CliKey[] };
  scatterRows: { cli_key: CliKey }[];
  isDark: boolean;
  loading: boolean;
  fetching: boolean;
  scatterCliFilter: CliFilter;
  onScatterCliFilterChange: (key: CliFilter) => void;
}) {
  return (
    <Card
      padding="sm"
      className="lg:col-span-6 flex flex-col min-h-[320px]"
      data-testid="home-cost-scatter-chart"
    >
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            总成本 × 总耗时
          </span>
          {scatterChartData.activeClis.length > 1 && (
            <div className="flex items-center gap-1.5">
              {scatterChartData.activeClis.map((cli) => (
                <div key={cli} className="flex items-center gap-1">
                  <span
                    className="inline-block h-2 w-2 rounded-full"
                    style={{ backgroundColor: SCATTER_COLORS[cli] }}
                  />
                  <span className="text-[10px] text-slate-500 dark:text-slate-400">
                    {cliShortLabel(cli)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="flex items-center gap-1">
          {CLI_ITEMS.map((item) => (
            <button
              key={item.key}
              type="button"
              onClick={() => onScatterCliFilterChange(item.key)}
              disabled={fetching}
              className={cn(
                "px-3 py-1 text-xs rounded-lg font-medium transition-all",
                scatterCliFilter === item.key
                  ? "bg-indigo-500 text-white shadow-sm"
                  : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600"
              )}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>
      {loading ? (
        <div className="text-sm text-slate-400 dark:text-slate-500">加载中…</div>
      ) : scatterRows.length === 0 ? (
        <div className="text-sm text-slate-600 dark:text-slate-400">暂无可展示的数据。</div>
      ) : (
        <div className="h-[280px] flex-1 min-h-0">
          <ScatterChartInner data={scatterChartData.data} isDark={isDark} />
        </div>
      )}
    </Card>
  );
}
