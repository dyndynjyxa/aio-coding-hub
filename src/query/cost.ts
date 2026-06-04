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
  normalizeCostQueryInput,
  type CostModelBreakdownRowV1,
  type CostPeriod,
  type CostProviderBreakdownRowV1,
  type CostScatterCliProviderModelRowV1,
  type CostSummaryV1,
  type CostTopRequestRowV1,
  type CostTrendRowV1,
  type NormalizedCostQueryInput,
} from "../services/usage/cost";
import { costKeys } from "./keys";

export type CostFilters = NormalizedCostQueryInput;

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
  const normalizedFilters = normalizeCostQueryInput(filters);

  return useQuery({
    queryKey: costKeys.analyticsV1(period, normalizedFilters),
    queryFn: async () => {
      const [summary, trend, providers, models, scatter, top] = await Promise.all([
        costSummaryV1(period, normalizedFilters),
        costTrendV1(period, normalizedFilters),
        costBreakdownProviderV1(period, { ...normalizedFilters, limit: 120 }),
        costBreakdownModelV1(period, { ...normalizedFilters, limit: 120 }),
        costScatterCliProviderModelV1(period, { ...normalizedFilters, limit: 500 }),
        costTopRequestsV1(period, { ...normalizedFilters, limit: 50 }),
      ]);

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
