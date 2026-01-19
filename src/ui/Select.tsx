import { forwardRef } from "react";
import { cn } from "../utils/cn";

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
        "h-10 w-full rounded-lg border border-slate-200 bg-white px-3 text-sm text-slate-900 shadow-sm outline-none transition",
        "focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20",
        "disabled:cursor-not-allowed disabled:opacity-50",
        mono ? "font-mono" : null,
        className
      )}
      {...props}
    />
  );
});
