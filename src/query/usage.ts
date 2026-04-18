import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  usageHourlySeries,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageSummary,
  usageSummaryV2,
  type UsagePeriod,
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

export function useUsageSummaryQuery(
  range: UsageRange,
  input: { cliKey: CliKey | null },
  options?: UsageQueryOptions
) {
  return useQuery({
    queryKey: usageKeys.summary(range, input),
    queryFn: () => usageSummary(range, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageHourlySeriesQuery(days: number, options?: UsageQueryOptions) {
  return useQuery({
    queryKey: usageKeys.hourlySeries(days),
    queryFn: () => usageHourlySeries(days),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageSummaryV2Query(
  period: UsagePeriod,
  input: {
    startTs: number | null;
    endTs: number | null;
    cliKey: CliKey | null;
    providerId: number | null;
  },
  options?: UsageV2QueryOptions
) {
  return useQuery({
    queryKey: usageKeys.summaryV2(period, input),
    queryFn: () => usageSummaryV2(period, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchOnMount: options?.refetchOnMount,
  });
}

export function useUsageLeaderboardV2Query(
  scope: UsageScope,
  period: UsagePeriod,
  input: {
    startTs: number | null;
    endTs: number | null;
    cliKey: CliKey | null;
    providerId: number | null;
    limit: number | null;
  },
  options?: UsageV2QueryOptions
) {
  return useQuery({
    queryKey: usageKeys.leaderboardV2(scope, period, input),
    queryFn: () => usageLeaderboardV2(scope, period, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
    refetchOnMount: options?.refetchOnMount,
  });
}

export function useUsageProviderCacheRateTrendV1Query(
  period: UsagePeriod,
  input: {
    startTs: number | null;
    endTs: number | null;
    cliKey: CliKey | null;
    providerId: number | null;
    limit: number | null;
  },
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: usageKeys.providerCacheRateTrendV1(period, input),
    queryFn: () => usageProviderCacheRateTrendV1(period, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}
