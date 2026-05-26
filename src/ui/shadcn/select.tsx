import { forwardRef } from "react";
import { cn } from "@/ui/shadcn/utils";

export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement> & {
  mono?: boolean;
};

export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  { className, mono, ...props },
  ref
) {
  return (
    <select
      ref={ref}
      className={cn(
        "h-10 w-full rounded-lg border border-line bg-surface-inset px-3 text-sm text-foreground outline-none transition-colors",
        "focus:border-ring focus:bg-surface-panel focus:ring-2 focus:ring-ring/20",
        "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:opacity-60",
        mono ? "font-mono" : null,
        className
      )}
      {...props}
    />
  );
});
