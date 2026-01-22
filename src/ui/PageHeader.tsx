import type { ReactNode } from "react";

export type PageHeaderProps = {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
};

export function PageHeader({ title, subtitle, actions }: PageHeaderProps) {
  const hasSubtitle = Boolean(subtitle);

  return (
    <div
      className={`flex min-h-12 flex-wrap justify-between gap-4 ${hasSubtitle ? "items-start" : "items-center"}`}
    >
      <div className="flex items-center gap-3">
        <div className="h-8 w-1 shrink-0 rounded-full bg-gradient-to-b from-accent to-accent-secondary" />
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold tracking-tight text-slate-900">{title}</h1>
          {subtitle ? <p className="mt-1 text-sm text-slate-600">{subtitle}</p> : null}
        </div>
      </div>
      {actions ? (
        <div className="flex min-h-12 flex-wrap items-center gap-2">{actions}</div>
      ) : null}
    </div>
  );
}
