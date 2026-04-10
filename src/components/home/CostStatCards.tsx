// Usage:
// - Extracted from HomeCostPanel. Renders summary stat cards and skeleton loading states.

import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import type { SummaryCard } from "./useCostFilters";
import type { CostPeriod } from "../../services/usage/cost";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";

function StatCard({
  title,
  value,
  hint,
  className,
  "data-testid": testId,
}: {
  title: string;
  value: string;
  hint?: string;
  className?: string;
  "data-testid"?: string;
}) {
  return (
    <Card padding="md" className={cn("flex h-full flex-col", className)} data-testid={testId}>
      <div className="text-xs font-medium text-slate-500 dark:text-slate-400">{title}</div>
      <div className="mt-2 text-lg font-semibold tracking-tight text-slate-900 dark:text-slate-100 xl:text-xl">
        {value}
      </div>
      {hint ? (
        <div className="mt-auto pt-2 text-xs text-slate-500 dark:text-slate-400">{hint}</div>
      ) : null}
    </Card>
  );
}

function StatCardSkeleton({ className }: { className?: string }) {
  return (
    <Card padding="md" className={cn("h-full animate-pulse", className)}>
      <div className="h-3 w-24 rounded bg-slate-200 dark:bg-slate-700" />
      <div className="mt-3 h-8 w-28 rounded bg-slate-200 dark:bg-slate-700" />
      <div className="mt-3 h-3 w-44 rounded bg-slate-100 dark:bg-slate-600" />
    </Card>
  );
}

export function CostStatCards({
  loading,
  summaryCards,
  period,
  customApplied,
}: {
  loading: boolean;
  summaryCards: SummaryCard[];
  period: CostPeriod;
  customApplied: CustomDateRangeApplied | null;
}) {
  if (loading) {
    return (
      <div className="grid grid-cols-2 gap-3">
        {Array.from({ length: 2 }).map((_, idx) => (
          <StatCardSkeleton key={idx} />
        ))}
      </div>
    );
  }

  if (summaryCards.length === 0) {
    return (
      <Card padding="md">
        <div className="text-sm text-slate-600 dark:text-slate-400">
          {period === "custom" && !customApplied
            ? "自定义范围：请选择日期后点击「应用」。"
            : "暂无花费数据。"}
        </div>
      </Card>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3">
      {summaryCards.map((card) => (
        <StatCard
          key={card.title}
          title={card.title}
          value={card.value}
          hint={card.hint}
          data-testid={card.testId}
        />
      ))}
    </div>
  );
}
