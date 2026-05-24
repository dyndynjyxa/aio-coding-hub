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
import { logToConsole } from "../../../services/consoleLog";
import { copyText } from "../../../services/clipboard";
import type { GatewayProviderCircuitStatus } from "../../../services/gateway/gateway";
import { type CliKey, type ProviderSummary } from "../../../services/providers/providers";
import {
  summarizeGatewayCircuitRows,
  useGatewayCircuitAutoRefresh,
  useGatewayCircuitResetCliMutation,
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitStatusQuery,
} from "../../../query/gateway";
import {
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderDuplicateMutation,
  useProviderSetEnabledMutation,
  useProviderTestAvailabilityMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../../../query/providers";
import type { ProviderEditorInitialValues } from "../providerDuplicate";
import { reorderVisibleItems } from "../reorderVisibleItems";

type CreateDialogState = {
  cliKey: CliKey;
  initialValues: ProviderEditorInitialValues | null;
};

type ProviderRefreshResult = { error: unknown | null };
type ProviderActionMap = Record<number, boolean>;
type ProviderActionMapSetter = Dispatch<SetStateAction<ProviderActionMap>>;

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

  const circuitQuery = useGatewayCircuitStatusQuery(activeCli);
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
  const [validateDialogOpen, setValidateDialogOpen] = useState(false);
  const [validateProvider, setValidateProvider] = useState<ProviderSummary | null>(null);
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
  const terminalLaunchCommandMutation = useProviderClaudeTerminalLaunchCommandMutation();
  const testAvailabilityMutation = useProviderTestAvailabilityMutation();

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
  }, [activeCli]);

  useEffect(() => {
    if (activeCli !== "claude" && validateDialogOpen) {
      setValidateDialogOpen(false);
      setValidateProvider(null);
    }
  }, [activeCli, validateDialogOpen]);

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

  const requestValidateProviderModel = useCallback((provider: ProviderSummary) => {
    if (activeCliRef.current !== "claude") return;
    setValidateProvider(provider);
    setValidateDialogOpen(true);
  }, []);

  const confirmRemoveProvider = useCallback(async () => {
    if (!deleteTarget || deletingRef.current) return;

    deletingRef.current = true;
    setDeleting(true);
    try {
      await providerDeleteMutation.mutateAsync({
        cliKey: deleteTarget.cli_key,
        providerId: deleteTarget.id,
      });

      logToConsole("info", "删除 Provider", {
        id: deleteTarget.id,
        name: deleteTarget.name,
      });
      toast("Provider 已删除");
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
  }, [deleteTarget, providerDeleteMutation]);

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
      toast("顺序已更新");
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

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const cliKey = activeCliRef.current;
    const previousProviders = providersRef.current;
    const nextProviders = reorderVisibleItems({
      items: previousProviders,
      activeId: active.id,
      overId: over.id,
      getId: (provider) => provider.id,
      isVisible: (provider) => provider.enabled,
    });
    if (!nextProviders) return;

    const saveToken = beginProviderReorderSave(cliKey);
    if (saveToken == null) return;

    void persistProvidersOrder(cliKey, saveToken, nextProviders);
  }

  return {
    providers,
    codexProviders,
    providersLoading,
    providersRefreshing: Boolean(providersRefreshingByCli[activeCli]),
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
    requestValidateProviderModel,
    handleDragEnd,
    sensors,
    createDialogState,
    setCreateDialogState,
    editTarget,
    setEditTarget,
    deleteTarget,
    setDeleteTarget,
    deleting,
    confirmRemoveProvider,
    validateDialogOpen,
    setValidateDialogOpen,
    validateProvider,
    setValidateProvider,
    sourceProviderNamesById,
    sourceProvidersById,
    terminalCopyingByProviderId,
    duplicatingByProviderId,
    testProviderAvailability,
    testingByProviderId,
  };
}
