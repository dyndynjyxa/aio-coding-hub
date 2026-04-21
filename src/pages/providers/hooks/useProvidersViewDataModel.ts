// Usage: Data-model hook for ProvidersView orchestration.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import {
  PointerSensor,
  type DragEndEvent,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { arrayMove } from "@dnd-kit/sortable";
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
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../../../query/providers";
import type { ProviderEditorInitialValues } from "../providerDuplicate";

type CreateDialogState = {
  cliKey: CliKey;
  initialValues: ProviderEditorInitialValues | null;
};

export function useProvidersViewDataModel(activeCli: CliKey) {
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
  const [circuitResettingAll, setCircuitResettingAll] = useState(false);
  const [createDialogState, setCreateDialogState] = useState<CreateDialogState | null>(null);
  const [editTarget, setEditTarget] = useState<ProviderSummary | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProviderSummary | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [terminalCopyingByProviderId, setTerminalCopyingByProviderId] = useState<
    Record<number, boolean>
  >({});
  const [duplicatingByProviderId, setDuplicatingByProviderId] = useState<Record<number, boolean>>(
    {}
  );
  const [validateDialogOpen, setValidateDialogOpen] = useState(false);
  const [validateProvider, setValidateProvider] = useState<ProviderSummary | null>(null);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [providerSearch, setProviderSearch] = useState("");

  const resetCircuitProviderMutation = useGatewayCircuitResetProviderMutation();
  const resetCircuitCliMutation = useGatewayCircuitResetCliMutation();
  const providerSetEnabledMutation = useProviderSetEnabledMutation();
  const providerDeleteMutation = useProviderDeleteMutation();
  const providerDuplicateMutation = useProviderDuplicateMutation();
  const providersReorderMutation = useProvidersReorderMutation();
  const terminalLaunchCommandMutation = useProviderClaudeTerminalLaunchCommandMutation();

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

  const refreshProviders = useCallback(async () => {
    const refreshes: Array<Promise<{ error: unknown | null }>> = [providersQuery.refetch()];
    if (activeCli === "claude") {
      refreshes.push(codexProvidersQuery.refetch());
    }

    const results = await Promise.all(refreshes);
    if (results.some((result) => result.error != null)) {
      toast("刷新供应商列表失败：请查看控制台日志");
    }
  }, [activeCli, codexProvidersQuery, providersQuery]);

  useEffect(() => {
    setSelectedTags(new Set());
    setProviderSearch("");
  }, [activeCli]);

  useEffect(() => {
    if (activeCli !== "claude" && validateDialogOpen) {
      setValidateDialogOpen(false);
      setValidateProvider(null);
    }
  }, [activeCli, validateDialogOpen]);

  useEffect(() => {
    setCircuitResetting({});
    setCircuitResettingAll(false);
    setDuplicatingByProviderId({});
  }, [activeCli]);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  function openCreateDialog(
    cliKey: CliKey,
    initialValues: ProviderEditorInitialValues | null = null
  ) {
    setCreateDialogState({ cliKey, initialValues });
  }

  const toggleProviderEnabled = useCallback(
    async (provider: ProviderSummary) => {
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
      }
    },
    [providerSetEnabledMutation]
  );

  const resetCircuit = useCallback(
    async (provider: ProviderSummary) => {
      if (circuitResetting[provider.id]) return;

      setCircuitResetting((current) => ({ ...current, [provider.id]: true }));
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
        setCircuitResetting((current) => ({ ...current, [provider.id]: false }));
      }
    },
    [circuitQuery, circuitResetting, resetCircuitProviderMutation]
  );

  const resetCircuitAll = useCallback(
    async (cliKey: CliKey) => {
      if (circuitResettingAll) return;

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
        setCircuitResettingAll(false);
      }
    },
    [circuitQuery, circuitResettingAll, resetCircuitCliMutation]
  );

  const requestValidateProviderModel = useCallback((provider: ProviderSummary) => {
    if (activeCliRef.current !== "claude") return;
    setValidateProvider(provider);
    setValidateDialogOpen(true);
  }, []);

  const confirmRemoveProvider = useCallback(async () => {
    if (!deleteTarget || deleting) return;

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
      setDeleting(false);
    }
  }, [deleteTarget, deleting, providerDeleteMutation]);

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
      if (terminalCopyingByProviderId[provider.id]) return;

      setTerminalCopyingByProviderId((current) => ({ ...current, [provider.id]: true }));

      let launchCommand: string | null = null;
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
      } finally {
        setTerminalCopyingByProviderId((current) => ({ ...current, [provider.id]: false }));
      }
    },
    [terminalCopyingByProviderId, terminalLaunchCommandMutation]
  );

  const duplicateProvider = useCallback(
    async (provider: ProviderSummary) => {
      if (duplicatingByProviderId[provider.id]) return;

      setDuplicatingByProviderId((current) => ({ ...current, [provider.id]: true }));
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
        setDuplicatingByProviderId((current) => ({ ...current, [provider.id]: false }));
      }
    },
    [duplicatingByProviderId, providerDuplicateMutation]
  );

  async function persistProvidersOrder(cliKey: CliKey, nextProviders: ProviderSummary[]) {
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
    }
  }

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const cliKey = activeCliRef.current;
    const previousProviders = providersRef.current;
    const oldIndex = previousProviders.findIndex((provider) => provider.id === active.id);
    const newIndex = previousProviders.findIndex((provider) => provider.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const nextProviders = arrayMove(previousProviders, oldIndex, newIndex);
    void persistProvidersOrder(cliKey, nextProviders);
  }

  return {
    providers,
    codexProviders,
    providersLoading,
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
  };
}
