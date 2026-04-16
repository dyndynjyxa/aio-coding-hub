import { useCallback, useMemo, useState } from "react";
import type { UsageLeaderboardRow, UsagePeriod, UsageSummary } from "../../services/usage/usage";
import { useUsageLeaderboardV2Query, useUsageSummaryV2Query } from "../../query/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Spinner } from "../../ui/Spinner";
import { TabList, type TabListItem } from "../../ui/TabList";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { formatUnknownError } from "../../utils/errors";
import {
  formatInteger,
  formatPercent,
  formatTokensPerSecond,
  formatUsdCompact,
} from "../../utils/formatters";
import { StatCard, StatCardSkeleton } from "../usage/StatCard";
import { TokenBreakdown } from "../usage/TokenBreakdown";

type TokenCostScope = "provider" | "model";
type TokenCostRange = "today" | "last3" | "last7" | "last15" | "last30" | "month";

const TOKEN_COST_SCOPE_ITEMS = [
  { key: "provider", label: "供应商" },
  { key: "model", label: "模型" },
] satisfies Array<TabListItem<TokenCostScope>>;

const TOKEN_COST_RANGE_ITEMS = [
  { key: "today", label: "今天" },
  { key: "last3", label: "最近3天" },
  { key: "last7", label: "最近7天" },
  { key: "last15", label: "最近15天" },
  { key: "last30", label: "最近30天" },
  { key: "month", label: "当月" },
] as const satisfies ReadonlyArray<{ key: TokenCostRange; label: string }>;

const TABLE_TH_CLASS =
  "border-b border-slate-200 dark:border-slate-700 bg-slate-50/70 dark:bg-slate-800/70 px-3 py-2.5 text-left text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400";
const TABLE_TD_CLASS = "border-b border-slate-100 dark:border-slate-700 px-3 py-3";
const TABLE_MONO_TD_CLASS =
  "border-b border-slate-100 dark:border-slate-700 px-3 py-3 font-mono text-xs tabular-nums text-slate-700 dark:text-slate-300";

const EMPTY_ROWS: UsageLeaderboardRow[] = [];
const SUMMARY_SKELETON_KEYS = [0, 1, 2, 3, 4];

const PREVIEW_TOKEN_PROVIDER_ROWS: UsageLeaderboardRow[] = [
  {
    key: "provider:201",
    name: "OpenAI Primary",
    requests_total: 18,
    requests_success: 17,
    requests_failed: 1,
    total_tokens: 49_200,
    io_total_tokens: 42_000,
    input_tokens: 28_000,
    output_tokens: 14_000,
    cache_creation_input_tokens: 2_600,
    cache_read_input_tokens: 4_600,
    avg_duration_ms: 980,
    avg_ttfb_ms: 240,
    avg_output_tokens_per_second: 96.5,
    cost_usd: 1.38,
  },
  {
    key: "provider:101",
    name: "Claude Main",
    requests_total: 15,
    requests_success: 14,
    requests_failed: 1,
    total_tokens: 41_400,
    io_total_tokens: 33_000,
    input_tokens: 21_000,
    output_tokens: 12_000,
    cache_creation_input_tokens: 2_100,
    cache_read_input_tokens: 6_300,
    avg_duration_ms: 1_120,
    avg_ttfb_ms: 310,
    avg_output_tokens_per_second: 84.2,
    cost_usd: 1.16,
  },
  {
    key: "provider:301",
    name: "Gemini Mirror",
    requests_total: 12,
    requests_success: 11,
    requests_failed: 1,
    total_tokens: 28_600,
    io_total_tokens: 24_000,
    input_tokens: 15_000,
    output_tokens: 9_000,
    cache_creation_input_tokens: 1_200,
    cache_read_input_tokens: 3_400,
    avg_duration_ms: 860,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 105.7,
    cost_usd: 0.82,
  },
];

const PREVIEW_TOKEN_MODEL_ROWS: UsageLeaderboardRow[] = [
  {
    key: "model:gpt-5.4",
    name: "gpt-5.4",
    requests_total: 14,
    requests_success: 13,
    requests_failed: 1,
    total_tokens: 37_100,
    io_total_tokens: 32_000,
    input_tokens: 21_000,
    output_tokens: 11_000,
    cache_creation_input_tokens: 1_900,
    cache_read_input_tokens: 3_200,
    avg_duration_ms: 930,
    avg_ttfb_ms: 230,
    avg_output_tokens_per_second: 98.4,
    cost_usd: 1.12,
  },
  {
    key: "model:claude-3.7-sonnet",
    name: "claude-3.7-sonnet",
    requests_total: 11,
    requests_success: 10,
    requests_failed: 1,
    total_tokens: 29_800,
    io_total_tokens: 24_000,
    input_tokens: 15_000,
    output_tokens: 9_000,
    cache_creation_input_tokens: 1_500,
    cache_read_input_tokens: 4_300,
    avg_duration_ms: 1_180,
    avg_ttfb_ms: 320,
    avg_output_tokens_per_second: 82.1,
    cost_usd: 0.86,
  },
  {
    key: "model:gemini-2.5-pro",
    name: "gemini-2.5-pro",
    requests_total: 8,
    requests_success: 7,
    requests_failed: 1,
    total_tokens: 17_900,
    io_total_tokens: 15_000,
    input_tokens: 9_000,
    output_tokens: 6_000,
    cache_creation_input_tokens: 800,
    cache_read_input_tokens: 2_100,
    avg_duration_ms: 900,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 97.8,
    cost_usd: 0.48,
  },
  {
    key: "model:gpt-4.1",
    name: "gpt-4.1",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 12_100,
    io_total_tokens: 10_000,
    input_tokens: 7_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 700,
    cache_read_input_tokens: 1_400,
    avg_duration_ms: 1_090,
    avg_ttfb_ms: 270,
    avg_output_tokens_per_second: 87.9,
    cost_usd: 0.33,
  },
  {
    key: "model:claude-3.5-haiku",
    name: "claude-3.5-haiku",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 11_600,
    io_total_tokens: 9_000,
    input_tokens: 6_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 600,
    cache_read_input_tokens: 2_000,
    avg_duration_ms: 910,
    avg_ttfb_ms: 230,
    avg_output_tokens_per_second: 92.7,
    cost_usd: 0.31,
  },
  {
    key: "model:gemini-2.5-flash",
    name: "gemini-2.5-flash",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 10_700,
    io_total_tokens: 9_000,
    input_tokens: 6_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 400,
    cache_read_input_tokens: 1_300,
    avg_duration_ms: 780,
    avg_ttfb_ms: 190,
    avg_output_tokens_per_second: 118.6,
    cost_usd: 0.26,
  },
];

function weightedAverage(
  rows: UsageLeaderboardRow[],
  value: (row: UsageLeaderboardRow) => number | null,
  weight: (row: UsageLeaderboardRow) => number
) {
  const totalWeight = rows.reduce((sum, row) => sum + Math.max(0, weight(row)), 0);
  if (totalWeight <= 0) return null;
  const totalValue = rows.reduce((sum, row) => {
    const current = value(row);
    if (current == null || !Number.isFinite(current)) return sum;
    return sum + current * Math.max(0, weight(row));
  }, 0);
  return totalValue / totalWeight;
}

function buildPreviewTokenSummary(rows: UsageLeaderboardRow[]): UsageSummary {
  const requestsTotal = rows.reduce((sum, row) => sum + row.requests_total, 0);
  const requestsSuccess = rows.reduce((sum, row) => sum + row.requests_success, 0);
  const requestsFailed = rows.reduce((sum, row) => sum + row.requests_failed, 0);
  const inputTokens = rows.reduce((sum, row) => sum + row.input_tokens, 0);
  const outputTokens = rows.reduce((sum, row) => sum + row.output_tokens, 0);
  const ioTotalTokens = rows.reduce((sum, row) => sum + row.io_total_tokens, 0);
  const totalTokens = rows.reduce((sum, row) => sum + row.total_tokens, 0);
  const cacheCreationTokens = rows.reduce((sum, row) => sum + row.cache_creation_input_tokens, 0);
  const cacheReadTokens = rows.reduce((sum, row) => sum + row.cache_read_input_tokens, 0);

  return {
    requests_total: requestsTotal,
    requests_with_usage: requestsTotal,
    requests_success: requestsSuccess,
    requests_failed: requestsFailed,
    avg_duration_ms: weightedAverage(
      rows,
      (row) => row.avg_duration_ms,
      (row) => row.requests_total
    ),
    avg_ttfb_ms: weightedAverage(
      rows,
      (row) => row.avg_ttfb_ms,
      (row) => row.requests_total
    ),
    avg_output_tokens_per_second: weightedAverage(
      rows,
      (row) => row.avg_output_tokens_per_second,
      (row) => row.output_tokens
    ),
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    io_total_tokens: ioTotalTokens,
    total_tokens: totalTokens,
    cache_read_input_tokens: cacheReadTokens,
    cache_creation_input_tokens: cacheCreationTokens,
    cache_creation_5m_input_tokens: Math.round(cacheCreationTokens * 0.68),
  };
}

type HomeTokenCostPanelProps = {
  devPreviewEnabled?: boolean;
};

type TokenCostQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: null;
  providerId: null;
};

type TokenCostQueryConfig = {
  label: string;
  period: UsagePeriod;
  input: TokenCostQueryInput;
  previewFactor: number;
};

function scopeLabel(scope: TokenCostScope) {
  return scope === "provider" ? "供应商" : "模型";
}

function rangeLabel(range: TokenCostRange) {
  return TOKEN_COST_RANGE_ITEMS.find((item) => item.key === range)?.label ?? "今天";
}

function formatTokenValue(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  return formatTokensMillions(value);
}

function formatCostValue(value: number | null | undefined) {
  return formatUsdCompact(value);
}

function successRate(row: UsageLeaderboardRow) {
  if (row.requests_total <= 0) return NaN;
  return row.requests_success / row.requests_total;
}

function tokenShare(row: UsageLeaderboardRow, summary: UsageSummary | null) {
  if (!summary || summary.io_total_tokens <= 0) return 0;
  return row.io_total_tokens / summary.io_total_tokens;
}

function isUsageSummaryEmpty(summary: UsageSummary | null) {
  return !summary || summary.requests_total <= 0 || summary.io_total_tokens <= 0;
}

function isUsageLeaderboardEmpty(rows: UsageLeaderboardRow[]) {
  return (
    rows.length === 0 || rows.every((row) => row.requests_total <= 0 || row.io_total_tokens <= 0)
  );
}

function unixSecondsFromDate(date: Date) {
  return Math.floor(date.getTime() / 1000);
}

function startOfLocalDay(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
}

function addLocalDays(date: Date, days: number) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days, 0, 0, 0, 0);
}

function emptyTokenCostQueryInput(): TokenCostQueryInput {
  return {
    startTs: null,
    endTs: null,
    cliKey: null,
    providerId: null,
  };
}

function buildTokenCostQueryConfig(range: TokenCostRange, now = new Date()): TokenCostQueryConfig {
  const todayStart = startOfLocalDay(now);
  const tomorrowStart = addLocalDays(todayStart, 1);

  switch (range) {
    case "last3":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -2)),
          endTs: unixSecondsFromDate(tomorrowStart),
        },
        previewFactor: 3,
      };
    case "last7":
      return {
        label: rangeLabel(range),
        period: "weekly",
        input: emptyTokenCostQueryInput(),
        previewFactor: 7,
      };
    case "last15":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -14)),
          endTs: unixSecondsFromDate(tomorrowStart),
        },
        previewFactor: 15,
      };
    case "last30":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -29)),
          endTs: unixSecondsFromDate(tomorrowStart),
        },
        previewFactor: 30,
      };
    case "month":
      return {
        label: rangeLabel(range),
        period: "monthly",
        input: emptyTokenCostQueryInput(),
        previewFactor: Math.max(1, now.getDate()),
      };
    case "today":
    default:
      return {
        label: rangeLabel("today"),
        period: "daily",
        input: emptyTokenCostQueryInput(),
        previewFactor: 1,
      };
  }
}

function scaleUsageCount(value: number, factor: number) {
  return Math.max(0, Math.round(value * factor));
}

function scalePreviewTokenRows(rows: UsageLeaderboardRow[], factor: number): UsageLeaderboardRow[] {
  return rows.map((row) => {
    const requestsTotal = scaleUsageCount(row.requests_total, factor);
    const requestsFailed = Math.min(requestsTotal, scaleUsageCount(row.requests_failed, factor));
    const requestsSuccess = Math.max(0, requestsTotal - requestsFailed);

    return {
      ...row,
      requests_total: requestsTotal,
      requests_success: requestsSuccess,
      requests_failed: requestsFailed,
      total_tokens: scaleUsageCount(row.total_tokens, factor),
      io_total_tokens: scaleUsageCount(row.io_total_tokens, factor),
      input_tokens: scaleUsageCount(row.input_tokens, factor),
      output_tokens: scaleUsageCount(row.output_tokens, factor),
      cache_creation_input_tokens: scaleUsageCount(row.cache_creation_input_tokens, factor),
      cache_read_input_tokens: scaleUsageCount(row.cache_read_input_tokens, factor),
      cost_usd: row.cost_usd == null ? null : row.cost_usd * factor,
    };
  });
}

function totalCostUsdFromRows(rows: UsageLeaderboardRow[]) {
  let hasFiniteCost = false;
  const total = rows.reduce((sum, row) => {
    if (row.cost_usd == null || !Number.isFinite(row.cost_usd)) return sum;
    hasFiniteCost = true;
    return sum + Math.max(0, row.cost_usd);
  }, 0);
  return hasFiniteCost ? total : null;
}

function TokenShareBar({ percent }: { percent: number }) {
  const pct = Number.isFinite(percent) ? Math.max(0, Math.min(1, percent)) : 0;
  const displayPct = (pct * 100).toFixed(1);

  return (
    <div
      className="flex items-center gap-1.5"
      role="progressbar"
      aria-valuenow={Number(displayPct)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={`Token 占比 ${displayPct}%`}
    >
      <div className="h-1.5 flex-1 rounded-full bg-slate-100 dark:bg-slate-700">
        <div
          className="h-full rounded-full bg-sky-500 transition-all duration-300"
          style={{ width: `${pct * 100}%` }}
        />
      </div>
      <span className="w-10 text-right text-[10px] tabular-nums text-slate-500 dark:text-slate-400">
        {displayPct}%
      </span>
    </div>
  );
}

function TokenSummaryCards({
  summary,
  rows,
  totalCostUsd,
  scope,
  loading,
}: {
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  totalCostUsd: number | null;
  scope: TokenCostScope;
  loading: boolean;
}) {
  if (loading && !summary) {
    return (
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
        {SUMMARY_SKELETON_KEYS.map((key) => (
          <StatCardSkeleton key={key} />
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
      <StatCard
        title="含缓存总 Token"
        value={formatTokenValue(summary?.total_tokens)}
        accent="purple"
      />
      <StatCard title="总 Token" value={formatTokenValue(summary?.io_total_tokens)} accent="blue" />
      <StatCard title="总花费" value={formatCostValue(totalCostUsd)} accent="orange" />
      <StatCard title="成功请求" value={formatInteger(summary?.requests_success)} accent="green" />
      <StatCard
        title={`${scopeLabel(scope)}数`}
        value={formatInteger(rows.length)}
        accent="slate"
      />
    </div>
  );
}

function TokenErrorCard({
  errorText,
  loading,
  onRetry,
}: {
  errorText: string | null;
  loading: boolean;
  onRetry: () => void;
}) {
  if (!errorText) return null;

  return (
    <Card
      padding="md"
      className="border-rose-200 bg-rose-50 dark:border-rose-700 dark:bg-rose-900/30"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="text-sm font-semibold text-rose-900 dark:text-rose-300">加载失败</div>
          <div className="mt-1 text-sm text-rose-800 dark:text-rose-200">
            用量刷新失败，请重试；必要时查看 Console 日志。
          </div>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={onRetry}
          disabled={loading}
          className="border-rose-200 bg-white text-rose-800 hover:bg-rose-50 dark:border-rose-700 dark:bg-slate-800 dark:text-rose-200 dark:hover:bg-rose-900/30"
        >
          重试
        </Button>
      </div>
      <div className="mt-3 rounded-lg border border-rose-200 bg-white/70 p-3 font-mono text-xs text-slate-800 dark:border-rose-700 dark:bg-slate-800/70 dark:text-slate-200">
        {errorText}
      </div>
    </Card>
  );
}

function TokenLeaderboardTable({
  scope,
  rows,
  summary,
  loading,
}: {
  scope: TokenCostScope;
  rows: UsageLeaderboardRow[];
  summary: UsageSummary | null;
  loading: boolean;
}) {
  if (loading && rows.length === 0) {
    return (
      <div className="flex items-center justify-center gap-3 px-6 py-14 text-sm text-slate-600 dark:text-slate-400">
        <Spinner />
        <span>加载用量中…</span>
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="px-6 py-14 text-center text-sm text-slate-600 dark:text-slate-400">
        当前时间范围暂无用量数据。
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-separate border-spacing-0 text-left text-sm">
        <caption className="sr-only">用量排行榜</caption>
        <thead className="sticky top-0 z-10">
          <tr>
            <th scope="col" className={TABLE_TH_CLASS}>
              排名
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              {scopeLabel(scope)}
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              Token 明细
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              总花费
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              请求数
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              成功率
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              Token 占比
            </th>
            <th scope="col" className={TABLE_TH_CLASS}>
              平均输出速度
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr
              key={row.key}
              className="align-top transition-colors hover:bg-slate-50/60 dark:hover:bg-slate-800/50"
            >
              <td
                className={`${TABLE_TD_CLASS} text-xs tabular-nums text-slate-400 dark:text-slate-500`}
              >
                {index + 1}
              </td>
              <td className={TABLE_TD_CLASS}>
                <div className="font-medium text-slate-900 dark:text-slate-100">{row.name}</div>
              </td>
              <td className={TABLE_MONO_TD_CLASS}>
                <TokenBreakdown
                  totalTokens={row.io_total_tokens}
                  inputTokens={row.input_tokens}
                  outputTokens={row.output_tokens}
                  totalTokensWithCache={row.total_tokens}
                  displayMode="compactRatio"
                  useCompactUnits={true}
                />
              </td>
              <td className={TABLE_MONO_TD_CLASS}>{formatCostValue(row.cost_usd)}</td>
              <td className={TABLE_MONO_TD_CLASS}>{formatInteger(row.requests_total)}</td>
              <td className={TABLE_MONO_TD_CLASS}>{formatPercent(successRate(row))}</td>
              <td className={`${TABLE_TD_CLASS} min-w-[120px]`}>
                <TokenShareBar percent={tokenShare(row, summary)} />
              </td>
              <td className={TABLE_MONO_TD_CLASS}>
                {formatTokensPerSecond(row.avg_output_tokens_per_second)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function HomeTokenCostPanel({ devPreviewEnabled = false }: HomeTokenCostPanelProps) {
  const [scope, setScope] = useState<TokenCostScope>("provider");
  const [range, setRange] = useState<TokenCostRange>("today");

  const queryConfig = useMemo(() => buildTokenCostQueryConfig(range), [range]);
  const queryInput = useMemo(
    () => ({
      ...queryConfig.input,
      limit: null,
    }),
    [queryConfig.input]
  );
  const previewRowsByScope = useMemo(
    () => ({
      provider: scalePreviewTokenRows(PREVIEW_TOKEN_PROVIDER_ROWS, queryConfig.previewFactor),
      model: scalePreviewTokenRows(PREVIEW_TOKEN_MODEL_ROWS, queryConfig.previewFactor),
    }),
    [queryConfig.previewFactor]
  );
  const previewSummary = useMemo(
    () => buildPreviewTokenSummary(previewRowsByScope.provider),
    [previewRowsByScope.provider]
  );

  const summaryQuery = useUsageSummaryV2Query(queryConfig.period, queryConfig.input);
  const leaderboardQuery = useUsageLeaderboardV2Query(scope, queryConfig.period, queryInput);

  const summaryRaw = summaryQuery.data ?? null;
  const rowsRaw = leaderboardQuery.data ?? EMPTY_ROWS;
  const loading = summaryQuery.isLoading || leaderboardQuery.isLoading;
  const fetching = summaryQuery.isFetching || leaderboardQuery.isFetching;
  const error = summaryQuery.error ?? leaderboardQuery.error;
  const errorText = error ? formatUnknownError(error) : null;
  const previewActive =
    devPreviewEnabled &&
    !loading &&
    isUsageSummaryEmpty(summaryRaw) &&
    isUsageLeaderboardEmpty(rowsRaw);

  const summary = previewActive ? previewSummary : summaryRaw;
  const rows = useMemo(() => {
    if (!previewActive) return rowsRaw;
    return scope === "provider" ? previewRowsByScope.provider : previewRowsByScope.model;
  }, [previewActive, previewRowsByScope.model, previewRowsByScope.provider, rowsRaw, scope]);
  const totalCostUsd = useMemo(() => totalCostUsdFromRows(rows), [rows]);

  const refresh = useCallback(() => {
    void summaryQuery.refetch();
    void leaderboardQuery.refetch();
  }, [leaderboardQuery, summaryQuery]);

  return (
    <div className="flex h-full flex-col gap-5 overflow-auto">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="用量时间范围">
          {TOKEN_COST_RANGE_ITEMS.map((item) => {
            const active = range === item.key;
            return (
              <Button
                key={item.key}
                size="sm"
                variant={active ? "primary" : "secondary"}
                aria-pressed={active}
                onClick={() => setRange(item.key)}
                className="whitespace-nowrap"
              >
                {item.label}
              </Button>
            );
          })}
        </div>
        <div className="flex flex-wrap items-center gap-3 lg:justify-end">
          <TabList
            ariaLabel="用量维度切换"
            items={TOKEN_COST_SCOPE_ITEMS}
            value={scope}
            onChange={setScope}
            size="sm"
          />
        </div>
      </div>

      <TokenSummaryCards
        summary={summary}
        rows={rows}
        totalCostUsd={totalCostUsd}
        scope={scope}
        loading={loading}
      />

      <TokenErrorCard errorText={errorText} loading={fetching} onRetry={refresh} />

      <Card padding="none" className="min-h-0 overflow-hidden">
        <div className="border-b border-slate-200 px-6 pb-4 pt-5 dark:border-slate-700">
          <div className="text-base font-semibold text-slate-900 dark:text-slate-100">
            {scopeLabel(scope)}排行
          </div>
        </div>
        <TokenLeaderboardTable scope={scope} rows={rows} summary={summary} loading={loading} />
      </Card>
    </div>
  );
}
