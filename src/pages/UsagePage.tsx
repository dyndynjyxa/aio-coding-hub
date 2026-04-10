// Usage: Usage analytics page. Backend commands: `usage_summary_v2`, `usage_leaderboard_v2` (and related `usage_*`).

import type { UsageScope } from "../services/usage/usage";
import { UsageFilters } from "../components/usage/UsageFilters";
import { UsageSummaryCards } from "../components/usage/UsageSummaryCards";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { PageHeader } from "../ui/PageHeader";
import { UsageDataPanel } from "./usage/UsageDataPanel";
import { PROVIDER_FILTER_ALL } from "./usage/constants";
import { useUsagePageDataModel, type UsagePageDataModel } from "./usage/useUsagePageDataModel";
import { useUsagePageErrorToast } from "./usage/useUsagePageErrorToast";
import { useUsagePageFiltersState } from "./usage/useUsagePageFiltersState";
import { useUsagePageProviderFilter } from "./usage/useUsagePageProviderFilter";
import { useUsagePageTableState } from "./usage/useUsagePageTableState";

export function UsagePage() {
  const baseFilters = useUsagePageFiltersState();
  const providerFilter = useUsagePageProviderFilter(baseFilters.cliKey);
  const filters = { ...baseFilters, ...providerFilter };
  const table = useUsagePageTableState();
  const model = useUsagePageDataModel({
    tableTab: table.tableTab,
    scope: table.scope,
    period: filters.period,
    cliKey: filters.cliKey,
    providerId: filters.providerId,
    customApplied: filters.customApplied,
    bounds: filters.bounds,
  });

  useUsagePageErrorToast(model.errorText, table.tableTab);

  return <UsagePageView filters={filters} table={table} model={model} />;
}

type UsagePageFiltersState = ReturnType<typeof useUsagePageFiltersState> &
  ReturnType<typeof useUsagePageProviderFilter>;
type UsagePageTableState = ReturnType<typeof useUsagePageTableState>;

function tableTitleForScope(scope: UsageScope) {
  switch (scope) {
    case "cli":
      return "CLI";
    case "provider":
      return "供应商";
    case "model":
      return "模型";
    default:
      return "Leaderboard";
  }
}

function UsageErrorCard({
  errorText,
  loading,
  onRetry,
}: {
  errorText: string | null;
  loading: boolean;
  onRetry: () => void;
}) {
  if (!errorText) return null;

  return (
    <Card
      padding="md"
      className="shrink-0 border-rose-200 dark:border-rose-700 bg-rose-50 dark:bg-rose-900/30"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="text-sm font-semibold text-rose-900 dark:text-rose-400">加载失败</div>
          <div className="mt-1 text-sm text-rose-800 dark:text-rose-300">
            用量数据刷新失败，请重试；必要时查看 Console 日志定位 error_code。
          </div>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={onRetry}
          disabled={loading}
          className="border-rose-200 dark:border-rose-700 bg-white dark:bg-slate-800 text-rose-800 dark:text-rose-300 hover:bg-rose-50 dark:hover:bg-rose-900/30"
        >
          重试
        </Button>
      </div>
      <div className="mt-3 rounded-lg border border-rose-200 dark:border-rose-700 bg-white/60 dark:bg-slate-800/60 p-3 font-mono text-xs text-slate-800 dark:text-slate-300">
        {errorText}
      </div>
    </Card>
  );
}

function TauriUnavailableHint({ open }: { open: boolean }) {
  if (!open) return null;

  return (
    <Card padding="md" className="shrink-0">
      <div className="text-sm text-slate-600 dark:text-slate-400">
        当前环境未检测到 Tauri Runtime。请通过桌面端运行（`pnpm tauri dev`）后查看用量。
      </div>
    </Card>
  );
}

function UsagePageHeader({
  loading,
  filters,
}: {
  loading: boolean;
  filters: UsagePageFiltersState;
}) {
  return (
    <PageHeader
      title="用量分析"
      actions={
        <UsageFilters
          cliKey={filters.cliKey}
          onCliKeyChange={filters.setCliKey}
          period={filters.period}
          onPeriodChange={filters.setPeriod}
          loading={loading}
          showCustomForm={filters.showCustomForm}
          customStartDate={filters.customStartDate}
          customEndDate={filters.customEndDate}
          onCustomStartDateChange={filters.setCustomStartDate}
          onCustomEndDateChange={filters.setCustomEndDate}
          customApplied={filters.customApplied}
          onApplyCustomRange={filters.applyCustomRange}
          onClearCustomRange={filters.clearCustomRange}
        />
      }
    />
  );
}

function UsageDataPanelSection({
  filters,
  table,
  model,
}: {
  filters: UsagePageFiltersState;
  table: UsagePageTableState;
  model: UsagePageDataModel;
}) {
  return (
    <UsageDataPanel
      tableTab={table.tableTab}
      onChangeTableTab={table.onChangeTableTab}
      scope={table.scope}
      onChangeScope={table.setScope}
      loading={model.loading}
      dataLoading={model.dataLoading}
      cacheTrendLoading={model.cacheTrendLoading}
      dataStale={model.dataStale}
      cacheTrendStale={model.cacheTrendStale}
      errorText={model.errorText}
      tableTitle={tableTitleForScope(table.scope)}
      summary={model.summary}
      rows={model.rows}
      totalCostUsd={model.totalCostUsd}
      cacheTrendRows={model.cacheTrendRows}
      cacheTrendProviderCount={model.cacheTrendProviderCount}
      providerSelectValue={
        filters.providerId == null ? PROVIDER_FILTER_ALL : String(filters.providerId)
      }
      providerOptions={filters.providerOptions}
      onProviderIdChange={filters.setProviderId}
      providersLoading={filters.providersLoading}
      period={filters.period}
      customApplied={filters.customApplied}
      customPending={model.customPending}
    />
  );
}

function UsagePageView({
  filters,
  table,
  model,
}: {
  filters: UsagePageFiltersState;
  table: UsagePageTableState;
  model: UsagePageDataModel;
}) {
  return (
    <div className="flex flex-col gap-5 h-full overflow-hidden">
      <div className="shrink-0">
        <UsagePageHeader loading={model.loading} filters={filters} />
      </div>
      <div className="shrink-0">
        <UsageSummaryCards
          summary={model.summary}
          totalCostUsd={model.totalCostUsd}
          leaderboardCount={model.rows.length}
          loading={model.dataLoading}
        />
      </div>
      <UsageErrorCard
        errorText={model.errorText}
        loading={model.loading}
        onRetry={model.handleRetry}
      />
      <TauriUnavailableHint open={model.tauriAvailable === false} />
      <UsageDataPanelSection filters={filters} table={table} model={model} />
    </div>
  );
}
