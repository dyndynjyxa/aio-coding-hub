// Usage:
// - Manages sort mode queries, active mode derivation, mode switching with
//   confirmation dialog support, and the switchingCliKey guard for HomePage.

import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import type { CliKey } from "../../../services/providers/providers";
import type { GatewayActiveSession } from "../../../services/gateway/gateway";
import type { SortModeSummary } from "../../../services/providers/sortModes";
import {
  useSortModeActiveListQuery,
  useSortModeActiveSetMutation,
  useSortModesListQuery,
} from "../../../query/sortModes";

export type PendingSortModeSwitch = {
  cliKey: CliKey;
  modeId: number | null;
  activeSessionCount: number;
};

export type HomeSortModeState = {
  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  pendingSortModeSwitch: PendingSortModeSwitch | null;
  setPendingSortModeSwitch: (v: PendingSortModeSwitch | null) => void;
  requestCliActiveModeSwitch: (cliKey: CliKey, modeId: number | null) => void;
  confirmPendingSortModeSwitch: () => void;
};

export function useHomeSortMode(activeSessions: GatewayActiveSession[]): HomeSortModeState {
  const [switchingCliKey, setSwitchingCliKey] = useState<CliKey | null>(null);
  const [pendingSortModeSwitch, setPendingSortModeSwitch] = useState<PendingSortModeSwitch | null>(
    null
  );

  const sortModesQuery = useSortModesListQuery();
  const sortModeActiveQuery = useSortModeActiveListQuery();
  const sortModeActiveSetMutation = useSortModeActiveSetMutation();

  const sortModes = useMemo(() => sortModesQuery.data ?? [], [sortModesQuery.data]);
  const sortModesLoading = sortModesQuery.isLoading || sortModeActiveQuery.isLoading;
  const sortModesAvailable: boolean | null = sortModesLoading
    ? null
    : sortModesQuery.data != null && sortModeActiveQuery.data != null;

  const activeModeByCli = useMemo<Record<CliKey, number | null>>(() => {
    const next: Record<CliKey, number | null> = {
      claude: null,
      codex: null,
      gemini: null,
    };
    for (const row of sortModeActiveQuery.data ?? []) {
      next[row.cli_key] = row.mode_id ?? null;
    }
    return next;
  }, [sortModeActiveQuery.data]);

  const activeModeToggling = useMemo<Record<CliKey, boolean>>(
    () => ({
      claude: switchingCliKey === "claude",
      codex: switchingCliKey === "codex",
      gemini: switchingCliKey === "gemini",
    }),
    [switchingCliKey]
  );

  const setCliActiveMode = useCallback(
    async (cliKey: CliKey, modeId: number | null) => {
      if (switchingCliKey != null) return;

      const prev = activeModeByCli[cliKey] ?? null;
      if (prev === modeId) return;

      setSwitchingCliKey(cliKey);
      try {
        const res = await sortModeActiveSetMutation.mutateAsync({ cliKey, modeId });
        if (!res) {
          return;
        }

        const next = res.mode_id ?? null;
        if (next == null) {
          toast("已切回：Default");
          return;
        }
        const label = sortModes.find((m) => m.id === next)?.name ?? `#${next}`;
        toast(`已激活：${label}`);
      } catch (err) {
        toast(`切换排序模板失败：${String(err)}`);
        logToConsole("error", "切换排序模板失败", {
          cli: cliKey,
          mode_id: modeId,
          error: String(err),
        });
      } finally {
        setSwitchingCliKey(null);
      }
    },
    [activeModeByCli, sortModeActiveSetMutation, sortModes, switchingCliKey]
  );
  const requestCliActiveModeSwitch = useCallback(
    (cliKey: CliKey, modeId: number | null) => {
      if (activeModeToggling[cliKey] || sortModesLoading) return;

      const prev = activeModeByCli[cliKey] ?? null;
      if (prev === modeId) return;

      const activeSessionCount = activeSessions.filter((row) => row.cli_key === cliKey).length;
      if (activeSessionCount > 0) {
        setPendingSortModeSwitch({ cliKey, modeId, activeSessionCount });
        return;
      }

      void setCliActiveMode(cliKey, modeId);
    },
    [activeModeByCli, activeModeToggling, activeSessions, setCliActiveMode, sortModesLoading]
  );

  const confirmPendingSortModeSwitch = useCallback(() => {
    const pending = pendingSortModeSwitch;
    if (!pending) return;
    setPendingSortModeSwitch(null);
    void setCliActiveMode(pending.cliKey, pending.modeId);
  }, [pendingSortModeSwitch, setCliActiveMode]);

  return {
    sortModes,
    sortModesLoading,
    sortModesAvailable,
    activeModeByCli,
    activeModeToggling,
    pendingSortModeSwitch,
    setPendingSortModeSwitch,
    requestCliActiveModeSwitch,
    confirmPendingSortModeSwitch,
  };
}
