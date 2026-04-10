// Usage: 用量页面筛选器组件 — CLI + 时间窗 + 自定义日期。

import { Button } from "../../ui/Button";
import { CLI_FILTER_ITEMS, type CliFilterKey } from "../../constants/clis";
import { PERIOD_ITEMS } from "../../constants/periods";
import type { UsagePeriod } from "../../services/usage/usage";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";

type UsageFiltersProps = {
  cliKey: CliFilterKey;
  onCliKeyChange: (key: CliFilterKey) => void;
  period: UsagePeriod;
  onPeriodChange: (period: UsagePeriod) => void;
  loading: boolean;
  showCustomForm: boolean;
  customStartDate: string;
  customEndDate: string;
  onCustomStartDateChange: (v: string) => void;
  onCustomEndDateChange: (v: string) => void;
  customApplied: CustomDateRangeApplied | null;
  onApplyCustomRange: () => void;
  onClearCustomRange: () => void;
};

type ButtonGroupItem<T extends string> = { key: T; label: string };

function FilterButtonGroup<T extends string>({
  ariaLabel,
  items,
  value,
  onChange,
  loading,
}: {
  ariaLabel: string;
  items: readonly ButtonGroupItem<T>[];
  value: T;
  onChange: (next: T) => void;
  loading: boolean;
}) {
  return (
    <div className="flex items-center gap-1.5" role="group" aria-label={ariaLabel}>
      {items.map((item) => (
        <Button
          key={item.key}
          size="sm"
          variant={value === item.key ? "primary" : "secondary"}
          aria-pressed={value === item.key}
          onClick={() => onChange(item.key)}
          disabled={loading}
          className="whitespace-nowrap"
        >
          {item.label}
        </Button>
      ))}
    </div>
  );
}

function CustomDateRangeForm({
  customStartDate,
  customEndDate,
  onCustomStartDateChange,
  onCustomEndDateChange,
  onApplyCustomRange,
  onClearCustomRange,
  customApplied,
  loading,
}: Pick<
  UsageFiltersProps,
  | "customStartDate"
  | "customEndDate"
  | "onCustomStartDateChange"
  | "onCustomEndDateChange"
  | "onApplyCustomRange"
  | "onClearCustomRange"
  | "customApplied"
  | "loading"
>) {
  return (
    <div className="flex w-full items-end gap-2 pt-2 sm:w-auto sm:pt-0">
      <input
        type="date"
        value={customStartDate}
        onChange={(e) => onCustomStartDateChange(e.currentTarget.value)}
        aria-label="开始日期"
        className="h-8 rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 text-xs text-slate-900 dark:text-slate-100 outline-none transition focus:border-accent focus:ring-2 focus:ring-accent/20"
      />
      <span className="text-xs text-slate-400">→</span>
      <input
        type="date"
        value={customEndDate}
        onChange={(e) => onCustomEndDateChange(e.currentTarget.value)}
        aria-label="结束日期"
        className="h-8 rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-2 text-xs text-slate-900 dark:text-slate-100 outline-none transition focus:border-accent focus:ring-2 focus:ring-accent/20"
      />
      <Button size="sm" variant="primary" onClick={onApplyCustomRange} disabled={loading}>
        应用
      </Button>
      <Button size="sm" variant="secondary" onClick={onClearCustomRange} disabled={loading}>
        清空
      </Button>
      {customApplied ? (
        <span className="text-xs font-medium text-slate-600 dark:text-slate-400">
          {customApplied.startDate} → {customApplied.endDate}
        </span>
      ) : null}
    </div>
  );
}

export function UsageFilters({
  cliKey,
  onCliKeyChange,
  period,
  onPeriodChange,
  loading,
  // 自定义日期
  showCustomForm,
  customStartDate,
  customEndDate,
  onCustomStartDateChange,
  onCustomEndDateChange,
  customApplied,
  onApplyCustomRange,
  onClearCustomRange,
}: UsageFiltersProps) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      {/* CLI 筛选 */}
      <FilterButtonGroup
        ariaLabel="CLI 筛选"
        items={CLI_FILTER_ITEMS}
        value={cliKey}
        onChange={onCliKeyChange}
        loading={loading}
      />

      {/* 分隔 */}
      <div className="hidden h-5 w-px bg-slate-200 dark:bg-slate-700 sm:block" />

      {/* 时间窗筛选 */}
      <FilterButtonGroup
        ariaLabel="时间窗筛选"
        items={PERIOD_ITEMS}
        value={period}
        onChange={onPeriodChange}
        loading={loading}
      />

      {/* 自定义日期范围 */}
      {showCustomForm ? (
        <CustomDateRangeForm
          customStartDate={customStartDate}
          customEndDate={customEndDate}
          onCustomStartDateChange={onCustomStartDateChange}
          onCustomEndDateChange={onCustomEndDateChange}
          onApplyCustomRange={onApplyCustomRange}
          onClearCustomRange={onClearCustomRange}
          customApplied={customApplied}
          loading={loading}
        />
      ) : null}
    </div>
  );
}
