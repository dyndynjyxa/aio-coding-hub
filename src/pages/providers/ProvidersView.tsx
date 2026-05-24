// Usage: Rendered by ProvidersPage when `view === "providers"`.

import { Search } from "lucide-react";
import { DndContext, closestCenter } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { ClaudeModelValidationDialog } from "../../components/ClaudeModelValidationDialog";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { EmptyState } from "../../ui/EmptyState";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { ProviderEditorDialog } from "./ProviderEditorDialog";
import { ProviderCard } from "./SortableProviderCard";
import { SortableProviderOrderItem } from "./SortableProviderOrderItem";
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
    providersRefreshing,
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
  } = model;
  const enabledProviders = providers.filter((provider) => provider.enabled);

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
                  : "border-border bg-white text-muted-foreground hover:border-border hover:bg-secondary dark:border-border dark:bg-secondary dark:text-secondary-foreground dark:hover:border-border dark:hover:bg-secondary"
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
                          : "border-border bg-white text-muted-foreground hover:border-border hover:bg-secondary dark:border-border dark:bg-secondary dark:text-secondary-foreground dark:hover:border-border dark:hover:bg-secondary"
                      }`}
                    >
                      {tag}({count})
                    </button>
                  );
                })}
              </>
            )}
            <span className="text-[11px] text-muted-foreground">路由顺序：右侧拖拽（上→下）</span>
            <span className="text-[11px] text-muted-foreground">
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
              <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
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
              disabled={providersRefreshing}
            >
              {providersRefreshing ? "刷新中…" : "刷新"}
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

        <div className="grid gap-4 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_280px] xl:grid-cols-[minmax(0,1fr)_320px]">
          <div className="lg:min-h-0 lg:overflow-auto lg:pr-1 scrollbar-overlay">
            {providersLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
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
              <div className="space-y-3">
                {filteredProviders.map((provider) => (
                  <ProviderCard
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
                    onTestAvailability={testProviderAvailability}
                    testAvailabilityLoading={Boolean(testingByProviderId[provider.id])}
                    onDuplicate={duplicateProvider}
                    duplicateLoading={Boolean(duplicatingByProviderId[provider.id])}
                    onEdit={setEditTarget}
                    onDelete={setDeleteTarget}
                  />
                ))}
              </div>
            )}
          </div>

          {providers.length > 0 ? (
            <aside
              aria-label="供应商调用顺序"
              className="flex flex-col rounded-lg border border-border bg-card p-3 lg:min-h-0"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <div className="text-sm font-semibold text-foreground">调用顺序</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    调用顺序按照从上到下依次调用
                  </div>
                </div>
                <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
                  {enabledProviders.length}
                </span>
              </div>

              <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay">
                {enabledProviders.length === 0 ? (
                  <div className="text-sm text-muted-foreground">当前没有已启用的 Provider。</div>
                ) : (
                  <DndContext
                    sensors={sensors}
                    collisionDetection={closestCenter}
                    onDragEnd={handleDragEnd}
                  >
                    <SortableContext
                      items={enabledProviders.map((p) => p.id)}
                      strategy={verticalListSortingStrategy}
                    >
                      <div className="space-y-2">
                        {enabledProviders.map((provider, index) => (
                          <SortableProviderOrderItem
                            key={provider.id}
                            provider={provider}
                            index={index}
                          />
                        ))}
                      </div>
                    </SortableContext>
                  </DndContext>
                )}
              </div>
            </aside>
          ) : null}
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
