import { useMemo, useState } from "react";
import type { UsageLeaderboardRow, UsagePeriod, UsageSummary } from "../../services/usage/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Spinner } from "../../ui/Spinner";
import { TabList, type TabListItem } from "../../ui/TabList";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import {
  formatInteger,
  formatPercent,
  formatTokensPerSecond,
  formatUsdCompact,
} from "../../utils/formatters";
import { StatCard, StatCardSkeleton } from "../usage/StatCard";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import { useHomeTokenCostDataModel } from "./useHomeTokenCostDataModel";

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

const SUMMARY_SKELETON_KEYS = [0, 1, 2, 3, 4, 5, 6];

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

function summaryCacheHitRate(summary: UsageSummary | null) {
  if (!summary) return null;
  return computeCacheHitRate(
    summary.input_tokens,
    summary.cache_creation_input_tokens,
    summary.cache_read_input_tokens
  );
}

function summaryCostCoverage(summary: UsageSummary | null) {
  if (!summary) return null;
  const denom = summary.requests_success;
  if (!Number.isFinite(denom) || denom <= 0) return null;
  const covered = summary.cost_covered_success;
  if (!Number.isFinite(covered) || covered < 0) return null;
  return covered / denom;
}

const CACHE_BREAKDOWN_SEPARATOR = " / ";

function trimCompactZero(value: string) {
  return value.replace(/\.0([KM])$/, "$1").replace(/\.0%$/, "%");
}

function CacheHitRateBreakdown({ row }: { row: UsageLeaderboardRow }) {
  const totalWithCache = row.total_tokens;
  const hasValidTotal = Number.isFinite(totalWithCache) && totalWithCache > 0;
  const cacheTokens = row.cache_creation_input_tokens + row.cache_read_input_tokens;
  const hitRate = computeCacheHitRate(
    row.input_tokens,
    row.cache_creation_input_tokens,
    row.cache_read_input_tokens
  );

  const totalText = trimCompactZero(formatTokensMillions(hasValidTotal ? totalWithCache : 0));
  const cacheText = hasValidTotal ? trimCompactZero(formatTokensMillions(cacheTokens)) : "—";
  const hitRateText =
    hasValidTotal && Number.isFinite(hitRate) ? trimCompactZero(formatPercent(hitRate)) : "—";

  return (
    <div className="whitespace-nowrap">
      {[totalText, cacheText, hitRateText].join(CACHE_BREAKDOWN_SEPARATOR)}
    </div>
  );
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
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-7">
        {SUMMARY_SKELETON_KEYS.map((key) => (
          <StatCardSkeleton key={key} />
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-7">
      <StatCard
        title="含缓存总 Token"
        value={formatTokenValue(summary?.total_tokens)}
        accent="purple"
      />
      <StatCard title="总 Token" value={formatTokenValue(summary?.io_total_tokens)} accent="blue" />
      <StatCard title="总花费" value={formatCostValue(totalCostUsd)} accent="orange" />
      <StatCard
        title="成本覆盖率"
        value={formatPercent(summaryCostCoverage(summary))}
        accent="orange"
      />
      <StatCard title="成功请求" value={formatInteger(summary?.requests_success)} accent="green" />
      <StatCard
        title="缓存命中率"
        value={formatPercent(summaryCacheHitRate(summary))}
        accent="purple"
      />
      <StatCard
        title={`${scopeLabel(scope)}数`}
        value={formatInteger(rows.length)}
        accent="slate"
      />
    </div>
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
              <div>Token 明细</div>
              <div className="mt-0.5 normal-case text-[10px] font-normal tracking-normal text-slate-400 dark:text-slate-500">
                含缓存总量 / 缓存量 / 缓存命中率
              </div>
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
                <CacheHitRateBreakdown row={row} />
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

type HomeTokenCostPanelProps = {
  devPreviewEnabled?: boolean;
};

export function HomeTokenCostPanel({ devPreviewEnabled = false }: HomeTokenCostPanelProps) {
  const [scope, setScope] = useState<TokenCostScope>("provider");
  const [range, setRange] = useState<TokenCostRange>("today");

  const queryConfig = useMemo(() => buildTokenCostQueryConfig(range), [range]);

  const model = useHomeTokenCostDataModel({
    scope,
    queryConfig,
    devPreviewEnabled,
  });

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
        summary={model.summary}
        rows={model.rows}
        totalCostUsd={model.totalCostUsd}
        scope={scope}
        loading={model.loading}
      />

      <QueryErrorCard
        errorText={model.errorText}
        loading={model.fetching}
        onRetry={model.refresh}
      />

      <Card padding="none" className="min-h-0 overflow-hidden">
        <div className="border-b border-slate-200 px-6 pb-4 pt-5 dark:border-slate-700">
          <div className="text-base font-semibold text-slate-900 dark:text-slate-100">
            {scopeLabel(scope)}排行
          </div>
        </div>
        <TokenLeaderboardTable
          scope={scope}
          rows={model.rows}
          summary={model.summary}
          loading={model.loading}
        />
      </Card>
    </div>
  );
}
