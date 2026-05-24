import { useMemo } from "react";
import { Loader2 } from "lucide-react";
import { useDocumentVisibility } from "../../hooks/useDocumentVisibility";
import { useNowMs } from "../../hooks/useNowMs";
import { useWindowForeground } from "../../hooks/useWindowForeground";
import type { GatewayActiveSession } from "../../services/gateway/gateway";
import type { TraceSession } from "../../services/gateway/traceStore";
import type { UsageLeaderboardRow, UsageSummary } from "../../services/usage/usage";
import { Card } from "../../ui/Card";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { formatInteger, formatPercent, formatUsdCompact } from "../../utils/formatters";
import { computeStatusBadge } from "./HomeLogShared";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import {
  useHomeTokenCostDataModel,
  type HomeTokenCostDataModelQueryRefreshConfig,
} from "./useHomeTokenCostDataModel";

const SUMMARY_SKELETON_KEYS = [0, 1, 2, 3, 4];
const PROVIDER_SKELETON_KEYS = [0, 1, 2];
const MAX_PROVIDER_ROWS = 3;
const LIVE_TRACE_MAX_AGE_MS = 15 * 60 * 1000;
const STALE_TRACE_TIMEOUT_MS = 5 * 60 * 1000;
const OVERVIEW_REFRESH_INTERVAL_MS = 60 * 1000;
const TABLE_TH_CLASS =
  "border-b border-border bg-secondary/70 px-3 py-2.5 text-left text-xs font-medium uppercase tracking-wide text-muted-foreground dark:border-border dark:bg-secondary/70 dark:text-muted-foreground";
const TABLE_TD_CLASS = "border-b border-border px-3 py-3 dark:border-border";
const TABLE_MONO_TD_CLASS =
  "border-b border-border px-3 py-3 font-mono text-xs tabular-nums text-secondary-foreground dark:border-border dark:text-secondary-foreground";
const TABLE_TH_MAIN_CLASS = "text-[11px] font-medium tracking-normal text-muted-foreground";
const TABLE_TH_NOTE_CLASS = "text-[9px] font-normal tracking-normal text-muted-foreground";
const TODAY_PROVIDER_QUERY_CONFIG = {
  period: "daily" as const,
  input: {
    startTs: null,
    endTs: null,
    cliKey: null,
    providerId: null,
    excludeCx2CcGatewayBridge: true,
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
type ProviderIdentity = {
  providerId: number | null;
  cliKey: string | null;
  normalizedName: string;
};
type ActiveProviderEntry = ProviderIdentity & {
  displayName: string;
};

const SUMMARY_METRIC_ACCENT_CLASS: Record<SummaryMetricAccent, string> = {
  blue: "bg-blue-500",
  purple: "bg-violet-500",
  green: "bg-emerald-500",
  orange: "bg-orange-500",
  slate: "bg-muted dark:bg-muted",
};

function successRate(row: UsageLeaderboardRow) {
  if (row.requests_total <= 0) return NaN;
  return row.requests_success / row.requests_total;
}

function normalizeCliKey(value: string | null | undefined) {
  const normalized = value?.trim().toLocaleLowerCase() ?? "";
  return normalized || null;
}

function normalizeProviderName(name: string | null | undefined, cliKey?: string | null) {
  const normalized = name?.trim().toLocaleLowerCase() ?? "";
  if (!normalized) return "";
  const normalizedCliKey = normalizeCliKey(cliKey);
  const prefix = normalizedCliKey ? `${normalizedCliKey}/` : null;
  if (prefix && normalized.startsWith(prefix)) {
    return normalized.slice(prefix.length).trim();
  }
  return normalized;
}

function parseProviderRowIdentity(row: UsageLeaderboardRow): ProviderIdentity {
  const match = row.key.match(/^([^:]+):(\d+)$/);
  const cliKey = normalizeCliKey(match?.[1] ?? null);
  const providerId = match ? Number(match[2]) : NaN;

  return {
    providerId: Number.isSafeInteger(providerId) && providerId > 0 ? providerId : null,
    cliKey,
    normalizedName: normalizeProviderName(row.name, cliKey),
  };
}

function activeProviderIdentity(session: GatewayActiveSession): ActiveProviderEntry {
  const cliKey = normalizeCliKey(session.cli_key);
  const providerId =
    Number.isSafeInteger(session.provider_id) && session.provider_id > 0
      ? session.provider_id
      : null;

  return {
    providerId,
    cliKey,
    normalizedName: normalizeProviderName(session.provider_name, cliKey),
    displayName: session.provider_name?.trim() || "未知",
  };
}

function providerIdentityKey(identity: ProviderIdentity) {
  if (identity.providerId != null) return `id:${identity.providerId}`;
  if (identity.cliKey && identity.normalizedName) {
    return `name:${identity.cliKey}:${identity.normalizedName}`;
  }
  return `name:${identity.normalizedName}`;
}

function providerScopedNameKey(identity: ProviderIdentity) {
  if (!identity.cliKey || !identity.normalizedName) return null;
  return `${identity.cliKey}:${identity.normalizedName}`;
}

function addUniqueValue<T>(map: Map<string, T | null>, key: string, value: T) {
  if (map.has(key)) {
    map.set(key, null);
    return;
  }
  map.set(key, value);
}

function getUniqueValue<T>(map: Map<string, T | null>, key: string) {
  return map.get(key) ?? null;
}

function formatSyntheticProviderName(
  entry: ActiveProviderEntry,
  options: { preferCliPrefix: boolean }
) {
  const rawName = entry.displayName.trim();
  if (!rawName) return "未知";
  if (!options.preferCliPrefix || !entry.cliKey) return rawName;
  const prefix = `${entry.cliKey}/`;
  return rawName.toLocaleLowerCase().startsWith(prefix) ? rawName : `${prefix}${rawName}`;
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

function buildActiveProviders(
  activeSessions: GatewayActiveSession[],
  options: { preferCliPrefix: boolean }
) {
  const seen = new Set<string>();
  const entries: ActiveProviderEntry[] = [];

  for (const session of activeSessions) {
    const identity = activeProviderIdentity(session);
    const key = providerIdentityKey(identity);
    if (!identity.normalizedName || seen.has(key)) continue;
    seen.add(key);
    entries.push({
      ...identity,
      displayName: formatSyntheticProviderName(identity, options),
    });
  }

  return entries;
}

function buildRunningProvidersFromTraces(
  traces: TraceSession[],
  nowMs: number,
  options: { preferCliPrefix: boolean }
) {
  const seen = new Set<string>();
  const entries: ActiveProviderEntry[] = [];

  for (const trace of traces) {
    if (trace.summary) continue;
    if (nowMs - trace.first_seen_ms >= LIVE_TRACE_MAX_AGE_MS) continue;
    if (nowMs - trace.last_seen_ms >= STALE_TRACE_TIMEOUT_MS) continue;

    const latestAttempt = (trace.attempts ?? [])
      .slice()
      .sort((left, right) => right.attempt_index - left.attempt_index)[0];
    const providerName = latestAttempt?.provider_name?.trim();
    if (!providerName || providerName === "Unknown") continue;

    const cliKey = normalizeCliKey(trace.cli_key);
    const providerId =
      latestAttempt &&
      Number.isSafeInteger(latestAttempt.provider_id) &&
      latestAttempt.provider_id > 0
        ? latestAttempt.provider_id
        : null;
    const entry: ActiveProviderEntry = {
      providerId,
      cliKey,
      normalizedName: normalizeProviderName(providerName, cliKey),
      displayName: providerName,
    };
    const key = providerIdentityKey(entry);
    if (!entry.normalizedName || seen.has(key)) continue;

    seen.add(key);
    entries.push({
      ...entry,
      displayName: formatSyntheticProviderName(entry, options),
    });
  }

  return entries;
}

function createSyntheticProviderRow(entry: ActiveProviderEntry): UsageLeaderboardRow {
  const normalized = entry.normalizedName.replace(/\s+/g, "-");
  return {
    key:
      entry.providerId != null
        ? `running:${entry.cliKey ?? "provider"}:${entry.providerId}`
        : `running:${normalized || "unknown"}`,
    name: entry.displayName,
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
  activeProviders: ActiveProviderEntry[]
): DisplayProviderRow[] {
  const sortedRows = sortProviderRows(rows);
  const rowIdentityByKey = new Map(
    sortedRows.map((row) => [row.key, parseProviderRowIdentity(row)] as const)
  );
  const activeProviderIdSet = new Set(
    activeProviders
      .map((entry) => entry.providerId)
      .filter((providerId): providerId is number => providerId != null)
  );
  const activeProviderById = new Map(
    activeProviders
      .filter(
        (entry): entry is ActiveProviderEntry & { providerId: number } => entry.providerId != null
      )
      .map((entry) => [entry.providerId, entry] as const)
  );
  const activeProviderByScopedName = new Map<string, ActiveProviderEntry>();
  const uniqueActiveProviderByName = new Map<string, ActiveProviderEntry | null>();
  activeProviders.forEach((entry) => {
    const scopedNameKey = providerScopedNameKey(entry);
    if (scopedNameKey && !activeProviderByScopedName.has(scopedNameKey)) {
      activeProviderByScopedName.set(scopedNameKey, entry);
    }
    if (entry.normalizedName) {
      addUniqueValue(uniqueActiveProviderByName, entry.normalizedName, entry);
    }
  });
  const rowById = new Map<number, UsageLeaderboardRow>();
  const rowByScopedName = new Map<string, UsageLeaderboardRow>();
  const uniqueRowByName = new Map<string, UsageLeaderboardRow | null>();
  const rankById = new Map<number, number>();
  const rankByScopedName = new Map<string, number>();
  const uniqueRankByName = new Map<string, number | null>();

  sortedRows.forEach((row, index) => {
    const identity = rowIdentityByKey.get(row.key);
    if (!identity) return;
    if (identity.providerId != null && !rowById.has(identity.providerId)) {
      rowById.set(identity.providerId, row);
      rankById.set(identity.providerId, index);
    }
    const scopedNameKey = providerScopedNameKey(identity);
    if (scopedNameKey && !rowByScopedName.has(scopedNameKey)) {
      rowByScopedName.set(scopedNameKey, row);
      rankByScopedName.set(scopedNameKey, index);
    }
    if (identity.normalizedName) {
      addUniqueValue(uniqueRowByName, identity.normalizedName, row);
      addUniqueValue(uniqueRankByName, identity.normalizedName, index);
    }
  });
  const selected = new Map<string, DisplayProviderRow>();
  const findActiveProvider = (identity: ProviderIdentity) => {
    const matchedById =
      identity.providerId != null ? activeProviderById.get(identity.providerId) : null;
    if (matchedById) return matchedById;

    const scopedNameKey = providerScopedNameKey(identity);
    const matchedByScopedName = scopedNameKey
      ? activeProviderByScopedName.get(scopedNameKey)
      : null;
    if (matchedByScopedName) return matchedByScopedName;

    if (!identity.normalizedName) return null;
    const matchedByName = getUniqueValue(uniqueActiveProviderByName, identity.normalizedName);
    if (!matchedByName) return null;
    if (identity.cliKey && matchedByName.cliKey) return null;
    return matchedByName;
  };
  const findRowForActiveProvider = (entry: ActiveProviderEntry) => {
    const matchedById = entry.providerId != null ? rowById.get(entry.providerId) : null;
    if (matchedById) return matchedById;

    const scopedNameKey = providerScopedNameKey(entry);
    const matchedByScopedName = scopedNameKey ? rowByScopedName.get(scopedNameKey) : null;
    if (matchedByScopedName) return matchedByScopedName;

    if (!entry.normalizedName) return null;
    const matchedByName = getUniqueValue(uniqueRowByName, entry.normalizedName);
    if (!matchedByName) return null;
    const matchedIdentity =
      rowIdentityByKey.get(matchedByName.key) ?? parseProviderRowIdentity(matchedByName);
    if (entry.cliKey && matchedIdentity.cliKey) return null;
    return matchedByName;
  };
  const hasActiveProviderMatch = (identity: ProviderIdentity) => {
    if (identity.providerId != null && activeProviderIdSet.has(identity.providerId)) return true;
    const scopedNameKey = providerScopedNameKey(identity);
    if (scopedNameKey && activeProviderByScopedName.has(scopedNameKey)) return true;
    return findActiveProvider(identity) != null;
  };
  const rankForIdentity = (identity: ProviderIdentity) => {
    const idRank = identity.providerId != null ? rankById.get(identity.providerId) : undefined;
    if (idRank != null) return idRank;
    const scopedNameKey = providerScopedNameKey(identity);
    const scopedRank = scopedNameKey ? rankByScopedName.get(scopedNameKey) : undefined;
    if (scopedRank != null) return scopedRank;
    return getUniqueValue(uniqueRankByName, identity.normalizedName) ?? Number.MAX_SAFE_INTEGER;
  };

  for (const row of sortedRows) {
    const identity = rowIdentityByKey.get(row.key);
    if (!identity) continue;
    const matchedActive = findActiveProvider(identity);
    if (!matchedActive) continue;
    selected.set(providerIdentityKey(matchedActive), { row, isRunning: true, isSynthetic: false });
    if (selected.size >= MAX_PROVIDER_ROWS) break;
  }

  if (selected.size < MAX_PROVIDER_ROWS) {
    for (const entry of activeProviders) {
      const key = providerIdentityKey(entry);
      if (!entry.normalizedName || selected.has(key)) continue;
      const matchedRow = findRowForActiveProvider(entry);
      selected.set(key, {
        row: matchedRow ?? createSyntheticProviderRow(entry),
        isRunning: true,
        isSynthetic: matchedRow == null,
      });
      if (selected.size >= MAX_PROVIDER_ROWS) break;
    }
  }

  if (selected.size < MAX_PROVIDER_ROWS) {
    for (const row of sortedRows) {
      const identity = rowIdentityByKey.get(row.key);
      if (!identity) continue;
      const key = providerIdentityKey(identity);
      if (selected.has(key)) continue;
      if (hasActiveProviderMatch(identity)) {
        continue;
      }
      selected.set(key, { row, isRunning: false, isSynthetic: false });
      if (selected.size >= MAX_PROVIDER_ROWS) break;
    }
  }

  return Array.from(selected.values()).sort((left, right) => {
    const leftIdentity = rowIdentityByKey.get(left.row.key) ?? parseProviderRowIdentity(left.row);
    const rightIdentity =
      rowIdentityByKey.get(right.row.key) ?? parseProviderRowIdentity(right.row);
    const leftRank = rankForIdentity(leftIdentity);
    const rightRank = rankForIdentity(rightIdentity);
    if (leftRank !== rightRank) return leftRank - rightRank;
    return left.row.name.localeCompare(right.row.name);
  });
}

function rowCacheHitRate(row: UsageLeaderboardRow) {
  return formatPercent(
    computeCacheHitRate(
      row.input_tokens,
      row.cache_creation_input_tokens,
      row.cache_read_input_tokens
    )
  );
}

function TableHeaderLabel({ label, note }: { label: string; note?: string }) {
  return (
    <div className="inline-flex items-baseline gap-1 whitespace-nowrap normal-case">
      <span className={TABLE_TH_MAIN_CLASS}>{label}</span>
      {note ? <span className={TABLE_TH_NOTE_CLASS}>（{note}）</span> : null}
    </div>
  );
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
      <div className="text-[11px] font-medium text-muted-foreground">{title}</div>
      <div className="mt-1 font-mono text-sm font-semibold tracking-tight text-foreground">
        {value}
      </div>
    </Card>
  );
}

function SummaryMetricCardSkeleton() {
  return (
    <Card padding="sm" className="h-full animate-pulse">
      <div className="h-3 w-14 rounded bg-muted dark:bg-secondary" />
      <div className="mt-2 h-5 w-16 rounded bg-muted dark:bg-secondary" />
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
        title="输入+输出 Token"
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
        <div className="h-4 w-28 rounded bg-muted dark:bg-secondary" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-16 rounded bg-secondary dark:bg-secondary" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-14 rounded bg-secondary dark:bg-secondary" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-12 rounded bg-secondary dark:bg-secondary" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-12 rounded bg-secondary dark:bg-secondary" />
      </td>
      <td className={TABLE_MONO_TD_CLASS}>
        <div className="h-3 w-12 rounded bg-secondary dark:bg-secondary" />
      </td>
    </tr>
  );
}

export function HomeTodayProviderUsageOverview({
  devPreviewEnabled = false,
  activeSessions = [],
  traces,
}: {
  devPreviewEnabled?: boolean;
  activeSessions?: GatewayActiveSession[];
  traces?: TraceSession[];
}) {
  const documentVisible = useDocumentVisibility();
  const queryRefreshConfig = useMemo<HomeTokenCostDataModelQueryRefreshConfig>(() => {
    const refetchIntervalMs: number | false = documentVisible
      ? OVERVIEW_REFRESH_INTERVAL_MS
      : false;

    return {
      summary: {
        refetchIntervalMs,
        refetchOnMount: "always" as const,
      },
      leaderboard: {
        refetchIntervalMs,
        refetchOnMount: "always" as const,
      },
    };
  }, [documentVisible]);
  const model = useHomeTokenCostDataModel({
    scope: "provider",
    queryConfig: TODAY_PROVIDER_QUERY_CONFIG,
    devPreviewEnabled,
    queryRefreshConfig,
  });

  useWindowForeground({
    enabled: true,
    onForeground: model.refresh,
    throttleMs: 1000,
  });

  const nowMs = useNowMs(Boolean(traces && traces.length > 0), 1000);
  const activeProviders = useMemo(
    () =>
      traces != null
        ? buildRunningProvidersFromTraces(traces, nowMs, {
            preferCliPrefix: !model.previewActive,
          })
        : buildActiveProviders(activeSessions, {
            preferCliPrefix: !model.previewActive,
          }),
    [activeSessions, model.previewActive, nowMs, traces]
  );

  const topRows = useMemo(
    () => selectProviderRows(model.rows, activeProviders),
    [activeProviders, model.rows]
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
                    <TableHeaderLabel label="供应商" note="前 3 个" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="总Token" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="缓存命中率" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="输入+输出Token" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    总花费
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    成功率
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
          <div className="px-4 py-10 text-center text-sm text-muted-foreground">
            今日暂无供应商用量数据。
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-separate border-spacing-0 text-left text-sm">
              <caption className="sr-only">今日供应商用量</caption>
              <thead className="sticky top-0 z-10">
                <tr>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="供应商" note="前 3 个" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="总Token" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="缓存命中率" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    <TableHeaderLabel label="输入+输出Token" />
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    总花费
                  </th>
                  <th scope="col" className={TABLE_TH_CLASS}>
                    成功率
                  </th>
                </tr>
              </thead>
              <tbody>
                {topRows.map(({ row, isRunning, isSynthetic }) => (
                  <tr
                    key={row.key}
                    className="align-top transition-colors hover:bg-secondary/60 dark:hover:bg-secondary/50"
                  >
                    <td className={TABLE_TD_CLASS}>
                      <div className="flex items-center gap-2">
                        <div className="min-w-0 font-medium text-foreground">{row.name}</div>
                        <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
                          {isRunning ? (
                            <span
                              aria-label={IN_PROGRESS_BADGE.text}
                              title={IN_PROGRESS_BADGE.text}
                              className={`inline-flex items-center rounded-md px-1.5 py-0.5 text-[11px] font-medium ${IN_PROGRESS_BADGE.tone}`}
                            >
                              <Loader2 className="h-3 w-3 shrink-0 animate-spin" />
                            </span>
                          ) : null}
                        </span>
                      </div>
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatTokenValue(row.total_tokens)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : rowCacheHitRate(row)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatTokenValue(row.io_total_tokens)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatUsdCompact(row.cost_usd)}
                    </td>
                    <td className={TABLE_MONO_TD_CLASS}>
                      {isSynthetic ? "—" : formatPercent(successRate(row))}
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
