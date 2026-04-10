// Usage: Rendered by ProvidersPage when `view === "providers"`.

import { Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  type DragEndEvent,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { SortableContext, arrayMove, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { ClaudeModelValidationDialog } from "../../components/ClaudeModelValidationDialog";
import { logToConsole } from "../../services/consoleLog";
import { copyText } from "../../services/clipboard";
import type { GatewayProviderCircuitStatus } from "../../services/gateway/gateway";
import {
  providerGetApiKey,
  type CliKey,
  type ProviderSummary,
} from "../../services/providers/providers";
import { gatewayKeys, providersKeys } from "../../query/keys";
import {
  summarizeGatewayCircuitRows,
  useGatewayCircuitResetCliMutation,
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitStatusQuery,
} from "../../query/gateway";
import {
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderSetEnabledMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../../query/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { EmptyState } from "../../ui/EmptyState";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { ProviderEditorDialog } from "./ProviderEditorDialog";
import { SortableProviderCard } from "./SortableProviderCard";
import {
  buildDuplicatedProviderInitialValues,
  type ProviderEditorInitialValues,
} from "./providerDuplicate";

export type ProvidersViewProps = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
};

type CreateDialogState = {
  cliKey: CliKey;
  initialValues: ProviderEditorInitialValues | null;
};

export function ProvidersView({ activeCli }: ProvidersViewProps) {
  const queryClient = useQueryClient();

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
  const providersLoading = providersQuery.isFetching;
  const sourceProviderNamesById = useMemo(
    () => Object.fromEntries((codexProvidersQuery.data ?? []).map((p) => [p.id, p.name])),
    [codexProvidersQuery.data]
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

  const [circuitResetting, setCircuitResetting] = useState<Record<number, boolean>>({});
  const [circuitResettingAll, setCircuitResettingAll] = useState(false);
  const circuitAutoRefreshTimerRef = useRef<number | null>(null);

  const hasUnavailableCircuit = circuitSummary.hasUnavailable;

  const resetCircuitProviderMutation = useGatewayCircuitResetProviderMutation();
  const resetCircuitCliMutation = useGatewayCircuitResetCliMutation();
  const providerSetEnabledMutation = useProviderSetEnabledMutation();
  const providerDeleteMutation = useProviderDeleteMutation();
  const providersReorderMutation = useProvidersReorderMutation();
  const terminalLaunchCommandMutation = useProviderClaudeTerminalLaunchCommandMutation();

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

  const tagCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const p of providers) {
      for (const tag of p.tags ?? []) {
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

  // Reset selected tags when switching CLI or when tags no longer exist
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

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  useEffect(() => {
    setCircuitResetting({});
    setCircuitResettingAll(false);
    setDuplicatingByProviderId({});
  }, [activeCli]);

  function openCreateDialog(
    cliKey: CliKey,
    initialValues: ProviderEditorInitialValues | null = null
  ) {
    setCreateDialogState({ cliKey, initialValues });
  }

  useEffect(() => {
    if (circuitAutoRefreshTimerRef.current != null) {
      window.clearTimeout(circuitAutoRefreshTimerRef.current);
      circuitAutoRefreshTimerRef.current = null;
    }

    if (!hasUnavailableCircuit) return;

    const nowUnix = Math.floor(Date.now() / 1000);
    const nextAvailableUntil = circuitSummary.hasUnavailableWithoutUntil
      ? nowUnix
      : circuitSummary.earliestUnavailableUntil;
    if (nextAvailableUntil == null) return;

    const delayMs = Math.max(200, (nextAvailableUntil - nowUnix) * 1000 + 250);
    circuitAutoRefreshTimerRef.current = window.setTimeout(() => {
      circuitAutoRefreshTimerRef.current = null;
      void circuitQuery.refetch();
    }, delayMs);

    return () => {
      if (circuitAutoRefreshTimerRef.current != null) {
        window.clearTimeout(circuitAutoRefreshTimerRef.current);
        circuitAutoRefreshTimerRef.current = null;
      }
    };
  }, [
    circuitQuery,
    circuitSummary.earliestUnavailableUntil,
    circuitSummary.hasUnavailableWithoutUntil,
    hasUnavailableCircuit,
  ]);

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
      } catch (err) {
        logToConsole("error", "更新 Provider 状态失败", { error: String(err), id: provider.id });
        toast(`更新失败：${String(err)}`);
      }
    },
    [providerSetEnabledMutation]
  );

  const resetCircuit = useCallback(
    async (provider: ProviderSummary) => {
      if (circuitResetting[provider.id]) return;
      setCircuitResetting((cur) => ({ ...cur, [provider.id]: true }));

      try {
        await resetCircuitProviderMutation.mutateAsync({
          cliKey: provider.cli_key,
          providerId: provider.id,
        });

        toast("已解除熔断");
        void circuitQuery.refetch();
      } catch (err) {
        logToConsole("error", "解除熔断失败", { provider_id: provider.id, error: String(err) });
        toast(`解除熔断失败：${String(err)}`);
      } finally {
        setCircuitResetting((cur) => ({ ...cur, [provider.id]: false }));
      }
    },
    [circuitResetting, resetCircuitProviderMutation, circuitQuery]
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
      } catch (err) {
        logToConsole("error", "解除熔断（全部）失败", { cli: cliKey, error: String(err) });
        toast(`解除熔���失败：${String(err)}`);
      } finally {
        setCircuitResettingAll(false);
      }
    },
    [circuitResettingAll, resetCircuitCliMutation, circuitQuery]
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
    } catch (err) {
      logToConsole("error", "删除 Provider 失败", {
        error: String(err),
        id: deleteTarget.id,
      });
      toast(`删除失败：${String(err)}`);
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

      setTerminalCopyingByProviderId((cur) => ({ ...cur, [provider.id]: true }));

      let launchCommand: string | null = null;
      try {
        launchCommand = await terminalLaunchCommandMutation.mutateAsync({
          providerId: provider.id,
        });
        if (!launchCommand) {
          toast("生成启动命令失败");
          return;
        }
      } catch (err) {
        logToConsole("error", "生成 Claude 终端启动命令失败", {
          provider_id: provider.id,
          error: String(err),
        });
        toast(`生成启动命令失败：${String(err)}`);
        return;
      }

      try {
        await copyText(launchCommand);
        toast(terminalLaunchCopiedToastMessage(launchCommand));
        logToConsole("info", "复制 Claude 终端启动命令", {
          provider_id: provider.id,
        });
      } catch (err) {
        logToConsole("error", "复制 Claude 终端启动命令失败", {
          provider_id: provider.id,
          error: String(err),
        });
        toast("复制失败：当前环境不支持剪贴板");
      } finally {
        setTerminalCopyingByProviderId((cur) => ({ ...cur, [provider.id]: false }));
      }
    },
    [terminalCopyingByProviderId, terminalLaunchCommandMutation]
  );

  const duplicateProvider = useCallback(
    async (provider: ProviderSummary) => {
      if (duplicatingByProviderId[provider.id]) return;
      setDuplicatingByProviderId((cur) => ({ ...cur, [provider.id]: true }));

      try {
        const needsApiKeyCopy =
          provider.auth_mode === "api_key" && provider.source_provider_id == null;
        const apiKey = needsApiKeyCopy ? await providerGetApiKey(provider.id) : null;
        if (needsApiKeyCopy && (!apiKey || !apiKey.trim())) {
          toast("复制失败：原 Provider 未保存 API Key");
          return;
        }

        openCreateDialog(
          provider.cli_key,
          buildDuplicatedProviderInitialValues(provider, providersRef.current, apiKey)
        );
        logToConsole("info", "复制 Provider 配置", {
          provider_id: provider.id,
          cli_key: provider.cli_key,
        });
      } catch (err) {
        logToConsole("error", "复制 Provider 配置失败", {
          provider_id: provider.id,
          cli_key: provider.cli_key,
          error: String(err),
        });
        toast(`复制失败：${String(err)}`);
      } finally {
        setDuplicatingByProviderId((cur) => ({ ...cur, [provider.id]: false }));
      }
    },
    [duplicatingByProviderId]
  );

  async function persistProvidersOrder(
    cliKey: CliKey,
    nextProviders: ProviderSummary[],
    prevProviders: ProviderSummary[]
  ) {
    try {
      const saved = await providersReorderMutation.mutateAsync({
        cliKey,
        orderedProviderIds: nextProviders.map((p) => p.id),
      });
      if (!saved) return;

      if (activeCliRef.current !== cliKey) {
        return;
      }

      logToConsole("info", "更新 Provider 顺序", {
        cli: cliKey,
        order: saved.map((p) => p.id),
      });
      toast("顺序已更新");
    } catch (err) {
      if (activeCliRef.current === cliKey) {
        queryClient.setQueryData(providersKeys.list(cliKey), prevProviders);
      }
      logToConsole("error", "更新 Provider 顺序失败", {
        cli: cliKey,
        error: String(err),
      });
      toast(`顺序更新失败：${String(err)}`);
    }
  }

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const cliKey = activeCliRef.current;
    const prevProviders = providersRef.current;
    const oldIndex = prevProviders.findIndex((p) => p.id === active.id);
    const newIndex = prevProviders.findIndex((p) => p.id === over.id);

    if (oldIndex === -1 || newIndex === -1) return;

    const nextProviders = arrayMove(prevProviders, oldIndex, newIndex);
    queryClient.setQueryData(providersKeys.list(cliKey), nextProviders);
    void persistProvidersOrder(cliKey, nextProviders, prevProviders);
  }

  return (
    <>
      <div className="flex flex-col gap-3 lg:min-h-0 lg:flex-1">
        <div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex flex-wrap items-center gap-1.5">
            <button
              type="button"
              onClick={() => setSelectedTags(new Set())}
              className={`inline-flex h-9 items-center rounded-full border px-3.5 text-xs font-medium transition-colors ${
                selectedTags.size === 0
                  ? "border-accent bg-accent text-white shadow-sm"
                  : "border-slate-300 bg-white text-slate-600 hover:border-slate-400 hover:bg-slate-50 dark:border-slate-500 dark:bg-slate-800 dark:text-slate-300 dark:hover:border-slate-400 dark:hover:bg-slate-700"
              }`}
            >
              全部({providers.length})
            </button>
            {tagCounts.size > 0 && (
              <>
                {Array.from(tagCounts.entries()).map(([tag, count]) => {
                  const isSelected = selectedTags.has(tag);
                  return (
                    <button
                      key={tag}
                      type="button"
                      onClick={() => {
                        setSelectedTags((prev) => {
                          const next = new Set(prev);
                          if (next.has(tag)) {
                            next.delete(tag);
                          } else {
                            next.add(tag);
                          }
                          return next;
                        });
                      }}
                      className={`inline-flex h-9 items-center rounded-full border px-3.5 text-xs font-medium transition-colors ${
                        isSelected
                          ? "border-accent bg-accent text-white shadow-sm"
                          : "border-slate-300 bg-white text-slate-600 hover:border-slate-400 hover:bg-slate-50 dark:border-slate-500 dark:bg-slate-800 dark:text-slate-300 dark:hover:border-slate-400 dark:hover:bg-slate-700"
                      }`}
                    >
                      {tag}({count})
                    </button>
                  );
                })}
              </>
            )}
            <span className="text-[11px] text-slate-500 dark:text-slate-400">
              路由顺序：按拖拽顺序（上→下）
            </span>
            <span className="text-[11px] text-slate-500 dark:text-slate-400">
              共 {filteredProviders.length} / {providers.length} 条
            </span>
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2">
            {hasUnavailableCircuit ? (
              <Button
                onClick={() => void resetCircuitAll(activeCli)}
                variant="secondary"
                size="sm"
                className="h-9"
                disabled={circuitResettingAll || circuitLoading || providers.length === 0}
              >
                {circuitResettingAll
                  ? "处理中…"
                  : circuitLoading
                    ? "熔断加载中…"
                    : "解除熔断（全部）"}
              </Button>
            ) : null}

            <div className="relative w-full sm:w-72">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-400" />
              <Input
                value={providerSearch}
                onChange={(e) => setProviderSearch(e.currentTarget.value)}
                placeholder="搜索当前 CLI 下的供应商名称"
                className="h-9 pl-8 text-sm"
                aria-label="搜索供应商名称"
              />
            </div>

            <Button
              onClick={() => void refreshProviders()}
              variant="secondary"
              size="sm"
              className="h-9"
            >
              刷新
            </Button>

            <Button
              onClick={() => {
                openCreateDialog(activeCli);
              }}
              variant="secondary"
              size="sm"
              className="h-9"
            >
              添加
            </Button>
          </div>
        </div>

        <div className="lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1">
          {providersLoading ? (
            <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
              <Spinner size="sm" />
              加载中…
            </div>
          ) : providers.length === 0 ? (
            <EmptyState title="暂无 Provider" description="请点击「添加」新增。" />
          ) : filteredProviders.length === 0 ? (
            <EmptyState
              title="无匹配的 Provider"
              description={
                selectedTags.size > 0 || providerSearch.trim()
                  ? "当前名称搜索或标签筛选无结果，请调整筛选条件。"
                  : "当前列表无可展示的 Provider。"
              }
            />
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={filteredProviders.map((p) => p.id)}
                strategy={verticalListSortingStrategy}
              >
                <div className="space-y-3">
                  {filteredProviders.map((provider) => (
                    <SortableProviderCard
                      key={provider.id}
                      provider={provider}
                      sourceProviderName={
                        provider.source_provider_id != null
                          ? (sourceProviderNamesById[provider.source_provider_id] ?? null)
                          : undefined
                      }
                      circuit={circuitByProviderId[provider.id] ?? null}
                      circuitResetting={Boolean(circuitResetting[provider.id]) || circuitLoading}
                      onToggleEnabled={toggleProviderEnabled}
                      onResetCircuit={resetCircuit}
                      onCopyTerminalLaunchCommand={
                        provider.cli_key === "claude" ? copyTerminalLaunchCommand : undefined
                      }
                      terminalLaunchCopying={Boolean(terminalCopyingByProviderId[provider.id])}
                      onValidateModel={
                        activeCli === "claude" ? requestValidateProviderModel : undefined
                      }
                      onDuplicate={duplicateProvider}
                      duplicateLoading={Boolean(duplicatingByProviderId[provider.id])}
                      onEdit={setEditTarget}
                      onDelete={setDeleteTarget}
                    />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          )}
        </div>
      </div>

      <ClaudeModelValidationDialog
        open={validateDialogOpen}
        onOpenChange={(open) => {
          setValidateDialogOpen(open);
          if (!open) setValidateProvider(null);
        }}
        provider={validateProvider}
      />

      {createDialogState ? (
        <ProviderEditorDialog
          mode="create"
          open={true}
          onOpenChange={(nextOpen) => {
            if (!nextOpen) setCreateDialogState(null);
          }}
          cliKey={createDialogState.cliKey}
          initialValues={createDialogState.initialValues}
          codexProviders={codexProvidersQuery.data ?? []}
          onSaved={(cliKey) => {
            queryClient.invalidateQueries({ queryKey: providersKeys.list(cliKey) });
            queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(cliKey) });
          }}
        />
      ) : null}

      {editTarget ? (
        <ProviderEditorDialog
          mode="edit"
          open={true}
          onOpenChange={(nextOpen) => {
            if (!nextOpen) setEditTarget(null);
          }}
          provider={editTarget}
          codexProviders={codexProvidersQuery.data ?? []}
          onSaved={(cliKey) => {
            queryClient.invalidateQueries({ queryKey: providersKeys.list(cliKey) });
            queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(cliKey) });
          }}
        />
      ) : null}

      <Dialog
        open={!!deleteTarget}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && deleting) return;
          if (!nextOpen) setDeleteTarget(null);
        }}
        title="确认删除 Provider"
        description={deleteTarget ? `将删除：${deleteTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button onClick={() => setDeleteTarget(null)} variant="secondary" disabled={deleting}>
            取消
          </Button>
          <Button onClick={confirmRemoveProvider} variant="primary" disabled={deleting}>
            {deleting ? "删除中…" : "确认删除"}
          </Button>
        </div>
      </Dialog>
    </>
  );
}
