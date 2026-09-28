import { useRef } from "react";
import type { RefObject } from "react";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import type {
  UsageLeaderboardRow,
  UsagePeriod,
  UsageProviderCacheRateTrendRowV1,
  UsageProviderMetricsTrendRowV1,
  UsageScope,
  UsageSummary,
} from "../../services/usage/usage";
import type { AvailabilityTimelineData } from "../../components/usage/usageAvailabilityTimeline";
import { Card } from "../../ui/Card";
import type { UsageTableTab } from "./types";
import { useAutoFocus, useInert } from "./useInert";
import { UsageDataPanelContent } from "./UsageDataPanelContent";

export type UsageDataPanelProps = {
  tableTab: UsageTableTab;
  onChangeTableTab: (next: UsageTableTab) => void;
  scope: UsageScope;
  onChangeScope: (next: UsageScope) => void;
  loading: boolean;
  dataLoading: boolean;
  cacheTrendLoading: boolean;
  metricsTrendLoading: boolean;
  dataStale: boolean;
  cacheTrendStale: boolean;
  metricsTrendStale: boolean;
  errorText: string | null;
  tableTitle: string;
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  totalCostUsd: number;
  cacheTrendRows: UsageProviderCacheRateTrendRowV1[];
  cacheTrendProviderCount: number;
  metricsTrendRows: UsageProviderMetricsTrendRowV1[];
  metricsTrendProviderCount: number;
  providerSelectValue: string;
  providerOptions: readonly { id: number; label: string }[];
  onProviderIdChange: (providerId: number | null) => void;
  providersLoading: boolean;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
  customPending: boolean;
  availabilityData: AvailabilityTimelineData | null;
  availabilityLoading: boolean;
  availabilityRefreshing: boolean;
  onRefreshAvailability: () => void;
};

function overlayOpenForCustomPending({
  customPending,
  tableTab,
  rows,
  summary,
  cacheTrendRows,
  metricsTrendRows,
  availabilityData,
}: Pick<
  UsageDataPanelProps,
  | "customPending"
  | "tableTab"
  | "rows"
  | "summary"
  | "cacheTrendRows"
  | "metricsTrendRows"
  | "availabilityData"
>) {
  if (!customPending) return false;
  if (tableTab === "cacheTrend") return cacheTrendRows.length > 0;
  if (tableTab === "metricsTrend") return metricsTrendRows.length > 0;
  if (tableTab === "availability")
    return availabilityData != null && availabilityData.providers.length > 0;
  return rows.length > 0 || summary != null;
}

function CustomPendingOverlay({
  open,
  overlayRef,
}: {
  open: boolean;
  overlayRef: RefObject<HTMLOutputElement | null>;
}) {
  if (!open) return null;

  return (
    <output
      ref={overlayRef}
      tabIndex={-1}
      aria-live="polite"
      className="absolute inset-0 z-20 flex items-center justify-center rounded-lg bg-white/60 dark:bg-card/60 backdrop-blur-[1px]"
    >
      <div className="rounded-lg border border-border bg-white dark:bg-secondary px-6 py-4 text-center shadow-lg">
        <div className="text-sm font-medium text-secondary-foreground">
          请选择日期后点击"应用"查看数据
        </div>
        <div className="mt-1 text-xs text-muted-foreground">当前显示为上一次查询的缓存数据</div>
      </div>
    </output>
  );
}

export function UsageDataPanel(props: UsageDataPanelProps) {
  const overlayOpen = overlayOpenForCustomPending({
    customPending: props.customPending,
    tableTab: props.tableTab,
    rows: props.rows,
    summary: props.summary,
    cacheTrendRows: props.cacheTrendRows,
    metricsTrendRows: props.metricsTrendRows,
    availabilityData: props.availabilityData,
  });
  const activeStale =
    props.tableTab === "cacheTrend"
      ? props.cacheTrendStale
      : props.tableTab === "metricsTrend"
        ? props.metricsTrendStale
        : props.tableTab === "availability"
          ? props.availabilityRefreshing
          : props.dataStale;

  const contentRef = useRef<HTMLDivElement | null>(null);
  const overlayRef = useRef<HTMLOutputElement | null>(null);
  useInert(contentRef, overlayOpen);
  useAutoFocus(overlayRef, overlayOpen);

  return (
    <Card padding="none" className="relative flex min-h-0 flex-1 flex-col lg:overflow-hidden">
      <UsageDataPanelContent
        {...props}
        contentRef={contentRef}
        overlayOpen={overlayOpen}
        activeStale={activeStale}
      />
      <CustomPendingOverlay open={overlayOpen} overlayRef={overlayRef} />
    </Card>
  );
}
