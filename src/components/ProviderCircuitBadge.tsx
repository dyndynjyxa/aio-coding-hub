import { useMemo, useState } from "react";
import { cliBadgeTone, cliShortLabel, createCliRecord } from "../constants/clis";
import { useNowUnix } from "../hooks/useNowUnix";
import type { CliKey } from "../services/providers/providers";
import type { CircuitDisplayState } from "../query/gateway";
import { Button } from "../ui/Button";
import { Popover } from "../ui/Popover";
import { cn } from "../utils/cn";
import { formatCountdownSeconds } from "../utils/formatters";

export type OpenCircuitRow = {
  cli_key: CliKey;
  provider_id: number;
  provider_name: string;
  displayState: Exclude<CircuitDisplayState, "healthy">;
  // Unix seconds until provider becomes available again (open / cooldown 行)。
  // half_open 行无 until 语义，恒为 null。
  open_until: number | null;
};

// 主页熔断徽章 popover 与概览“熔断信息”面板共用的行状态词/配色，防止两处漂移。
export const CIRCUIT_ROW_STATUS: Record<
  OpenCircuitRow["displayState"],
  { label: string; className: string }
> = {
  open: { label: "熔断", className: "text-rose-600 dark:text-rose-400" },
  cooldown: { label: "冷却中", className: "text-muted-foreground" },
  half_open: { label: "试探恢复中", className: "text-amber-600 dark:text-amber-400" },
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
  const unavailableCount = rows.filter((row) => row.displayState !== "half_open").length;
  const halfOpenCount = count - unavailableCount;
  const [popoverState, setPopoverState] = useState({ rowCount: count, open: false });
  let popoverOpen = popoverState.open;

  if (popoverState.rowCount !== count) {
    popoverOpen = count > 0 && popoverOpen;
    setPopoverState({ rowCount: count, open: popoverOpen });
  }

  const nowUnix = useNowUnix(popoverOpen);

  const groupedByCli = useMemo(() => {
    const grouped = createCliRecord<OpenCircuitRow[]>(() => []);

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

  const visibleCliKeys = useMemo(() => {
    const keys: CliKey[] = [];
    for (const cliKey of Object.keys(groupedByCli) as CliKey[]) {
      if (groupedByCli[cliKey].length > 0) {
        keys.push(cliKey);
      }
    }
    return keys;
  }, [groupedByCli]);

  if (count === 0) return null;

  return (
    <Popover
      open={popoverOpen}
      onOpenChange={(open) => setPopoverState({ rowCount: count, open: count > 0 && open })}
      placement="bottom"
      align="end"
      trigger={
        <span
          className={cn(
            "inline-flex items-center rounded-lg px-3 py-2 text-sm font-semibold transition-colors duration-200",
            // 存在 open/cooldown 行时保持红色；仅剩半开行时整体转琥珀“试探恢复”。
            unavailableCount > 0
              ? popoverOpen
                ? "bg-rose-600 text-white shadow-sm"
                : "bg-rose-50 text-rose-700 border border-rose-200/60 hover:bg-rose-100 dark:bg-rose-900/30 dark:text-rose-400 dark:border-rose-700/60 dark:hover:bg-rose-900/50"
              : popoverOpen
                ? "bg-amber-600 text-white shadow-sm"
                : "bg-amber-50 text-amber-700 border border-amber-200/60 hover:bg-amber-100 dark:bg-amber-900/30 dark:text-amber-400 dark:border-amber-700/60 dark:hover:bg-amber-900/50"
          )}
        >
          {unavailableCount > 0
            ? `当前熔断 ${unavailableCount}${halfOpenCount > 0 ? ` · 恢复中 ${halfOpenCount}` : ""}`
            : `试探恢复 ${halfOpenCount}`}
        </span>
      }
      contentClassName="w-[480px] overflow-hidden rounded-2xl border border-border bg-white dark:bg-secondary shadow-card"
    >
      <div className="border-b border-border px-4 py-3">
        <span className="text-sm font-semibold text-foreground">
          {/* 仅半开行时不再称"熔断"，与触发器状态词保持一致。 */}
          {unavailableCount > 0 ? `熔断列表 (${count})` : `试探恢复列表 (${count})`}
        </span>
      </div>
      <div className="max-h-[400px] overflow-y-auto p-3">
        {visibleCliKeys.map((cliKey) => (
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
              <span className="text-xs text-muted-foreground">
                {groupedByCli[cliKey].length} 个供应商
              </span>
            </div>
            <div className="space-y-2">
              {groupedByCli[cliKey].map((row) => {
                const status = CIRCUIT_ROW_STATUS[row.displayState];
                // half_open 无 until 语义，不渲染倒计时。
                const remaining =
                  row.displayState !== "half_open" &&
                  row.open_until != null &&
                  Number.isFinite(row.open_until)
                    ? formatCountdownSeconds(row.open_until - nowUnix)
                    : null;
                const isResetting = resettingProviderIds.has(row.provider_id);
                return (
                  <div
                    key={`${row.cli_key}:${row.provider_id}`}
                    className="flex items-center justify-between gap-3 rounded-lg border border-border bg-secondary/50 dark:bg-secondary/50 px-3 py-2 transition-colors hover:bg-secondary dark:hover:bg-secondary"
                  >
                    <div className="min-w-0 flex-1">
                      <div
                        className="truncate text-sm font-medium text-secondary-foreground"
                        title={row.provider_name}
                      >
                        {row.provider_name || "未知"}
                      </div>
                    </div>
                    <div className="shrink-0 text-xs">
                      <span className={cn("font-medium", status.className)}>{status.label}</span>
                      {remaining != null ? (
                        <span className="ml-1 font-mono text-muted-foreground">{remaining}</span>
                      ) : null}
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
