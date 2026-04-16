// Usage:
// - Rendered by `src/pages/HomePage.tsx` when the Home tab is switched to "花费".
// - Provides cost analytics with period + CLI + provider + model filters and charts.
// - Composed from extracted sub-components for maintainability.

import { useCallback } from "react";
import { useCostFilters } from "./useCostFilters";
import { CostFilterPanel } from "./CostFilterPanel";
import { CostStatCards } from "./CostStatCards";
import { CostDonutCharts } from "./CostDonutChart";
import { CostTrendChart } from "./CostTrendChart";
import { CostScatterChartCard } from "./CostScatterChart";
import { CostErrorCard } from "./CostErrorCard";

type HomeCostPanelProps = {
  devPreviewEnabled?: boolean;
};

export function HomeCostPanel({ devPreviewEnabled = false }: HomeCostPanelProps) {
  const f = useCostFilters({ devPreviewEnabled });

  const { setProviderId, setModel, costQuery } = f;

  const handleProviderChange = useCallback(
    (value: string) => {
      if (value === "all") {
        setProviderId(null);
        return;
      }
      const n = Number(value);
      if (!Number.isFinite(n) || n <= 0) {
        setProviderId(null);
        return;
      }
      setProviderId(Math.floor(n));
    },
    [setProviderId]
  );

  const handleModelChange = useCallback(
    (value: string) => {
      setModel(value === "all" ? null : value);
    },
    [setModel]
  );

  const handleRefresh = useCallback(() => {
    void costQuery.refetch();
  }, [costQuery]);

  const hasData = !!(f.summary && f.summary.requests_success > 0);

  return (
    <div className="flex flex-col gap-5 h-full overflow-auto">
      <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
        <CostFilterPanel
          period={f.period}
          setPeriod={f.setPeriod}
          cliKey={f.cliKey}
          setCliKey={f.setCliKey}
          providerSelectValue={f.providerSelectValue}
          onProviderChange={handleProviderChange}
          modelSelectValue={f.modelSelectValue}
          onModelChange={handleModelChange}
          providerOptions={f.providerOptions}
          modelOptions={f.modelOptions}
          fetching={f.fetching}
          tauriAvailable={f.tauriAvailable}
          showCustomForm={f.showCustomForm}
          customStartDate={f.customStartDate}
          setCustomStartDate={f.setCustomStartDate}
          customEndDate={f.customEndDate}
          setCustomEndDate={f.setCustomEndDate}
          customApplied={f.customApplied}
          applyCustomRange={f.applyCustomRange}
          clearCustomRange={f.clearCustomRange}
          onRefresh={handleRefresh}
        />

        <div className="lg:col-span-5 flex flex-col gap-3">
          <CostStatCards
            loading={f.loading}
            summaryCards={f.summaryCards}
            period={f.period}
            customApplied={f.customApplied}
          />

          <CostDonutCharts
            providerData={f.providerDonutData}
            modelData={f.modelDonutData}
            period={f.period}
            loading={f.loading}
            isDark={f.isDark}
            hasData={hasData}
            customApplied={f.customApplied}
          />
        </div>
      </div>

      {f.errorText ? (
        <CostErrorCard errorText={f.errorText} fetching={f.fetching} onRetry={handleRefresh} />
      ) : null}

      <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
        <CostTrendChart
          data={f.trendChartData}
          period={f.period}
          isDark={f.isDark}
          loading={f.loading}
          fetching={f.fetching}
          hasData={hasData}
          cliKey={f.cliKey}
          onCliKeyChange={f.setCliKey}
        />

        <CostScatterChartCard
          scatterChartData={f.scatterChartData}
          scatterRows={f.scatterRows}
          isDark={f.isDark}
          loading={f.loading}
          fetching={f.fetching}
          scatterCliFilter={f.scatterCliFilter}
          onScatterCliFilterChange={f.setScatterCliFilter}
        />
      </div>
    </div>
  );
}
