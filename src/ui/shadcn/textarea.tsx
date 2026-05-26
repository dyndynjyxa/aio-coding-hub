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
        "w-full resize-y rounded-lg border border-line bg-surface-inset px-3 py-2 text-sm text-foreground outline-none transition-colors",
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
