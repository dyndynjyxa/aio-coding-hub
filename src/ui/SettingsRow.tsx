import type { ReactNode } from "react";
import { cn } from "../utils/cn";

export type SettingsRowProps = {
  label: ReactNode;
  subtitle?: string;
  children: ReactNode;
  className?: string;
};

export function SettingsRow({ label, subtitle, children, className }: SettingsRowProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between",
        className
      )}
    >
      <div className="min-w-0">
        <div className="text-sm text-slate-700 dark:text-slate-300">{label}</div>
        {subtitle ? (
          <div className="mt-1 text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
            {subtitle}
          </div>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}
