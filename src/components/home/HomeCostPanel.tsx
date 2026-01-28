// Usage:
// - Rendered by `src/pages/HomePage.tsx` when the Home tab is switched to "花费".
// - Provides cost analytics with period + CLI + provider + model filters, charts, and top expensive requests.

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { PERIOD_ITEMS } from "../../constants/periods";
import { useCustomDateRange } from "../../hooks/useCustomDateRange";
import type { CliKey } from "../../services/providers";
import {
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
  type CostModelBreakdownRowV1,
  type CostPeriod,
  type CostProviderBreakdownRowV1,
  type CostScatterCliProviderModelRowV1,
  type CostSummaryV1,
  type CostTopRequestRowV1,
  type CostTrendRowV1,
} from "../../services/cost";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { cn } from "../../utils/cn";
import { buildRecentDayKeys, dayKeyFromLocalDate } from "../../utils/dateKeys";
import {
  formatDurationMs,
  formatDurationMsShort,
  formatInteger,
  formatPercent,
  formatRelativeTimeFromUnixSeconds,
  formatUsd,
  formatUsdShort,
} from "../../utils/formatters";
import { EChartsCanvas } from "../charts/EChartsCanvas";

type CliFilter = "all" | CliKey;

type CliItem = { key: CliFilter; label: string };

const CLI_ITEMS: CliItem[] = [
  { key: "all", label: "全部" },
  { key: "claude", label: "Claude" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
];

const FILTER_LABEL_CLASS = "w-16 shrink-0 text-right text-xs font-medium text-slate-500";
const FILTER_OPTIONS_CLASS = "min-w-0 flex flex-1 flex-wrap items-center gap-2";
const FILTER_OPTION_BUTTON_CLASS = "w-24 whitespace-nowrap";

function buildDayKeysBetweenUnixSeconds(startTs: number, endTs: number) {
  const startMs = startTs * 1000;
  const endMs = (endTs - 1) * 1000;
  const start = new Date(startMs);
  const end = new Date(endMs);
  start.setHours(0, 0, 0, 0);
  end.setHours(0, 0, 0, 0);

  const out: string[] = [];
  const cur = new Date(start);
  while (cur.getTime() <= end.getTime()) {
    out.push(dayKeyFromLocalDate(cur));
    cur.setDate(cur.getDate() + 1);
    cur.setHours(0, 0, 0, 0);
    if (out.length > 3660) break;
  }
  return out;
}

function buildMonthDayKeysToToday() {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);
  const end = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 0, 0, 0, 0);
  const out: string[] = [];
  const cur = new Date(start);
  while (cur.getTime() <= end.getTime()) {
    out.push(dayKeyFromLocalDate(cur));
    cur.setDate(cur.getDate() + 1);
    cur.setHours(0, 0, 0, 0);
    if (out.length > 62) break;
  }
  return out;
}

function StatCard({
  title,
  value,
  hint,
  className,
}: {
  title: string;
  value: string;
  hint?: string;
  className?: string;
}) {
  return (
    <Card padding="md" className={cn("flex h-full flex-col", className)}>
      <div className="text-xs font-medium text-slate-500">{title}</div>
      <div className="mt-2 text-lg font-semibold tracking-tight text-slate-900 xl:text-xl">
        {value}
      </div>
      {hint ? <div className="mt-auto pt-2 text-xs text-slate-500">{hint}</div> : null}
    </Card>
  );
}

function StatCardSkeleton({ className }: { className?: string }) {
  return (
    <Card padding="md" className={cn("h-full animate-pulse", className)}>
      <div className="h-3 w-24 rounded bg-slate-200" />
      <div className="mt-3 h-8 w-28 rounded bg-slate-200" />
      <div className="mt-3 h-3 w-44 rounded bg-slate-100" />
    </Card>
  );
}

function toMmDd(dayKey: string) {
  const mmdd = dayKey.slice(5);
  return mmdd.replace("-", "/");
}

function pickTopSlices<T extends { cost_usd: number }>(rows: T[], topN: number) {
  const sorted = rows.slice().sort((a, b) => b.cost_usd - a.cost_usd);
  const head = sorted.slice(0, Math.max(1, Math.floor(topN)));
  const tail = sorted.slice(head.length);
  const tailSum = tail.reduce((acc, cur) => acc + (Number(cur.cost_usd) || 0), 0);
  return { head, tailSum };
}

export type HomeCostPanelProps = {
  onSelectLogId: (id: number | null) => void;
};

export function HomeCostPanel({ onSelectLogId }: HomeCostPanelProps) {
  const [period, setPeriod] = useState<CostPeriod>("daily");
  const [cliKey, setCliKey] = useState<CliFilter>("all");
  const [providerId, setProviderId] = useState<number | null>(null);
  const [model, setModel] = useState<string | null>(null);
  const [reloadSeq, setReloadSeq] = useState(0);

  const {
    customStartDate,
    setCustomStartDate,
    customEndDate,
    setCustomEndDate,
    customApplied,
    bounds,
    showCustomForm,
    applyCustomRange,
    clearCustomRange,
  } = useCustomDateRange(period, { onInvalid: (message) => toast(message) });

  const [tauriAvailable, setTauriAvailable] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorText, setErrorText] = useState<string | null>(null);

  const [summary, setSummary] = useState<CostSummaryV1 | null>(null);
  const [trendRows, setTrendRows] = useState<CostTrendRowV1[]>([]);
  const [providerRows, setProviderRows] = useState<CostProviderBreakdownRowV1[]>([]);
  const [modelRows, setModelRows] = useState<CostModelBreakdownRowV1[]>([]);
  const [scatterRows, setScatterRows] = useState<CostScatterCliProviderModelRowV1[]>([]);
  const [topRequests, setTopRequests] = useState<CostTopRequestRowV1[]>([]);

  const [scatterCliFilter, setScatterCliFilter] = useState<CliFilter>("all");

  const filters = useMemo(() => {
    const filterCliKey = cliKey === "all" ? null : cliKey;
    return {
      cliKey: filterCliKey,
      providerId,
      model,
      ...bounds,
    };
  }, [bounds, cliKey, model, providerId]);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (period === "custom" && !customApplied) {
        setErrorText(null);
        setSummary(null);
        setTrendRows([]);
        setProviderRows([]);
        setModelRows([]);
        setScatterRows([]);
        setTopRequests([]);
        setLoading(false);
        setTauriAvailable(null);
        return;
      }

      setErrorText(null);
      setLoading(true);
      try {
        const [sum, trend, providers, models, scatter, top] = await Promise.all([
          costSummaryV1(period, filters),
          costTrendV1(period, filters),
          costBreakdownProviderV1(period, { ...filters, limit: 120 }),
          costBreakdownModelV1(period, { ...filters, limit: 120 }),
          costScatterCliProviderModelV1(period, { ...filters, limit: 500 }),
          costTopRequestsV1(period, { ...filters, limit: 50 }),
        ]);
        if (cancelled) return;

        if (!sum || !trend || !providers || !models || !scatter || !top) {
          setTauriAvailable(false);
          setSummary(null);
          setTrendRows([]);
          setProviderRows([]);
          setModelRows([]);
          setScatterRows([]);
          setTopRequests([]);
          return;
        }

        setTauriAvailable(true);
        setSummary(sum);
        setTrendRows(trend);
        setProviderRows(providers);
        setModelRows(models);
        setScatterRows(scatter);
        setTopRequests(top);
      } catch (err) {
        if (cancelled) return;
        setTauriAvailable(true);
        setErrorText(String(err));
        toast("加载花费失败：请重试（详情见页面错误信息）");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    load();
    return () => {
      cancelled = true;
    };
  }, [customApplied, filters, period, reloadSeq]);

  const providerOptions = useMemo(() => {
    const sorted = providerRows.slice().sort((a, b) => b.cost_usd - a.cost_usd);
    return sorted.filter((row) => Number.isFinite(row.provider_id) && row.provider_id > 0);
  }, [providerRows]);

  const modelOptions = useMemo(() => {
    return modelRows.slice().sort((a, b) => b.cost_usd - a.cost_usd);
  }, [modelRows]);

  useEffect(() => {
    if (providerId == null) return;
    if (providerOptions.some((row) => row.provider_id === providerId)) return;
    setProviderId(null);
  }, [providerId, providerOptions]);

  useEffect(() => {
    if (model == null) return;
    if (modelOptions.some((row) => row.model === model)) return;
    setModel(null);
  }, [model, modelOptions]);

  const coverage = useMemo(() => {
    if (!summary) return null;
    const denom = summary.requests_success;
    if (!Number.isFinite(denom) || denom <= 0) return null;
    return summary.cost_covered_success / denom;
  }, [summary]);

  const trendDayKeys = useMemo(() => {
    if (period === "daily") return [];
    if (period === "weekly") return buildRecentDayKeys(7);
    if (period === "monthly") return buildMonthDayKeysToToday();
    if (period === "custom" && customApplied) {
      return buildDayKeysBetweenUnixSeconds(customApplied.startTs, customApplied.endTs);
    }
    const uniq = Array.from(new Set(trendRows.map((r) => r.day))).sort();
    return uniq;
  }, [customApplied, period, trendRows]);

  const trendOption = useMemo(() => {
    const isHourly = period === "daily";
    const color = "#0052FF";

    if (isHourly) {
      const byHour = new Map<number, number>();
      for (const row of trendRows) {
        if (row.hour == null) continue;
        byHour.set(row.hour, Number(row.cost_usd) || 0);
      }
      const hours = Array.from({ length: 24 }).map((_, h) => h);
      const x = hours.map((h) => String(h).padStart(2, "0"));
      const y = hours.map((h) => byHour.get(h) ?? 0);

      return {
        animation: false,
        grid: { left: 0, right: 16, top: 8, bottom: 24, containLabel: true },
        tooltip: {
          trigger: "axis",
          confine: true,
          axisPointer: { type: "line" },
          valueFormatter: (v: unknown) => formatUsd(Number(v)),
        },
        xAxis: {
          type: "category",
          data: x,
          boundaryGap: false,
          axisLabel: { color: "#64748b", fontSize: 10, interval: 3 },
          axisLine: { lineStyle: { color: "rgba(15,23,42,0.12)" } },
          axisTick: { show: false },
        },
        yAxis: {
          type: "value",
          min: 0,
          axisLabel: {
            color: "#64748b",
            fontSize: 10,
            formatter: (value: number) => formatUsdShort(value),
          },
          axisTick: { show: false },
          axisLine: { show: false },
          splitLine: { lineStyle: { color: "rgba(0,82,255,0.10)", type: "dashed" } },
        },
        series: [
          {
            name: "cost_usd",
            type: "line",
            data: y,
            showSymbol: false,
            smooth: true,
            lineStyle: { color, width: 3 },
            areaStyle: {
              color: {
                type: "linear",
                x: 0,
                y: 0,
                x2: 0,
                y2: 1,
                colorStops: [
                  { offset: 0, color: "rgba(0,82,255,0.25)" },
                  { offset: 1, color: "rgba(0,82,255,0.0)" },
                ],
              },
            },
          },
        ],
      };
    }

    const byDay = new Map<string, number>();
    for (const row of trendRows) {
      byDay.set(row.day, Number(row.cost_usd) || 0);
    }
    const x = trendDayKeys.map(toMmDd);
    const y = trendDayKeys.map((d) => byDay.get(d) ?? 0);

    return {
      animation: false,
      grid: { left: 0, right: 16, top: 8, bottom: 24, containLabel: true },
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "line" },
        valueFormatter: (v: unknown) => formatUsd(Number(v)),
      },
      xAxis: {
        type: "category",
        data: x,
        boundaryGap: false,
        axisLabel: { color: "#64748b", fontSize: 10, interval: 2 },
        axisLine: { lineStyle: { color: "rgba(15,23,42,0.12)" } },
        axisTick: { show: false },
      },
      yAxis: {
        type: "value",
        min: 0,
        axisLabel: {
          color: "#64748b",
          fontSize: 10,
          formatter: (value: number) => formatUsd(value),
        },
        axisTick: { show: false },
        axisLine: { show: false },
        splitLine: { lineStyle: { color: "rgba(0,82,255,0.10)", type: "dashed" } },
      },
      series: [
        {
          name: "cost_usd",
          type: "line",
          data: y,
          showSymbol: false,
          smooth: true,
          lineStyle: { color, width: 3 },
          areaStyle: {
            color: {
              type: "linear",
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                { offset: 0, color: "rgba(0,82,255,0.25)" },
                { offset: 1, color: "rgba(0,82,255,0.0)" },
              ],
            },
          },
        },
      ],
    };
  }, [period, trendDayKeys, trendRows]);

  const providerDonutOption = useMemo(() => {
    const filtered = providerRows.filter((row) => row.cost_usd > 0);
    const { head, tailSum } = pickTopSlices(filtered, 7);
    const seriesData = head.map((row) => ({
      name: `${cliShortLabel(row.cli_key)} · ${row.provider_name}`,
      value: row.cost_usd,
    }));
    if (tailSum > 0) seriesData.push({ name: "其他", value: tailSum });

    const total = seriesData.reduce((sum, d) => sum + d.value, 0);

    return {
      animation: false,
      tooltip: {
        trigger: "item",
        confine: true,
        formatter: (params: any) => {
          const name = params?.name ?? "";
          const value = params?.value ?? 0;
          const percent = params?.percent ?? 0;
          return `${name}<br/>${formatUsd(value)} (${percent.toFixed(1)}%)`;
        },
      },
      series: [
        {
          type: "pie",
          radius: ["50%", "75%"],
          avoidLabelOverlap: true,
          itemStyle: { borderColor: "#fff", borderWidth: 2 },
          label: {
            show: true,
            position: "center",
            fontSize: 14,
            fontWeight: 600,
            color: "#334155",
            formatter: () => formatUsdShort(total),
          },
          labelLine: { show: false },
          data: seriesData,
        },
      ],
    };
  }, [providerRows]);

  const modelDonutOption = useMemo(() => {
    const filtered = modelRows.filter((row) => row.cost_usd > 0);
    const { head, tailSum } = pickTopSlices(filtered, 7);
    const seriesData = head.map((row) => ({
      name: row.model,
      value: row.cost_usd,
    }));
    if (tailSum > 0) seriesData.push({ name: "其他", value: tailSum });

    const total = seriesData.reduce((sum, d) => sum + d.value, 0);

    return {
      animation: false,
      tooltip: {
        trigger: "item",
        confine: true,
        formatter: (params: any) => {
          const name = params?.name ?? "";
          const value = params?.value ?? 0;
          const percent = params?.percent ?? 0;
          return `${name}<br/>${formatUsd(value)} (${percent.toFixed(1)}%)`;
        },
      },
      series: [
        {
          type: "pie",
          radius: ["50%", "75%"],
          avoidLabelOverlap: true,
          itemStyle: { borderColor: "#fff", borderWidth: 2 },
          label: {
            show: true,
            position: "center",
            fontSize: 14,
            fontWeight: 600,
            color: "#334155",
            formatter: () => formatUsdShort(total),
          },
          labelLine: { show: false },
          data: seriesData,
        },
      ],
    };
  }, [modelRows]);

  const scatterOption = useMemo(() => {
    type ScatterPoint = {
      name: string;
      value: [number, number];
      meta: CostScatterCliProviderModelRowV1;
    };

    const symbolSize = (value: [number, number]) => {
      const costForSizing = value?.[0] ?? 0;
      const size = 10 + Math.log10(1 + Math.max(0, costForSizing)) * 10;
      return Math.max(10, Math.min(26, size));
    };

    const buildSeries = (name: string, data: ScatterPoint[]) => ({
      name,
      type: "scatter" as const,
      data,
      symbolSize,
      itemStyle: { opacity: 0.85 },
      emphasis: { focus: "series" as const },
      label: {
        show: true,
        position: "right" as const,
        fontSize: 9,
        color: "#64748b",
        formatter: (params: any) => {
          const meta = params?.data?.meta;
          if (!meta) return "";
          const providerRaw = meta.provider_name?.trim() || "Unknown";
          const modelRaw = meta.model?.trim() || "Unknown";
          const providerText = providerRaw === "Unknown" ? "未知" : providerRaw;
          const modelText = modelRaw === "Unknown" ? "未知" : modelRaw;
          return `${providerText}\n${modelText}`;
        },
      },
      labelLayout: {
        hideOverlap: true,
      },
    });

    const filteredRows =
      scatterCliFilter === "all"
        ? scatterRows
        : scatterRows.filter((row) => row.cli_key === scatterCliFilter);

    const byCli = new Map<CliKey, ScatterPoint[]>();
    for (const row of filteredRows) {
      const providerRaw = row.provider_name?.trim() ? row.provider_name.trim() : "Unknown";
      const modelRaw = row.model?.trim() ? row.model.trim() : "Unknown";
      const providerText = providerRaw === "Unknown" ? "未知" : providerRaw;
      const modelText = modelRaw === "Unknown" ? "未知" : modelRaw;
      const cliLabel = cliShortLabel(row.cli_key);
      const name = `${cliLabel} · ${providerText} · ${modelText}`;
      const point: ScatterPoint = {
        name,
        value: [row.total_cost_usd, row.total_duration_ms],
        meta: row,
      };
      const bucket = byCli.get(row.cli_key) ?? [];
      bucket.push(point);
      byCli.set(row.cli_key, bucket);
    }

    const cliOrder: CliKey[] = ["claude", "codex", "gemini"];
    const series = cliOrder
      .map((cli) => ({ cli, points: byCli.get(cli) ?? [] }))
      .filter((item) => item.points.length > 0)
      .map((item) => buildSeries(cliShortLabel(item.cli), item.points));

    const showLegend = series.length > 1;
    const gridTop = showLegend ? 60 : 8;

    return {
      animation: false,
      ...(showLegend
        ? {
            legend: {
              type: "plain",
              top: 4,
              left: 0,
              itemWidth: 10,
              itemHeight: 10,
              itemGap: 16,
              textStyle: { color: "#64748b", fontSize: 10 },
            },
          }
        : {}),
      grid: { left: 0, right: 80, top: gridTop, bottom: 32, containLabel: true },
      tooltip: {
        trigger: "item",
        confine: true,
        formatter: (params: any) => {
          const meta: CostScatterCliProviderModelRowV1 | undefined = params?.data?.meta;
          if (!meta) return "";
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
          return [
            `<div style="font-size:12px;font-weight:600;margin-bottom:4px;">${cliLabel} · ${providerText} · ${modelText}</div>`,
            `<div style="font-size:11px;color:#64748b;">总成本：${formatUsd(meta.total_cost_usd)}</div>`,
            `<div style="font-size:11px;color:#64748b;">总耗时：${formatDurationMs(meta.total_duration_ms)}</div>`,
            `<div style="font-size:11px;color:#64748b;">请求数：${formatInteger(requests)}</div>`,
            avgCostUsd == null
              ? `<div style="font-size:11px;color:#94a3b8;">均值：—</div>`
              : `<div style="font-size:11px;color:#94a3b8;">均值：${formatUsd(avgCostUsd)} / ${formatDurationMs(avgDurationMs ?? 0)}</div>`,
          ].join("");
        },
      },
      xAxis: {
        type: "value",
        name: "总成本(USD)",
        axisLabel: {
          color: "#64748b",
          fontSize: 10,
          formatter: (value: number) => formatUsdShort(value),
        },
        axisPointer: {
          label: { formatter: (p: any) => formatUsd(Number(p?.value)) },
        },
        splitLine: { lineStyle: { color: "rgba(15,23,42,0.08)", type: "dashed" } },
      },
      yAxis: {
        type: "value",
        name: "总耗时",
        axisLabel: {
          color: "#64748b",
          fontSize: 10,
          formatter: (value: number) => formatDurationMsShort(value),
        },
        splitLine: { lineStyle: { color: "rgba(15,23,42,0.08)", type: "dashed" } },
      },
      series,
    };
  }, [scatterCliFilter, scatterRows]);

  const summaryCards = useMemo(() => {
    if (!summary) return [];

    const successHint = `${formatInteger(summary.requests_success)} 成功 · ${formatInteger(
      summary.requests_failed
    )} 失败`;

    const avgCostHint =
      summary.avg_cost_usd_per_covered_success != null
        ? `按已覆盖请求均摊`
        : `暂无可计算的成本均值`;

    return [
      {
        title: "总花费（已计算）",
        value: formatUsd(summary.total_cost_usd),
        hint: successHint,
      },
      {
        title: "成本覆盖率",
        value: coverage == null ? "—" : formatPercent(coverage, 1),
        hint: `${formatInteger(summary.cost_covered_success)} / ${formatInteger(
          summary.requests_success
        )} 成功请求有成本`,
      },
      {
        title: "平均成本 / 成功请求",
        value:
          summary.avg_cost_usd_per_covered_success == null
            ? "—"
            : formatUsd(summary.avg_cost_usd_per_covered_success),
        hint: avgCostHint,
      },
    ];
  }, [coverage, summary]);

  const providerSelectValue = providerId == null ? "all" : String(providerId);
  const modelSelectValue = model == null ? "all" : model;

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
        <Card padding="md" className="lg:col-span-7">
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <div className="text-sm font-semibold text-slate-900">花费</div>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setReloadSeq((v) => v + 1)}
                  disabled={loading}
                >
                  刷新
                </Button>
              </div>
            </div>

            <div className="flex flex-col gap-3">
              <div className="flex items-start gap-2">
                <span className={FILTER_LABEL_CLASS}>CLI：</span>
                <div className={FILTER_OPTIONS_CLASS}>
                  {CLI_ITEMS.map((item) => (
                    <Button
                      key={item.key}
                      size="sm"
                      variant={cliKey === item.key ? "primary" : "secondary"}
                      onClick={() => setCliKey(item.key)}
                      disabled={loading}
                      className={FILTER_OPTION_BUTTON_CLASS}
                    >
                      {item.label}
                    </Button>
                  ))}
                </div>
              </div>

              <div className="flex items-start gap-2">
                <span className={FILTER_LABEL_CLASS}>时间窗：</span>
                <div className={FILTER_OPTIONS_CLASS}>
                  {PERIOD_ITEMS.map((item) => (
                    <Button
                      key={item.key}
                      size="sm"
                      variant={period === item.key ? "primary" : "secondary"}
                      onClick={() => setPeriod(item.key)}
                      disabled={loading}
                      className={FILTER_OPTION_BUTTON_CLASS}
                    >
                      {item.label}
                    </Button>
                  ))}
                </div>
              </div>

              {showCustomForm ? (
                <div className="flex items-start gap-2">
                  <div className="w-16 shrink-0" aria-hidden="true" />
                  <div className="min-w-0 flex flex-1 flex-col gap-2 md:flex-row md:items-end">
                    <div className="flex flex-col gap-1">
                      <div className="text-xs font-medium text-slate-500">Start</div>
                      <Input
                        type="date"
                        value={customStartDate}
                        onChange={(e) => setCustomStartDate(e.currentTarget.value)}
                        className="h-9"
                        disabled={loading}
                      />
                    </div>
                    <div className="flex flex-col gap-1">
                      <div className="text-xs font-medium text-slate-500">End</div>
                      <Input
                        type="date"
                        value={customEndDate}
                        onChange={(e) => setCustomEndDate(e.currentTarget.value)}
                        className="h-9"
                        disabled={loading}
                      />
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        size="sm"
                        variant="primary"
                        onClick={applyCustomRange}
                        disabled={loading}
                      >
                        应用
                      </Button>
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={clearCustomRange}
                        disabled={loading}
                      >
                        清空
                      </Button>
                      {customApplied ? (
                        <span className="text-xs text-slate-500">
                          已应用：{customApplied.startDate} → {customApplied.endDate}
                        </span>
                      ) : (
                        <span className="text-xs text-slate-500">请选择日期范围后点击"应用"</span>
                      )}
                    </div>
                  </div>
                </div>
              ) : null}

              <div className="flex flex-col gap-3 md:flex-row md:items-center">
                <div className="flex items-center gap-2 md:flex-1">
                  <span className={FILTER_LABEL_CLASS}>供应商：</span>
                  <div className={cn(FILTER_OPTIONS_CLASS, "w-full")}>
                    <Select
                      value={providerSelectValue}
                      onChange={(e) => {
                        const v = e.currentTarget.value;
                        if (v === "all") {
                          setProviderId(null);
                          return;
                        }
                        const n = Number(v);
                        if (!Number.isFinite(n) || n <= 0) {
                          setProviderId(null);
                          return;
                        }
                        setProviderId(Math.floor(n));
                      }}
                      disabled={loading || tauriAvailable === false}
                      mono
                      className="h-9"
                    >
                      <option value="all">全部</option>
                      {providerOptions.map((row) => (
                        <option
                          key={`${row.cli_key}:${row.provider_id}`}
                          value={String(row.provider_id)}
                        >
                          {cliShortLabel(row.cli_key)} · {row.provider_name}（
                          {formatUsd(row.cost_usd)}）
                        </option>
                      ))}
                    </Select>
                  </div>
                </div>

                <div className="flex items-center gap-2 md:flex-1">
                  <span className={FILTER_LABEL_CLASS}>模型：</span>
                  <div className={cn(FILTER_OPTIONS_CLASS, "w-full")}>
                    <Select
                      value={modelSelectValue}
                      onChange={(e) => {
                        const v = e.currentTarget.value;
                        setModel(v === "all" ? null : v);
                      }}
                      disabled={loading || tauriAvailable === false}
                      mono
                      className="h-9"
                    >
                      <option value="all">全部</option>
                      {modelOptions.map((row) => (
                        <option key={row.model} value={row.model}>
                          {row.model}（{formatUsd(row.cost_usd)}）
                        </option>
                      ))}
                    </Select>
                  </div>
                </div>
              </div>
            </div>

            {tauriAvailable === false ? (
              <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-600">
                当前环境未检测到 Tauri Runtime。请通过桌面端运行（`pnpm tauri dev`）后查看花费。
              </div>
            ) : null}
          </div>
        </Card>

        <div className="lg:col-span-5 grid grid-cols-2 gap-3 content-start">
          {loading ? (
            Array.from({ length: 3 }).map((_, idx) => <StatCardSkeleton key={idx} />)
          ) : summaryCards.length === 0 ? (
            <Card padding="md">
              <div className="text-sm text-slate-600">
                {period === "custom" && !customApplied
                  ? "自定义范围：请选择日期后点击「应用」。"
                  : "暂无花费数据。"}
              </div>
            </Card>
          ) : (
            summaryCards.map((card) => (
              <StatCard key={card.title} title={card.title} value={card.value} hint={card.hint} />
            ))
          )}
        </div>
      </div>

      {errorText ? (
        <Card padding="md" className="border-rose-200 bg-rose-50">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className="text-sm font-semibold text-rose-900">加载失败</div>
              <div className="mt-1 text-sm text-rose-800">花费数据刷新失败，请重试。</div>
            </div>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setReloadSeq((v) => v + 1)}
              disabled={loading}
              className="border-rose-200 bg-white text-rose-800 hover:bg-rose-50"
            >
              重试
            </Button>
          </div>
          <div className="mt-3 rounded-lg border border-rose-200 bg-white/60 p-3 font-mono text-xs text-slate-800">
            {errorText}
          </div>
        </Card>
      ) : null}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
        <Card padding="sm" className="lg:col-span-7 flex flex-col min-h-[280px]">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-baseline gap-2">
              <span className="text-sm font-semibold text-slate-900">总花费趋势</span>
              <span className="text-xs text-slate-400">
                {period === "daily" ? "按小时" : "按天"}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <div className="flex items-center gap-1">
                {CLI_ITEMS.map((item) => (
                  <button
                    key={item.key}
                    type="button"
                    onClick={() => setCliKey(item.key)}
                    disabled={loading}
                    className={cn(
                      "px-2 py-0.5 text-xs rounded-full transition-colors",
                      cliKey === item.key
                        ? "bg-accent text-white"
                        : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                    )}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
          {loading ? (
            <div className="text-sm text-slate-400">加载中…</div>
          ) : summary && summary.requests_success > 0 ? (
            <div className="min-h-0 flex-1">
              <EChartsCanvas option={trendOption as any} className="h-full" />
            </div>
          ) : (
            <div className="text-sm text-slate-600">暂无可展示的数据。</div>
          )}
        </Card>

        <Card padding="sm" className="lg:col-span-5 flex flex-col min-h-[180px]">
          <div className="mb-2 flex items-center justify-between gap-3">
            <div className="text-sm font-semibold text-slate-900">花费占比</div>
            <div className="text-xs text-slate-500">供应商 / 模型</div>
          </div>
          {loading ? (
            <div className="text-sm text-slate-400">加载中…</div>
          ) : (
            <div className="grid grid-cols-2 gap-4 min-h-0 flex-1">
              <div className="flex flex-col">
                <div className="text-xs font-medium text-slate-600 mb-1">供应商</div>
                <div className="min-h-[140px] flex-1">
                  <EChartsCanvas option={providerDonutOption as any} className="h-full" />
                </div>
              </div>
              <div className="flex flex-col">
                <div className="text-xs font-medium text-slate-600 mb-1">模型</div>
                <div className="min-h-[140px] flex-1">
                  <EChartsCanvas option={modelDonutOption as any} className="h-full" />
                </div>
              </div>
            </div>
          )}
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-start">
        <Card padding="sm" className="lg:col-span-5 flex flex-col lg:h-[600px] min-h-0">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
            <div className="text-sm font-semibold text-slate-900">总成本 × 总耗时</div>
            <div className="flex items-center gap-1">
              {CLI_ITEMS.map((item) => (
                <button
                  key={item.key}
                  type="button"
                  onClick={() => setScatterCliFilter(item.key)}
                  disabled={loading}
                  className={cn(
                    "px-2 py-0.5 text-xs rounded-full transition-colors",
                    scatterCliFilter === item.key
                      ? "bg-accent text-white"
                      : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                  )}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
          {loading ? (
            <div className="text-sm text-slate-400">加载中…</div>
          ) : scatterRows.length === 0 ? (
            <div className="text-sm text-slate-600">暂无可展示的数据。</div>
          ) : (
            <div className="h-[320px] lg:h-auto lg:flex-1 lg:min-h-0">
              <EChartsCanvas option={scatterOption as any} className="h-full" />
            </div>
          )}
        </Card>

        <Card padding="sm" className="lg:col-span-7 flex flex-col lg:h-[600px] min-h-0">
          <div className="flex items-center justify-between gap-4">
            <div className="text-sm font-semibold text-slate-900">Top 50 最贵请求</div>
            <div className="text-xs text-slate-500">点击行查看详情</div>
          </div>

          <div className="mt-3 max-h-[600px] overflow-y-auto relative lg:max-h-none lg:flex-1 lg:min-h-0">
            {loading ? (
              <div className="text-sm text-slate-400">加载中…</div>
            ) : topRequests.length === 0 ? (
              <div className="text-sm text-slate-600">暂无可展示的数据。</div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full border-separate border-spacing-0 text-left text-sm">
                  <thead>
                    <tr className="text-xs text-slate-500">
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2">
                        #
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2">
                        时间
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2">
                        CLI
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2">
                        供应商
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2">
                        模型
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2 text-right">
                        成本
                      </th>
                      <th className="sticky top-0 z-10 border-b border-slate-200 bg-white px-2 py-2 text-right">
                        耗时
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {topRequests.map((row, index) => {
                      const modelText = row.requested_model?.trim()
                        ? row.requested_model.trim()
                        : "未知";
                      const showCostMultiplier =
                        Number.isFinite(row.cost_multiplier) &&
                        row.cost_multiplier > 0 &&
                        Math.abs(row.cost_multiplier - 1) > 0.0001;

                      return (
                        <tr
                          key={row.log_id}
                          className="align-top cursor-pointer hover:bg-slate-50"
                          onClick={() => onSelectLogId(row.log_id)}
                        >
                          <td className="border-b border-slate-100 px-2 py-2 text-xs text-slate-500">
                            {index + 1}
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2 text-xs text-slate-600">
                            {formatRelativeTimeFromUnixSeconds(row.created_at)}
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2">
                            <span
                              className={cn(
                                "inline-flex min-w-[3.25rem] justify-center rounded-full px-2 py-0.5 text-[10px] font-medium",
                                cliBadgeTone(row.cli_key)
                              )}
                            >
                              {cliShortLabel(row.cli_key)}
                            </span>
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2 text-xs text-slate-700">
                            <div className="font-medium">{row.provider_name}</div>
                            {showCostMultiplier ? (
                              <div className="mt-0.5 font-mono text-[10px] text-slate-400">
                                x{row.cost_multiplier.toFixed(2)}
                              </div>
                            ) : null}
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2 text-xs text-slate-700">
                            <span className="font-mono">{modelText}</span>
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2 font-mono text-xs text-slate-700 text-right">
                            {formatUsd(row.cost_usd)}
                          </td>
                          <td className="border-b border-slate-100 px-2 py-2 font-mono text-xs text-slate-700 text-right">
                            {formatDurationMs(row.duration_ms)}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
