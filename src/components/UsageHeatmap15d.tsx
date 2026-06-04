import { useMemo, useState, type MouseEvent } from "react";
import type { UsageHourlyRow } from "../services/usage/usage";
import { cn } from "../utils/cn";
import { buildRecentDayKeys } from "../utils/dateKeys";

type TooltipState = {
  day: string;
  hour: number;
  requests_total: number;
  requests_success: number;
  requests_failed: number;
  requests_with_usage: number;
  total_tokens: number;
  left: number;
  top: number;
  placement: "above" | "below";
};

const LEVEL_CLASS: Record<number, string> = {
  0: "bg-heatmap-0",
  1: "bg-heatmap-1",
  2: "bg-heatmap-2",
  3: "bg-heatmap-3",
  4: "bg-heatmap-4",
};

function clampNumber(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function pad2(v: number) {
  const n = Math.floor(v);
  if (!Number.isFinite(n)) return "00";
  return String(n).padStart(2, "0");
}

function formatNumber(value: number) {
  if (!Number.isFinite(value)) return "0";
  try {
    return new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(value);
  } catch {
    return String(value);
  }
}

function formatTokensMillions(value: number) {
  if (!Number.isFinite(value) || value === 0) return "0";
  const millions = value / 1_000_000;
  if (millions >= 1) {
    return `${millions.toFixed(1)}M`;
  }
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}K`;
  }
  return String(Math.round(value));
}

function computeIntensityLevel(value: number, maxValue: number) {
  if (!Number.isFinite(value) || value <= 0) return 0;
  if (!Number.isFinite(maxValue) || maxValue <= 0) return 0;
  const ratio = value / maxValue;
  const raw = Math.ceil(ratio * 4);
  return clampNumber(raw, 1, 4);
}

export function UsageHeatmap15d({
  rows,
  days = 15,
  className,
  onRefresh,
  refreshing = false,
}: {
  rows: UsageHourlyRow[];
  days?: number;
  className?: string;
  onRefresh?: () => void;
  refreshing?: boolean;
}) {
  const HOURS_PER_COLUMN = 8;
  const HOURS_PER_DAY = 24;
  const CELL_GAP = "clamp(2px, 0.5vw, 6px)";

  const dayKeys = useMemo(() => buildRecentDayKeys(days), [days]);
  const columnsPerDay = Math.ceil(HOURS_PER_DAY / HOURS_PER_COLUMN);
  const columnCount = dayKeys.length * columnsPerDay;

  const rowByKey = useMemo(() => {
    const m = new Map<string, UsageHourlyRow>();
    for (const row of rows) {
      const hour = Number(row.hour);
      if (!row.day || !Number.isFinite(hour)) continue;
      m.set(`${row.day}|${hour}`, row);
    }
    return m;
  }, [rows]);

  const maxRequests = useMemo(() => {
    let max = 0;
    for (const row of rows) {
      const v = Number(row.requests_total);
      if (!Number.isFinite(v)) continue;
      max = Math.max(max, v);
    }
    return max;
  }, [rows]);

  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const hoveredKey = tooltip ? `${tooltip.day}|${tooltip.hour}` : null;

  const cells = useMemo(() => {
    const out: Array<{ day: string; hour: number }> = [];
    for (const day of dayKeys) {
      for (let hour = 0; hour < HOURS_PER_DAY; hour += 1) {
        out.push({ day, hour });
      }
    }
    return out;
  }, [dayKeys]);

  function showTooltip(e: MouseEvent<HTMLDivElement>, value: UsageHourlyRow) {
    const rect = e.currentTarget.getBoundingClientRect();

    const TOOLTIP_WIDTH = 240;
    const TOOLTIP_HEIGHT = 120;
    const VERTICAL_OFFSET = 12;
    const H_MARGIN = 20;
    const V_MARGIN = 24;

    let left = rect.left + rect.width / 2 - TOOLTIP_WIDTH / 2;
    left = clampNumber(left, H_MARGIN, window.innerWidth - TOOLTIP_WIDTH - H_MARGIN);

    const topAbove = rect.top - TOOLTIP_HEIGHT - VERTICAL_OFFSET;
    const topBelow = rect.bottom + VERTICAL_OFFSET;
    const placement: "above" | "below" = topAbove >= V_MARGIN ? "above" : "below";
    let top = placement === "above" ? topAbove : topBelow;
    top = clampNumber(top, V_MARGIN, window.innerHeight - TOOLTIP_HEIGHT - V_MARGIN);

    setTooltip({
      day: value.day,
      hour: value.hour,
      requests_total: value.requests_total,
      requests_success: value.requests_success,
      requests_failed: value.requests_failed,
      requests_with_usage: value.requests_with_usage,
      total_tokens: value.total_tokens,
      left,
      top,
      placement,
    });
  }

  function hideTooltip() {
    setTooltip(null);
  }

  return (
    <div className={cn("select-none", className)} onMouseLeave={hideTooltip}>
      <div className="w-full pb-1">
        <div
          className="grid w-full"
          style={{
            gridAutoFlow: "column",
            gap: CELL_GAP,
            gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
            gridTemplateRows: `repeat(${HOURS_PER_COLUMN}, auto)`,
          }}
        >
          {cells.map(({ day, hour }) => {
            const key = `${day}|${hour}`;
            const row =
              rowByKey.get(key) ??
              ({
                day,
                hour,
                requests_total: 0,
                requests_with_usage: 0,
                requests_success: 0,
                requests_failed: 0,
                total_tokens: 0,
              } satisfies UsageHourlyRow);
            const level = computeIntensityLevel(row.requests_total, maxRequests);
            const isHovered = hoveredKey === key;

            return (
              <div
                key={key}
                onMouseEnter={(e) => showTooltip(e, row)}
                className={cn(
                  "w-full min-w-2.5 min-h-2.5 rounded-[6px] ring-1 ring-black/5 dark:ring-white/5",
                  "hover:scale-125 hover:z-10 hover:shadow-lg hover:ring-2 hover:ring-foreground/20 dark:hover:ring-white/40 transition-all duration-200 cursor-pointer",
                  LEVEL_CLASS[level],
                  level === 4 ? "shadow-heatmap-strong" : null,
                  isHovered ? "scale-110 z-10 ring-2 ring-foreground/35 dark:ring-white/50" : null
                )}
                style={{
                  aspectRatio: "1 / 1",
                }}
              />
            );
          })}
        </div>
      </div>

      <div className="mt-3 flex items-center justify-between gap-2">
        <div className="text-xs text-muted-foreground min-w-[4rem]"></div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <span>低</span>
            {([0, 1, 2, 3, 4] as const).map((level) => (
              <div
                key={level}
                className={cn(
                  "h-2.5 w-2.5 rounded-[4px] ring-1 ring-black/5 dark:ring-white/5",
                  LEVEL_CLASS[level]
                )}
              />
            ))}
            <span>高</span>
          </div>
          {onRefresh && (
            <button
              type="button"
              onClick={onRefresh}
              disabled={refreshing}
              className={cn(
                "ml-1 p-0.5 text-muted-foreground hover:text-muted-foreground dark:text-muted-foreground dark:hover:text-muted-foreground transition-colors",
                "disabled:opacity-50 disabled:cursor-not-allowed",
                refreshing && "animate-spin"
              )}
              title="刷新"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                className="w-3.5 h-3.5"
              >
                <path
                  fillRule="evenodd"
                  d="M15.312 11.424a5.5 5.5 0 01-9.201 2.466l-.312-.311h2.433a.75.75 0 000-1.5H3.989a.75.75 0 00-.75.75v4.242a.75.75 0 001.5 0v-2.43l.31.31a7 7 0 0011.712-3.138.75.75 0 00-1.449-.39zm1.23-3.723a.75.75 0 00.219-.53V2.929a.75.75 0 00-1.5 0v2.43l-.31-.31A7 7 0 003.239 8.188a.75.75 0 101.448.389A5.5 5.5 0 0113.89 6.11l.311.31h-2.432a.75.75 0 000 1.5h4.243a.75.75 0 00.53-.219z"
                  clipRule="evenodd"
                />
              </svg>
            </button>
          )}
        </div>
        <div className="text-xs text-muted-foreground min-w-[4rem] text-right">
          {dayKeys.length > 0 ? dayKeys[dayKeys.length - 1].slice(5) : "—"}
        </div>
      </div>

      {tooltip ? (
        <div
          className="fixed z-40 pointer-events-none transition-all duration-150 ease-out"
          style={{ left: tooltip.left, top: tooltip.top, width: 240 }}
        >
          <div
            className={cn(
              "rounded-2xl border border-glass-border bg-glass backdrop-blur-md shadow-2xl",
              "px-3.5 py-2.5"
            )}
          >
            <div className="flex items-center justify-between gap-2 border-b border-border/40 pb-1.5 mb-1.5">
              <div className="flex items-center gap-1.5 text-[11px] font-semibold text-foreground">
                <div className="h-1.5 w-1.5 rounded-full bg-heatmap-4 animate-pulse" />
                {tooltip.day} {pad2(tooltip.hour)}:00
              </div>
              <div className="text-[9px] font-semibold text-muted-foreground/80 uppercase tracking-wide">
                {tooltip.placement === "above" ? "↑" : "↓"} 本地时间
              </div>
            </div>

            <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-[11px]">
              <div className="text-muted-foreground font-medium">请求</div>
              <div className="text-right font-mono font-semibold text-foreground">
                {formatNumber(tooltip.requests_total)}
              </div>

              <div className="text-muted-foreground font-medium">成功率</div>
              <div
                className={`text-right font-mono font-semibold ${
                  tooltip.requests_total > 0 &&
                  tooltip.requests_success / tooltip.requests_total < 0.95
                    ? "text-destructive"
                    : "text-emerald-500 dark:text-emerald-400"
                }`}
              >
                {tooltip.requests_total > 0
                  ? `${Math.round((tooltip.requests_success / tooltip.requests_total) * 1000) / 10}%`
                  : "—"}
              </div>

              <div className="text-muted-foreground font-medium">Token</div>
              <div className="text-right font-mono font-semibold text-heatmap-4">
                {tooltip.requests_with_usage > 0 ? formatTokensMillions(tooltip.total_tokens) : "—"}
              </div>

              <div className="text-muted-foreground font-medium">有用量请求</div>
              <div className="text-right font-mono font-semibold text-foreground">
                {formatNumber(tooltip.requests_with_usage)}
              </div>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
