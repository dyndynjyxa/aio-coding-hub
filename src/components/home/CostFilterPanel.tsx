// Usage:
// - Extracted from HomeCostPanel. Renders the filter controls card (CLI, period, provider, model).

import { cliShortLabel } from "../../constants/clis";
import { PERIOD_ITEMS } from "../../constants/periods";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { cn } from "../../utils/cn";
import { formatUsd } from "../../utils/formatters";
import { Calendar, Filter, RefreshCw, ChevronDown } from "lucide-react";
import type { CostPeriod } from "../../services/usage/cost";
import type {
  CostProviderBreakdownRowV1,
  CostModelBreakdownRowV1,
} from "../../services/usage/cost";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { CLI_ITEMS, type CliFilter } from "./useCostFilters";

export function CostFilterPanel({
  period,
  setPeriod,
  cliKey,
  setCliKey,
  providerSelectValue,
  onProviderChange,
  modelSelectValue,
  onModelChange,
  providerOptions,
  modelOptions,
  fetching,
  tauriAvailable,
  showCustomForm,
  customStartDate,
  setCustomStartDate,
  customEndDate,
  setCustomEndDate,
  customApplied,
  applyCustomRange,
  clearCustomRange,
  onRefresh,
}: {
  period: CostPeriod;
  setPeriod: (p: CostPeriod) => void;
  cliKey: CliFilter;
  setCliKey: (k: CliFilter) => void;
  providerSelectValue: string;
  onProviderChange: (value: string) => void;
  modelSelectValue: string;
  onModelChange: (value: string) => void;
  providerOptions: CostProviderBreakdownRowV1[];
  modelOptions: CostModelBreakdownRowV1[];
  fetching: boolean;
  tauriAvailable: boolean | null;
  showCustomForm: boolean;
  customStartDate: string;
  setCustomStartDate: (v: string) => void;
  customEndDate: string;
  setCustomEndDate: (v: string) => void;
  customApplied: CustomDateRangeApplied | null;
  applyCustomRange: () => void;
  clearCustomRange: () => void;
  onRefresh: () => void;
}) {
  return (
    <Card padding="md" className="lg:col-span-7" data-testid="home-cost-filter-panel">
      <div className="flex flex-col gap-4">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Filter className="h-4 w-4 text-indigo-500" />
            <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              筛选条件
            </span>
          </div>
          <button
            type="button"
            onClick={onRefresh}
            disabled={fetching}
            className={cn(
              "flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all",
              fetching
                ? "bg-slate-100 dark:bg-slate-700 text-slate-400 dark:text-slate-500 cursor-not-allowed"
                : "bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-900/50"
            )}
          >
            <RefreshCw className={cn("h-3.5 w-3.5", fetching && "animate-spin")} />
            刷新
          </button>
        </div>

        {/* Primary Filters: CLI + Period in one row */}
        <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-slate-500 dark:text-slate-400 whitespace-nowrap">
              CLI
            </span>
            <div className="flex items-center gap-1">
              {CLI_ITEMS.map((item) => (
                <button
                  key={item.key}
                  type="button"
                  onClick={() => setCliKey(item.key)}
                  disabled={fetching}
                  className={cn(
                    "rounded-md px-2.5 py-1 text-xs font-medium transition-all",
                    cliKey === item.key
                      ? "bg-indigo-500 text-white shadow-sm"
                      : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600",
                    fetching && "opacity-50 cursor-not-allowed"
                  )}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>

          <div className="h-4 w-px bg-slate-200 dark:bg-slate-700 hidden sm:block" />

          <div className="flex items-center gap-2">
            <Calendar className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
            <div className="flex items-center gap-1">
              {PERIOD_ITEMS.map((item) => (
                <button
                  key={item.key}
                  type="button"
                  onClick={() => setPeriod(item.key)}
                  disabled={fetching}
                  className={cn(
                    "rounded-md px-2.5 py-1 text-xs font-medium transition-all",
                    period === item.key
                      ? "bg-indigo-500 text-white shadow-sm"
                      : "bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600",
                    fetching && "opacity-50 cursor-not-allowed"
                  )}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
        </div>
        {/* PLACEHOLDER_FILTER_CUSTOM_AND_ADVANCED */}

        {/* Custom Date Range */}
        {showCustomForm && (
          <div
            className="rounded-lg bg-slate-50 dark:bg-slate-800 p-3"
            data-testid="home-cost-custom-range"
          >
            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
              <div className="flex items-center gap-2 flex-1">
                <label className="sr-only" htmlFor="home-cost-custom-start-date">
                  开始日期
                </label>
                <Input
                  id="home-cost-custom-start-date"
                  type="date"
                  value={customStartDate}
                  onChange={(e) => setCustomStartDate(e.currentTarget.value)}
                  className="h-8 text-xs border-slate-200 dark:border-slate-700 flex-1"
                  disabled={fetching}
                />
                <span className="text-slate-400 dark:text-slate-500 text-xs">→</span>
                <label className="sr-only" htmlFor="home-cost-custom-end-date">
                  结束日期
                </label>
                <Input
                  id="home-cost-custom-end-date"
                  type="date"
                  value={customEndDate}
                  onChange={(e) => setCustomEndDate(e.currentTarget.value)}
                  className="h-8 text-xs border-slate-200 dark:border-slate-700 flex-1"
                  disabled={fetching}
                />
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={applyCustomRange}
                  disabled={fetching}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-xs font-medium transition-all",
                    fetching
                      ? "bg-slate-200 dark:bg-slate-700 text-slate-400 dark:text-slate-500 cursor-not-allowed"
                      : "bg-indigo-500 text-white hover:bg-indigo-600"
                  )}
                >
                  应用
                </button>
                <button
                  type="button"
                  onClick={clearCustomRange}
                  disabled={fetching}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-xs font-medium transition-all",
                    fetching
                      ? "bg-slate-200 dark:bg-slate-700 text-slate-400 dark:text-slate-500 cursor-not-allowed"
                      : "bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700"
                  )}
                >
                  清空
                </button>
                {customApplied && (
                  <span className="text-[10px] text-indigo-600 dark:text-indigo-400 bg-indigo-50 dark:bg-indigo-900/30 px-2 py-0.5 rounded">
                    {customApplied.startDate} → {customApplied.endDate}
                  </span>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Advanced Filters */}
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
          <div className="flex items-center gap-2 flex-1">
            <label className="text-xs font-medium text-slate-500 dark:text-slate-400 whitespace-nowrap">
              供应商
            </label>
            <div className="relative flex-1">
              <select
                value={providerSelectValue}
                onChange={(e) => onProviderChange(e.currentTarget.value)}
                disabled={fetching || tauriAvailable === false}
                title="选择供应商"
                className={cn(
                  "w-full rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 pl-3 pr-8 py-1.5 text-xs text-slate-700 dark:text-slate-300",
                  "focus:border-indigo-300 focus:outline-none focus:ring-1 focus:ring-indigo-100",
                  "disabled:bg-slate-50 dark:disabled:bg-slate-900 disabled:text-slate-400 dark:disabled:text-slate-500 disabled:cursor-not-allowed",
                  "appearance-none cursor-pointer"
                )}
              >
                <option value="all">全部</option>
                {providerOptions.map((row) => (
                  <option key={`${row.cli_key}:${row.provider_id}`} value={String(row.provider_id)}>
                    {cliShortLabel(row.cli_key)} · {row.provider_name} ({formatUsd(row.cost_usd)})
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-400 dark:text-slate-500 pointer-events-none" />
            </div>
          </div>

          <div className="flex items-center gap-2 flex-1">
            <label className="text-xs font-medium text-slate-500 dark:text-slate-400 whitespace-nowrap">
              模型
            </label>
            <div className="relative flex-1">
              <select
                value={modelSelectValue}
                onChange={(e) => onModelChange(e.currentTarget.value)}
                disabled={fetching || tauriAvailable === false}
                title="选择模型"
                className={cn(
                  "w-full rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 pl-3 pr-8 py-1.5 text-xs text-slate-700 dark:text-slate-300",
                  "focus:border-indigo-300 focus:outline-none focus:ring-1 focus:ring-indigo-100",
                  "disabled:bg-slate-50 dark:disabled:bg-slate-900 disabled:text-slate-400 dark:disabled:text-slate-500 disabled:cursor-not-allowed",
                  "appearance-none cursor-pointer"
                )}
              >
                <option value="all">全部</option>
                {modelOptions.map((row) => (
                  <option key={row.model} value={row.model}>
                    {row.model} ({formatUsd(row.cost_usd)})
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-400 dark:text-slate-500 pointer-events-none" />
            </div>
          </div>
        </div>

        {/* Tauri Warning */}
        {tauriAvailable === false && (
          <div className="rounded-lg bg-amber-50 dark:bg-amber-900/30 border border-amber-200 dark:border-amber-700 px-3 py-2 text-xs text-amber-800 dark:text-amber-400">
            当前环境未检测到 Tauri Runtime。请通过桌面端运行后查看花费。
          </div>
        )}
      </div>
    </Card>
  );
}
