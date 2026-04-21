import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../utils/cn";

export type DisclosureSectionProps = {
  label: string;
  defaultOpen?: boolean;
  className?: string;
  children: React.ReactNode;
};

export function DisclosureSection({
  label,
  defaultOpen = false,
  className,
  children,
}: DisclosureSectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className={cn("rounded-lg border border-slate-200/60 dark:border-slate-700/60", className)}>
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-xs font-medium text-slate-600 hover:bg-slate-50/50 dark:text-slate-400 dark:hover:bg-slate-700/30 transition-colors"
        onClick={() => setOpen((prev) => !prev)}
        aria-expanded={open}
      >
        {label}
        <ChevronDown
          className={cn(
            "h-3.5 w-3.5 shrink-0 text-slate-400 transition-transform",
            open && "rotate-180"
          )}
        />
      </button>
      {open && (
        <div className="border-t border-slate-200/60 px-3 py-2.5 dark:border-slate-700/60">
          {children}
        </div>
      )}
    </div>
  );
}
