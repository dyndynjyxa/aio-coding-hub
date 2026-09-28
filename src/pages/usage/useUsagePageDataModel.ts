import { useMemo } from "react";
import type { CliKey } from "../../services/providers/providers";
import type { CliFilterKey } from "../../constants/clis";
import type {
  UsageLeaderboardRow,
  UsagePeriod,
  UsageProviderCacheRateTrendRowV1,
  UsageProviderMetricsTrendRowV1,
  UsageScope,
  UsageSummary,
} from "../../services/usage/usage";
import type { CustomDateRangeApplied, CustomDateRangeBounds } from "../../hooks/useCustomDateRange";
import {
  useUsageLeaderboardV2Query,
  useUsageProviderCacheRateTrendV1Query,
  useUsageProviderMetricsTrendV1Query,
  useUsageSummaryV2Query,
} from "../../query/usage";
import { formatUnknownError } from "../../utils/errors";
import type { UsageTableTab } from "./types";

type UsagePageDataModelArgs = {
  tableTab: UsageTableTab;
  scope: UsageScope;
  period: UsagePeriod;
  cliKey: CliFilterKey;
  providerId: number | null;
  customApplied: CustomDateRangeApplied | null;
  bounds: CustomDateRangeBounds;
};

export type UsagePageDataModel = {
  tauriAvailable: boolean;
  shouldLoad: boolean;
  customPending: boolean;
  loading: boolean;
  dataLoading: boolean;
  cacheTrendLoading: boolean;
  dataStale: boolean;
  cacheTrendStale: boolean;
  errorText: string | null;
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  cacheTrendRows: UsageProviderCacheRateTrendRowV1[];
  cacheTrendProviderCount: number;
  metricsTrendLoading: boolean;
  metricsTrendStale: boolean;
  metricsTrendRows: UsageProviderMetricsTrendRowV1[];
  metricsTrendProviderCount: number;
  totalCostUsd: number;
  handleRetry: () => void;
};

const EMPTY_USAGE_ROWS: UsageLeaderboardRow[] = [];
const EMPTY_CACHE_TREND_ROWS: UsageProviderCacheRateTrendRowV1[] = [];
const EMPTY_METRICS_TREND_ROWS: UsageProviderMetricsTrendRowV1[] = [];

function toCliKeyOrNull(cliKey: CliFilterKey): CliKey | null {
  return cliKey === "all" ? null : cliKey;
}

function totalCostUsdFromRows(rows: UsageLeaderboardRow[]): number {
  return rows.reduce((sum, row) => sum + (row.cost_usd ?? 0), 0);
}

function providerCountFromRows(rows: Array<{ key: string }>): number {
  return new Set(rows.map((row) => row.key)).size;
}

function useUsagePageQueryInput({
  period,
  cliKey,
  providerId,
  customApplied,
  bounds,
}: Pick<UsagePageDataModelArgs, "period" | "cliKey" | "providerId" | "customApplied" | "bounds">) {
  const tauriAvailable = true;
  const shouldLoad = period !== "custom" || customApplied != null;
  const customPending = period === "custom" && !customApplied;
  const input = {
    startTs: bounds.startTs,
    endTs: bounds.endTs,
    cliKey: toCliKeyOrNull(cliKey),
    providerId,
  };

  return { tauriAvailable, shouldLoad, customPending, input };
}

function useUsagePageQueries({
  scope,
  period,
  tableTab,
  shouldLoad,
  input,
}: Pick<UsagePageDataModelArgs, "scope" | "period" | "tableTab"> & {
  shouldLoad: boolean;
  input: {
    startTs: number | null;
    endTs: number | null;
    cliKey: CliKey | null;
    providerId: number | null;
  };
}) {
  const dataEnabled = shouldLoad;
  const cacheTrendEnabled = shouldLoad && tableTab === "cacheTrend";
  const metricsTrendEnabled = shouldLoad && tableTab === "metricsTrend";
  const summaryQuery = useUsageSummaryV2Query(period, input, { enabled: dataEnabled });
  const leaderboardQuery = useUsageLeaderboardV2Query(
    scope,
    period,
    { ...input, limit: null },
    { enabled: dataEnabled }
  );
  const cacheTrendQuery = useUsageProviderCacheRateTrendV1Query(
    period,
    { ...input, limit: null },
    { enabled: cacheTrendEnabled }
  );
  const metricsTrendQuery = useUsageProviderMetricsTrendV1Query(
    period,
    { ...input, limit: null },
    { enabled: metricsTrendEnabled }
  );

  const dataLoading = dataEnabled && (summaryQuery.isFetching || leaderboardQuery.isFetching);
  const cacheTrendLoading = cacheTrendEnabled && cacheTrendQuery.isFetching;
  const metricsTrendLoading = metricsTrendEnabled && metricsTrendQuery.isFetching;
  const loading = shouldLoad && (dataLoading || cacheTrendLoading || metricsTrendLoading);

  const dataStale =
    dataEnabled &&
    (summaryQuery.isFetching || leaderboardQuery.isFetching) &&
    (summaryQuery.data != null || leaderboardQuery.data != null);
  const cacheTrendStale =
    cacheTrendEnabled && cacheTrendQuery.isFetching && cacheTrendQuery.data != null;
  const metricsTrendStale =
    metricsTrendEnabled && metricsTrendQuery.isFetching && metricsTrendQuery.data != null;

  function handleRetry() {
    if (tableTab === "cacheTrend") void cacheTrendQuery.refetch();
    else if (tableTab === "metricsTrend") void metricsTrendQuery.refetch();
    else {
      void summaryQuery.refetch();
      void leaderboardQuery.refetch();
    }
  }

  return {
    summaryQuery,
    leaderboardQuery,
    cacheTrendQuery,
    metricsTrendQuery,
    dataLoading,
    cacheTrendLoading,
    metricsTrendLoading,
    loading,
    dataStale,
    cacheTrendStale,
    metricsTrendStale,
    handleRetry,
  };
}

export function useUsagePageDataModel({
  tableTab,
  scope,
  period,
  cliKey,
  providerId,
  customApplied,
  bounds,
}: UsagePageDataModelArgs): UsagePageDataModel {
  const { tauriAvailable, shouldLoad, customPending, input } = useUsagePageQueryInput({
    period,
    cliKey,
    providerId,
    customApplied,
    bounds,
  });
  const {
    summaryQuery,
    leaderboardQuery,
    cacheTrendQuery,
    metricsTrendQuery,
    dataLoading,
    cacheTrendLoading,
    metricsTrendLoading,
    loading,
    dataStale,
    cacheTrendStale,
    metricsTrendStale,
    handleRetry,
  } = useUsagePageQueries({ scope, period, tableTab, shouldLoad, input });

  const summary: UsageSummary | null = summaryQuery.data ?? null;
  const rows: UsageLeaderboardRow[] = leaderboardQuery.data ?? EMPTY_USAGE_ROWS;
  const cacheTrendRows: UsageProviderCacheRateTrendRowV1[] =
    cacheTrendQuery.data ?? EMPTY_CACHE_TREND_ROWS;
  const metricsTrendRows: UsageProviderMetricsTrendRowV1[] =
    metricsTrendQuery.data ?? EMPTY_METRICS_TREND_ROWS;

  const cacheTrendProviderCount = useMemo(
    () => providerCountFromRows(cacheTrendRows),
    [cacheTrendRows]
  );
  const metricsTrendProviderCount = useMemo(
    () => providerCountFromRows(metricsTrendRows),
    [metricsTrendRows]
  );
  const totalCostUsd = useMemo(() => totalCostUsdFromRows(rows), [rows]);

  const err =
    tableTab === "cacheTrend"
      ? cacheTrendQuery.error
      : tableTab === "metricsTrend"
        ? metricsTrendQuery.error
        : (summaryQuery.error ?? leaderboardQuery.error);
  const errorText = err ? formatUnknownError(err) : null;

  return {
    tauriAvailable,
    shouldLoad,
    customPending,
    loading,
    dataLoading,
    cacheTrendLoading,
    dataStale,
    cacheTrendStale,
    errorText,
    summary,
    rows,
    cacheTrendRows,
    cacheTrendProviderCount,
    metricsTrendLoading,
    metricsTrendStale,
    metricsTrendRows,
    metricsTrendProviderCount,
    totalCostUsd,
    handleRetry,
  };
}
