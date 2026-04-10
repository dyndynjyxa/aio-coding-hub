// Usage:
// - Used by `src/components/home/HomeProviderLimitPanel.tsx` to load provider limit usage data.

import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "./providers";

export type ProviderLimitUsageRow = {
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  enabled: boolean;
  // Limits (null if not configured)
  limit_5h_usd: number | null;
  limit_daily_usd: number | null;
  daily_reset_mode: string | null;
  daily_reset_time: string | null;
  limit_weekly_usd: number | null;
  limit_monthly_usd: number | null;
  limit_total_usd: number | null;
  // Current usage for each window
  usage_5h_usd: number;
  usage_daily_usd: number;
  usage_weekly_usd: number;
  usage_monthly_usd: number;
  usage_total_usd: number;
  // Window start timestamps (unix seconds) for UI display
  window_5h_start_ts: number;
  window_daily_start_ts: number;
  window_weekly_start_ts: number;
  window_monthly_start_ts: number;
};

export async function providerLimitUsageV1(cliKey?: CliKey | null) {
  return invokeService<ProviderLimitUsageRow[]>(
    "读取 Provider 限额用量失败",
    "provider_limit_usage_v1",
    {
      cliKey: cliKey ?? null,
    }
  );
}
