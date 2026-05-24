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
        "h-10 w-full rounded-lg border border-input bg-card px-3 text-sm text-foreground shadow-sm outline-none transition",
        "focus:border-ring focus:ring-2 focus:ring-ring/20",
        "disabled:cursor-not-allowed disabled:opacity-50",
        mono ? "font-mono" : null,
        className
      )}
      {...props}
    />
  );
});
