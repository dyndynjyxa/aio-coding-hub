// Usage: Card component for displaying a cost-limit input in ProviderEditorDialog.

import type React from "react";
import { Input } from "../../ui/Input";
import { cn } from "../../utils/cn";

export type LimitCardProps = {
  icon: React.ReactNode;
  iconBgClass: string;
  label: string;
  hint?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  disabled?: boolean;
};

export function LimitCard({
  icon,
  iconBgClass,
  label,
  hint,
  value,
  onChange,
  placeholder,
  disabled,
}: LimitCardProps) {
  return (
    <div className="group relative rounded-xl border border-border bg-white p-4 shadow-sm transition-all hover:border-border hover:shadow-md dark:border-border dark:bg-secondary dark:hover:border-border">
      <div className="flex items-start gap-3">
        <div
          className={cn(
            "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg",
            iconBgClass
          )}
        >
          {icon}
        </div>
        <div className="min-w-0 flex-1">
          <label className="text-sm font-medium text-secondary-foreground">{label}</label>
          {hint ? <p className="mt-0.5 text-xs text-muted-foreground">{hint}</p> : null}
          <div className="relative mt-2">
            <Input
              type="number"
              min="0"
              step="0.01"
              value={value}
              onChange={(e) => onChange(e.currentTarget.value)}
              placeholder={placeholder}
              disabled={disabled}
              className="pr-12"
            />
            <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs font-medium text-muted-foreground">
              USD
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
