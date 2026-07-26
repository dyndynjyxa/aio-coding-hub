import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  USAGE_LEADERBOARD_V2_DEFAULT_LIMIT,
  usageHourlySeries,
  usageFolderOptionsV1,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageProviderMetricsTrendV1,
  usageSummary,
  usageSummaryV2,
  normalizeUsageHourlySeriesDays,
  normalizeUsageLeaderboardV2Limit,
  normalizeUsageProviderTrendLimit,
  normalizeUsageQueryInputV2,
  normalizeUsageSummaryInput,
  type UsagePeriod,
  type UsageQueryInputV2,
  type UsageRange,
  type UsageScope,
} from "../services/usage/usage";
import { usageKeys } from "./keys";

type UsageQueryOptions = {
  enabled?: boolean;
  refetchIntervalMs?: number | false;
};

export type UsageV2QueryOptions = UsageQueryOptions & {
  refetchOnMount?: boolean | "always";
};

type UsageLeaderboardV2QueryInput = UsageQueryInputV2 & {
  limit: number | null;
};
type UsageQueryInputV2WithoutFolderKeys = Omit<UsageQueryInputV2, "folderKeys">;
type UsageProviderTrendQueryInput = Omit<
  UsageQueryInputV2,
  "folderKeys" | "dayStartHour" | "fullIdleGapMinutes" | "sessionBreakGapMinutes"
> & {
  limit: number | null;
};
type NormalizedUsageProviderTrendQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: CliKey | null;
  providerId: number | null;
  limit: number | null;
  excludeCx2CcGatewayBridge: boolean | null;
};

export function useUsageSummaryQuery(
  range: UsageRange,
  input: { cliKey: CliKey | null },
  options?: UsageQueryOptions
) {
  const normalizedInput = normalizeUsageSummaryInput(input);

  return useQuery({
    queryKey: usageKeys.summary(range, normalizedInput),
    queryFn: () => usageSummary(range, normalizedInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageHourlySeriesQuery(days: number, options?: UsageQueryOptions) {
  const normalizedDays = normalizeUsageHourlySeriesDays(days);

  return useQuery({
    queryKey: usageKeys.hourlySeries(normalizedDays),
    queryFn: () => usageHourlySeries(normalizedDays),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageSummaryV2Query(
  period: UsagePeriod,
  input: UsageQueryInputV2,
  options?: UsageV2QueryOptions
) {
  const normalizedInput = normalizeUsageQueryInputV2(input);

  return useQuery({
    queryKey: usageKeys.summaryV2(period, normalizedInput),
    queryFn: () => usageSummaryV2(period, normalizedInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchOnMount: options?.refetchOnMount,
  });
}

export function useUsageLeaderboardV2Query(
  scope: UsageScope,
  period: UsagePeriod,
  input: UsageLeaderboardV2QueryInput,
  options?: UsageV2QueryOptions
) {
  const normalizedInput = {
    ...normalizeUsageQueryInputV2(input),
    limit: normalizeUsageLeaderboardV2Limit(input.limit) ?? USAGE_LEADERBOARD_V2_DEFAULT_LIMIT,
  };

  return useQuery({
    queryKey: usageKeys.leaderboardV2(scope, period, normalizedInput),
    queryFn: () => usageLeaderboardV2(scope, period, normalizedInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchOnMount: options?.refetchOnMount,
  });
}

export function useUsageFolderOptionsV1Query(
  period: UsagePeriod,
  input: UsageQueryInputV2WithoutFolderKeys,
  options?: UsageQueryOptions
) {
  const normalizedInput = normalizeUsageQueryInputV2({ ...input, folderKeys: null });

  return useQuery({
    queryKey: usageKeys.folderOptionsV1(period, normalizedInput),
    queryFn: () => usageFolderOptionsV1(period, normalizedInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageProviderCacheRateTrendV1Query(
  period: UsagePeriod,
  input: UsageProviderTrendQueryInput,
  options?: { enabled?: boolean }
) {
  const normalizedInput = {
    ...normalizeUsageQueryInputV2({ ...input, folderKeys: null, dayStartHour: null }),
    limit: normalizeUsageProviderTrendLimit(input.limit),
  };
  const queryInput: NormalizedUsageProviderTrendQueryInput = {
    startTs: normalizedInput.startTs,
    endTs: normalizedInput.endTs,
    cliKey: normalizedInput.cliKey,
    providerId: normalizedInput.providerId,
    limit: normalizedInput.limit,
    excludeCx2CcGatewayBridge: normalizedInput.excludeCx2CcGatewayBridge,
  };

  return useQuery({
    queryKey: usageKeys.providerCacheRateTrendV1(period, queryInput),
    queryFn: () => usageProviderCacheRateTrendV1(period, queryInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useUsageProviderMetricsTrendV1Query(
  period: UsagePeriod,
  input: UsageProviderTrendQueryInput,
  options?: { enabled?: boolean }
) {
  const normalizedInput = {
    ...normalizeUsageQueryInputV2({ ...input, folderKeys: null, dayStartHour: null }),
    limit: normalizeUsageProviderTrendLimit(input.limit),
  };
  const queryInput: NormalizedUsageProviderTrendQueryInput = {
    startTs: normalizedInput.startTs,
    endTs: normalizedInput.endTs,
    cliKey: normalizedInput.cliKey,
    providerId: normalizedInput.providerId,
    limit: normalizedInput.limit,
    excludeCx2CcGatewayBridge: normalizedInput.excludeCx2CcGatewayBridge,
  };

  return useQuery({
    queryKey: usageKeys.providerMetricsTrendV1(period, queryInput),
    queryFn: () => usageProviderMetricsTrendV1(period, queryInput),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}
