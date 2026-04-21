// Usage: Rendered by ProvidersPage when `view === "providers"`.

import { Search } from "lucide-react";
import {
  DndContext,
  closestCenter,
} from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { ClaudeModelValidationDialog } from "../../components/ClaudeModelValidationDialog";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { EmptyState } from "../../ui/EmptyState";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { ProviderEditorDialog } from "./ProviderEditorDialog";
import { SortableProviderCard } from "./SortableProviderCard";
import { useProvidersViewDataModel } from "./hooks/useProvidersViewDataModel";

export type ProvidersViewProps = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
};

export function ProvidersView({ activeCli }: ProvidersViewProps) {
  const model = useProvidersViewDataModel(activeCli);
  const {
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
  } = model;

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
            {circuitSummary.hasUnavailable ? (
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
                          : provider.bridge_type === "cx2cc"
                            ? "当前 AIO 服务 Codex 网关"
                            : undefined
                      }
                      sourceProvider={
                        provider.source_provider_id != null
                          ? (sourceProvidersById[provider.source_provider_id] ?? null)
                          : null
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
          codexProviders={codexProviders}
          onSaved={() => {}}
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
          codexProviders={codexProviders}
          onSaved={() => {}}
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
