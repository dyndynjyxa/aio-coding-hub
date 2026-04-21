import {
  ChevronDown,
  Clock,
  DollarSign,
  CalendarDays,
  CalendarRange,
  Gauge,
  RotateCcw,
} from "lucide-react";
import { Input } from "../../ui/Input";
import { LimitCard } from "./LimitCard";
import { RadioButtonGroup } from "./RadioButtonGroup";
import type { DailyResetMode } from "./providerEditorUtils";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function LimitsSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    setValue,
    saving,
    dailyResetMode,
    limit5hUsd,
    limitDailyUsd,
    limitWeeklyUsd,
    limitMonthlyUsd,
    limitTotalUsd,
  } = props.form;

  return (
    <details className="group rounded-xl border border-slate-200 bg-gradient-to-br from-slate-50/80 to-white shadow-sm open:ring-2 open:ring-accent/10 transition-all dark:border-slate-700 dark:from-slate-800/80 dark:to-slate-900">
      <summary className="flex cursor-pointer items-center justify-between px-5 py-4 select-none">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-amber-400 to-orange-500 shadow-sm">
            <DollarSign className="h-4 w-4 text-white" />
          </div>
          <div>
            <span className="text-sm font-semibold text-slate-700 group-open:text-accent dark:text-slate-300">
              限流配置
            </span>
            <p className="text-xs text-slate-400 dark:text-slate-500">
              配置不同时间窗口的消费限制以控制成本
            </p>
          </div>
        </div>
        <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-6 border-t border-slate-100 px-5 py-5 dark:border-slate-700">
        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
            时间维度限制
          </h4>
          <div className="grid gap-4 sm:grid-cols-2">
            <LimitCard
              icon={<Clock className="h-5 w-5 text-blue-600" />}
              iconBgClass="bg-blue-50 dark:bg-blue-900/30"
              label="5 小时消费上限"
              hint="留空表示不限制"
              value={limit5hUsd}
              onChange={(value) => setValue("limit_5h_usd", value, { shouldDirty: true })}
              placeholder="例如: 10"
              disabled={saving}
            />
            <LimitCard
              icon={<DollarSign className="h-5 w-5 text-emerald-600" />}
              iconBgClass="bg-emerald-50 dark:bg-emerald-900/30"
              label="每日消费上限"
              hint="留空表示不限制"
              value={limitDailyUsd}
              onChange={(value) => setValue("limit_daily_usd", value, { shouldDirty: true })}
              placeholder="例如: 100"
              disabled={saving}
            />
            <LimitCard
              icon={<CalendarDays className="h-5 w-5 text-violet-600" />}
              iconBgClass="bg-violet-50 dark:bg-violet-900/30"
              label="周消费上限"
              hint="自然周：周一 00:00:00"
              value={limitWeeklyUsd}
              onChange={(value) => setValue("limit_weekly_usd", value, { shouldDirty: true })}
              placeholder="例如: 500"
              disabled={saving}
            />
            <LimitCard
              icon={<CalendarRange className="h-5 w-5 text-orange-600" />}
              iconBgClass="bg-orange-50 dark:bg-orange-900/30"
              label="月消费上限"
              hint="自然月：每月 1 号 00:00:00"
              value={limitMonthlyUsd}
              onChange={(value) => setValue("limit_monthly_usd", value, { shouldDirty: true })}
              placeholder="例如: 2000"
              disabled={saving}
            />
          </div>
        </div>

        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
            每日重置设置
          </h4>
          <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-700 dark:bg-slate-800">
            <div className="flex items-start gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-sky-50 dark:bg-sky-900/30">
                <RotateCcw className="h-5 w-5 text-sky-600" />
              </div>
              <div className="min-w-0 flex-1 space-y-4">
                <div className="grid gap-4 sm:grid-cols-2">
                  <div>
                    <label className="text-sm font-medium text-slate-700 dark:text-slate-300">
                      每日重置模式
                    </label>
                    <p className="mb-2 text-xs text-slate-400 dark:text-slate-500">
                      rolling 为过去 24 小时窗口
                    </p>
                    <RadioButtonGroup<DailyResetMode>
                      items={[
                        { value: "fixed", label: "固定时间" },
                        { value: "rolling", label: "滚动窗口 (24h)" },
                      ]}
                      ariaLabel="每日重置模式"
                      value={dailyResetMode}
                      onChange={(value) =>
                        setValue("daily_reset_mode", value, { shouldDirty: true })
                      }
                      disabled={saving}
                    />
                  </div>
                  <div>
                    <label className="text-sm font-medium text-slate-700 dark:text-slate-300">
                      每日重置时间
                    </label>
                    <p className="mb-2 text-xs text-slate-400 dark:text-slate-500">
                      {dailyResetMode === "fixed"
                        ? "默认 00:00:00（本机时区）"
                        : "rolling 模式下忽略"}
                    </p>
                    <Input
                      type="time"
                      step="1"
                      disabled={saving || dailyResetMode !== "fixed"}
                      {...register("daily_reset_time")}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
            其他限制
          </h4>
          <div className="grid gap-4 sm:grid-cols-2">
            <LimitCard
              icon={<Gauge className="h-5 w-5 text-rose-600" />}
              iconBgClass="bg-rose-50 dark:bg-rose-900/30"
              label="总消费上限"
              hint="达到后需手动调整/清除"
              value={limitTotalUsd}
              onChange={(value) => setValue("limit_total_usd", value, { shouldDirty: true })}
              placeholder="例如: 1000"
              disabled={saving}
            />
          </div>
        </div>
      </div>
    </details>
  );
}
