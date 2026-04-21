import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { DragEndEvent } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { CLIS, cliFromKeyOrDefault } from "../../constants/clis";
import { logToConsole } from "../../services/consoleLog";
import type { CliKey, ProviderSummary } from "../../services/providers/providers";
import type {
  SortModeActiveRow,
  SortModeProviderRow,
  SortModeSummary,
} from "../../services/providers/sortModes";
import {
  useSortModeActiveListQuery,
  useSortModeCreateMutation,
  useSortModeDeleteMutation,
  useSortModeProviderSetEnabledMutation,
  useSortModeProvidersListQuery,
  useSortModeProvidersSetOrderMutation,
  useSortModeRenameMutation,
  useSortModesListQuery,
} from "../../query/sortModes";

const EMPTY_SORT_MODES: SortModeSummary[] = [];
const EMPTY_MODE_PROVIDERS: SortModeProviderRow[] = [];

function emptyActiveModeByCli(): Record<CliKey, number | null> {
  return {
    claude: null,
    codex: null,
    gemini: null,
  };
}

function buildActiveModeByCli(rows: SortModeActiveRow[]) {
  const next = emptyActiveModeByCli();
  for (const row of rows) {
    next[row.cli_key] = row.mode_id ?? null;
  }
  return next;
}

export type UseSortModesDataModelArgs = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
  providers: ProviderSummary[];
  providersLoading: boolean;
};

export type SortModesDataModel = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
  currentCli: ReturnType<typeof cliFromKeyOrDefault>;
  providers: ProviderSummary[];
  providersLoading: boolean;
  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  activeModeId: number | null;
  selectedMode: SortModeSummary | null;
  providersById: Record<number, ProviderSummary>;
  modeProviders: SortModeProviderRow[];
  modeProvidersLoading: boolean;
  modeProvidersAvailable: boolean | null;
  modeProvidersSaving: boolean;
  modeProviderIdSet: Set<number>;
  createModeDialogOpen: boolean;
  setCreateModeDialogOpen: (open: boolean) => void;
  createModeName: string;
  setCreateModeName: (name: string) => void;
  createModeSaving: boolean;
  renameModeDialogOpen: boolean;
  setRenameModeDialogOpen: (open: boolean) => void;
  renameModeName: string;
  setRenameModeName: (name: string) => void;
  renameModeSaving: boolean;
  deleteModeTarget: SortModeSummary | null;
  setDeleteModeTarget: (mode: SortModeSummary | null) => void;
  deleteModeDeleting: boolean;
  selectEditingMode: (modeId: number | null) => void;
  refreshSortModes: () => Promise<void>;
  createSortMode: () => Promise<void>;
  renameSortMode: () => Promise<void>;
  deleteSortMode: () => Promise<void>;
  addProviderToMode: (providerId: number) => void;
  removeProviderFromMode: (providerId: number) => void;
  setModeProviderEnabled: (providerId: number, enabled: boolean) => Promise<void>;
  handleModeDragEnd: (event: DragEndEvent) => void;
};

export function useSortModesDataModel({
  activeCli,
  setActiveCli,
  providers,
  providersLoading,
}: UseSortModesDataModelArgs): SortModesDataModel {
  const activeCliRef = useRef(activeCli);
  useEffect(() => {
    activeCliRef.current = activeCli;
  }, [activeCli]);

  const currentCli = useMemo(() => cliFromKeyOrDefault(activeCli), [activeCli]);

  const [sortModes, setSortModes] = useState<SortModeSummary[]>(EMPTY_SORT_MODES);
  const [sortModesLoading, setSortModesLoading] = useState(false);
  const [sortModesAvailable, setSortModesAvailable] = useState<boolean | null>(null);
  const [activeModeId, setActiveModeId] = useState<number | null>(null);
  const [activeModeIdTouched, setActiveModeIdTouched] = useState(false);
  const activeModeIdRef = useRef(activeModeId);
  const [activeModeByCli, setActiveModeByCli] =
    useState<Record<CliKey, number | null>>(emptyActiveModeByCli);

  const [modeProviders, setModeProviders] = useState<SortModeProviderRow[]>(EMPTY_MODE_PROVIDERS);
  const modeProvidersRef = useRef(modeProviders);
  const [modeProvidersLoading, setModeProvidersLoading] = useState(false);
  const [modeProvidersAvailable, setModeProvidersAvailable] = useState<boolean | null>(null);
  const [modeProvidersSaving, setModeProvidersSaving] = useState(false);

  const [createModeDialogOpen, setCreateModeDialogOpen] = useState(false);
  const [createModeName, setCreateModeName] = useState("");
  const [createModeSaving, setCreateModeSaving] = useState(false);

  const [renameModeDialogOpen, setRenameModeDialogOpen] = useState(false);
  const [renameModeName, setRenameModeName] = useState("");
  const [renameModeSaving, setRenameModeSaving] = useState(false);

  const [deleteModeTarget, setDeleteModeTarget] = useState<SortModeSummary | null>(null);
  const [deleteModeDeleting, setDeleteModeDeleting] = useState(false);

  const sortModesListQuery = useSortModesListQuery({ enabled: false });
  const sortModeActiveListQuery = useSortModeActiveListQuery({ enabled: false });
  const sortModeProvidersListQuery = useSortModeProvidersListQuery(
    { modeId: activeModeId, cliKey: activeCli },
    { enabled: false }
  );

  const createSortModeMutation = useSortModeCreateMutation();
  const renameSortModeMutation = useSortModeRenameMutation();
  const deleteSortModeMutation = useSortModeDeleteMutation();
  const sortModeProvidersSetOrderMutation = useSortModeProvidersSetOrderMutation();
  const sortModeProviderSetEnabledMutation = useSortModeProviderSetEnabledMutation();

  useEffect(() => {
    activeModeIdRef.current = activeModeId;
  }, [activeModeId]);

  useEffect(() => {
    modeProvidersRef.current = modeProviders;
  }, [modeProviders]);

  const selectedMode = useMemo(
    () =>
      activeModeId == null ? null : (sortModes.find((mode) => mode.id === activeModeId) ?? null),
    [activeModeId, sortModes]
  );

  const activeModeForCurrentCli = activeModeByCli[activeCli] ?? null;

  const providersById = useMemo(() => {
    const map: Record<number, ProviderSummary> = {};
    for (const provider of providers) {
      map[provider.id] = provider;
    }
    return map;
  }, [providers]);

  const modeProviderIdSet = useMemo(() => {
    const set = new Set<number>();
    for (const row of modeProviders) {
      set.add(row.provider_id);
    }
    return set;
  }, [modeProviders]);

  const refetchSortModesList = sortModesListQuery.refetch;
  const refetchSortModeActiveList = sortModeActiveListQuery.refetch;

  const refreshSortModes = useCallback(async () => {
    setSortModesLoading(true);
    try {
      const [modesResult, activeResult] = await Promise.all([
        refetchSortModesList(),
        refetchSortModeActiveList(),
      ]);

      if (modesResult.error) {
        throw modesResult.error;
      }
      if (activeResult.error) {
        throw activeResult.error;
      }

      const modes = modesResult.data ?? null;
      const activeRows = activeResult.data ?? null;
      if (!modes || !activeRows) {
        setSortModesAvailable(false);
        setSortModes(EMPTY_SORT_MODES);
        setActiveModeByCli(emptyActiveModeByCli());
        return;
      }

      setSortModesAvailable(true);
      setSortModes(modes);
      setActiveModeByCli(buildActiveModeByCli(activeRows));
    } catch (err) {
      setSortModesAvailable(true);
      setSortModes(EMPTY_SORT_MODES);
      setActiveModeByCli(emptyActiveModeByCli());
      logToConsole("error", "读取排序模板失败", { error: String(err) });
      toast(`读取排序模板失败：${String(err)}`);
    } finally {
      setSortModesLoading(false);
    }
  }, [refetchSortModeActiveList, refetchSortModesList]);

  useEffect(() => {
    void refreshSortModes();
  }, [refreshSortModes]);

  function selectEditingMode(modeId: number | null) {
    setActiveModeIdTouched(true);
    setActiveModeId(modeId);
  }

  useEffect(() => {
    if (activeModeId == null) return;
    if (sortModes.some((mode) => mode.id === activeModeId)) return;
    setActiveModeId(null);
  }, [activeModeId, sortModes]);

  useEffect(() => {
    if (activeModeIdTouched) return;
    if (activeModeId != null) return;
    if (sortModesAvailable !== true) return;
    if (activeModeForCurrentCli == null) return;
    if (!sortModes.some((mode) => mode.id === activeModeForCurrentCli)) return;
    setActiveModeId(activeModeForCurrentCli);
  }, [activeModeForCurrentCli, activeModeId, activeModeIdTouched, sortModes, sortModesAvailable]);

  const refetchModeProviders = sortModeProvidersListQuery.refetch;

  useEffect(() => {
    if (activeModeId == null) {
      setModeProvidersAvailable(true);
      setModeProviders(EMPTY_MODE_PROVIDERS);
      modeProvidersRef.current = EMPTY_MODE_PROVIDERS;
      setModeProvidersLoading(false);
      return;
    }

    let cancelled = false;
    setModeProvidersLoading(true);
    void refetchModeProviders()
      .then((result) => {
        if (cancelled) return;
        if (result.error) {
          throw result.error;
        }

        const rows = result.data ?? null;
        if (!rows) {
          setModeProvidersAvailable(false);
          setModeProviders(EMPTY_MODE_PROVIDERS);
          modeProvidersRef.current = EMPTY_MODE_PROVIDERS;
          return;
        }

        setModeProvidersAvailable(true);
        setModeProviders(rows);
        modeProvidersRef.current = rows;
      })
      .catch((err) => {
        if (cancelled) return;
        setModeProvidersAvailable(true);
        setModeProviders(EMPTY_MODE_PROVIDERS);
        modeProvidersRef.current = EMPTY_MODE_PROVIDERS;
        logToConsole("error", "读取排序模板 Provider 列表失败", {
          error: String(err),
          mode_id: activeModeId,
          cli: activeCli,
        });
        toast(`读取排序模板 Provider 列表失败：${String(err)}`);
      })
      .finally(() => {
        if (cancelled) return;
        setModeProvidersLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [activeCli, activeModeId, refetchModeProviders]);

  useEffect(() => {
    if (!createModeDialogOpen) return;
    setCreateModeName("");
  }, [createModeDialogOpen]);

  useEffect(() => {
    if (!renameModeDialogOpen) return;
    setRenameModeName(selectedMode?.name ?? "");
  }, [renameModeDialogOpen, selectedMode]);

  async function createSortMode() {
    if (createModeSaving) return;
    const name = createModeName.trim();
    if (!name) {
      toast("模式名称不能为空");
      return;
    }

    setCreateModeSaving(true);
    try {
      const saved = await createSortModeMutation.mutateAsync({ name });
      if (!saved) {
        return;
      }
      setSortModes((prev) => [...prev, saved]);
      selectEditingMode(saved.id);
      setCreateModeDialogOpen(false);
      toast("排序模板已创建");
    } catch (err) {
      logToConsole("error", "创建排序模板失败", { error: String(err) });
      toast(`创建失败：${String(err)}`);
    } finally {
      setCreateModeSaving(false);
    }
  }

  async function renameSortMode() {
    if (renameModeSaving) return;
    if (!selectedMode) return;
    const name = renameModeName.trim();
    if (!name) {
      toast("模式名称不能为空");
      return;
    }

    setRenameModeSaving(true);
    try {
      const saved = await renameSortModeMutation.mutateAsync({ modeId: selectedMode.id, name });
      if (!saved) {
        return;
      }
      setSortModes((prev) => prev.map((mode) => (mode.id === saved.id ? saved : mode)));
      setRenameModeDialogOpen(false);
      toast("排序模板已更新");
    } catch (err) {
      logToConsole("error", "重命名排序模板失败", { error: String(err), mode_id: selectedMode.id });
      toast(`重命名失败：${String(err)}`);
    } finally {
      setRenameModeSaving(false);
    }
  }

  async function deleteSortMode() {
    if (!deleteModeTarget || deleteModeDeleting) return;
    setDeleteModeDeleting(true);
    try {
      const ok = await deleteSortModeMutation.mutateAsync({ modeId: deleteModeTarget.id });
      if (!ok) {
        return;
      }
      setSortModes((prev) => prev.filter((mode) => mode.id !== deleteModeTarget.id));
      setActiveModeByCli((prev) => {
        const next: Record<CliKey, number | null> = { ...prev };
        let changed = false;
        for (const cli of CLIS) {
          if (next[cli.key] === deleteModeTarget.id) {
            next[cli.key] = null;
            changed = true;
          }
        }
        return changed ? next : prev;
      });
      if (activeModeIdRef.current === deleteModeTarget.id) {
        setActiveModeId(null);
      }
      setDeleteModeTarget(null);
      toast("排序模板已删除");
    } catch (err) {
      logToConsole("error", "删除排序模板失败", {
        error: String(err),
        mode_id: deleteModeTarget.id,
      });
      toast(`删除失败：${String(err)}`);
    } finally {
      setDeleteModeDeleting(false);
    }
  }

  async function persistModeProvidersOrder(
    modeId: number,
    cliKey: CliKey,
    nextRows: SortModeProviderRow[],
    prevRows: SortModeProviderRow[]
  ) {
    if (modeProvidersSaving) return;
    setModeProvidersSaving(true);
    try {
      const saved = await sortModeProvidersSetOrderMutation.mutateAsync({
        modeId,
        cliKey,
        orderedProviderIds: nextRows.map((row) => row.provider_id),
      });

      if (!saved) {
        if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
          setModeProviders(prevRows);
          modeProvidersRef.current = prevRows;
        }
        return;
      }

      if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
        setModeProviders(saved);
        modeProvidersRef.current = saved;
        toast("模式顺序已更新");
      }
    } catch (err) {
      if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
        setModeProviders(prevRows);
        modeProvidersRef.current = prevRows;
      }
      logToConsole("error", "更新排序模板顺序失败", {
        error: String(err),
        mode_id: modeId,
        cli: cliKey,
      });
      toast(`模式顺序更新失败：${String(err)}`);
    } finally {
      setModeProvidersSaving(false);
    }
  }

  function addProviderToMode(providerId: number) {
    if (activeModeIdRef.current == null) return;
    const modeId = activeModeIdRef.current;
    const cliKey = activeCliRef.current;
    const prevRows = modeProvidersRef.current;
    if (prevRows.some((row) => row.provider_id === providerId)) return;
    const nextRows: SortModeProviderRow[] = [
      ...prevRows,
      { provider_id: providerId, enabled: true },
    ];
    setModeProviders(nextRows);
    modeProvidersRef.current = nextRows;
    void persistModeProvidersOrder(modeId, cliKey, nextRows, prevRows);
  }

  function removeProviderFromMode(providerId: number) {
    if (activeModeIdRef.current == null) return;
    const modeId = activeModeIdRef.current;
    const cliKey = activeCliRef.current;
    const prevRows = modeProvidersRef.current;
    if (!prevRows.some((row) => row.provider_id === providerId)) return;
    const nextRows = prevRows.filter((row) => row.provider_id !== providerId);
    setModeProviders(nextRows);
    modeProvidersRef.current = nextRows;
    void persistModeProvidersOrder(modeId, cliKey, nextRows, prevRows);
  }

  async function setModeProviderEnabled(providerId: number, enabled: boolean) {
    if (modeProvidersSaving) return;

    const modeId = activeModeIdRef.current;
    if (modeId == null) return;

    const cliKey = activeCliRef.current;
    const prevRows = modeProvidersRef.current;
    const existing = prevRows.find((row) => row.provider_id === providerId) ?? null;
    if (!existing || existing.enabled === enabled) {
      return;
    }

    const nextRows = prevRows.map((row) =>
      row.provider_id === providerId ? { ...row, enabled } : row
    );
    setModeProviders(nextRows);
    modeProvidersRef.current = nextRows;

    setModeProvidersSaving(true);
    try {
      const saved = await sortModeProviderSetEnabledMutation.mutateAsync({
        modeId,
        cliKey,
        providerId,
        enabled,
      });

      if (!saved) {
        if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
          setModeProviders(prevRows);
          modeProvidersRef.current = prevRows;
        }
        return;
      }

      if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
        const finalRows = nextRows.map((row) => (row.provider_id === providerId ? saved : row));
        setModeProviders(finalRows);
        modeProvidersRef.current = finalRows;
        toast(saved.enabled ? "模板已启用 Provider" : "模板已禁用 Provider");
      }
    } catch (err) {
      if (activeModeIdRef.current === modeId && activeCliRef.current === cliKey) {
        setModeProviders(prevRows);
        modeProvidersRef.current = prevRows;
      }
      logToConsole("error", "更新排序模板 Provider 启用状态失败", {
        error: String(err),
        mode_id: modeId,
        cli: cliKey,
        provider_id: providerId,
        enabled,
      });
      toast(`模板启用状态更新失败：${String(err)}`);
    } finally {
      setModeProvidersSaving(false);
    }
  }

  function handleModeDragEnd(event: DragEndEvent) {
    const modeId = activeModeIdRef.current;
    if (modeId == null) return;

    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const prevRows = modeProvidersRef.current;
    const oldIndex = prevRows.findIndex((row) => row.provider_id === active.id);
    const newIndex = prevRows.findIndex((row) => row.provider_id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const nextRows = arrayMove(prevRows, oldIndex, newIndex);
    setModeProviders(nextRows);
    modeProvidersRef.current = nextRows;
    void persistModeProvidersOrder(modeId, activeCliRef.current, nextRows, prevRows);
  }

  return {
    activeCli,
    setActiveCli,
    currentCli,
    providers,
    providersLoading,
    sortModes,
    sortModesLoading,
    activeModeId,
    selectedMode,
    providersById,
    modeProviders,
    modeProvidersLoading,
    modeProvidersAvailable,
    modeProvidersSaving,
    modeProviderIdSet,
    createModeDialogOpen,
    setCreateModeDialogOpen,
    createModeName,
    setCreateModeName,
    createModeSaving,
    renameModeDialogOpen,
    setRenameModeDialogOpen,
    renameModeName,
    setRenameModeName,
    renameModeSaving,
    deleteModeTarget,
    setDeleteModeTarget,
    deleteModeDeleting,
    selectEditingMode,
    refreshSortModes,
    createSortMode,
    renameSortMode,
    deleteSortMode,
    addProviderToMode,
    removeProviderFromMode,
    setModeProviderEnabled,
    handleModeDragEnd,
  };
}
