import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef } from "react";
import { cn } from "@/ui/shadcn/utils";

export const buttonVariants = cva(
  [
    "inline-flex items-center justify-center gap-2 rounded-lg border border-transparent font-medium transition-colors",
    "active:scale-[0.97]",
    "disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30",
    "focus-visible:ring-offset-2 focus-visible:ring-offset-background",
  ].join(" "),
  {
    variants: {
      variant: {
        primary:
          "border-state-selected-border bg-state-selected text-state-selected-foreground hover:bg-accent/18 dark:hover:bg-accent/24",
        secondary:
          "border-line bg-surface-panel text-foreground hover:bg-state-hover hover:border-line-strong",
        ghost: "text-foreground hover:bg-state-hover",
        warning:
          "border-amber-300/70 bg-amber-50 text-amber-800 hover:bg-amber-100 dark:border-amber-700/60 dark:bg-amber-900/30 dark:text-amber-300 dark:hover:bg-amber-900/50",
        danger:
          "border-destructive/30 bg-destructive/5 text-destructive hover:bg-destructive/10 dark:border-destructive/40 dark:bg-destructive/10 dark:hover:bg-destructive/20",
      },
      size: {
        sm: "px-2.5 py-1.5 text-xs",
        md: "px-3 py-2 text-sm",
        icon: "h-8 w-8 p-0",
      },
    },
    defaultVariants: {
      variant: "secondary",
      size: "md",
    },
  }
);

export type ButtonVariant = NonNullable<VariantProps<typeof buttonVariants>["variant"]>;
export type ButtonSize = NonNullable<VariantProps<typeof buttonVariants>["size"]>;

export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  };

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { className, variant, size, asChild = false, type = "button", ...props },
  ref
) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      ref={ref}
      type={type}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
});
