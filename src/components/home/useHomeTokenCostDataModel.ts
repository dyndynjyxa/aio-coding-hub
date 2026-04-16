// Usage: Data model hook for HomeTokenCostPanel.
// Encapsulates query orchestration, preview fallback, and derived state.

import { useCallback, useMemo } from "react";
import type { UsageLeaderboardRow, UsagePeriod, UsageSummary } from "../../services/usage/usage";
import { useUsageLeaderboardV2Query, useUsageSummaryV2Query } from "../../query/usage";
import { formatUnknownError } from "../../utils/errors";
import {
  buildPreviewTokenSummary,
  scalePreviewTokenRows,
  PREVIEW_TOKEN_PROVIDER_ROWS,
  PREVIEW_TOKEN_MODEL_ROWS,
} from "./previewTokenData";

type TokenCostScope = "provider" | "model";

type TokenCostQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: null;
  providerId: null;
};

type TokenCostQueryConfig = {
  period: UsagePeriod;
  input: TokenCostQueryInput;
  previewFactor: number;
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
}: {
  scope: TokenCostScope;
  queryConfig: TokenCostQueryConfig;
  devPreviewEnabled: boolean;
}): HomeTokenCostDataModel {
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
