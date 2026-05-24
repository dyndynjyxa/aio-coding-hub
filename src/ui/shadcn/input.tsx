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
        "h-10 w-full rounded-lg border border-input bg-card px-3 text-sm text-foreground shadow-sm outline-none transition",
        "placeholder:text-muted-foreground",
        "focus:border-ring focus:ring-2 focus:ring-ring/20",
        "disabled:cursor-not-allowed disabled:opacity-50",
        mono ? "font-mono" : null,
        className
      )}
      {...props}
    />
  );
});
