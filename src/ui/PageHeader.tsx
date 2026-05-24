import type { ReactNode } from "react";
import { cn } from "../utils/cn";

export type PageHeaderProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
};

export function PageHeader({ title, subtitle, actions }: PageHeaderProps) {
  const hasSubtitle = Boolean(subtitle);

  return (
    <div
      className={cn(
        "flex min-h-10 flex-wrap justify-between gap-3 sm:min-h-12 sm:gap-4",
        hasSubtitle ? "items-start" : "items-center"
      )}
    >
      <div className="flex items-center gap-2 sm:gap-3">
        {/* Accent bar - smaller on mobile */}
        <div className="h-6 w-1 shrink-0 rounded-full bg-gradient-to-b from-accent to-accent-secondary sm:h-8" />
        <div className="min-w-0">
          {/* Responsive title size */}
          <h1 className="text-xl font-semibold tracking-tight text-foreground sm:text-2xl">
            {title}
          </h1>
          {subtitle ? (
            <p className="mt-0.5 text-xs text-muted-foreground sm:mt-1 sm:text-sm">
              {subtitle}
            </p>
          ) : null}
        </div>
      </div>
      {actions ? (
        <div className="flex min-h-10 flex-wrap items-center gap-2 sm:min-h-12">{actions}</div>
      ) : null}
    </div>
  );
}
