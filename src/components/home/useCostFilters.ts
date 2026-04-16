// Usage:
// - Custom hook extracted from HomeCostPanel to manage filter state and data transformations.
// - Encapsulates period, CLI, provider, model filters and all derived useMemo computations.

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { CLI_FILTER_SHORT_ITEMS, cliShortLabel } from "../../constants/clis";
import { useCustomDateRange } from "../../hooks/useCustomDateRange";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useTheme } from "../../hooks/useTheme";
import { useCostAnalyticsV1Query } from "../../query/cost";
import type { CostAnalyticsV1 } from "../../query/cost";
import type { CliKey } from "../../services/providers/providers";
import type {
  CostModelBreakdownRowV1,
  CostPeriod,
  CostProviderBreakdownRowV1,
  CostScatterCliProviderModelRowV1,
  CostSummaryV1,
  CostTopRequestRowV1,
  CostTrendRowV1,
} from "../../services/usage/cost";
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

type UseCostFiltersOptions = {
  devPreviewEnabled?: boolean;
};

type CostQueryFilters = {
  startTs: number | null;
  endTs: number | null;
  cliKey: CliKey | null;
  providerId: number | null;
  model: string | null;
};

type PreviewCostRecord = {
  log_id: number;
  trace_id: string;
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  model: string;
  requested_model: string;
  method: string;
  path: string;
  created_at: number;
  day: string;
  hour: number;
  duration_ms: number;
  ttfb_ms: number;
  cost_usd: number;
  cost_multiplier: number;
  success: boolean;
  cost_covered: boolean;
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

function createPreviewCostRecord(
  now: Date,
  input: {
    logId: number;
    traceId: string;
    dayOffset: number;
    hour: number;
    minute?: number;
    cliKey: CliKey;
    providerId: number;
    providerName: string;
    model: string;
    costUsd: number;
    durationMs: number;
    ttfbMs: number;
    success?: boolean;
    costCovered?: boolean;
    costMultiplier?: number;
    method?: string;
    path?: string;
  }
): PreviewCostRecord {
  const date = new Date(now);
  date.setDate(date.getDate() + input.dayOffset);
  date.setHours(input.hour, input.minute ?? 0, 0, 0);

  return {
    log_id: input.logId,
    trace_id: input.traceId,
    cli_key: input.cliKey,
    provider_id: input.providerId,
    provider_name: input.providerName,
    model: input.model,
    requested_model: input.model,
    method: input.method ?? "POST",
    path: input.path ?? "/v1/messages",
    created_at: Math.floor(date.getTime() / 1000),
    day: dayKeyFromLocalDate(date),
    hour: date.getHours(),
    duration_ms: input.durationMs,
    ttfb_ms: input.ttfbMs,
    cost_usd: input.costUsd,
    cost_multiplier: input.costMultiplier ?? 1,
    success: input.success ?? true,
    cost_covered: input.costCovered ?? true,
  };
}

function buildPreviewCostRecords(now = new Date()): PreviewCostRecord[] {
  return [
    createPreviewCostRecord(now, {
      logId: 9001,
      traceId: "preview-cost-claude-main-today",
      dayOffset: 0,
      hour: 9,
      minute: 12,
      cliKey: "claude",
      providerId: 101,
      providerName: "Claude Main",
      model: "claude-3.7-sonnet",
      costUsd: 1.84,
      durationMs: 1180,
      ttfbMs: 290,
    }),
    createPreviewCostRecord(now, {
      logId: 9002,
      traceId: "preview-cost-openai-primary-today",
      dayOffset: 0,
      hour: 10,
      minute: 28,
      cliKey: "codex",
      providerId: 201,
      providerName: "OpenAI Primary",
      model: "gpt-5.4",
      costUsd: 1.22,
      durationMs: 920,
      ttfbMs: 240,
    }),
    createPreviewCostRecord(now, {
      logId: 9003,
      traceId: "preview-cost-gemini-pro-today",
      dayOffset: 0,
      hour: 11,
      minute: 36,
      cliKey: "gemini",
      providerId: 301,
      providerName: "Gemini Mirror",
      model: "gemini-2.5-pro",
      costUsd: 0.66,
      durationMs: 840,
      ttfbMs: 210,
    }),
    createPreviewCostRecord(now, {
      logId: 9004,
      traceId: "preview-cost-openai-uncovered-today",
      dayOffset: 0,
      hour: 13,
      minute: 8,
      cliKey: "codex",
      providerId: 201,
      providerName: "OpenAI Primary",
      model: "gpt-5.4",
      costUsd: 0,
      durationMs: 1010,
      ttfbMs: 260,
      costCovered: false,
    }),
    createPreviewCostRecord(now, {
      logId: 9005,
      traceId: "preview-cost-claude-fallback-today",
      dayOffset: 0,
      hour: 15,
      minute: 42,
      cliKey: "claude",
      providerId: 102,
      providerName: "Claude Fallback",
      model: "claude-3.5-haiku",
      costUsd: 0.44,
      durationMs: 760,
      ttfbMs: 220,
    }),
    createPreviewCostRecord(now, {
      logId: 9006,
      traceId: "preview-cost-gemini-failed-today",
      dayOffset: 0,
      hour: 19,
      minute: 4,
      cliKey: "gemini",
      providerId: 301,
      providerName: "Gemini Mirror",
      model: "gemini-2.5-flash",
      costUsd: 0,
      durationMs: 690,
      ttfbMs: 180,
      success: false,
      costCovered: false,
    }),
    createPreviewCostRecord(now, {
      logId: 9007,
      traceId: "preview-cost-claude-main-yesterday",
      dayOffset: -1,
      hour: 8,
      minute: 24,
      cliKey: "claude",
      providerId: 101,
      providerName: "Claude Main",
      model: "claude-3.7-sonnet",
      costUsd: 2.16,
      durationMs: 1250,
      ttfbMs: 320,
    }),
    createPreviewCostRecord(now, {
      logId: 9008,
      traceId: "preview-cost-openai-primary-yesterday",
      dayOffset: -1,
      hour: 12,
      minute: 16,
      cliKey: "codex",
      providerId: 201,
      providerName: "OpenAI Primary",
      model: "gpt-5.4",
      costUsd: 1.48,
      durationMs: 960,
      ttfbMs: 250,
    }),
    createPreviewCostRecord(now, {
      logId: 9009,
      traceId: "preview-cost-gemini-yesterday",
      dayOffset: -1,
      hour: 18,
      minute: 14,
      cliKey: "gemini",
      providerId: 301,
      providerName: "Gemini Mirror",
      model: "gemini-2.5-pro",
      costUsd: 0.57,
      durationMs: 870,
      ttfbMs: 230,
    }),
    createPreviewCostRecord(now, {
      logId: 9010,
      traceId: "preview-cost-claude-fallback-two-days",
      dayOffset: -2,
      hour: 14,
      minute: 30,
      cliKey: "claude",
      providerId: 102,
      providerName: "Claude Fallback",
      model: "claude-3.5-haiku",
      costUsd: 0.38,
      durationMs: 780,
      ttfbMs: 210,
    }),
    createPreviewCostRecord(now, {
      logId: 9011,
      traceId: "preview-cost-openai-fourone",
      dayOffset: -3,
      hour: 16,
      minute: 45,
      cliKey: "codex",
      providerId: 201,
      providerName: "OpenAI Primary",
      model: "gpt-4.1",
      costUsd: 1.74,
      durationMs: 1080,
      ttfbMs: 280,
    }),
    createPreviewCostRecord(now, {
      logId: 9012,
      traceId: "preview-cost-gemini-flash",
      dayOffset: -4,
      hour: 10,
      minute: 6,
      cliKey: "gemini",
      providerId: 301,
      providerName: "Gemini Mirror",
      model: "gemini-2.5-flash",
      costUsd: 0.24,
      durationMs: 720,
      ttfbMs: 190,
    }),
    createPreviewCostRecord(now, {
      logId: 9013,
      traceId: "preview-cost-claude-main-older",
      dayOffset: -5,
      hour: 9,
      minute: 18,
      cliKey: "claude",
      providerId: 101,
      providerName: "Claude Main",
      model: "claude-3.7-sonnet",
      costUsd: 1.92,
      durationMs: 1210,
      ttfbMs: 300,
    }),
    createPreviewCostRecord(now, {
      logId: 9014,
      traceId: "preview-cost-openai-fourone-older",
      dayOffset: -6,
      hour: 21,
      minute: 2,
      cliKey: "codex",
      providerId: 201,
      providerName: "OpenAI Primary",
      model: "gpt-4.1",
      costUsd: 1.31,
      durationMs: 990,
      ttfbMs: 260,
    }),
    createPreviewCostRecord(now, {
      logId: 9015,
      traceId: "preview-cost-gemini-pro-older",
      dayOffset: -10,
      hour: 10,
      minute: 34,
      cliKey: "gemini",
      providerId: 301,
      providerName: "Gemini Mirror",
      model: "gemini-2.5-pro",
      costUsd: 0.73,
      durationMs: 880,
      ttfbMs: 220,
    }),
  ];
}

function isCostAnalyticsEmpty(data: CostAnalyticsV1 | null | undefined) {
  if (!data) return true;
  return (
    data.summary.requests_total <= 0 &&
    data.trend.length === 0 &&
    data.providers.length === 0 &&
    data.models.length === 0 &&
    data.scatter.length === 0 &&
    data.topRequests.length === 0
  );
}

function matchesPreviewCostPeriod(
  record: PreviewCostRecord,
  period: CostPeriod,
  customApplied: CustomDateRangeApplied | null,
  now: Date
) {
  if (period === "allTime") return true;
  if (period === "custom") {
    if (!customApplied) return false;
    return record.created_at >= customApplied.startTs && record.created_at < customApplied.endTs;
  }
  if (period === "daily") return record.day === dayKeyFromLocalDate(now);
  if (period === "weekly") return buildRecentDayKeys(7).includes(record.day);
  if (period === "monthly") {
    const recordDate = new Date(record.created_at * 1000);
    return (
      recordDate.getFullYear() === now.getFullYear() && recordDate.getMonth() === now.getMonth()
    );
  }
  return true;
}

function buildPreviewCostAnalytics(
  period: CostPeriod,
  filters: CostQueryFilters,
  customApplied: CustomDateRangeApplied | null
): CostAnalyticsV1 {
  const now = new Date();
  const records = buildPreviewCostRecords(now)
    .filter((record) => matchesPreviewCostPeriod(record, period, customApplied, now))
    .filter((record) => (filters.cliKey == null ? true : record.cli_key === filters.cliKey))
    .filter((record) =>
      filters.providerId == null ? true : record.provider_id === filters.providerId
    )
    .filter((record) => (filters.model == null ? true : record.model === filters.model));

  const successRecords = records.filter((record) => record.success);
  const coveredSuccessRecords = successRecords.filter((record) => record.cost_covered);

  const summary: CostSummaryV1 = {
    requests_total: records.length,
    requests_success: successRecords.length,
    requests_failed: records.length - successRecords.length,
    cost_covered_success: coveredSuccessRecords.length,
    total_cost_usd: coveredSuccessRecords.reduce((sum, record) => sum + record.cost_usd, 0),
    avg_cost_usd_per_covered_success:
      coveredSuccessRecords.length > 0
        ? coveredSuccessRecords.reduce((sum, record) => sum + record.cost_usd, 0) /
          coveredSuccessRecords.length
        : null,
  };

  const trendMap = new Map<string, CostTrendRowV1>();
  for (const record of records) {
    const bucketKey = period === "daily" ? String(record.hour) : record.day;
    const existing = trendMap.get(bucketKey) ?? {
      day: record.day,
      hour: period === "daily" ? record.hour : null,
      cost_usd: 0,
      requests_success: 0,
      cost_covered_success: 0,
    };
    if (record.success) existing.requests_success += 1;
    if (record.success && record.cost_covered) {
      existing.cost_covered_success += 1;
      existing.cost_usd += record.cost_usd;
    }
    trendMap.set(bucketKey, existing);
  }

  const trend = Array.from(trendMap.values()).sort((a, b) => {
    if (period === "daily") return (a.hour ?? 0) - (b.hour ?? 0);
    return a.day.localeCompare(b.day);
  });

  const providerMap = new Map<string, CostProviderBreakdownRowV1>();
  for (const record of successRecords) {
    const key = `${record.cli_key}:${record.provider_id}`;
    const existing = providerMap.get(key) ?? {
      cli_key: record.cli_key,
      provider_id: record.provider_id,
      provider_name: record.provider_name,
      requests_success: 0,
      cost_covered_success: 0,
      cost_usd: 0,
    };
    existing.requests_success += 1;
    if (record.cost_covered) {
      existing.cost_covered_success += 1;
      existing.cost_usd += record.cost_usd;
    }
    providerMap.set(key, existing);
  }
  const providers = Array.from(providerMap.values()).sort((a, b) => b.cost_usd - a.cost_usd);

  const modelMap = new Map<string, CostModelBreakdownRowV1>();
  for (const record of successRecords) {
    const existing = modelMap.get(record.model) ?? {
      model: record.model,
      requests_success: 0,
      cost_covered_success: 0,
      cost_usd: 0,
    };
    existing.requests_success += 1;
    if (record.cost_covered) {
      existing.cost_covered_success += 1;
      existing.cost_usd += record.cost_usd;
    }
    modelMap.set(record.model, existing);
  }
  const models = Array.from(modelMap.values()).sort((a, b) => b.cost_usd - a.cost_usd);

  const scatterMap = new Map<string, CostScatterCliProviderModelRowV1>();
  for (const record of successRecords) {
    const key = `${record.cli_key}:${record.provider_id}:${record.model}`;
    const existing = scatterMap.get(key) ?? {
      cli_key: record.cli_key,
      provider_name: record.provider_name,
      model: record.model,
      requests_success: 0,
      total_cost_usd: 0,
      total_duration_ms: 0,
    };
    existing.requests_success += 1;
    existing.total_duration_ms += record.duration_ms;
    if (record.cost_covered) existing.total_cost_usd += record.cost_usd;
    scatterMap.set(key, existing);
  }
  const scatter = Array.from(scatterMap.values()).sort(
    (a, b) => b.total_cost_usd - a.total_cost_usd
  );

  const topRequests: CostTopRequestRowV1[] = coveredSuccessRecords
    .slice()
    .sort((a, b) => b.cost_usd - a.cost_usd)
    .slice(0, 50)
    .map((record) => ({
      log_id: record.log_id,
      trace_id: record.trace_id,
      cli_key: record.cli_key,
      method: record.method,
      path: record.path,
      requested_model: record.requested_model,
      provider_id: record.provider_id,
      provider_name: record.provider_name,
      duration_ms: record.duration_ms,
      ttfb_ms: record.ttfb_ms,
      cost_usd: record.cost_usd,
      cost_multiplier: record.cost_multiplier,
      created_at: record.created_at,
    }));

  return {
    summary,
    trend,
    providers,
    models,
    scatter,
    topRequests,
  };
}

// --- Main hook ---

export function useCostFilters({ devPreviewEnabled = false }: UseCostFiltersOptions = {}) {
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
  const previewActive =
    devPreviewEnabled && queryEnabled && !loading && isCostAnalyticsEmpty(costQuery.data);

  const analytics = useMemo<CostAnalyticsV1 | null>(() => {
    if (previewActive) {
      return buildPreviewCostAnalytics(period, filters, customApplied);
    }
    return costQuery.data ?? null;
  }, [costQuery.data, customApplied, filters, period, previewActive]);

  const tauriAvailable: boolean | null = !queryEnabled ? null : loading ? null : analytics != null;

  const summary = analytics?.summary ?? null;
  const trendRows = useMemo(() => analytics?.trend ?? [], [analytics?.trend]);
  const providerRows = useMemo(() => analytics?.providers ?? [], [analytics?.providers]);
  const modelRows = useMemo(() => analytics?.models ?? [], [analytics?.models]);
  const scatterRows = useMemo(() => analytics?.scatter ?? [], [analytics?.scatter]);

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
