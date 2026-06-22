// Usage: Data-model hook for ProvidersView orchestration.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import { toast } from "sonner";
import { PointerSensor, type DragEndEvent, useSensor, useSensors } from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
import { logToConsole } from "../../../services/consoleLog";
import { copyText } from "../../../services/clipboard";
import type { GatewayProviderCircuitStatus } from "../../../services/gateway/gateway";
import {
  type CliKey,
  type ProviderRouteRow,
  type ProviderSummary,
} from "../../../services/providers/providers";
import {
  summarizeGatewayCircuitRows,
  useGatewayCircuitAutoRefresh,
  useGatewayCircuitResetCliMutation,
  useGatewayCircuitResetProviderMutation,
  useGatewaySessionsListQuery,
  useGatewayCircuitStatusQuery,
} from "../../../query/gateway";
import {
  useDefaultRouteProvidersQuery,
  useDefaultRouteProvidersSetOrderMutation,
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderDuplicateMutation,
  useProviderSetEnabledMutation,
  useProviderTestAvailabilityMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../../../query/providers";
import {
  useSortModeActiveListQuery,
  useSortModeActiveSetMutation,
  useSortModeCreateMutation,
  useSortModeDeleteMutation,
  useSortModeProvidersListQuery,
  useSortModeProviderSetEnabledMutation,
  useSortModeProvidersSetOrderMutation,
  useSortModeRenameMutation,
  useSortModesListQuery,
} from "../../../query/sortModes";
import type {
  SortModeActiveRow,
  SortModeProviderRow,
  SortModeSummary,
} from "../../../services/providers/sortModes";
import type { ProviderEditorInitialValues } from "../providerDuplicate";
import { reorderVisibleItems } from "../reorderVisibleItems";

type CreateDialogState = {
  cliKey: CliKey;
  initialValues: ProviderEditorInitialValues | null;
};

type ProviderRefreshResult = { error: unknown | null };
type ProviderActionMap = Record<number, boolean>;
type ProviderActionMapSetter = Dispatch<SetStateAction<ProviderActionMap>>;

type RouteDraftSelection = { kind: "default"; modeId: null } | { kind: "mode"; modeId: number };

type PendingRouteActivation = {
  cliKey: CliKey;
  modeId: number | null;
  label: string;
  activeSessionCount: number;
};

const EMPTY_SORT_MODES: SortModeSummary[] = [];
const EMPTY_MODE_PROVIDERS: SortModeProviderRow[] = [];
const EMPTY_ROUTE_ROWS: ProviderRouteRow[] = [];

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

function routeDraftKey(selection: RouteDraftSelection) {
  return selection.kind === "default" ? "default" : `mode:${selection.modeId}`;
}

function beginProviderAction(ref: MutableRefObject<ProviderActionMap>, providerId: number) {
  if (ref.current[providerId]) {
    return false;
  }

  ref.current = { ...ref.current, [providerId]: true };
  return true;
}

function finishProviderAction(ref: MutableRefObject<ProviderActionMap>, providerId: number) {
  if (!ref.current[providerId]) {
    return;
  }

  const next = { ...ref.current };
  delete next[providerId];
  ref.current = next;
}

function beginStatefulProviderAction(
  ref: MutableRefObject<ProviderActionMap>,
  setState: ProviderActionMapSetter,
  providerId: number
) {
  if (!beginProviderAction(ref, providerId)) {
    return false;
  }

  setState((current) => ({ ...current, [providerId]: true }));
  return true;
}

function finishStatefulProviderAction(
  ref: MutableRefObject<ProviderActionMap>,
  setState: ProviderActionMapSetter,
  providerId: number
) {
  if (!ref.current[providerId]) {
    return;
  }

  finishProviderAction(ref, providerId);
  setState((current) => {
    if (!current[providerId]) return current;
    const next = { ...current };
    delete next[providerId];
    return next;
  });
}

export function useProvidersViewDataModel(activeCli: CliKey) {
  const mountedRef = useRef(false);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const activeCliRef = useRef(activeCli);
  useEffect(() => {
    activeCliRef.current = activeCli;
  }, [activeCli]);

  const providersQuery = useProvidersListQuery(activeCli);
  const providers = useMemo<ProviderSummary[]>(
    () => providersQuery.data ?? [],
    [providersQuery.data]
  );
  const codexProvidersQuery = useProvidersListQuery("codex", { enabled: activeCli === "claude" });
  const codexProviders = useMemo<ProviderSummary[]>(
    () => codexProvidersQuery.data ?? [],
    [codexProvidersQuery.data]
  );
  const providersLoading = providersQuery.isFetching;
  const defaultRouteQuery = useDefaultRouteProvidersQuery(activeCli);
  const sortModesQuery = useSortModesListQuery();
  const sortModeActiveQuery = useSortModeActiveListQuery();

  const sourceProvidersById = useMemo(
    () => Object.fromEntries(codexProviders.map((provider) => [provider.id, provider])),
    [codexProviders]
  );
  const sourceProviderNamesById = useMemo(
    () => Object.fromEntries(codexProviders.map((provider) => [provider.id, provider.name])),
    [codexProviders]
  );

  const providersRef = useRef(providers);
  useEffect(() => {
    providersRef.current = providers;
  }, [providers]);
  const providersRefreshTokenByCliRef = useRef<Partial<Record<CliKey, number>>>({});
  const providersRefreshNextTokenRef = useRef(0);
  const providerReorderSaveTokenByCliRef = useRef<Partial<Record<CliKey, number>>>({});
  const providerReorderNextSaveTokenRef = useRef(0);
  const [routeDraftSelection, setRouteDraftSelection] = useState<RouteDraftSelection>({
    kind: "default",
    modeId: null,
  });
  const routeDraftSelectionRef = useRef(routeDraftSelection);
  const [modeProviders, setModeProviders] = useState<SortModeProviderRow[]>(EMPTY_MODE_PROVIDERS);
  const modeProvidersRef = useRef(modeProviders);
  const [routeSaving, setRouteSaving] = useState(false);
  const routeSavingRef = useRef(false);
  const [createModeDialogOpen, setCreateModeDialogOpen] = useState(false);
  const [createModeName, setCreateModeName] = useState("");
  const [createModeSaving, setCreateModeSaving] = useState(false);
  const [renameModeDialogOpen, setRenameModeDialogOpen] = useState(false);
  const [renameModeName, setRenameModeName] = useState("");
  const [renameModeSaving, setRenameModeSaving] = useState(false);
  const [deleteModeTarget, setDeleteModeTarget] = useState<SortModeSummary | null>(null);
  const [deleteModeDeleting, setDeleteModeDeleting] = useState(false);
  const [activatingRoute, setActivatingRoute] = useState(false);
  const activatingRouteRef = useRef(false);
  const [pendingRouteActivation, setPendingRouteActivation] =
    useState<PendingRouteActivation | null>(null);

  const circuitQuery = useGatewayCircuitStatusQuery(activeCli);
  const activeSessionsQuery = useGatewaySessionsListQuery(50, {
    refetchIntervalMs: 5000,
  });
  const activeSessions = activeSessionsQuery.data ?? [];
  const circuitRows = useMemo<GatewayProviderCircuitStatus[]>(
    () => circuitQuery.data ?? [],
    [circuitQuery.data]
  );
  const circuitLoading = circuitQuery.isFetching;
  const circuitSummary = useMemo(() => summarizeGatewayCircuitRows(circuitRows), [circuitRows]);
  const circuitByProviderId = circuitSummary.byProviderId;
  useGatewayCircuitAutoRefresh(activeCli, circuitSummary);

  const [circuitResetting, setCircuitResetting] = useState<Record<number, boolean>>({});
  const circuitResettingRef = useRef<ProviderActionMap>({});
  const [circuitResettingAll, setCircuitResettingAll] = useState(false);
  const circuitResettingAllRef = useRef(false);
  const [createDialogState, setCreateDialogState] = useState<CreateDialogState | null>(null);
  const [editTarget, setEditTarget] = useState<ProviderSummary | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProviderSummary | null>(null);
  const [deleting, setDeleting] = useState(false);
  const deletingRef = useRef(false);
  const [terminalCopyingByProviderId, setTerminalCopyingByProviderId] = useState<
    Record<number, boolean>
  >({});
  const terminalCopyingByProviderIdRef = useRef<ProviderActionMap>({});
  const [duplicatingByProviderId, setDuplicatingByProviderId] = useState<Record<number, boolean>>(
    {}
  );
  const duplicatingByProviderIdRef = useRef<ProviderActionMap>({});
  const [testingByProviderId, setTestingByProviderId] = useState<Record<number, boolean>>({});
  const testingByProviderIdRef = useRef<ProviderActionMap>({});
  const togglingByProviderIdRef = useRef<ProviderActionMap>({});
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [providerSearch, setProviderSearch] = useState("");
  const [providersRefreshingByCli, setProvidersRefreshingByCli] = useState<
    Partial<Record<CliKey, boolean>>
  >({});

  const resetCircuitProviderMutation = useGatewayCircuitResetProviderMutation();
  const resetCircuitCliMutation = useGatewayCircuitResetCliMutation();
  const providerSetEnabledMutation = useProviderSetEnabledMutation();
  const providerDeleteMutation = useProviderDeleteMutation();
  const providerDuplicateMutation = useProviderDuplicateMutation();
  const providersReorderMutation = useProvidersReorderMutation();
  const defaultRouteSetOrderMutation = useDefaultRouteProvidersSetOrderMutation();
  const sortModeCreateMutation = useSortModeCreateMutation();
  const sortModeRenameMutation = useSortModeRenameMutation();
  const sortModeDeleteMutation = useSortModeDeleteMutation();
  const sortModeActiveSetMutation = useSortModeActiveSetMutation();
  const sortModeProvidersSetOrderMutation = useSortModeProvidersSetOrderMutation();
  const sortModeProviderSetEnabledMutation = useSortModeProviderSetEnabledMutation();
  const terminalLaunchCommandMutation = useProviderClaudeTerminalLaunchCommandMutation();
  const testAvailabilityMutation = useProviderTestAvailabilityMutation();

  const sortModes = sortModesQuery.data ?? EMPTY_SORT_MODES;
  const sortModesLoading = sortModesQuery.isLoading || sortModeActiveQuery.isLoading;
  const sortModesAvailable =
    sortModesQuery.data != null && sortModeActiveQuery.data != null ? true : null;
  const activeModeByCli = useMemo(
    () => buildActiveModeByCli(sortModeActiveQuery.data ?? []),
    [sortModeActiveQuery.data]
  );
  const activeModeId = activeModeByCli[activeCli] ?? null;
  const selectedSortMode = useMemo(
    () =>
      routeDraftSelection.kind === "mode"
        ? (sortModes.find((mode) => mode.id === routeDraftSelection.modeId) ?? null)
        : null,
    [routeDraftSelection, sortModes]
  );
  const selectedRouteLabel =
    routeDraftSelection.kind === "default"
      ? "Default"
      : (selectedSortMode?.name ?? `#${routeDraftSelection.modeId}`);
  const currentRouteActive =
    routeDraftSelection.kind === "default"
      ? activeModeId == null
      : activeModeId === routeDraftSelection.modeId;
  const providersById = useMemo(
    () => Object.fromEntries(providers.map((provider) => [provider.id, provider])),
    [providers]
  );
  const defaultRouteRows = defaultRouteQuery.data ?? EMPTY_ROUTE_ROWS;
  const activeModeForQuery =
    routeDraftSelection.kind === "mode" ? routeDraftSelection.modeId : null;
  const modeProvidersQuery = useSortModeProvidersListQuery(
    { modeId: activeModeForQuery, cliKey: activeCli },
    { enabled: routeDraftSelection.kind === "mode" && activeModeForQuery != null }
  );

  const tagCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const provider of providers) {
      for (const tag of provider.tags ?? []) {
        counts.set(tag, (counts.get(tag) ?? 0) + 1);
      }
    }
    return counts;
  }, [providers]);

  const filteredProviders = useMemo(() => {
    const normalizedSearch = providerSearch.trim().toLowerCase();

    return providers.filter((provider) => {
      const matchesTags =
        selectedTags.size === 0 || (provider.tags ?? []).some((tag) => selectedTags.has(tag));
      if (!matchesTags) return false;
      if (!normalizedSearch) return true;
      return provider.name.toLowerCase().includes(normalizedSearch);
    });
  }, [providerSearch, providers, selectedTags]);

  useEffect(() => {
    routeDraftSelectionRef.current = routeDraftSelection;
  }, [routeDraftSelection]);

  useEffect(() => {
    modeProvidersRef.current = modeProviders;
  }, [modeProviders]);

  useEffect(() => {
    if (routeDraftSelection.kind !== "mode") {
      setModeProviders(EMPTY_MODE_PROVIDERS);
      modeProvidersRef.current = EMPTY_MODE_PROVIDERS;
      return;
    }
    const rows = modeProvidersQuery.data ?? EMPTY_MODE_PROVIDERS;
    setModeProviders(rows);
    modeProvidersRef.current = rows;
  }, [modeProvidersQuery.data, routeDraftSelection.kind, routeDraftSelection.modeId]);

  useEffect(() => {
    if (routeDraftSelection.kind !== "mode") return;
    if (sortModes.some((mode) => mode.id === routeDraftSelection.modeId)) return;
    setRouteDraftSelection({ kind: "default", modeId: null });
  }, [routeDraftSelection, sortModes]);

  useEffect(() => {
    if (!createModeDialogOpen) return;
    setCreateModeName("");
  }, [createModeDialogOpen]);

  useEffect(() => {
    if (!renameModeDialogOpen) return;
    setRenameModeName(selectedSortMode?.name ?? "");
  }, [renameModeDialogOpen, selectedSortMode]);

  const beginProvidersRefresh = useCallback((cliKey: CliKey) => {
    if (providersRefreshTokenByCliRef.current[cliKey] != null) {
      return null;
    }

    const token = providersRefreshNextTokenRef.current + 1;
    providersRefreshNextTokenRef.current = token;
    providersRefreshTokenByCliRef.current = {
      ...providersRefreshTokenByCliRef.current,
      [cliKey]: token,
    };
    if (mountedRef.current) {
      setProvidersRefreshingByCli((current) => ({ ...current, [cliKey]: true }));
    }
    return token;
  }, []);

  const finishProvidersRefresh = useCallback((cliKey: CliKey, token: number) => {
    if (providersRefreshTokenByCliRef.current[cliKey] !== token) {
      return;
    }

    const next = { ...providersRefreshTokenByCliRef.current };
    delete next[cliKey];
    providersRefreshTokenByCliRef.current = next;
    if (!mountedRef.current) {
      return;
    }

    setProvidersRefreshingByCli((current) => {
      if (!current[cliKey]) return current;
      const nextState = { ...current };
      delete nextState[cliKey];
      return nextState;
    });
  }, []);

  const refreshProviders = useCallback(async () => {
    const cliKey = activeCliRef.current;
    const refreshToken = beginProvidersRefresh(cliKey);
    if (refreshToken == null) return;

    const refreshes: Array<Promise<ProviderRefreshResult>> = [providersQuery.refetch()];
    if (cliKey === "claude") {
      refreshes.push(codexProvidersQuery.refetch());
    }

    try {
      const results = await Promise.allSettled(refreshes);
      const hasError = results.some(
        (result) => result.status === "rejected" || result.value.error != null
      );
      if (mountedRef.current && activeCliRef.current === cliKey && hasError) {
        toast("刷新供应商列表失败：请查看控制台日志");
      }
    } finally {
      finishProvidersRefresh(cliKey, refreshToken);
    }
  }, [beginProvidersRefresh, codexProvidersQuery, finishProvidersRefresh, providersQuery]);

  useEffect(() => {
    setSelectedTags(new Set());
    setProviderSearch("");
    setCreateDialogState(null);
    setEditTarget(null);
    setDeleteTarget(null);
    setRouteDraftSelection({ kind: "default", modeId: null });
  }, [activeCli]);

  useEffect(() => {
    togglingByProviderIdRef.current = {};
    circuitResettingRef.current = {};
    circuitResettingAllRef.current = false;
    terminalCopyingByProviderIdRef.current = {};
    duplicatingByProviderIdRef.current = {};
    testingByProviderIdRef.current = {};
    setCircuitResetting({});
    setCircuitResettingAll(false);
    setTerminalCopyingByProviderId({});
    setDuplicatingByProviderId({});
    setTestingByProviderId({});
  }, [activeCli]);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  const beginProviderReorderSave = useCallback((cliKey: CliKey) => {
    if (providerReorderSaveTokenByCliRef.current[cliKey] != null) {
      return null;
    }

    const token = providerReorderNextSaveTokenRef.current + 1;
    providerReorderNextSaveTokenRef.current = token;
    providerReorderSaveTokenByCliRef.current = {
      ...providerReorderSaveTokenByCliRef.current,
      [cliKey]: token,
    };
    return token;
  }, []);

  const finishProviderReorderSave = useCallback((cliKey: CliKey, token: number) => {
    if (providerReorderSaveTokenByCliRef.current[cliKey] !== token) {
      return;
    }

    const next = { ...providerReorderSaveTokenByCliRef.current };
    delete next[cliKey];
    providerReorderSaveTokenByCliRef.current = next;
  }, []);

  function openCreateDialog(
    cliKey: CliKey,
    initialValues: ProviderEditorInitialValues | null = null
  ) {
    setCreateDialogState({ cliKey, initialValues });
  }

  const toggleProviderEnabled = useCallback(
    async (provider: ProviderSummary) => {
      if (!beginProviderAction(togglingByProviderIdRef, provider.id)) {
        return;
      }

      try {
        const next = await providerSetEnabledMutation.mutateAsync({
          providerId: provider.id,
          enabled: !provider.enabled,
        });
        if (!next) return;

        logToConsole("info", "更新 Provider 状态", { id: next.id, enabled: next.enabled });
        toast(next.enabled ? "已启用 Provider" : "已禁用 Provider");
      } catch (error) {
        logToConsole("error", "更新 Provider 状态失败", {
          error: String(error),
          id: provider.id,
        });
        toast(`更新失败：${String(error)}`);
      } finally {
        finishProviderAction(togglingByProviderIdRef, provider.id);
      }
    },
    [providerSetEnabledMutation]
  );

  const resetCircuit = useCallback(
    async (provider: ProviderSummary) => {
      if (!beginStatefulProviderAction(circuitResettingRef, setCircuitResetting, provider.id)) {
        return;
      }

      try {
        await resetCircuitProviderMutation.mutateAsync({
          cliKey: provider.cli_key,
          providerId: provider.id,
        });

        toast("已解除熔断");
        void circuitQuery.refetch();
      } catch (error) {
        logToConsole("error", "解除熔断失败", {
          provider_id: provider.id,
          error: String(error),
        });
        toast(`解除熔断失败：${String(error)}`);
      } finally {
        finishStatefulProviderAction(circuitResettingRef, setCircuitResetting, provider.id);
      }
    },
    [circuitQuery, resetCircuitProviderMutation]
  );

  const resetCircuitAll = useCallback(
    async (cliKey: CliKey) => {
      if (circuitResettingAllRef.current) return;

      circuitResettingAllRef.current = true;
      setCircuitResettingAll(true);
      try {
        const count = await resetCircuitCliMutation.mutateAsync({ cliKey });
        toast(
          count != null && count > 0 ? `已解除 ${count} 个 Provider 的熔断` : "无 Provider 需要处理"
        );
        void circuitQuery.refetch();
      } catch (error) {
        logToConsole("error", "解除熔断（全部）失败", {
          cli: cliKey,
          error: String(error),
        });
        toast(`解除熔断失败：${String(error)}`);
      } finally {
        circuitResettingAllRef.current = false;
        setCircuitResettingAll(false);
      }
    },
    [circuitQuery, resetCircuitCliMutation]
  );

  const confirmRemoveProvider = useCallback(
    async (options?: { clearUsageStats?: boolean }) => {
      if (!deleteTarget || deletingRef.current) return;
      const clearUsageStats = options?.clearUsageStats === true;

      deletingRef.current = true;
      setDeleting(true);
      try {
        await providerDeleteMutation.mutateAsync({
          cliKey: deleteTarget.cli_key,
          providerId: deleteTarget.id,
          clearUsageStats,
        });

        logToConsole("info", "删除 Provider", {
          id: deleteTarget.id,
          name: deleteTarget.name,
          clear_usage_stats: clearUsageStats,
          delete_request_logs: clearUsageStats,
        });
        toast(
          clearUsageStats ? "Provider 已删除，相关请求日志和用量统计已删除" : "Provider 已删除"
        );
        setDeleteTarget(null);
      } catch (error) {
        logToConsole("error", "删除 Provider 失败", {
          error: String(error),
          id: deleteTarget.id,
        });
        toast(`删除失败：${String(error)}`);
      } finally {
        deletingRef.current = false;
        setDeleting(false);
      }
    },
    [deleteTarget, providerDeleteMutation]
  );

  function terminalLaunchCopiedToastMessage(command: string) {
    const normalized = command.trim().toLowerCase();
    if (
      normalized.startsWith("powershell ") ||
      normalized.startsWith("powershell.exe ") ||
      normalized.startsWith("pwsh ")
    ) {
      return "已复制, 请在目标文件夹 PowerShell 粘贴执行";
    }
    return "已复制, 请在目标文件夹终端粘贴执行";
  }

  const copyTerminalLaunchCommand = useCallback(
    async (provider: ProviderSummary) => {
      if (provider.cli_key !== "claude") return;
      if (
        !beginStatefulProviderAction(
          terminalCopyingByProviderIdRef,
          setTerminalCopyingByProviderId,
          provider.id
        )
      ) {
        return;
      }

      let launchCommand: string | null = null;
      try {
        try {
          launchCommand = await terminalLaunchCommandMutation.mutateAsync({
            providerId: provider.id,
          });
          if (!launchCommand) {
            toast("生成启动命令失败");
            return;
          }
        } catch (error) {
          logToConsole("error", "生成 Claude 终端启动命令失败", {
            provider_id: provider.id,
            error: String(error),
          });
          toast(`生成启动命令失败：${String(error)}`);
          return;
        }

        try {
          await copyText(launchCommand);
          toast(terminalLaunchCopiedToastMessage(launchCommand));
          logToConsole("info", "复制 Claude 终端启动命令", {
            provider_id: provider.id,
          });
        } catch (error) {
          logToConsole("error", "复制 Claude 终端启动命令失败", {
            provider_id: provider.id,
            error: String(error),
          });
          toast("复制失败：当前环境不支持剪贴板");
        }
      } finally {
        finishStatefulProviderAction(
          terminalCopyingByProviderIdRef,
          setTerminalCopyingByProviderId,
          provider.id
        );
      }
    },
    [terminalLaunchCommandMutation]
  );

  const duplicateProvider = useCallback(
    async (provider: ProviderSummary) => {
      if (
        !beginStatefulProviderAction(
          duplicatingByProviderIdRef,
          setDuplicatingByProviderId,
          provider.id
        )
      ) {
        return;
      }

      try {
        const duplicated = await providerDuplicateMutation.mutateAsync({
          providerId: provider.id,
        });
        if (!duplicated) return;

        logToConsole("info", "复制 Provider", {
          source_provider_id: provider.id,
          provider_id: duplicated.id,
          cli_key: duplicated.cli_key,
          name: duplicated.name,
        });
        toast(`已复制 Provider：${duplicated.name}`);
      } catch (error) {
        logToConsole("error", "复制 Provider 失败", {
          provider_id: provider.id,
          cli_key: provider.cli_key,
          error: String(error),
        });
        toast(`复制失败：${String(error)}`);
      } finally {
        finishStatefulProviderAction(
          duplicatingByProviderIdRef,
          setDuplicatingByProviderId,
          provider.id
        );
      }
    },
    [providerDuplicateMutation]
  );

  const testProviderAvailability = useCallback(
    async (provider: ProviderSummary) => {
      if (
        !beginStatefulProviderAction(testingByProviderIdRef, setTestingByProviderId, provider.id)
      ) {
        return;
      }

      try {
        const result = await testAvailabilityMutation.mutateAsync({
          providerId: provider.id,
        });
        if (!result) return;

        if (result.ok) {
          toast(`${provider.name}: 可用 (${result.latency_ms}ms)`);
        } else {
          toast(`${provider.name}: 不可用 — ${result.error ?? "未知错误"}`);
        }
        logToConsole("info", "供应商可用性测试", {
          provider_id: provider.id,
          ok: result.ok,
          latency_ms: result.latency_ms,
          status: result.status,
          error: result.error,
        });
      } catch (error) {
        logToConsole("error", "供应商可用性测试失败", {
          provider_id: provider.id,
          error: String(error),
        });
        toast(`测试失败：${String(error)}`);
      } finally {
        finishStatefulProviderAction(testingByProviderIdRef, setTestingByProviderId, provider.id);
      }
    },
    [testAvailabilityMutation]
  );

  async function persistProvidersOrder(
    cliKey: CliKey,
    saveToken: number,
    nextProviders: ProviderSummary[]
  ) {
    try {
      const saved = await providersReorderMutation.mutateAsync({
        cliKey,
        orderedProviderIds: nextProviders.map((provider) => provider.id),
        optimisticProviders: nextProviders,
      });
      if (!saved) return;
      if (activeCliRef.current !== cliKey) return;

      logToConsole("info", "更新 Provider 顺序", {
        cli: cliKey,
        order: saved.map((provider) => provider.id),
      });
      toast("资源池展示顺序已更新");
    } catch (error) {
      logToConsole("error", "更新 Provider 顺序失败", {
        cli: cliKey,
        error: String(error),
      });
      toast(`顺序更新失败：${String(error)}`);
    } finally {
      finishProviderReorderSave(cliKey, saveToken);
    }
  }

  function reorderProvidersByVisibility(
    event: DragEndEvent,
    isVisible: (provider: ProviderSummary) => boolean
  ) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const cliKey = activeCliRef.current;
    const previousProviders = providersRef.current;
    const nextProviders = reorderVisibleItems({
      items: previousProviders,
      activeId: active.id,
      overId: over.id,
      getId: (provider) => provider.id,
      isVisible,
    });
    if (!nextProviders) return;

    const saveToken = beginProviderReorderSave(cliKey);
    if (saveToken == null) return;

    void persistProvidersOrder(cliKey, saveToken, nextProviders);
  }

  const routeRows = routeDraftSelection.kind === "default" ? defaultRouteRows : modeProviders;
  const routeProviderIds = useMemo(() => routeRows.map((row) => row.provider_id), [routeRows]);
  const routeProviderIdSet = useMemo(() => new Set(routeProviderIds), [routeProviderIds]);
  const callableRouteCount = useMemo(
    () =>
      routeRows.filter((row) => {
        const provider = providersById[row.provider_id] ?? null;
        const rowEnabled = "enabled" in row ? row.enabled : true;
        return Boolean(provider?.enabled && rowEnabled);
      }).length,
    [providersById, routeRows]
  );
  const routeLoading =
    routeDraftSelection.kind === "default"
      ? defaultRouteQuery.isFetching
      : modeProvidersQuery.isFetching;

  function selectRouteDraft(value: string) {
    if (value === "default") {
      setRouteDraftSelection({ kind: "default", modeId: null });
      return;
    }
    const modeId = Number(value.replace("mode:", ""));
    if (!Number.isSafeInteger(modeId) || modeId <= 0) return;
    setRouteDraftSelection({ kind: "mode", modeId });
  }

  async function persistRouteRows(nextRows: Array<ProviderRouteRow | SortModeProviderRow>) {
    if (routeSavingRef.current) return;
    const selection = routeDraftSelectionRef.current;
    const cliKey = activeCliRef.current;
    routeSavingRef.current = true;
    setRouteSaving(true);

    try {
      if (selection.kind === "default") {
        await defaultRouteSetOrderMutation.mutateAsync({
          cliKey,
          orderedProviderIds: nextRows.map((row) => row.provider_id),
          optimisticRows: nextRows.map((row) => ({ provider_id: row.provider_id })),
        });
        toast("Default 调用顺序已更新");
      } else {
        const saved = await sortModeProvidersSetOrderMutation.mutateAsync({
          modeId: selection.modeId,
          cliKey,
          orderedProviderIds: nextRows.map((row) => row.provider_id),
        });
        if (
          routeDraftSelectionRef.current.kind === "mode" &&
          routeDraftSelectionRef.current.modeId === selection.modeId
        ) {
          setModeProviders(saved);
          modeProvidersRef.current = saved;
        }
        toast("模板调用顺序已更新");
      }
    } catch (error) {
      logToConsole("error", "更新调用顺序失败", {
        cli: cliKey,
        route: routeDraftKey(selection),
        error: String(error),
      });
      toast(`调用顺序更新失败：${String(error)}`);
    } finally {
      routeSavingRef.current = false;
      setRouteSaving(false);
    }
  }

  async function setModeProviderEnabled(providerId: number, enabled: boolean) {
    const selection = routeDraftSelectionRef.current;
    const cliKey = activeCliRef.current;
    if (selection.kind !== "mode" || routeSavingRef.current) return;

    const previousRows = modeProvidersRef.current;
    const currentRow = previousRows.find((row) => row.provider_id === providerId);
    if (!currentRow || currentRow.enabled === enabled) return;

    const nextRows = previousRows.map((row) =>
      row.provider_id === providerId ? { ...row, enabled } : row
    );
    routeSavingRef.current = true;
    setRouteSaving(true);
    setModeProviders(nextRows);
    modeProvidersRef.current = nextRows;

    try {
      const saved = await sortModeProviderSetEnabledMutation.mutateAsync({
        modeId: selection.modeId,
        cliKey,
        providerId,
        enabled,
      });
      if (
        routeDraftSelectionRef.current.kind === "mode" &&
        routeDraftSelectionRef.current.modeId === selection.modeId &&
        activeCliRef.current === cliKey
      ) {
        const savedRows = modeProvidersRef.current.map((row) =>
          row.provider_id === saved.provider_id ? saved : row
        );
        setModeProviders(savedRows);
        modeProvidersRef.current = savedRows;
      }
      toast(enabled ? "模板成员已启用" : "模板成员已关闭");
    } catch (error) {
      if (
        routeDraftSelectionRef.current.kind === "mode" &&
        routeDraftSelectionRef.current.modeId === selection.modeId &&
        activeCliRef.current === cliKey
      ) {
        setModeProviders(previousRows);
        modeProvidersRef.current = previousRows;
      }
      logToConsole("error", "更新模板成员状态失败", {
        cli: cliKey,
        route: routeDraftKey(selection),
        provider_id: providerId,
        enabled,
        error: String(error),
      });
      toast(`模板成员状态更新失败：${String(error)}`);
    } finally {
      routeSavingRef.current = false;
      setRouteSaving(false);
    }
  }

  function addProviderToCurrentRoute(providerId: number) {
    if (routeProviderIdSet.has(providerId)) return;
    const nextRows =
      routeDraftSelection.kind === "default"
        ? [...defaultRouteRows, { provider_id: providerId }]
        : [...modeProvidersRef.current, { provider_id: providerId, enabled: true }];
    void persistRouteRows(nextRows);
  }

  function removeProviderFromCurrentRoute(providerId: number) {
    if (!routeProviderIdSet.has(providerId)) return;
    const nextRows =
      routeDraftSelection.kind === "default"
        ? defaultRouteRows.filter((row) => row.provider_id !== providerId)
        : modeProvidersRef.current.filter((row) => row.provider_id !== providerId);
    void persistRouteRows(nextRows);
  }

  function handleRouteDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const rows =
      routeDraftSelectionRef.current.kind === "default"
        ? defaultRouteRows
        : modeProvidersRef.current;
    const oldIndex = rows.findIndex((row) => row.provider_id === active.id);
    const newIndex = rows.findIndex((row) => row.provider_id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;
    void persistRouteRows(arrayMove(rows, oldIndex, newIndex));
  }

  async function createSortMode() {
    const name = createModeName.trim();
    if (!name || createModeSaving) return;
    setCreateModeSaving(true);
    try {
      const saved = await sortModeCreateMutation.mutateAsync({ name });
      setRouteDraftSelection({ kind: "mode", modeId: saved.id });
      setCreateModeDialogOpen(false);
      toast("排序模板已创建");
    } catch (error) {
      toast(`创建失败：${String(error)}`);
    } finally {
      setCreateModeSaving(false);
    }
  }

  async function renameSortMode() {
    if (!selectedSortMode || renameModeSaving) return;
    const name = renameModeName.trim();
    if (!name) return;
    setRenameModeSaving(true);
    try {
      await sortModeRenameMutation.mutateAsync({ modeId: selectedSortMode.id, name });
      setRenameModeDialogOpen(false);
      toast("排序模板已更新");
    } catch (error) {
      toast(`重命名失败：${String(error)}`);
    } finally {
      setRenameModeSaving(false);
    }
  }

  async function deleteSortMode() {
    if (!deleteModeTarget || deleteModeDeleting) return;
    setDeleteModeDeleting(true);
    try {
      await sortModeDeleteMutation.mutateAsync({ modeId: deleteModeTarget.id });
      if (
        routeDraftSelectionRef.current.kind === "mode" &&
        routeDraftSelectionRef.current.modeId === deleteModeTarget.id
      ) {
        setRouteDraftSelection({ kind: "default", modeId: null });
      }
      setDeleteModeTarget(null);
      toast("排序模板已删除");
    } catch (error) {
      toast(`删除失败：${String(error)}`);
    } finally {
      setDeleteModeDeleting(false);
    }
  }

  async function applyCurrentRouteActive(input: {
    cliKey: CliKey;
    modeId: number | null;
    label: string;
  }) {
    if (activatingRouteRef.current) return;
    activatingRouteRef.current = true;
    setActivatingRoute(true);
    try {
      await sortModeActiveSetMutation.mutateAsync({
        cliKey: input.cliKey,
        modeId: input.modeId,
      });
      toast(input.modeId == null ? "已切回：Default" : `已激活：${input.label}`);
    } catch (error) {
      toast(`切换排序模板失败：${String(error)}`);
      logToConsole("error", "切换排序模板失败", {
        cli: input.cliKey,
        mode_id: input.modeId,
        error: String(error),
      });
    } finally {
      activatingRouteRef.current = false;
      setActivatingRoute(false);
    }
  }

  function setCurrentRouteActive() {
    const selection = routeDraftSelectionRef.current;
    const cliKey = activeCliRef.current;
    const modeId = selection.kind === "default" ? null : selection.modeId;
    if ((activeModeByCli[cliKey] ?? null) === modeId) return;

    const label = selection.kind === "default" ? "Default" : selectedRouteLabel;
    const activeSessionCount = activeSessions.filter((row) => row.cli_key === cliKey).length;
    if (activeSessionCount > 0) {
      setPendingRouteActivation({ cliKey, modeId, label, activeSessionCount });
      return;
    }

    void applyCurrentRouteActive({ cliKey, modeId, label });
  }

  function confirmPendingRouteActivation() {
    const pending = pendingRouteActivation;
    if (!pending) return;
    setPendingRouteActivation(null);
    void applyCurrentRouteActive({
      cliKey: pending.cliKey,
      modeId: pending.modeId,
      label: pending.label,
    });
  }

  function handleDragEnd(event: DragEndEvent) {
    reorderProvidersByVisibility(event, (provider) => {
      const normalizedSearch = providerSearch.trim().toLowerCase();
      const matchesTags =
        selectedTags.size === 0 || (provider.tags ?? []).some((tag) => selectedTags.has(tag));
      if (!matchesTags) return false;
      if (!normalizedSearch) return true;
      return provider.name.toLowerCase().includes(normalizedSearch);
    });
  }

  function handleProviderCardDragEnd(event: DragEndEvent) {
    const visibleProviderIds = new Set(filteredProviders.map((provider) => provider.id));
    reorderProvidersByVisibility(event, (provider) => visibleProviderIds.has(provider.id));
  }

  return {
    providers,
    codexProviders,
    providersLoading,
    providersRefreshing: Boolean(providersRefreshingByCli[activeCli]),
    defaultRouteRows,
    sortModes,
    sortModesLoading,
    sortModesAvailable,
    activeModeId,
    activeModeByCli,
    routeDraftSelection,
    selectedSortMode,
    selectedRouteLabel,
    currentRouteActive,
    activatingRoute,
    pendingRouteActivation,
    setPendingRouteActivation,
    confirmPendingRouteActivation,
    providersById,
    routeRows,
    routeProviderIdSet,
    callableRouteCount,
    routeLoading,
    routeSaving,
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
    selectRouteDraft,
    addProviderToCurrentRoute,
    removeProviderFromCurrentRoute,
    setModeProviderEnabled,
    handleRouteDragEnd,
    createSortMode,
    renameSortMode,
    deleteSortMode,
    setCurrentRouteActive,
    filteredProviders,
    tagCounts,
    selectedTags,
    setSelectedTags,
    providerSearch,
    setProviderSearch,
    circuitSummary,
    circuitLoading,
    circuitByProviderId,
    circuitResetting,
    circuitResettingAll,
    refreshProviders,
    resetCircuitAll,
    openCreateDialog,
    toggleProviderEnabled,
    resetCircuit,
    copyTerminalLaunchCommand,
    duplicateProvider,
    handleDragEnd,
    handleProviderCardDragEnd,
    sensors,
    createDialogState,
    setCreateDialogState,
    editTarget,
    setEditTarget,
    deleteTarget,
    setDeleteTarget,
    deleting,
    confirmRemoveProvider,
    sourceProviderNamesById,
    sourceProvidersById,
    terminalCopyingByProviderId,
    duplicatingByProviderId,
    testProviderAvailability,
    testingByProviderId,
  };
}
