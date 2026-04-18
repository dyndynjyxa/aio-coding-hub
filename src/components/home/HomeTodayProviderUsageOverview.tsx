import { useMemo } from "react";
import { Loader2 } from "lucide-react";
import type { GatewayActiveSession } from "../../services/gateway/gateway";
import type { UsageLeaderboardRow, UsageSummary } from "../../services/usage/usage";
import { Card } from "../../ui/Card";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { formatInteger, formatPercent, formatUsdCompact } from "../../utils/formatters";
import { computeStatusBadge } from "./HomeLogShared";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import { useHomeTokenCostDataModel } from "./useHomeTokenCostDataModel";

const SUMMARY_SKELETON_KEYS = [0, 1, 2, 3, 4];
const PROVIDER_SKELETON_KEYS = [0, 1, 2];
const MAX_PROVIDER_ROWS = 3;
const PROVIDER_HEADER_LABEL = "供应商（前 3 个）";
const TABLE_TH_CLASS =
  "border-b border-slate-200 bg-slate-50/70 px-3 py-2.5 text-left text-xs font-medium uppercase tracking-wide text-slate-500 dark:border-slate-700 dark:bg-slate-800/70 dark:text-slate-400";
const TABLE_TD_CLASS = "border-b border-slate-100 px-3 py-3 dark:border-slate-800";
const TABLE_MONO_TD_CLASS =
  "border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700 dark:border-slate-800 dark:text-slate-300";
const TODAY_PROVIDER_QUERY_CONFIG = {
  period: "daily" as const,
  input: {
    startTs: null,
    endTs: null,
    cliKey: null,
    providerId: null,
  },
  previewFactor: 1,
};
const IN_PROGRESS_BADGE = computeStatusBadge({
  status: null,
  errorCode: null,
  inProgress: true,
});

function formatTokenValue(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  return formatTokensMillions(value);
}

function summaryCacheHitRate(summary: UsageSummary | null) {
  if (!summary) return NaN;
  return computeCacheHitRate(
    summary.input_tokens,
    summary.cache_creation_input_tokens,
    summary.cache_read_input_tokens
  );
}

type SummaryMetricAccent = "blue" | "purple" | "green" | "orange" | "slate";
type DisplayProviderRow = {
  row: UsageLeaderboardRow;
  isRunning: boolean;
  isSynthetic: boolean;
};

const SUMMARY_METRIC_ACCENT_CLASS: Record<SummaryMetricAccent, string> = {
  blue: "bg-blue-500",
  purple: "bg-violet-500",
  green: "bg-emerald-500",
  orange: "bg-orange-500",
  slate: "bg-slate-400 dark:bg-slate-500",
};

function tokenShare(row: UsageLeaderboardRow, summary: UsageSummary | null) {
  if (!summary || summary.io_total_tokens <= 0) return 0;
  return row.io_total_tokens / summary.io_total_tokens;
}

function successRate(row: UsageLeaderboardRow) {
  if (row.requests_total <= 0) return NaN;
  return row.requests_success / row.requests_total;
}

function normalizeProviderName(name: string | null | undefined) {
  return name?.trim().toLocaleLowerCase() ?? "";
}

function sortProviderRows(rows: UsageLeaderboardRow[]) {
  return rows.slice().sort((left, right) => {
    if (right.io_total_tokens !== left.io_total_tokens) {
      return right.io_total_tokens - left.io_total_tokens;
    }
    if (right.requests_total !== left.requests_total) {
      return right.requests_total - left.requests_total;
    }
    return left.name.localeCompare(right.name);
  });
}

function buildActiveProviderNames(activeSessions: GatewayActiveSession[]) {
  const seen = new Set<string>();
  const names: string[] = [];

  for (const session of activeSessions) {
    const name = session.provider_name?.trim();
    const normalized = normalizeProviderName(name);
    if (!name || !normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    names.push(name);
  }

  return names;
}

function createSyntheticProviderRow(name: string): UsageLeaderboardRow {
  const normalized = normalizeProviderName(name).replace(/\s+/g, "-");
  return {
    key: `running:${normalized || "unknown"}`,
    name,
    requests_total: 0,
    requests_success: 0,
    requests_failed: 0,
    total_tokens: 0,
    io_total_tokens: 0,
    input_tokens: 0,
    output_tokens: 0,
    cache_creation_input_tokens: 0,
    cache_read_input_tokens: 0,
    avg_duration_ms: null,
    avg_ttfb_ms: null,
    avg_output_tokens_per_second: null,
    cost_usd: null,
  };
}

function selectProviderRows(
  rows: UsageLeaderboardRow[],
  activeSessions: GatewayActiveSession[]
): DisplayProviderRow[] {
  const sortedRows = sortProviderRows(rows);
  const activeProviderNames = buildActiveProviderNames(activeSessions);
  const activeProviderSet = new Set(activeProviderNames.map((name) => normalizeProviderName(name)));
  const rowByName = new Map(
    sortedRows.map((row) => [normalizeProviderName(row.name), row] as const)
  );
  const rankByName = new Map(
    sortedRows.map((row, index) => [normalizeProviderName(row.name), index] as const)
  );
  const selected = new Map<string, DisplayProviderRow>();

  for (const row of sortedRows) {
    const normalized = normalizeProviderName(row.name);
    if (!activeProviderSet.has(normalized)) continue;
    selected.set(normalized, { row, isRunning: true, isSynthetic: false });
    if (selected.size >= MAX_PROVIDER_ROWS) break;
  }

  if (selected.size < MAX_PROVIDER_ROWS) {
    for (const name of activeProviderNames) {
      const normalized = normalizeProviderName(name);
      if (!normalized || selected.has(normalized)) continue;
      selected.set(normalized, {
        row: rowByName.get(normalized) ?? createSyntheticProviderRow(name),
        isRunning: true,
        isSynthetic: !rowByName.has(normalized),
      });
      if (selected.size >= MAX_PROVIDER_ROWS) break;
    }
  }

  if (selected.size < MAX_PROVIDER_ROWS) {
    for (const row of sortedRows) {
      const normalized = normalizeProviderName(row.name);
      if (selected.has(normalized)) continue;
      selected.set(normalized, { row, isRunning: false, isSynthetic: false });
      if (selected.size >= MAX_PROVIDER_ROWS) break;
    }
  }

  return Array.from(selected.values()).sort((left, right) => {
    const leftRank =
      rankByName.get(normalizeProviderName(left.row.name)) ?? Number.MAX_SAFE_INTEGER;
    const rightRank =
      rankByName.get(normalizeProviderName(right.row.name)) ?? Number.MAX_SAFE_INTEGER;
    if (leftRank !== rightRank) return leftRank - rightRank;
    return left.row.name.localeCompare(right.row.name);
  });
}

function rowTokenBreakdown(row: UsageLeaderboardRow) {
  return [
    formatTokenValue(row.total_tokens),
    formatTokenValue(Math.max(0, row.total_tokens - row.io_total_tokens)),
    formatPercent(
      computeCacheHitRate(
        row.input_tokens,
        row.cache_creation_input_tokens,
        row.cache_read_input_tokens
      )
    ),
  ].join(" / ");
}

function SummaryMetricCard({
  title,
  value,
  accent,
}: {
  title: string;
  value: string;
  accent: SummaryMetricAccent;
}) {
  return (
    <Card padding="sm" className="relative h-full overflow-hidden">
      <div className={`absolute inset-x-0 top-0 h-0.5 ${SUMMARY_METRIC_ACCENT_CLASS[accent]}`} />
      <div className="text-[11px] font-medium text-slate-500 dark:text-slate-400">{title}</div>
      <div className="mt-1 font-mono text-sm font-semibold tracking-tight text-slate-900 dark:text-slate-100">
        {value}
      </div>
    </Card>
  );
}

function SummaryMetricCardSkeleton() {
  return (
    <Card padding="sm" className="h-full animate-pulse">
      <div className="h-3 w-14 rounded bg-slate-200 dark:bg-slate-700" />
      <div className="mt-2 h-5 w-16 rounded bg-slate-200 dark:bg-slate-700" />
    </Card>
  );
}

function SummaryCards({
  summary,
  totalCostUsd,
  loading,
}: {
  summary: UsageSummary | null;
  totalCostUsd: number | null;
  loading: boolean;
}) {
  if (loading && !summary) {
    return (
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {SUMMARY_SKELETON_KEYS.map((key) => (
          <SummaryMetricCardSkeleton key={key} />
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-2 xl:grid-cols-5">
      <SummaryMetricCard
        title="含缓存总 Token"
        value={formatTokenValue(summary?.total_tokens)}
        accent="purple"
      />
      <SummaryMetricCard
        title="总 Token"
        value={formatTokenValue(summary?.io_total_tokens)}
        accent="blue"
      />
      <SummaryMetricCard
        title="缓存命中率"
        value={formatPercent(summaryCacheHitRate(summary))}
        accent="purple"
      />
      <SummaryMetricCard
        title="今日请求数"
        value={formatInteger(summary?.requests_total)}
        accent="green"
      />
      <SummaryMetricCard title="今日花费" value={formatUsdCompact(totalCostUsd)} accent="orange" />
    </div>
  );
}

function ProviderUsageSkeleton() {
  return (
    <tr className="animate-pulse">
      <td className={TABLE_TD_CLASS}>
        <div className="h-4 w-28 rounded bg-slate-200 dark:bg-slate-700" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-40 rounded bg-slate-100 dark:bg-slate-600" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-14 rounded bg-slate-100 dark:bg-slate-600" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-12 rounded bg-slate-100 dark:bg-slate-600" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-12 rounded bg-slate-100 dark:bg-slate-600" />
      </td>
    </tr>
  );
}

export function HomeTodayProviderUsageOverview({
  devPreviewEnabled = false,
  activeSessions = [],
}: {
  devPreviewEnabled?: boolean;
  activeSessions?: GatewayActiveSession[];
}) {
  const model = useHomeTokenCostDataModel({
    scope: "provider",
    queryConfig: TODAY_PROVIDER_QUERY_CONFIG,
    devPreviewEnabled,
  });

  const topRows = useMemo(
    () => selectProviderRows(model.rows, activeSessions),
    [activeSessions, model.rows]
  );

  return (
    <div className="flex flex-col gap-4">
      <SummaryCards
        summary={model.summary}
        totalCostUsd={model.totalCostUsd}
        loading={model.loading}
      />

      <QueryErrorCard
        errorText={model.errorText}
        loading={model.fetching}
        onRetry={model.refresh}
        message="读取今日供应商用量失败，请重试；必要时查看 Console 日志。"
      />

      <Card padding="none" className="overflow-hidden">
        {model.loading && model.summary == null && topRows.length === 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full border-separate border-spacing-0 text-left text-sm">
              <caption className="sr-only">今日供应商用量</caption>
              <thead>
                <tr>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    {PROVIDER_HEADER_LABEL}
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <div className="inline-flex items-center gap-1 whitespace-nowrap normal-case">
                      <span className="text-[11px] font-medium tracking-normal text-slate-500 dark:text-slate-400">
                        Token
                      </span>
                      <span className="text-[9px] font-normal tracking-normal text-slate-400 dark:text-slate-500">
                        含缓存 / 缓存 / 命中率
                      </span>
                    </div>
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    总花费
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    成功率
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    Token 占比
                  </th>
                </tr>
              </thead>
              <tbody>
                {PROVIDER_SKELETON_KEYS.map((key) => (
                  <ProviderUsageSkeleton key={key} />
                ))}
              </tbody>
            </table>
          </div>
        ) : topRows.length === 0 ? (
          <div className="px-4 py-10 text-center text-sm text-slate-600 dark:text-slate-400">
            今日暂无供应商用量数据。
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-separate border-spacing-0 text-left text-sm">
              <caption className="sr-only">今日供应商用量</caption>
              <thead className="sticky top-0 z-10">
                <tr>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    {PROVIDER_HEADER_LABEL}
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <div className="inline-flex items-center gap-1 whitespace-nowrap normal-case">
                      <span className="text-[11px] font-medium tracking-normal text-slate-500 dark:text-slate-400">
                        Token
                      </span>
                      <span className="text-[9px] font-normal tracking-normal text-slate-400 dark:text-slate-500">
                        含缓存 / 缓存 / 命中率
                      </span>
                    </div>
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    总花费
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    成功率
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    Token 占比
                  </th>
                </tr>
              </thead>
              <tbody>
                {topRows.map(({ row, isRunning, isSynthetic }) => (
                  <tr
                    key={row.key}
                    className="align-top transition-colors hover:bg-slate-50/60 dark:hover:bg-slate-800/50"
                  >
                    <td className={TABLE_TD_CLASS}>
                      <div className="flex items-center gap-2">
                        {isRunning ? (
                          <span
                            aria-label={IN_PROGRESS_BADGE.text}
                            title={IN_PROGRESS_BADGE.text}
                            className={`inline-flex shrink-0 items-center rounded-md px-1.5 py-0.5 text-[11px] font-medium ${IN_PROGRESS_BADGE.tone}`}
                          >
                            <Loader2 className="h-3 w-3 shrink-0 animate-spin" />
                          </span>
                        ) : null}
                        <div className="font-medium text-slate-900 dark:text-slate-100">
                          {row.name}
                        </div>
                      </div>
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "— / — / —" : rowTokenBreakdown(row)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatUsdCompact(row.cost_usd)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatPercent(successRate(row))}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatPercent(tokenShare(row, model.summary))}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}
