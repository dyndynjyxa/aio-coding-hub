import { invokeTauriOrNull } from "./tauriInvoke";
import type { CliKey } from "./providers";

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
};

export async function usageSummary(range: UsageRange, input?: { cliKey?: CliKey | null }) {
  return invokeTauriOrNull<UsageSummary>("usage_summary", {
    range,
    cliKey: input?.cliKey ?? null,
  });
}

export async function usageLeaderboardProvider(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeTauriOrNull<UsageProviderRow[]>("usage_leaderboard_provider", {
    range,
    cliKey: input?.cliKey ?? null,
    limit: input?.limit,
  });
}

export async function usageLeaderboardDay(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeTauriOrNull<UsageDayRow[]>("usage_leaderboard_day", {
    range,
    cliKey: input?.cliKey ?? null,
    limit: input?.limit,
  });
}

export async function usageHourlySeries(days: number) {
  return invokeTauriOrNull<UsageHourlyRow[]>("usage_hourly_series", { days });
}

export async function usageSummaryV2(
  period: UsagePeriod,
  input?: { startTs?: number | null; endTs?: number | null; cliKey?: CliKey | null }
) {
  return invokeTauriOrNull<UsageSummary>("usage_summary_v2", {
    period,
    startTs: input?.startTs ?? null,
    endTs: input?.endTs ?? null,
    cliKey: input?.cliKey ?? null,
  });
}

export async function usageLeaderboardV2(
  scope: UsageScope,
  period: UsagePeriod,
  input?: { startTs?: number | null; endTs?: number | null; cliKey?: CliKey | null; limit?: number }
) {
  return invokeTauriOrNull<UsageLeaderboardRow[]>("usage_leaderboard_v2", {
    scope,
    period,
    startTs: input?.startTs ?? null,
    endTs: input?.endTs ?? null,
    cliKey: input?.cliKey ?? null,
    limit: input?.limit,
  });
}
