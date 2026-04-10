// Usage:
// - Used by `src/components/home/HomeCostPanel.tsx` to load cost analytics for the Home "花费" tab.

import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

export type CostPeriod = "daily" | "weekly" | "monthly" | "allTime" | "custom";

export type CostSummaryV1 = {
  requests_total: number;
  requests_success: number;
  requests_failed: number;
  cost_covered_success: number;
  total_cost_usd: number;
  avg_cost_usd_per_covered_success: number | null;
};

export type CostTrendRowV1 = {
  day: string;
  hour: number | null;
  cost_usd: number;
  requests_success: number;
  cost_covered_success: number;
};

export type CostProviderBreakdownRowV1 = {
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  requests_success: number;
  cost_covered_success: number;
  cost_usd: number;
};

export type CostModelBreakdownRowV1 = {
  model: string;
  requests_success: number;
  cost_covered_success: number;
  cost_usd: number;
};

export type CostScatterCliProviderModelRowV1 = {
  cli_key: CliKey;
  provider_name: string;
  model: string;
  requests_success: number;
  total_cost_usd: number;
  total_duration_ms: number;
};
export type CostTopRequestRowV1 = {
  log_id: number;
  trace_id: string;
  cli_key: CliKey;
  method: string;
  path: string;
  requested_model: string | null;
  provider_id: number;
  provider_name: string;
  duration_ms: number;
  ttfb_ms: number | null;
  cost_usd: number;
  cost_multiplier: number;
  created_at: number;
};

export type CostBackfillReportV1 = {
  scanned: number;
  updated: number;
  skipped_no_model: number;
  skipped_no_usage: number;
  skipped_no_price: number;
  skipped_other: number;
  capped: boolean;
  max_rows: number;
};

type CostQueryInput = {
  startTs?: number | null;
  endTs?: number | null;
  cliKey?: CliKey | null;
  providerId?: number | null;
  model?: string | null;
};

function buildParams(period: CostPeriod, input?: CostQueryInput) {
  return {
    period,
    startTs: input?.startTs ?? null,
    endTs: input?.endTs ?? null,
    cliKey: input?.cliKey ?? null,
    providerId: input?.providerId ?? null,
    model: input?.model ?? null,
  };
}

export async function costSummaryV1(period: CostPeriod, input?: CostQueryInput) {
  return invokeService<CostSummaryV1>("读取花费汇总失败", "cost_summary_v1", {
    params: buildParams(period, input),
  });
}
export async function costTrendV1(period: CostPeriod, input?: CostQueryInput) {
  return invokeService<CostTrendRowV1[]>("读取花费趋势失败", "cost_trend_v1", {
    params: buildParams(period, input),
  });
}

export async function costBreakdownProviderV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  return invokeService<CostProviderBreakdownRowV1[]>(
    "读取按供应商花费分布失败",
    "cost_breakdown_provider_v1",
    {
      params: buildParams(period, input),
      limit: input?.limit ?? null,
    }
  );
}

export async function costBreakdownModelV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  return invokeService<CostModelBreakdownRowV1[]>(
    "读取按模型花费分布失败",
    "cost_breakdown_model_v1",
    {
      params: buildParams(period, input),
      limit: input?.limit ?? null,
    }
  );
}

export async function costTopRequestsV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  return invokeService<CostTopRequestRowV1[]>("读取高花费请求失败", "cost_top_requests_v1", {
    params: buildParams(period, input),
    limit: input?.limit ?? null,
  });
}

export async function costScatterCliProviderModelV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  return invokeService<CostScatterCliProviderModelRowV1[]>(
    "读取花费散点数据失败",
    "cost_scatter_cli_provider_model_v1",
    {
      params: buildParams(period, input),
      limit: input?.limit ?? null,
    }
  );
}

export async function costBackfillMissingV1(
  period: CostPeriod,
  input?: CostQueryInput & { maxRows?: number | null }
) {
  return invokeService<CostBackfillReportV1>("回填花费数据失败", "cost_backfill_missing_v1", {
    params: buildParams(period, input),
    maxRows: input?.maxRows ?? null,
  });
}
