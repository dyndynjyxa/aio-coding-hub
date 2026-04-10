import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

export type UsageRange = "today" | "last7" | "last30" | "month" | "all";
export type UsageScope = "cli" | "provider" | "model";
export type UsagePeriod = "daily" | "weekly" | "monthly" | "allTime" | "custom";

export type UsageSummary = {
  requests_total: number;
  requests_with_usage: number;
  requests_success: number;
  requests_failed: number;
  avg_duration_ms: number | null;
  avg_ttfb_ms: number | null;
  avg_output_tokens_per_second: number | null;
  input_tokens: number;
  output_tokens: number;
  io_total_tokens: number;
  total_tokens: number;
  cache_read_input_tokens: number;
  cache_creation_input_tokens: number;
  cache_creation_5m_input_tokens: number;
};

export type UsageProviderRow = {
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  requests_total: number;
  requests_success: number;
  requests_failed: number;
  avg_duration_ms: number | null;
  avg_ttfb_ms: number | null;
  avg_output_tokens_per_second: number | null;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  cache_read_input_tokens: number;
  cache_creation_input_tokens: number;
  cache_creation_5m_input_tokens: number;
};

export type UsageDayRow = {
  day: string;
  requests_total: number;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  cache_read_input_tokens: number;
  cache_creation_input_tokens: number;
  cache_creation_5m_input_tokens: number;
};

export type UsageHourlyRow = {
  day: string;
  hour: number;
  requests_total: number;
  requests_with_usage: number;
  requests_success: number;
  requests_failed: number;
  total_tokens: number;
};

export type UsageProviderCacheRateTrendRowV1 = {
  day: string;
  hour: number | null;
  key: string;
  name: string;
  denom_tokens: number;
  cache_read_input_tokens: number;
  requests_success: number;
};

export type UsageLeaderboardRow = {
  key: string;
  name: string;
  requests_total: number;
  requests_success: number;
  requests_failed: number;
  total_tokens: number;
  io_total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_input_tokens: number;
  cache_read_input_tokens: number;
  avg_duration_ms: number | null;
  avg_ttfb_ms: number | null;
  avg_output_tokens_per_second: number | null;
  cost_usd: number | null;
};

type UsageQueryInputV2 = {
  startTs?: number | null;
  endTs?: number | null;
  cliKey?: CliKey | null;
  providerId?: number | null;
};

function buildQueryParamsV2(period: UsagePeriod, input?: UsageQueryInputV2) {
  return {
    period,
    startTs: input?.startTs ?? null,
    endTs: input?.endTs ?? null,
    cliKey: input?.cliKey ?? null,
    providerId: input?.providerId ?? null,
  };
}

export async function usageSummary(range: UsageRange, input?: { cliKey?: CliKey | null }) {
  return invokeService<UsageSummary>("读取用量汇总失败", "usage_summary", {
    range,
    cliKey: input?.cliKey ?? null,
  });
}

export async function usageLeaderboardProvider(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeService<UsageProviderRow[]>(
    "读取按供应商用量排行失败",
    "usage_leaderboard_provider",
    {
      range,
      cliKey: input?.cliKey ?? null,
      limit: input?.limit,
    }
  );
}

export async function usageLeaderboardDay(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeService<UsageDayRow[]>("读取按日期用量排行失败", "usage_leaderboard_day", {
    range,
    cliKey: input?.cliKey ?? null,
    limit: input?.limit,
  });
}

export async function usageHourlySeries(days: number) {
  return invokeService<UsageHourlyRow[]>("读取小时用量序列失败", "usage_hourly_series", { days });
}

export async function usageSummaryV2(period: UsagePeriod, input?: UsageQueryInputV2) {
  return invokeService<UsageSummary>("读取用量汇总失败", "usage_summary_v2", {
    params: buildQueryParamsV2(period, input),
  });
}

export async function usageLeaderboardV2(
  scope: UsageScope,
  period: UsagePeriod,
  input?: UsageQueryInputV2 & { limit?: number | null }
) {
  return invokeService<UsageLeaderboardRow[]>("读取用量排行榜失败", "usage_leaderboard_v2", {
    scope,
    params: buildQueryParamsV2(period, input),
    limit: input?.limit,
  });
}

export async function usageProviderCacheRateTrendV1(
  period: UsagePeriod,
  input?: UsageQueryInputV2 & { limit?: number | null }
) {
  return invokeService<UsageProviderCacheRateTrendRowV1[]>(
    "读取供应商缓存命中趋势失败",
    "usage_provider_cache_rate_trend_v1",
    {
      params: buildQueryParamsV2(period, input),
      limit: input?.limit,
    }
  );
}
