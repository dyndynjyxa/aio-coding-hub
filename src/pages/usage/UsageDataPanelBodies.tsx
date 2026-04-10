import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import type {
  UsageLeaderboardRow,
  UsagePeriod,
  UsageProviderCacheRateTrendRowV1,
  UsageSummary,
} from "../../services/usage/usage";
import { UsageProviderCacheRateTrendChart } from "../../components/UsageProviderCacheRateTrendChart";
import { UsageLeaderboardTable } from "../../components/usage/UsageLeaderboardTable";
import { UsageTableSkeleton } from "../../components/usage/UsageTableSkeleton";

export function CacheTrendBody({
  cacheTrendLoading,
  cacheTrendRows,
  errorText,
  customPending,
  period,
  customApplied,
}: {
  cacheTrendLoading: boolean;
  cacheTrendRows: UsageProviderCacheRateTrendRowV1[];
  errorText: string | null;
  customPending: boolean;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
}) {
  if (cacheTrendLoading && cacheTrendRows.length === 0) {
    return <div className="h-80 animate-pulse rounded-lg bg-slate-100 dark:bg-slate-800" />;
  }

  if (cacheTrendRows.length === 0) {
    return (
      <div className="text-sm text-slate-600 dark:text-slate-400">
        {errorText
          ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
          : customPending
            ? '自定义范围：请选择日期后点击"应用"。'
            : "暂无可展示的缓存命中率数据。"}
      </div>
    );
  }

  return (
    <>
      <div className="h-80">
        <UsageProviderCacheRateTrendChart
          rows={cacheTrendRows}
          period={period}
          customApplied={customApplied}
          className="h-full"
        />
      </div>
      <div className="mt-3 text-xs text-slate-500 dark:text-slate-400">
        命中率=读取 /（有效输入 + 创建 + 读取）。有效输入：Codex/Gemini 做 input-cache_read
        纠偏；Claude 原样。预警阈值：60%（低于阈值的时间段会高亮背景）。
      </div>
    </>
  );
}

export function UsageTableBody({
  dataLoading,
  rows,
  summary,
  totalCostUsd,
  errorText,
  customPending,
}: {
  dataLoading: boolean;
  rows: UsageLeaderboardRow[];
  summary: UsageSummary | null;
  totalCostUsd: number;
  errorText: string | null;
  customPending: boolean;
}) {
  if (dataLoading && rows.length === 0) return <UsageTableSkeleton />;

  if (rows.length === 0 && !summary) {
    return (
      <div className="px-6 pb-5 text-sm text-slate-600 dark:text-slate-400">
        {errorText
          ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
          : customPending
            ? '自定义范围：请选择日期后点击"应用"。'
            : "暂无用量数据。请先通过网关发起请求。"}
      </div>
    );
  }

  return (
    <UsageLeaderboardTable
      rows={rows}
      summary={summary}
      totalCostUsd={totalCostUsd}
      errorText={errorText}
    />
  );
}
