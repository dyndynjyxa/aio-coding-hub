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
    <div className={cn("rounded-lg border border-border/60 dark:border-border/60", className)}>
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-xs font-medium text-muted-foreground hover:bg-secondary/50 dark:text-muted-foreground dark:hover:bg-secondary/30 transition-colors"
        onClick={() => setOpen((prev) => !prev)}
        aria-expanded={open}
      >
        {label}
        <ChevronDown
          className={cn(
            "h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform",
            open && "rotate-180"
          )}
        />
      </button>
      {open && (
        <div className="border-t border-border/60 px-3 py-2.5 dark:border-border/60">
          {children}
        </div>
      )}
    </div>
  );
}
