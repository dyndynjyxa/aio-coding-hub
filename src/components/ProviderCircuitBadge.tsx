import { useEffect, useMemo, useState } from "react";
import { cliBadgeTone, cliShortLabel } from "../constants/clis";
import { useNowUnix } from "../hooks/useNowUnix";
import type { CliKey } from "../services/providers/providers";
import { Button } from "../ui/Button";
import { Popover } from "../ui/Popover";
import { cn } from "../utils/cn";
import { formatCountdownSeconds } from "../utils/formatters";

export type OpenCircuitRow = {
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  // Unix seconds until provider becomes available again.
  // Note: This value may represent either "OPEN" (circuit breaker open) or a short cooldown window.
  open_until: number | null;
};

export type ProviderCircuitBadgeProps = {
  rows: OpenCircuitRow[];
  onResetProvider: (providerId: number) => void;
  resettingProviderIds: Set<number>;
};

export function ProviderCircuitBadge({
  rows,
  onResetProvider,
  resettingProviderIds,
}: ProviderCircuitBadgeProps) {
  const count = rows.length;
  const [popoverOpen, setPopoverOpen] = useState(false);
  const nowUnix = useNowUnix(count > 0 && popoverOpen);

  useEffect(() => {
    if (count === 0 && popoverOpen) {
      setPopoverOpen(false);
    }
  }, [count, popoverOpen]);

  const groupedByCli = useMemo(() => {
    const grouped: Record<CliKey, OpenCircuitRow[]> = {
      claude: [],
      codex: [],
      gemini: [],
    };

    for (const row of rows) {
      if (grouped[row.cli_key]) {
        grouped[row.cli_key].push(row);
      }
    }

    for (const key of Object.keys(grouped) as CliKey[]) {
      grouped[key].sort((a, b) => {
        const aUntil = a.open_until ?? Number.POSITIVE_INFINITY;
        const bUntil = b.open_until ?? Number.POSITIVE_INFINITY;
        return bUntil - aUntil;
      });
    }

    return grouped;
  }, [rows]);

  if (count === 0) return null;

  return (
    <Popover
      open={popoverOpen}
      onOpenChange={setPopoverOpen}
      placement="bottom"
      align="end"
      trigger={
        <span
          className={cn(
            "inline-flex items-center rounded-lg px-3 py-2 text-sm font-semibold transition-colors duration-200",
            popoverOpen
              ? "bg-rose-600 text-white shadow-sm"
              : "bg-rose-50 text-rose-700 border border-rose-200/60 hover:bg-rose-100 dark:bg-rose-900/30 dark:text-rose-400 dark:border-rose-700/60 dark:hover:bg-rose-900/50"
          )}
        >
          当前熔断 {count}
        </span>
      }
      contentClassName="w-[480px] overflow-hidden rounded-2xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-card"
    >
      <div className="border-b border-slate-200 dark:border-slate-700 px-4 py-3">
        <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
          熔断列表 ({count})
        </span>
      </div>
      <div className="max-h-[400px] overflow-y-auto p-3">
        {(Object.keys(groupedByCli) as CliKey[])
          .filter((cliKey) => groupedByCli[cliKey].length > 0)
          .map((cliKey) => (
            <div key={cliKey} className="mb-3 last:mb-0">
              <div className="mb-2 flex items-center gap-2">
                <span
                  className={cn(
                    "rounded px-1.5 py-0.5 text-xs font-bold uppercase tracking-wider",
                    cliBadgeTone(cliKey)
                  )}
                >
                  {cliShortLabel(cliKey)}
                </span>
                <span className="text-xs text-slate-500 dark:text-slate-400">
                  {groupedByCli[cliKey].length} 个熔断
                </span>
              </div>
              <div className="space-y-2">
                {groupedByCli[cliKey].map((row) => {
                  const remaining =
                    row.open_until != null && Number.isFinite(row.open_until)
                      ? formatCountdownSeconds(row.open_until - nowUnix)
                      : "—";
                  const isResetting = resettingProviderIds.has(row.provider_id);
                  return (
                    <div
                      key={`${row.cli_key}:${row.provider_id}`}
                      className="flex items-center justify-between gap-3 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-700/50 px-3 py-2 transition-colors hover:bg-slate-100 dark:hover:bg-slate-700"
                    >
                      <div className="min-w-0 flex-1">
                        <div
                          className="truncate text-sm font-medium text-slate-700 dark:text-slate-300"
                          title={row.provider_name}
                        >
                          {row.provider_name || "未知"}
                        </div>
                      </div>
                      <div className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400">
                        {remaining}
                      </div>
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          onResetProvider(row.provider_id);
                        }}
                        disabled={isResetting}
                      >
                        {isResetting ? "解除中..." : "解除熔断"}
                      </Button>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
      </div>
    </Popover>
  );
}
