// Usage:
// - Render in `HomeOverviewPanel` left column below work status to show active sessions list.
// - Use `HomeActiveSessionsCardContent` for inline rendering without Card wrapper.

import { useMemo } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import type { GatewayActiveSession } from "../../services/gateway/gateway";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { formatDurationMs, formatInteger, formatUsd } from "../../utils/formatters";
import { DollarSign } from "lucide-react";

export type HomeActiveSessionsCardProps = {
  activeSessions: GatewayActiveSession[];
  activeSessionsLoading: boolean;
  activeSessionsAvailable: boolean | null;
};

/** Content-only version for embedding in external Card */
export function HomeActiveSessionsCardContent({
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
}: HomeActiveSessionsCardProps) {
  const activeSessionsSorted = useMemo(() => {
    return activeSessions
      .slice()
      .sort((a, b) => b.expires_at - a.expires_at || a.session_id.localeCompare(b.session_id));
  }, [activeSessions]);

  if (activeSessionsLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
        <Spinner size="sm" />
        加载中…
      </div>
    );
  }

  if (activeSessionsAvailable === false) {
    return <div className="text-sm text-slate-600 dark:text-slate-400">数据不可用</div>;
  }

  if (activeSessions.length === 0) {
    return <EmptyState title="暂无活跃 Session。" />;
  }

  return (
    <div className="space-y-2 h-full overflow-auto pr-1 scrollbar-overlay">
      {activeSessionsSorted.map((row) => {
        const providerLabel =
          row.provider_name && row.provider_name !== "Unknown" ? row.provider_name : "未知";

        return (
          <div
            key={`${row.cli_key}:${row.session_id}`}
            className="flex-1 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2.5 shadow-sm transition-all duration-200 hover:bg-slate-50 dark:hover:bg-slate-700 hover:border-indigo-200 dark:hover:border-indigo-700 hover:shadow-md"
          >
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-300">
                  <span
                    className={cn(
                      "shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium",
                      cliBadgeTone(row.cli_key)
                    )}
                  >
                    {cliShortLabel(row.cli_key)}
                  </span>
                  <span className="font-mono text-xs text-slate-400 dark:text-slate-500">
                    {row.session_suffix}
                  </span>
                  <span className="truncate max-w-[150px]">{providerLabel}</span>
                </div>

                <div className="flex items-center gap-1 rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500 dark:text-slate-400 shadow-sm">
                  <DollarSign className="h-3 w-3 text-slate-400 dark:text-slate-500" />
                  <span className="font-mono font-medium text-slate-700 dark:text-slate-300">
                    {formatUsd(row.total_cost_usd)}
                  </span>
                </div>
              </div>

              <div className="grid grid-cols-4 gap-x-4 text-[10px] font-mono text-slate-500 dark:text-slate-400">
                <span>请求</span>
                <span>输入</span>
                <span>输出</span>
                <span>耗时</span>
                <span className="tabular-nums">{formatInteger(row.request_count)}</span>
                <span className="tabular-nums">{formatInteger(row.total_input_tokens)}</span>
                <span className="tabular-nums">{formatInteger(row.total_output_tokens)}</span>
                <span className="tabular-nums">{formatDurationMs(row.total_duration_ms)}</span>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function HomeActiveSessionsCard({
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
}: HomeActiveSessionsCardProps) {
  return (
    <Card padding="sm" className="flex flex-col h-full">
      <div className="flex items-center justify-between gap-2 shrink-0">
        <div className="text-sm font-semibold">活跃 Session</div>
        <div className="text-xs text-slate-400 dark:text-slate-500">{activeSessions.length}</div>
      </div>

      <div className="mt-3 flex-1 min-h-0">
        <HomeActiveSessionsCardContent
          activeSessions={activeSessions}
          activeSessionsLoading={activeSessionsLoading}
          activeSessionsAvailable={activeSessionsAvailable}
        />
      </div>
    </Card>
  );
}
