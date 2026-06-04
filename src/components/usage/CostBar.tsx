// Usage: 表格行内费用占比进度条。

import { cn } from "../../utils/cn";

export function CostBar({
  percent,
  className,
}: {
  /** 占比 0~1 */
  percent: number;
  className?: string;
}) {
  const pct = Number.isFinite(percent) ? Math.max(0, Math.min(1, percent)) : 0;
  const displayPct = (pct * 100).toFixed(1);

  return (
    <div
      className={cn("flex items-center gap-1.5", className)}
      role="progressbar"
      aria-valuenow={Number(displayPct)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={`费用占比 ${displayPct}%`}
    >
      <div className="h-1.5 flex-1 rounded-full bg-secondary">
        <div
          className="h-full rounded-full bg-orange-400 dark:bg-orange-500 transition-all duration-300"
          style={{ width: `${pct * 100}%` }}
        />
      </div>
      <span className="w-10 text-right tabular-nums text-[10px] text-muted-foreground">
        {displayPct}%
      </span>
    </div>
  );
}
