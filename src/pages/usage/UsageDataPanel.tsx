import { useRef } from "react";
import type { RefObject } from "react";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import type {
  UsageLeaderboardRow,
  UsagePeriod,
  UsageProviderCacheRateTrendRowV1,
  UsageScope,
  UsageSummary,
} from "../../services/usage/usage";
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
  dataStale: boolean;
  cacheTrendStale: boolean;
  errorText: string | null;
  tableTitle: string;
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  totalCostUsd: number;
  cacheTrendRows: UsageProviderCacheRateTrendRowV1[];
  cacheTrendProviderCount: number;
  providerSelectValue: string;
  providerOptions: readonly { id: number; label: string }[];
  onProviderIdChange: (providerId: number | null) => void;
  providersLoading: boolean;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
  customPending: boolean;
};

function overlayOpenForCustomPending({
  customPending,
  tableTab,
  rows,
  summary,
  cacheTrendRows,
}: Pick<
  UsageDataPanelProps,
  "customPending" | "tableTab" | "rows" | "summary" | "cacheTrendRows"
>) {
  if (!customPending) return false;
  if (tableTab === "cacheTrend") return cacheTrendRows.length > 0;
  return rows.length > 0 || summary != null;
}

function CustomPendingOverlay({
  open,
  overlayRef,
}: {
  open: boolean;
  overlayRef: RefObject<HTMLDivElement | null>;
}) {
  if (!open) return null;

  return (
    <div
      ref={overlayRef}
      tabIndex={-1}
      role="status"
      aria-live="polite"
      className="absolute inset-0 z-20 flex items-center justify-center rounded-lg bg-white/60 dark:bg-slate-900/60 backdrop-blur-[1px]"
    >
      <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-6 py-4 text-center shadow-lg">
        <div className="text-sm font-medium text-slate-700 dark:text-slate-300">
          请选择日期后点击"应用"查看数据
        </div>
        <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
          当前显示为上一次查询的缓存数据
        </div>
      </div>
    </div>
  );
}

export function UsageDataPanel(props: UsageDataPanelProps) {
  const overlayOpen = overlayOpenForCustomPending({
    customPending: props.customPending,
    tableTab: props.tableTab,
    rows: props.rows,
    summary: props.summary,
    cacheTrendRows: props.cacheTrendRows,
  });
  const activeStale = props.tableTab === "cacheTrend" ? props.cacheTrendStale : props.dataStale;

  const contentRef = useRef<HTMLDivElement | null>(null);
  const overlayRef = useRef<HTMLDivElement | null>(null);
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
