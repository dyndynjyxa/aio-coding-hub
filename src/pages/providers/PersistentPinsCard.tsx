// Usage:
// - Rendered in `ProvidersView` (供应商页路由策略区) to manage persistent (disk-backed)
//   session→sort_mode pins independent of whether the session is currently active.
// - Persistent pins live in SQLite and survive restart / TTL / resume, so they need a
//   management surface that does NOT depend on the active-session list.
// - Session ids are resolved to readable `目录名 / 会话名` via cli_sessions metadata
//   (the same metadata source), so rows are identifiable instead of showing raw UUIDs.

import { useMemo } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import {
  useGatewayPersistentPinsListQuery,
  useGatewaySessionPersistSortModeMutation,
  useGatewaySessionUnpersistSortModeMutation,
} from "../../query/gateway";
import { useCliSessionsMetadataLookupByIdsQuery } from "../../query/cliSessions";
import { useSortModesListQuery } from "../../query/sortModes";
import { validateGatewayCliKey } from "../../services/gateway/gateway";
import type { CliKey } from "../../services/providers/providers";
import type { CliSessionsSource } from "../../services/cli/cliSessions";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Select } from "../../ui/Select";
import { Spinner } from "../../ui/Spinner";
import { formatRelativeTimeFromUnixSeconds } from "../../utils/formatters";
import { Lock } from "lucide-react";

const DEFAULT_MODE_VALUE = "default";

function isSupportedCliKey(cliKey: string): cliKey is CliKey {
  try {
    validateGatewayCliKey(cliKey);
    return true;
  } catch {
    return false;
  }
}

/** Map a `cli_key` to the `cli_sessions` source, if supported (gemini → null). */
function cliKeyToSessionsSource(cliKey: string): CliSessionsSource | null {
  if (cliKey === "claude") return "claude";
  if (cliKey === "codex") return "codex";
  return null;
}

function cwdBasename(cwd: string | null | undefined): string | null {
  if (!cwd) return null;
  const trimmed = cwd.replace(/\/+$/, "");
  const seg = trimmed.split(/[/\\]/).pop();
  return seg && seg.length > 0 ? seg : null;
}

export function PersistentPinsCard() {
  const pinsQuery = useGatewayPersistentPinsListQuery();
  const sortModesQuery = useSortModesListQuery();
  const persistMutation = useGatewaySessionPersistSortModeMutation();
  const unpersistMutation = useGatewaySessionUnpersistSortModeMutation();

  const modes = useMemo(() => sortModesQuery.data ?? [], [sortModesQuery.data]);
  const modeNameById = useMemo(() => {
    const map = new Map<number, string>();
    for (const mode of modes) map.set(mode.id, mode.name);
    return map;
  }, [modes]);

  const pins = useMemo(() => pinsQuery.data ?? [], [pinsQuery.data]);

  // Batch-resolve readable names (目录 / 会话) for all pinned sessions.
  const metadataItems = useMemo(() => {
    const seen = new Set<string>();
    const out: { source: CliSessionsSource; session_id: string }[] = [];
    for (const pin of pins) {
      const source = cliKeyToSessionsSource(pin.cli_key);
      if (!source) continue;
      const key = `${source}:${pin.session_id}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ source, session_id: pin.session_id });
    }
    return out;
  }, [pins]);
  const metadataQuery = useCliSessionsMetadataLookupByIdsQuery(metadataItems);
  const metadataMap = metadataQuery.data;

  const busy = persistMutation.isPending || unpersistMutation.isPending;

  return (
    <Card padding="sm" className="flex flex-col">
      <div className="flex items-center justify-between gap-2 shrink-0">
        <div className="flex items-center gap-1.5 text-sm font-semibold">
          <Lock className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
          持久指派
        </div>
        <div className="text-xs text-muted-foreground">{pins.length}</div>
      </div>
      <p className="mt-1 text-[11px] text-muted-foreground">
        落盘的会话路由策略，跨重启 / 会话 resume 仍生效（不依赖会话是否活跃）。临时档优先于持久档。
      </p>

      <div className="mt-3">
        {pinsQuery.isLoading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Spinner size="sm" />
            加载持久指派…
          </div>
        ) : pins.length === 0 ? (
          <EmptyState title="暂无持久指派。" />
        ) : (
          <div className="space-y-2">
            {pins.map((pin) => {
              const supported = isSupportedCliKey(pin.cli_key);
              const selectedValue =
                pin.sort_mode_id == null ? DEFAULT_MODE_VALUE : String(pin.sort_mode_id);
              const modeLabel =
                pin.sort_mode_id == null
                  ? "Default"
                  : (modeNameById.get(pin.sort_mode_id) ?? `#${pin.sort_mode_id}`);

              const source = cliKeyToSessionsSource(pin.cli_key);
              const meta = source ? metadataMap?.get(`${source}:${pin.session_id}`) : undefined;
              const folder = cwdBasename(meta?.cwd);
              const title = meta?.title || null;

              return (
                <div
                  key={`${pin.cli_key}:${pin.session_id}`}
                  className="flex flex-col gap-2 rounded-lg border border-border bg-white px-3 py-2 shadow-sm dark:bg-secondary"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium ${cliBadgeTone(
                        pin.cli_key
                      )}`}
                    >
                      {cliShortLabel(pin.cli_key)}
                    </span>
                    <span
                      className="min-w-0 flex-1 truncate text-xs font-medium text-secondary-foreground"
                      title={meta?.cwd ?? pin.session_id}
                    >
                      {folder && title
                        ? `${folder} / ${title}`
                        : folder
                          ? folder
                          : title
                            ? title
                            : pin.session_id.slice(-8)}
                    </span>
                    <span
                      className="text-xs text-emerald-600 dark:text-emerald-400"
                      title={`持久策略：${modeLabel}`}
                    >
                      {modeLabel}
                    </span>
                    <span className="text-[10px] text-muted-foreground">
                      {formatRelativeTimeFromUnixSeconds(pin.updated_at)}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <Select
                      aria-label="修改持久策略"
                      className="h-7 w-full max-w-[150px] px-2 text-xs"
                      disabled={!supported || busy}
                      value={selectedValue}
                      onChange={(event) => {
                        const raw = event.target.value;
                        if (raw === selectedValue) return;
                        const sortModeId = raw === DEFAULT_MODE_VALUE ? null : Number(raw);
                        if (
                          sortModeId != null &&
                          (!Number.isSafeInteger(sortModeId) || sortModeId <= 0)
                        ) {
                          return;
                        }
                        persistMutation.mutate({
                          cliKey: pin.cli_key as CliKey,
                          sessionId: pin.session_id,
                          sortModeId,
                        });
                      }}
                    >
                      <option value={DEFAULT_MODE_VALUE}>Default</option>
                      {modes.map((mode) => (
                        <option key={mode.id} value={String(mode.id)}>
                          {mode.name}
                        </option>
                      ))}
                    </Select>
                    <Button
                      variant="secondary"
                      size="sm"
                      className="ml-auto h-7 px-2 text-xs"
                      disabled={!supported || busy}
                      onClick={() =>
                        unpersistMutation.mutate({
                          cliKey: pin.cli_key as CliKey,
                          sessionId: pin.session_id,
                        })
                      }
                    >
                      解除持久 pin
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </Card>
  );
}
