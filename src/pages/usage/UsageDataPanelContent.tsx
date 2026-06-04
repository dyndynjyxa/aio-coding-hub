import type { ReactNode, RefObject } from "react";
import type { UsageDataPanelProps } from "./UsageDataPanel";
import { Button } from "../../ui/Button";
import { TabList } from "../../ui/TabList";
import { formatInteger } from "../../utils/formatters";
import { PROVIDER_FILTER_ALL, SCOPE_ITEMS, USAGE_TABLE_TAB_ITEMS } from "./constants";
import { CacheTrendBody, UsageTableBody } from "./UsageDataPanelBodies";
import { UsageAvailabilityPanel } from "../../components/usage/UsageAvailabilityPanel";

function UsageScopeGroup({
  scope,
  onChangeScope,
  loading,
}: Pick<UsageDataPanelProps, "scope" | "onChangeScope" | "loading">) {
  return (
    <div className="flex items-center gap-1.5" role="group" aria-label="维度筛选">
      {SCOPE_ITEMS.map((item) => (
        <Button
          key={item.key}
          size="sm"
          variant={scope === item.key ? "primary" : "secondary"}
          aria-pressed={scope === item.key}
          onClick={() => onChangeScope(item.key)}
          disabled={loading}
          className="whitespace-nowrap"
        >
          {item.label}
        </Button>
      ))}
    </div>
  );
}

function UsagePanelTitle({
  tableTab,
  cacheTrendProviderCount,
}: Pick<UsageDataPanelProps, "tableTab" | "cacheTrendProviderCount">) {
  if (tableTab === "cacheTrend") {
    if (cacheTrendProviderCount > 0) {
      return `${formatInteger(cacheTrendProviderCount)} 供应商 · 命中率走势`;
    }
    return "命中率走势";
  }
  return null;
}

function UsageProviderFilterSelect({
  providerSelectValue,
  providerOptions,
  onProviderIdChange,
  loading,
}: Pick<UsageDataPanelProps, "providerSelectValue" | "providerOptions" | "onProviderIdChange"> & {
  loading: boolean;
}) {
  return (
    <label className="flex items-center gap-2">
      <span className="text-xs font-medium text-muted-foreground whitespace-nowrap">供应商</span>
      <select
        value={providerSelectValue}
        aria-label="供应商筛选"
        onChange={(e) => {
          const next = e.currentTarget.value;
          onProviderIdChange(next === PROVIDER_FILTER_ALL ? null : Number(next));
        }}
        disabled={loading}
        className="h-8 min-w-44 rounded-md border border-border bg-white dark:bg-secondary px-2 text-xs text-foreground outline-none transition focus:border-accent focus:ring-2 focus:ring-accent/20 disabled:cursor-not-allowed disabled:bg-secondary dark:disabled:bg-card"
      >
        <option value={PROVIDER_FILTER_ALL}>全部</option>
        {providerOptions.map((option) => (
          <option key={option.id} value={String(option.id)}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function UsageDataPanelHeader({
  tableTab,
  onChangeTableTab,
  scope,
  onChangeScope,
  loading,
  cacheTrendProviderCount,
  providerSelectValue,
  providerOptions,
  onProviderIdChange,
  providersLoading,
}: Pick<
  UsageDataPanelProps,
  | "tableTab"
  | "onChangeTableTab"
  | "scope"
  | "onChangeScope"
  | "loading"
  | "cacheTrendProviderCount"
  | "providerSelectValue"
  | "providerOptions"
  | "onProviderIdChange"
  | "providersLoading"
>) {
  const titleText = UsagePanelTitle({
    tableTab,
    cacheTrendProviderCount,
  });

  return (
    <div className="flex shrink-0 flex-wrap items-center justify-between gap-3 px-6 pb-0 pt-5">
      <div className="flex items-center gap-4">
        <TabList
          ariaLabel="用量数据视图"
          items={USAGE_TABLE_TAB_ITEMS}
          value={tableTab}
          onChange={onChangeTableTab}
          className="shrink-0"
          size="sm"
        />
        {tableTab === "usage" && (
          <UsageScopeGroup scope={scope} onChangeScope={onChangeScope} loading={loading} />
        )}
      </div>
      <div className="flex items-center gap-3">
        {titleText ? <div className="text-xs text-muted-foreground">{titleText}</div> : null}
        <UsageProviderFilterSelect
          providerSelectValue={providerSelectValue}
          providerOptions={providerOptions}
          onProviderIdChange={onProviderIdChange}
          loading={loading || providersLoading}
        />
      </div>
    </div>
  );
}

function UsageStaleBar({ active }: { active: boolean }) {
  if (!active) return null;
  return (
    <div className="h-0.5 w-full overflow-hidden bg-secondary">
      <div className="h-full w-1/3 animate-[loading_1.5s_ease-in-out_infinite] bg-accent" />
    </div>
  );
}

function UsageDataPanelScrollArea({
  activeStale,
  children,
}: {
  activeStale: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`mt-4 min-h-0 flex-1 lg:overflow-y-auto scrollbar-overlay transition-opacity ${
        activeStale ? "opacity-60" : ""
      }`}
    >
      {children}
    </div>
  );
}

function CacheTrendPanelBody({
  activeStale,
  cacheTrendLoading,
  cacheTrendRows,
  errorText,
  customPending,
  period,
  customApplied,
}: Pick<
  UsageDataPanelProps,
  | "cacheTrendLoading"
  | "cacheTrendRows"
  | "errorText"
  | "customPending"
  | "period"
  | "customApplied"
> & { activeStale: boolean }) {
  return (
    <UsageDataPanelScrollArea activeStale={activeStale}>
      <div className="px-6 pb-6">
        <CacheTrendBody
          cacheTrendLoading={cacheTrendLoading}
          cacheTrendRows={cacheTrendRows}
          errorText={errorText}
          customPending={customPending}
          period={period}
          customApplied={customApplied}
        />
      </div>
    </UsageDataPanelScrollArea>
  );
}

function UsageTablePanelBody({
  activeStale,
  dataLoading,
  rows,
  summary,
  totalCostUsd,
  errorText,
  customPending,
}: Pick<
  UsageDataPanelProps,
  "dataLoading" | "rows" | "summary" | "totalCostUsd" | "errorText" | "customPending"
> & { activeStale: boolean }) {
  return (
    <UsageDataPanelScrollArea activeStale={activeStale}>
      <UsageTableBody
        dataLoading={dataLoading}
        rows={rows}
        summary={summary}
        totalCostUsd={totalCostUsd}
        errorText={errorText}
        customPending={customPending}
      />
    </UsageDataPanelScrollArea>
  );
}

function AvailabilityPanelBody({
  activeStale,
  availabilityData,
  availabilityLoading,
  availabilityRefreshing,
  onRefreshAvailability,
}: Pick<
  UsageDataPanelProps,
  "availabilityData" | "availabilityLoading" | "availabilityRefreshing" | "onRefreshAvailability"
> & { activeStale: boolean }) {
  return (
    <UsageDataPanelScrollArea activeStale={activeStale}>
      <UsageAvailabilityPanel
        data={availabilityData}
        loading={availabilityLoading}
        onRefresh={onRefreshAvailability}
        refreshing={availabilityRefreshing}
      />
    </UsageDataPanelScrollArea>
  );
}

function UsageDataPanelBody({
  tableTab,
  activeStale,
  cacheTrendLoading,
  cacheTrendRows,
  errorText,
  customPending,
  period,
  customApplied,
  dataLoading,
  rows,
  summary,
  totalCostUsd,
  availabilityData,
  availabilityLoading,
  availabilityRefreshing,
  onRefreshAvailability,
}: Pick<
  UsageDataPanelProps,
  | "tableTab"
  | "cacheTrendLoading"
  | "cacheTrendRows"
  | "errorText"
  | "customPending"
  | "period"
  | "customApplied"
  | "dataLoading"
  | "rows"
  | "summary"
  | "totalCostUsd"
  | "availabilityData"
  | "availabilityLoading"
  | "availabilityRefreshing"
  | "onRefreshAvailability"
> & { activeStale: boolean }) {
  if (tableTab === "availability") {
    return (
      <AvailabilityPanelBody
        activeStale={activeStale}
        availabilityData={availabilityData}
        availabilityLoading={availabilityLoading}
        availabilityRefreshing={availabilityRefreshing}
        onRefreshAvailability={onRefreshAvailability}
      />
    );
  }

  if (tableTab === "cacheTrend") {
    return (
      <CacheTrendPanelBody
        activeStale={activeStale}
        cacheTrendLoading={cacheTrendLoading}
        cacheTrendRows={cacheTrendRows}
        errorText={errorText}
        customPending={customPending}
        period={period}
        customApplied={customApplied}
      />
    );
  }

  return (
    <UsageTablePanelBody
      activeStale={activeStale}
      dataLoading={dataLoading}
      rows={rows}
      summary={summary}
      totalCostUsd={totalCostUsd}
      errorText={errorText}
      customPending={customPending}
    />
  );
}

export function UsageDataPanelContent({
  contentRef,
  overlayOpen,
  activeStale,
  ...props
}: UsageDataPanelProps & {
  contentRef: RefObject<HTMLDivElement | null>;
  overlayOpen: boolean;
  activeStale: boolean;
}) {
  return (
    <div
      ref={contentRef}
      className={`flex min-h-0 flex-1 flex-col ${overlayOpen ? "pointer-events-none" : ""}`}
      aria-hidden={overlayOpen || undefined}
    >
      <UsageDataPanelHeader
        tableTab={props.tableTab}
        onChangeTableTab={props.onChangeTableTab}
        scope={props.scope}
        onChangeScope={props.onChangeScope}
        loading={props.loading}
        cacheTrendProviderCount={props.cacheTrendProviderCount}
        providerSelectValue={props.providerSelectValue}
        providerOptions={props.providerOptions}
        onProviderIdChange={props.onProviderIdChange}
        providersLoading={props.providersLoading}
      />
      <UsageStaleBar active={activeStale} />
      <UsageDataPanelBody
        tableTab={props.tableTab}
        activeStale={activeStale}
        cacheTrendLoading={props.cacheTrendLoading}
        cacheTrendRows={props.cacheTrendRows}
        errorText={props.errorText}
        customPending={props.customPending}
        period={props.period}
        customApplied={props.customApplied}
        dataLoading={props.dataLoading}
        rows={props.rows}
        summary={props.summary}
        totalCostUsd={props.totalCostUsd}
        availabilityData={props.availabilityData}
        availabilityLoading={props.availabilityLoading}
        availabilityRefreshing={props.availabilityRefreshing}
        onRefreshAvailability={props.onRefreshAvailability}
      />
    </div>
  );
}
