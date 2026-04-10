// Usage:
// - Custom hook extracted from HomeCostPanel to manage filter state and data transformations.
// - Encapsulates period, CLI, provider, model filters and all derived useMemo computations.

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { CLI_FILTER_SHORT_ITEMS, cliShortLabel } from "../../constants/clis";
import { useCustomDateRange } from "../../hooks/useCustomDateRange";
import { useTheme } from "../../hooks/useTheme";
import { useCostAnalyticsV1Query } from "../../query/cost";
import type { CliKey } from "../../services/providers/providers";
import type { CostPeriod, CostScatterCliProviderModelRowV1 } from "../../services/usage/cost";
import { buildRecentDayKeys, dayKeyFromLocalDate } from "../../utils/dateKeys";
import { formatInteger, formatPercent, formatUsd } from "../../utils/formatters";
import { pickTopSlices, toDateLabel } from "../../utils/chartHelpers";

export type CliFilter = "all" | CliKey;

export type CliItem = { key: CliFilter; label: string };
export const CLI_ITEMS: CliItem[] = CLI_FILTER_SHORT_ITEMS;

// Scatter chart point type used by CostScatterChart
export type ScatterPoint = {
  name: string;
  shortLabel: string;
  x: number;
  y: number;
  z: number;
  cli: CliKey;
  meta: CostScatterCliProviderModelRowV1;
};

export type SummaryCard = {
  title: string;
  value: string;
  hint: string;
  testId: string;
};

// --- Date utility helpers ---

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

// --- Main hook ---

export function useCostFilters() {
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme === "dark";

  const [period, setPeriod] = useState<CostPeriod>("daily");
  const [cliKey, setCliKey] = useState<CliFilter>("all");
  const [providerId, setProviderId] = useState<number | null>(null);
  const [model, setModel] = useState<string | null>(null);

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

  const queryEnabled = period !== "custom" || Boolean(customApplied);

  const costQuery = useCostAnalyticsV1Query(period, filters, { enabled: queryEnabled });
  const loading = costQuery.isLoading;
  const fetching = costQuery.isFetching;
  const errorText = costQuery.error ? String(costQuery.error) : null;

  const tauriAvailable: boolean | null = !queryEnabled
    ? null
    : loading
      ? null
      : costQuery.data != null;

  const summary = costQuery.data?.summary ?? null;
  const trendRows = useMemo(() => costQuery.data?.trend ?? [], [costQuery.data?.trend]);
  const providerRows = useMemo(() => costQuery.data?.providers ?? [], [costQuery.data?.providers]);
  const modelRows = useMemo(() => costQuery.data?.models ?? [], [costQuery.data?.models]);
  const scatterRows = useMemo(() => costQuery.data?.scatter ?? [], [costQuery.data?.scatter]);

  useEffect(() => {
    if (!costQuery.error) return;
    toast("加载花费失败：请重试（详情见页面错误信息）");
  }, [costQuery.error]);

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

  const trendChartData = useMemo(() => {
    const isHourly = period === "daily";

    if (isHourly) {
      const byHour = new Map<number, number>();
      for (const row of trendRows) {
        if (row.hour == null) continue;
        byHour.set(row.hour, Number(row.cost_usd) || 0);
      }
      return Array.from({ length: 24 }).map((_, h) => ({
        label: String(h).padStart(2, "0"),
        cost: byHour.get(h) ?? 0,
      }));
    }

    const byDay = new Map<string, number>();
    for (const row of trendRows) {
      byDay.set(row.day, Number(row.cost_usd) || 0);
    }
    return trendDayKeys.map((d) => ({
      label: toDateLabel(d),
      cost: byDay.get(d) ?? 0,
    }));
  }, [period, trendDayKeys, trendRows]);

  const providerDonutData = useMemo(() => {
    const filtered = providerRows.filter((row) => row.cost_usd > 0);
    const { head, tailSum } = pickTopSlices(filtered, 7);
    const seriesData = head.map((row) => ({
      name: `${cliShortLabel(row.cli_key)} · ${row.provider_name}`,
      value: row.cost_usd,
    }));
    if (tailSum > 0) seriesData.push({ name: "其他", value: tailSum });

    const total = seriesData.reduce((sum, d) => sum + d.value, 0);
    return { data: seriesData, total };
  }, [providerRows]);

  const modelDonutData = useMemo(() => {
    const filtered = modelRows.filter((row) => row.cost_usd > 0);
    const { head, tailSum } = pickTopSlices(filtered, 7);
    const seriesData = head.map((row) => ({
      name: row.model,
      value: row.cost_usd,
    }));
    if (tailSum > 0) seriesData.push({ name: "其他", value: tailSum });

    const total = seriesData.reduce((sum, d) => sum + d.value, 0);
    return { data: seriesData, total };
  }, [modelRows]);

  const scatterChartData = useMemo<{ data: ScatterPoint[]; activeClis: CliKey[] }>(() => {
    const symbolSize = (costForSizing: number) => {
      const size = 10 + Math.log10(1 + Math.max(0, costForSizing)) * 10;
      return Math.max(10, Math.min(26, size));
    };

    const filteredRows =
      scatterCliFilter === "all"
        ? scatterRows
        : scatterRows.filter((row) => row.cli_key === scatterCliFilter);

    const points: ScatterPoint[] = filteredRows.map((row) => {
      const providerRaw = row.provider_name?.trim() ? row.provider_name.trim() : "Unknown";
      const modelRaw = row.model?.trim() ? row.model.trim() : "Unknown";
      const providerText = providerRaw === "Unknown" ? "未知" : providerRaw;
      const modelText = modelRaw === "Unknown" ? "未知" : modelRaw;
      const cliLabel = cliShortLabel(row.cli_key);

      return {
        name: `${cliLabel} · ${providerText} · ${modelText}`,
        shortLabel: modelText,
        x: row.total_cost_usd,
        y: row.total_duration_ms,
        z: symbolSize(row.total_cost_usd),
        cli: row.cli_key,
        meta: row,
      };
    });

    const uniqueClis = new Set(points.map((p) => p.cli));
    const activeClis = (["claude", "codex", "gemini"] as CliKey[]).filter((c) => uniqueClis.has(c));

    return { data: points, activeClis };
  }, [scatterCliFilter, scatterRows]);

  const summaryCards = useMemo<SummaryCard[]>(() => {
    if (!summary) return [];

    const successHint = `${formatInteger(summary.requests_success)} 成功 · ${formatInteger(
      summary.requests_failed
    )} 失败`;

    return [
      {
        title: "总花费（已计算）",
        value: formatUsd(summary.total_cost_usd),
        hint: successHint,
        testId: "home-cost-total-cost",
      },
      {
        title: "成本覆盖率",
        value: coverage == null ? "—" : formatPercent(coverage, 1),
        hint: `${formatInteger(summary.cost_covered_success)} / ${formatInteger(
          summary.requests_success
        )} 成功请求有成本`,
        testId: "home-cost-coverage",
      },
    ];
  }, [coverage, summary]);

  const providerSelectValue = providerId == null ? "all" : String(providerId);
  const modelSelectValue = model == null ? "all" : model;

  return {
    // Theme
    isDark,
    // Period
    period,
    setPeriod,
    // CLI filter
    cliKey,
    setCliKey,
    // Provider filter
    providerId,
    setProviderId,
    providerOptions,
    providerSelectValue,
    // Model filter
    model,
    setModel,
    modelOptions,
    modelSelectValue,
    // Custom date range
    customStartDate,
    setCustomStartDate,
    customEndDate,
    setCustomEndDate,
    customApplied,
    showCustomForm,
    applyCustomRange,
    clearCustomRange,
    // Scatter CLI filter
    scatterCliFilter,
    setScatterCliFilter,
    // Query state
    costQuery,
    loading,
    fetching,
    errorText,
    tauriAvailable,
    // Data
    summary,
    scatterRows,
    // Derived chart data
    trendChartData,
    providerDonutData,
    modelDonutData,
    scatterChartData,
    summaryCards,
  };
}
