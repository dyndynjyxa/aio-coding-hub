// Usage: 用量页面 KPI 统计卡片组件。

import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";

/** 语义色彩映射 —— 通过 accentColor 控制卡片顶部指示条颜色 */
const ACCENT_COLORS = {
  blue: "bg-blue-500",
  orange: "bg-orange-500",
  green: "bg-emerald-500",
  purple: "bg-violet-500",
  slate: "bg-muted dark:bg-muted",
} as const;

export type StatCardAccent = keyof typeof ACCENT_COLORS;

export function StatCard({
  title,
  value,
  hint,
  accent = "slate",
  className,
}: {
  title: string;
  value: string;
  hint?: string;
  accent?: StatCardAccent;
  className?: string;
}) {
  return (
    <Card padding="md" className={cn("relative flex h-full flex-col overflow-hidden", className)}>
      {/* 顶部色条 */}
      <div className={cn("absolute inset-x-0 top-0 h-0.5", ACCENT_COLORS[accent])} />
      <div className="text-xs font-medium text-muted-foreground">{title}</div>
      <div className="mt-1.5 text-xl font-semibold tracking-tight text-foreground">{value}</div>
      {hint ? (
        <div className="mt-auto pt-1.5 text-[11px] leading-4 text-muted-foreground">{hint}</div>
      ) : null}
    </Card>
  );
}

export function StatCardSkeleton({ className }: { className?: string }) {
  return (
    <Card padding="md" className={cn("h-full animate-pulse", className)}>
      <div className="h-3 w-16 rounded bg-muted dark:bg-secondary" />
      <div className="mt-2 h-6 w-20 rounded bg-muted dark:bg-secondary" />
      <div className="mt-2 h-3 w-28 rounded bg-secondary dark:bg-secondary" />
    </Card>
  );
}
