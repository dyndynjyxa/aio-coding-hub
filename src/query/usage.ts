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

export function useUsageSummaryQuery(
  range: UsageRange,
  input: { cliKey: CliKey | null },
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  return useQuery({
    queryKey: usageKeys.summary(range, input),
    queryFn: () => usageSummary(range, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useUsageHourlySeriesQuery(
  days: number,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
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
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: usageKeys.summaryV2(period, input),
    queryFn: () => usageSummaryV2(period, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
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
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: usageKeys.leaderboardV2(scope, period, input),
    queryFn: () => usageLeaderboardV2(scope, period, input),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
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
