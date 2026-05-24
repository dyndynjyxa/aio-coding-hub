import { forwardRef } from "react";
import { cn } from "@/ui/shadcn/utils";

export type TextareaProps = React.TextareaHTMLAttributes<HTMLTextAreaElement> & {
  mono?: boolean;
};

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(function Textarea(
  { className, mono, ...props },
  ref
) {
  return (
    <textarea
      ref={ref}
      className={cn(
        "w-full resize-y rounded-lg border border-input bg-card px-3 py-2 text-sm text-foreground shadow-sm outline-none transition",
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
