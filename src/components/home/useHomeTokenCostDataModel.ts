// Usage: Data model hook for HomeTokenCostPanel.
// Encapsulates query orchestration, preview fallback, and derived state.

import { useCallback, useMemo } from "react";
import type { UsageLeaderboardRow, UsagePeriod, UsageSummary } from "../../services/usage/usage";
import {
  useUsageLeaderboardV2Query,
  useUsageSummaryV2Query,
  type UsageV2QueryOptions,
} from "../../query/usage";
import { formatUnknownError } from "../../utils/errors";
import {
  buildPreviewTokenSummary,
  previewFolderSelectionFactor,
  scalePreviewTokenRows,
  PREVIEW_TOKEN_PROVIDER_ROWS,
  PREVIEW_TOKEN_MODEL_ROWS,
  PREVIEW_TOKEN_DAY_ROWS,
} from "./previewTokenData";

type TokenCostScope = "provider" | "model" | "day";

type TokenCostQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: null;
  providerId: null;
  folderKeys?: string[] | null;
  dayStartHour?: number | null;
  excludeCx2CcGatewayBridge?: boolean | null;
};

type TokenCostQueryConfig = {
  period: UsagePeriod;
  input: TokenCostQueryInput;
  previewFactor: number;
};

export type HomeTokenCostDataModelQueryRefreshConfig = {
  summary?: UsageV2QueryOptions;
  leaderboard?: UsageV2QueryOptions;
};

const EMPTY_ROWS: UsageLeaderboardRow[] = [];

function isUsageSummaryEmpty(summary: UsageSummary | null) {
  return !summary || summary.requests_total <= 0 || summary.io_total_tokens <= 0;
}

function isUsageLeaderboardEmpty(rows: UsageLeaderboardRow[]) {
  return (
    rows.length === 0 || rows.every((row) => row.requests_total <= 0 || row.io_total_tokens <= 0)
  );
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

export type HomeTokenCostDataModel = {
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  totalCostUsd: number | null;
  loading: boolean;
  fetching: boolean;
  errorText: string | null;
  previewActive: boolean;
  refresh: () => void;
};

export function useHomeTokenCostDataModel({
  scope,
  queryConfig,
  devPreviewEnabled,
  queryRefreshConfig,
}: {
  scope: TokenCostScope;
  queryConfig: TokenCostQueryConfig;
  devPreviewEnabled: boolean;
  queryRefreshConfig?: HomeTokenCostDataModelQueryRefreshConfig;
}): HomeTokenCostDataModel {
  const queryInput = useMemo(
    () => ({
      ...queryConfig.input,
      limit: null,
    }),
    [queryConfig.input]
  );

  const previewFactor = useMemo(
    () => queryConfig.previewFactor * previewFolderSelectionFactor(queryConfig.input.folderKeys),
    [queryConfig.input.folderKeys, queryConfig.previewFactor]
  );

  const previewRowsByScope = useMemo(
    () => ({
      provider: scalePreviewTokenRows(PREVIEW_TOKEN_PROVIDER_ROWS, previewFactor),
      model: scalePreviewTokenRows(PREVIEW_TOKEN_MODEL_ROWS, previewFactor),
      day: scalePreviewTokenRows(PREVIEW_TOKEN_DAY_ROWS, previewFactor),
    }),
    [previewFactor]
  );
  const previewSummary = useMemo(
    () => buildPreviewTokenSummary(previewRowsByScope.provider),
    [previewRowsByScope.provider]
  );

  const summaryQuery = useUsageSummaryV2Query(
    queryConfig.period,
    queryConfig.input,
    queryRefreshConfig?.summary
  );
  const leaderboardQuery = useUsageLeaderboardV2Query(
    scope,
    queryConfig.period,
    queryInput,
    queryRefreshConfig?.leaderboard
  );

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
    return previewRowsByScope[scope];
  }, [previewActive, previewRowsByScope, rowsRaw, scope]);
  const totalCostUsd = useMemo(() => totalCostUsdFromRows(rows), [rows]);

  const refresh = useCallback(() => {
    void summaryQuery.refetch();
    void leaderboardQuery.refetch();
  }, [leaderboardQuery, summaryQuery]);

  return {
    summary,
    rows,
    totalCostUsd,
    loading,
    fetching,
    errorText,
    previewActive,
    refresh,
  };
}
