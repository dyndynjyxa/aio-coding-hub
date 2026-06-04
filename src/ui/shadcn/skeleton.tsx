import { cn } from "@/ui/shadcn/utils";

export type SkeletonVariant = "text" | "circular" | "rectangular";

export type SkeletonProps = {
  className?: string;
  variant?: SkeletonVariant;
};

const VARIANT_CLASS: Record<SkeletonVariant, string> = {
  text: "h-4 w-full rounded-md",
  circular: "rounded-full",
  rectangular: "rounded-lg",
};

export function Skeleton({ variant = "text", className }: SkeletonProps) {
  return (
    <div
      aria-hidden="true"
      className={cn("animate-pulse bg-muted", VARIANT_CLASS[variant], className)}
    />
  );
}
