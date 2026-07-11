// Usage:
// - Render in `HomeOverviewPanel` left column below work status to show active sessions list.
// - Use `HomeActiveSessionsCardContent` for inline rendering without Card wrapper.

import { useMemo } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import type { GatewayActiveSession } from "../../services/gateway/gateway";
import type { CliSessionsSource } from "../../services/cli/cliSessions";
import { useCliSessionsMetadataLookupByIdsQuery } from "../../query/cliSessions";
import { useSortModesListQuery } from "../../query/sortModes";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { formatDurationMs, formatInteger, formatUsd } from "../../utils/formatters";
import { DollarSign } from "lucide-react";
import { PinTierButton, SortModeSelect, type PinTier } from "../gateway/pinControls";

export type HomeActiveSessionsCardProps = {
  activeSessions: GatewayActiveSession[];
  activeSessionsLoading: boolean;
  activeSessionsAvailable: boolean | null;
};

/** Map a `cli_key` to the `cli_sessions` source, if supported (gemini → null). */
function cliKeyToSessionsSource(cliKey: string): CliSessionsSource | null {
  if (cliKey === "claude") return "claude";
  if (cliKey === "codex") return "codex";
  return null;
}

/** Derive the current pin tier + currently-pinned sort_mode from the active-session row. */
function derivePinState(row: GatewayActiveSession): {
  tier: PinTier;
  pinnedModeId: number | null | undefined;
} {
  if (row.persistent_pinned) {
    return { tier: "persistent", pinnedModeId: row.persistent_pinned_sort_mode_id };
  }
  if (row.sort_mode_pinned) {
    return { tier: "ephemeral", pinnedModeId: row.pinned_sort_mode_id };
  }
  return { tier: "none", pinnedModeId: undefined };
}

/** Content-only version for embedding in external Card */
export function HomeActiveSessionsCardContent({
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
}: HomeActiveSessionsCardProps) {
  const sortModesQuery = useSortModesListQuery();
  const modes = useMemo(() => sortModesQuery.data ?? [], [sortModesQuery.data]);

  // Batch-resolve session metadata (cwd / title) for all active sessions.
  const metadataItems = useMemo(() => {
    const seen = new Set<string>();
    const out: { source: CliSessionsSource; session_id: string }[] = [];
    for (const row of activeSessions) {
      const source = cliKeyToSessionsSource(row.cli_key);
      if (!source) continue;
      const key = `${source}:${row.session_id}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ source, session_id: row.session_id });
    }
    return out;
  }, [activeSessions]);
  const metadataQuery = useCliSessionsMetadataLookupByIdsQuery(metadataItems);
  const metadataMap = metadataQuery.data;

  const activeSessionsSorted = useMemo(() => {
    return activeSessions
      .slice()
      .sort((a, b) => b.expires_at - a.expires_at || a.session_id.localeCompare(b.session_id));
  }, [activeSessions]);

  if (activeSessionsLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Spinner size="sm" />
        加载中…
      </div>
    );
  }

  if (activeSessionsAvailable === false) {
    return <div className="text-sm text-muted-foreground">数据不可用</div>;
  }

  if (activeSessions.length === 0) {
    return <EmptyState title="暂无活跃 Session。" />;
  }

  return (
    <div className="space-y-2 h-full overflow-auto pr-1 scrollbar-overlay">
      {activeSessionsSorted.map((row) => {
        const providerLabel =
          row.provider_name && row.provider_name !== "Unknown" ? row.provider_name : "未知";
        const { tier, pinnedModeId } = derivePinState(row);
        const anyPinned = tier !== "none";

        const source = cliKeyToSessionsSource(row.cli_key);
        const meta = source ? metadataMap?.get(`${source}:${row.session_id}`) : undefined;
        const cwdBasename = meta?.cwd
          ? meta.cwd.replace(/\/+$/, "").split(/[/\\]/).pop() || null
          : null;
        const title = meta?.title || null;
        const showMetaLine = Boolean(cwdBasename || title);

        return (
          <div
            key={`${row.cli_key}:${row.session_id}`}
            className={cn(
              "flex-1 rounded-lg border bg-white dark:bg-secondary px-3 py-2.5 shadow-sm transition-all duration-200 hover:bg-secondary dark:hover:bg-secondary hover:shadow-md",
              anyPinned
                ? "border-indigo-300 dark:border-indigo-600 ring-1 ring-indigo-200 dark:ring-indigo-800"
                : "border-border hover:border-indigo-200 dark:hover:border-indigo-700"
            )}
          >
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs text-secondary-foreground">
                  <span
                    className={cn(
                      "shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium",
                      cliBadgeTone(row.cli_key)
                    )}
                  >
                    {cliShortLabel(row.cli_key)}
                  </span>
                  <span className="font-mono text-xs text-muted-foreground">
                    {row.session_suffix}
                  </span>
                  <span className="truncate max-w-[110px]" title={`当前命中：${providerLabel}`}>
                    {providerLabel}
                  </span>
                </div>

                <div className="flex items-center gap-1 rounded-md border border-border bg-white dark:bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground shadow-sm">
                  <DollarSign className="h-3 w-3 text-muted-foreground" />
                  <span className="font-mono font-medium text-secondary-foreground">
                    {formatUsd(row.total_cost_usd)}
                  </span>
                </div>
              </div>

              {showMetaLine ? (
                <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[10px] text-muted-foreground">
                  {cwdBasename ? (
                    <span className="inline-flex items-center gap-1">
                      <span className="text-muted-foreground/70">目录</span>
                      <span
                        className="font-medium text-secondary-foreground"
                        title={meta?.cwd ?? undefined}
                      >
                        {cwdBasename}
                      </span>
                    </span>
                  ) : null}
                  {title ? (
                    <span className="inline-flex min-w-0 items-center gap-1">
                      <span className="text-muted-foreground/70">会话</span>
                      <span
                        className="truncate font-medium text-secondary-foreground max-w-[180px]"
                        title={title}
                      >
                        {title}
                      </span>
                    </span>
                  ) : null}
                </div>
              ) : null}

              <div className="flex flex-wrap items-center gap-1.5">
                <span className="shrink-0 text-[10px] text-muted-foreground">路由</span>
                <SortModeSelect
                  cliKey={row.cli_key}
                  sessionId={row.session_id}
                  tier={tier}
                  selectedModeId={pinnedModeId}
                  modes={modes}
                  autoLabel="follow"
                  className="w-full max-w-[150px]"
                />
                <PinTierButton
                  cliKey={row.cli_key}
                  sessionId={row.session_id}
                  tier={tier}
                  selectedModeId={pinnedModeId}
                />
              </div>

              <div className="grid grid-cols-4 gap-x-4 text-[10px] font-mono text-muted-foreground">
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
        <div className="text-xs text-muted-foreground">{activeSessions.length}</div>
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
