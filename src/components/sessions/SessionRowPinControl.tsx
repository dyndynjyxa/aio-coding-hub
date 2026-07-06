// Usage:
// - Embedded in `SessionsProjectPage` session rows to expose the same pin
//   controls as the active-session card. Visual + behavioural language is
//   shared via `components/gateway/pinControls` so the two surfaces stay
//   in lockstep: same icons, same colors, same tooltips, same mutation wiring.
// - Resolves a session's pin state by joining the active-session list (ephemeral)
//   and the persistent-pin list (disk-backed). Sessions shown here may not be
//   currently active — that's fine, the pin button still works (it just won't
//   be reflected in routing until the session resumes).

import { useMemo } from "react";
import {
  useGatewaySessionsListQuery,
  useGatewayPersistentPinsListQuery,
} from "../../query/gateway";
import { useSortModesListQuery } from "../../query/sortModes";
import type { SortModeSummary } from "../../services/providers/sortModes";
import { cn } from "../../utils/cn";
import { PinTierButton, SortModeSelect, type PinTier } from "../gateway/pinControls";

type ResolvedSessionPin = {
  tier: PinTier;
  modeId: number | null | undefined;
};

/**
 * Join the active-session list and persistent-pin list to derive each row's pin
 * state. Both queries are already used elsewhere in the app, so this is normally
 * a cache hit.
 */
function useResolvedSessionPins(): {
  byKey: Map<string, ResolvedSessionPin>;
} {
  const sessionsQuery = useGatewaySessionsListQuery(null, { enabled: true });
  const persistentQuery = useGatewayPersistentPinsListQuery({ enabled: true });

  const byKey = useMemo(() => {
    const map = new Map<string, ResolvedSessionPin>();
    // Persistent first (lower precedence; ephemeral overrides below).
    for (const pin of persistentQuery.data ?? []) {
      map.set(`${pin.cli_key}:${pin.session_id}`, {
        tier: "persistent",
        modeId: pin.sort_mode_id,
      });
    }
    for (const s of sessionsQuery.data ?? []) {
      const key = `${s.cli_key}:${s.session_id}`;
      if (s.sort_mode_pinned) {
        map.set(key, { tier: "ephemeral", modeId: s.pinned_sort_mode_id });
      } else if (s.persistent_pinned) {
        map.set(key, { tier: "persistent", modeId: s.persistent_pinned_sort_mode_id });
      }
    }
    return map;
  }, [sessionsQuery.data, persistentQuery.data]);

  return { byKey };
}

type SessionRowPinControlProps = {
  cliKey: string;
  sessionId: string;
};

/**
 * Compact pin controls for a session list row. Mirrors the active-session
 * card's `SortModeSelect` + `PinTierButton` exactly, so the two surfaces
 * share icons, colors, tooltips and mutation wiring.
 */
export function SessionRowPinControl({ cliKey, sessionId }: SessionRowPinControlProps) {
  const { byKey } = useResolvedSessionPins();
  const sortModesQuery = useSortModesListQuery();
  const modes = useMemo<SortModeSummary[]>(() => sortModesQuery.data ?? [], [sortModesQuery.data]);

  const resolved = byKey.get(`${cliKey}:${sessionId}`) ?? { tier: "none", modeId: undefined };
  const tier = resolved.tier;
  const pinnedModeId = tier === "none" ? undefined : (resolved.modeId ?? null);

  return (
    <div
      className={cn(
        "flex shrink-0 items-center gap-1"
        // Row itself is a clickable container in SessionsProjectPage; don't let
        // pin clicks bubble up and trigger row navigation.
      )}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
    >
      <SortModeSelect
        cliKey={cliKey}
        sessionId={sessionId}
        tier={tier}
        selectedModeId={pinnedModeId}
        modes={modes}
        autoLabel="clear"
        className="shrink-0 w-[110px]"
        stopClickPropagation={false}
      />
      <PinTierButton
        cliKey={cliKey}
        sessionId={sessionId}
        tier={tier}
        selectedModeId={pinnedModeId}
        stopClickPropagation
      />
    </div>
  );
}
