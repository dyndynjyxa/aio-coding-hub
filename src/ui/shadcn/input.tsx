import { forwardRef } from "react";
import { cn } from "@/ui/shadcn/utils";

export type InputProps = React.InputHTMLAttributes<HTMLInputElement> & {
  mono?: boolean;
};

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, mono, ...props },
  ref
) {
  return (
    <input
      ref={ref}
      className={cn(
        "h-10 w-full rounded-lg border border-line bg-surface-inset px-3 text-sm text-foreground outline-none transition-colors",
        "placeholder:text-muted-foreground",
        "focus:border-ring focus:bg-surface-panel focus:ring-2 focus:ring-ring/20",
        "disabled:cursor-not-allowed disabled:bg-surface-muted disabled:opacity-60",
        mono ? "font-mono" : null,
        className
      )}
      {...props}
    />
  );
});
