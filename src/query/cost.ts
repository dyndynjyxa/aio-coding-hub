// Usage:
// - Query adapters for `src/services/cost.ts` used by `src/components/home/HomeCostPanel.tsx`.

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import {
  costBreakdownModelV1,
  costBreakdownProviderV1,
  costScatterCliProviderModelV1,
  costSummaryV1,
  costTopRequestsV1,
  costTrendV1,
  type CostModelBreakdownRowV1,
  type CostPeriod,
  type CostProviderBreakdownRowV1,
  type CostScatterCliProviderModelRowV1,
  type CostSummaryV1,
  type CostTopRequestRowV1,
  type CostTrendRowV1,
} from "../services/usage/cost";
import type { CliKey } from "../services/providers/providers";
import { costKeys } from "./keys";

export type CostFilters = {
  startTs: number | null;
  endTs: number | null;
  cliKey: CliKey | null;
  providerId: number | null;
  model: string | null;
};

export type CostAnalyticsV1 = {
  summary: CostSummaryV1;
  trend: CostTrendRowV1[];
  providers: CostProviderBreakdownRowV1[];
  models: CostModelBreakdownRowV1[];
  scatter: CostScatterCliProviderModelRowV1[];
  topRequests: CostTopRequestRowV1[];
};

export function useCostAnalyticsV1Query(
  period: CostPeriod,
  filters: CostFilters,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: costKeys.analyticsV1(period, filters),
    queryFn: async () => {
      const [summary, trend, providers, models, scatter, top] = await Promise.all([
        costSummaryV1(period, filters),
        costTrendV1(period, filters),
        costBreakdownProviderV1(period, { ...filters, limit: 120 }),
        costBreakdownModelV1(period, { ...filters, limit: 120 }),
        costScatterCliProviderModelV1(period, { ...filters, limit: 500 }),
        costTopRequestsV1(period, { ...filters, limit: 50 }),
      ]);

      if (!summary || !trend || !providers || !models || !scatter || !top) return null;

      return {
        summary,
        trend,
        providers,
        models,
        scatter,
        topRequests: top,
      } satisfies CostAnalyticsV1;
    },
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}
