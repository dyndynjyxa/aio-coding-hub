import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useState } from "react";
import type {
  UsageLeaderboardRow,
  UsagePeriod,
  UsageProviderCacheRateTrendRowV1,
  UsageProviderMetricsTrendRowV1,
  UsageSummary,
} from "../../services/usage/usage";
import { UsageProviderCacheRateTrendChart } from "../../components/UsageProviderCacheRateTrendChart";
import {
  UsageProviderMetricsTrendChart,
  type UsageTrendMetric,
} from "../../components/UsageProviderMetricsTrendChart";
import { UsageLeaderboardTable } from "../../components/usage/UsageLeaderboardTable";
import { UsageTableSkeleton } from "../../components/usage/UsageTableSkeleton";
import { TabList } from "../../ui/TabList";
import { USAGE_METRICS_TREND_ITEMS } from "./constants";

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
    return <div className="h-80 animate-pulse rounded-lg bg-secondary dark:bg-secondary" />;
  }

  if (cacheTrendRows.length === 0) {
    return (
      <div className="text-sm text-muted-foreground">
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
      <div className="mt-3 text-xs text-muted-foreground">
        命中率=读取 /（有效输入 + 创建 + 读取）。有效输入：Codex/Gemini 做 input-cache_read
        纠偏；Claude 原样。预警阈值：60%（低于阈值的时间段会高亮背景）。
      </div>
    </>
  );
}

const METRIC_HINT: Record<UsageTrendMetric, string> = {
  duration: "平均耗时=成功请求总耗时 / 成功请求数（毫秒）。按 provider 分线。",
  ttfb: "平均首字=首字节时间之和 / 有效样本数。仅统计首字 < 总耗时的成功请求（排除异常/非流式）。",
  rate: "平均速率=输出 token / 生成时间（tokens/s）。生成时间=总耗时 − 首字，仅统计首字有效的请求。",
};

export function MetricsTrendBody({
  metricsTrendLoading,
  metricsTrendRows,
  errorText,
  customPending,
  period,
  customApplied,
}: {
  metricsTrendLoading: boolean;
  metricsTrendRows: UsageProviderMetricsTrendRowV1[];
  errorText: string | null;
  customPending: boolean;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
}) {
  const [metric, setMetric] = useState<UsageTrendMetric>("duration");

  const toggle = (
    <TabList
      ariaLabel="指标切换"
      items={USAGE_METRICS_TREND_ITEMS}
      value={metric}
      onChange={setMetric}
      size="sm"
      className="shrink-0"
    />
  );

  if (metricsTrendLoading && metricsTrendRows.length === 0) {
    return (
      <>
        <div className="mb-3">{toggle}</div>
        <div className="h-80 animate-pulse rounded-lg bg-secondary dark:bg-secondary" />
      </>
    );
  }

  if (metricsTrendRows.length === 0) {
    return (
      <>
        <div className="mb-3">{toggle}</div>
        <div className="text-sm text-muted-foreground">
          {errorText
            ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
            : customPending
              ? '自定义范围：请选择日期后点击"应用"。'
              : "暂无可展示的指标数据。"}
        </div>
      </>
    );
  }

  return (
    <>
      <div className="mb-3">{toggle}</div>
      <div className="h-80">
        <UsageProviderMetricsTrendChart
          rows={metricsTrendRows}
          period={period}
          metric={metric}
          customApplied={customApplied}
          className="h-full"
        />
      </div>
      <div className="mt-3 text-xs text-muted-foreground">{METRIC_HINT[metric]}</div>
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
      <div className="px-6 pb-5 text-sm text-muted-foreground">
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
