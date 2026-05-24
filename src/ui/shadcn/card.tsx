import type { HTMLAttributes } from "react";
import { cn } from "@/ui/shadcn/utils";

export type CardPadding = "none" | "sm" | "md";

export type CardProps = HTMLAttributes<HTMLDivElement> & {
  padding?: CardPadding;
};

const PADDING_CLASS: Record<CardPadding, string> = {
  none: "",
  sm: "p-3 sm:p-4",
  md: "p-4 sm:p-5 md:p-6",
};

export function Card({ padding = "md", className, ...props }: CardProps) {
  return (
    <div
      className={cn("overflow-hidden bg-card", "rounded-[10px]", PADDING_CLASS[padding], className)}
      {...props}
    />
  );
}
