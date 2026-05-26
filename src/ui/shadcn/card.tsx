import type { HTMLAttributes } from "react";
import { cn } from "@/ui/shadcn/utils";

export type CardPadding = "none" | "sm" | "md";
export type CardVariant = "panel" | "raised" | "inset";

export type CardProps = HTMLAttributes<HTMLDivElement> & {
  padding?: CardPadding;
  variant?: CardVariant;
};

const PADDING_CLASS: Record<CardPadding, string> = {
  none: "",
  sm: "p-3 sm:p-4",
  md: "p-4 sm:p-5 md:p-6",
};

const VARIANT_CLASS: Record<CardVariant, string> = {
  panel: "border border-line-subtle bg-surface-panel",
  raised: "border border-line bg-surface-raised shadow-[0_1px_2px_rgba(15,23,42,0.04)]",
  inset: "border border-line-subtle bg-surface-inset",
};

export function Card({ padding = "md", variant = "panel", className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-2xl",
        VARIANT_CLASS[variant],
        PADDING_CLASS[padding],
        className
      )}
      {...props}
    />
  );
}
