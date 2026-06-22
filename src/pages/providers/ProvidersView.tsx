// Usage: Rendered by ProvidersPage when `view === "providers"`.

import { useEffect, useRef, useState } from "react";
import { Search } from "lucide-react";
import { DndContext, closestCenter } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CLIS } from "../../constants/clis";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { EmptyState } from "../../ui/EmptyState";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { ProviderEditorDialog } from "./ProviderEditorDialog";
import { SortableProviderCard } from "./SortableProviderCard";
import { SortableProviderOrderItem } from "./SortableProviderOrderItem";
import { useProvidersViewDataModel } from "./hooks/useProvidersViewDataModel";

export type ProvidersViewProps = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
};

type PendingProvidersScrollRestore = {
  cliKey: CliKey;
  scrollTop: number;
  observedRefresh: boolean;
};

function getRouteRowEnabled(row: unknown) {
  if (!row || typeof row !== "object" || !("enabled" in row)) return true;
  return typeof row.enabled === "boolean" ? row.enabled : true;
}

export function ProvidersView({ activeCli }: ProvidersViewProps) {
  const model = useProvidersViewDataModel(activeCli);
  const {
    providers,
    codexProviders,
    providersLoading,
    providersRefreshing,
    sortModes,
    sortModesLoading,
    activeModeId,
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
  } = model;
  const providersListScrollRef = useRef<HTMLDivElement | null>(null);
  const pendingProvidersScrollRestoreRef = useRef<PendingProvidersScrollRestore | null>(null);
  const routeDraftValue =
    routeDraftSelection.kind === "default" ? "default" : `mode:${routeDraftSelection.modeId}`;
  const selectedCliName = CLIS.find((cli) => cli.key === activeCli)?.name ?? activeCli;
  const [clearUsageStatsOnDelete, setClearUsageStatsOnDelete] = useState(false);

  useEffect(() => {
    const pendingRestore = pendingProvidersScrollRestoreRef.current;
    if (!pendingRestore) return;

    if (pendingRestore.cliKey !== activeCli) {
      pendingProvidersScrollRestoreRef.current = null;
      return;
    }

    if (providersLoading) {
      pendingProvidersScrollRestoreRef.current = {
        ...pendingRestore,
        observedRefresh: true,
      };
      return;
    }

    // 等待保存后的刷新确实开始并结束，再恢复位置，避免过早清掉待恢复记录。
    if (!pendingRestore.observedRefresh) return;

    const providersListElement = providersListScrollRef.current;
    if (!providersListElement) return;

    providersListElement.scrollTop = pendingRestore.scrollTop;
    pendingProvidersScrollRestoreRef.current = null;
  }, [activeCli, providersLoading, providers.length, filteredProviders.length]);

  function captureProvidersListScrollPosition(cliKey: CliKey) {
    const providersListElement = providersListScrollRef.current;
    if (!providersListElement) return;

    // 保存前先记录滚动位置，便于编辑成功后的后台刷新完成后恢复原视口。
    pendingProvidersScrollRestoreRef.current = {
      cliKey,
      scrollTop: providersListElement.scrollTop,
      observedRefresh: false,
    };
  }

  function openDeleteDialog(provider: (typeof providers)[number]) {
    setClearUsageStatsOnDelete(false);
    setDeleteTarget(provider);
  }

  function closeDeleteDialog() {
    setClearUsageStatsOnDelete(false);
    setDeleteTarget(null);
  }

  return (
    <>
      <div className="flex flex-col gap-4 lg:min-h-0 lg:flex-1">
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
            <span className="text-[11px] text-muted-foreground">资源池展示顺序：左侧拖拽保存</span>
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

        <div className="grid gap-4 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_360px] xl:grid-cols-[minmax(0,1fr)_400px]">
          <div
            ref={providersListScrollRef}
            className="lg:min-h-0 lg:overflow-auto lg:pr-1 scrollbar-overlay"
          >
            {providersLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Spinner size="sm" />
                加载中…
              </div>
            ) : providers.length === 0 ? (
              <EmptyState title="暂无供应商" description="请点击「添加」新增。" />
            ) : filteredProviders.length === 0 ? (
              <EmptyState
                title="无匹配的供应商"
                description={
                  selectedTags.size > 0 || providerSearch.trim()
                    ? "当前名称搜索或标签筛选无结果，请调整筛选条件。"
                    : "当前列表无可展示的供应商。"
                }
              />
            ) : (
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleProviderCardDragEnd}
              >
                <SortableContext
                  items={filteredProviders.map((provider) => provider.id)}
                  strategy={verticalListSortingStrategy}
                >
                  <div className="space-y-3">
                    {filteredProviders.map((provider) => {
                      const joined = routeProviderIdSet.has(provider.id);
                      return (
                        <SortableProviderCard
                          key={provider.id}
                          provider={provider}
                          trailing={
                            joined ? (
                              <Button
                                variant="secondary"
                                size="sm"
                                className="px-2 py-1 text-[11px]"
                                disabled
                              >
                                已加入
                              </Button>
                            ) : (
                              <Button
                                variant="primary"
                                size="sm"
                                className="px-2 py-1 text-[11px]"
                                disabled={routeSaving}
                                onClick={() => addProviderToCurrentRoute(provider.id)}
                              >
                                加入
                              </Button>
                            )
                          }
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
                          circuitResetting={
                            Boolean(circuitResetting[provider.id]) || circuitLoading
                          }
                          onToggleEnabled={toggleProviderEnabled}
                          onResetCircuit={resetCircuit}
                          onCopyTerminalLaunchCommand={
                            provider.cli_key === "claude" ? copyTerminalLaunchCommand : undefined
                          }
                          terminalLaunchCopying={Boolean(terminalCopyingByProviderId[provider.id])}
                          onTestAvailability={testProviderAvailability}
                          testAvailabilityLoading={Boolean(testingByProviderId[provider.id])}
                          onDuplicate={duplicateProvider}
                          duplicateLoading={Boolean(duplicatingByProviderId[provider.id])}
                          onEdit={setEditTarget}
                          onDelete={openDeleteDialog}
                        />
                      );
                    })}
                  </div>
                </SortableContext>
              </DndContext>
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
                    {selectedRouteLabel} 按照从上到下依次调用
                  </div>
                </div>
                <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
                  {callableRouteCount}/{routeRows.length}
                </span>
              </div>

              <div className="mt-3 space-y-2">
                <Select
                  value={routeDraftValue}
                  onChange={(event) => selectRouteDraft(event.currentTarget.value)}
                  disabled={sortModesLoading || routeSaving}
                  aria-label="选择调用顺序"
                  className="h-9"
                >
                  <option value="default">Default{activeModeId == null ? "（当前）" : ""}</option>
                  {sortModes.map((mode) => (
                    <option key={mode.id} value={`mode:${mode.id}`}>
                      {mode.name}
                      {activeModeId === mode.id ? "（当前）" : ""}
                    </option>
                  ))}
                </Select>
                <div className="flex flex-wrap items-center gap-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <Button
                      variant="secondary"
                      size="sm"
                      className="h-8"
                      onClick={() => setCreateModeDialogOpen(true)}
                    >
                      新建模板
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      className="h-8"
                      disabled={!selectedSortMode}
                      onClick={() => setRenameModeDialogOpen(true)}
                    >
                      重命名
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      className="h-8"
                      disabled={!selectedSortMode}
                      onClick={() => {
                        if (selectedSortMode) setDeleteModeTarget(selectedSortMode);
                      }}
                    >
                      删除
                    </Button>
                  </div>
                  <Button
                    variant={currentRouteActive ? "secondary" : "primary"}
                    size="sm"
                    className="ml-auto h-8"
                    disabled={currentRouteActive || activatingRoute || sortModesLoading}
                    onClick={setCurrentRouteActive}
                    title={currentRouteActive ? "当前路由策略" : "设为当前路由策略"}
                  >
                    {activatingRoute ? "切换中…" : "设为当前路由策略"}
                  </Button>
                </div>
              </div>

              <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay">
                {routeLoading ? (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Spinner size="sm" />
                    加载调用顺序…
                  </div>
                ) : routeRows.length === 0 ? (
                  <div className="rounded-md border border-dashed border-border px-3 py-4 text-sm text-muted-foreground">
                    当前方案没有供应商，请从左侧资源池加入。
                  </div>
                ) : (
                  <DndContext
                    sensors={sensors}
                    collisionDetection={closestCenter}
                    onDragEnd={handleRouteDragEnd}
                  >
                    <SortableContext
                      items={routeRows.map((row) => row.provider_id)}
                      strategy={verticalListSortingStrategy}
                    >
                      <div className="space-y-2">
                        {routeRows.map((row, index) => {
                          const provider = providersById[row.provider_id] ?? null;
                          const providerLabel = provider?.name?.trim()
                            ? provider.name
                            : `未知 Provider #${provider?.id ?? row.provider_id}`;
                          const routeRowEnabled = getRouteRowEnabled(row);
                          const showModeProviderSwitch = routeDraftSelection.kind === "mode";
                          return (
                            <SortableProviderOrderItem
                              key={row.provider_id}
                              provider={provider}
                              providerId={row.provider_id}
                              index={index}
                              disabled={routeSaving}
                              trailing={
                                <div className="flex shrink-0 items-center gap-2">
                                  {showModeProviderSwitch ? (
                                    <div
                                      className="flex shrink-0 items-center gap-1.5"
                                      onPointerDown={(event) => event.stopPropagation()}
                                    >
                                      <span className="text-[11px] text-muted-foreground">
                                        {routeRowEnabled ? "启用" : "关闭"}
                                      </span>
                                      <Switch
                                        checked={routeRowEnabled}
                                        onCheckedChange={(checked) =>
                                          void setModeProviderEnabled(row.provider_id, checked)
                                        }
                                        size="sm"
                                        disabled={routeSaving}
                                        aria-label={`${providerLabel} 在模板中启用`}
                                      />
                                    </div>
                                  ) : null}
                                  <Button
                                    variant="secondary"
                                    size="sm"
                                    className="h-7 px-2 text-xs"
                                    disabled={routeSaving}
                                    onPointerDown={(event) => event.stopPropagation()}
                                    onClick={() => removeProviderFromCurrentRoute(row.provider_id)}
                                  >
                                    移出
                                  </Button>
                                </div>
                              }
                            />
                          );
                        })}
                      </div>
                    </SortableContext>
                  </DndContext>
                )}
              </div>
            </aside>
          ) : null}
        </div>
      </div>

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
          onSaved={(cliKey) => {
            captureProvidersListScrollPosition(cliKey);
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
          codexProviders={codexProviders}
          onSaved={(cliKey) => {
            captureProvidersListScrollPosition(cliKey);
          }}
        />
      ) : null}

      <Dialog
        open={!!deleteTarget}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && deleting) return;
          if (!nextOpen) closeDeleteDialog();
        }}
        title="确认删除 Provider"
        description={deleteTarget ? `将删除：${deleteTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="space-y-3">
          <label className="flex items-start gap-2 rounded-lg border border-border bg-muted p-3">
            <input
              type="checkbox"
              aria-label="同时删除该 Provider 的用量统计和请求日志"
              checked={clearUsageStatsOnDelete}
              onChange={(event) => setClearUsageStatsOnDelete(event.currentTarget.checked)}
              disabled={deleting}
              className="mt-0.5 h-4 w-4 shrink-0 rounded border-border bg-background text-primary accent-primary focus:ring-ring"
            />
            <span className="min-w-0">
              <span className="block text-sm font-medium text-foreground">
                同时删除该 Provider 的用量统计和请求日志
              </span>
              <span className="mt-1 block text-xs text-muted-foreground">
                删除后该 Provider 的历史请求日志和用量统计都将移除。
              </span>
            </span>
          </label>
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button onClick={closeDeleteDialog} variant="secondary" disabled={deleting}>
              取消
            </Button>
            <Button
              onClick={() =>
                void confirmRemoveProvider({ clearUsageStats: clearUsageStatsOnDelete })
              }
              variant="primary"
              disabled={deleting}
            >
              {deleting ? "删除中…" : "确认删除"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={createModeDialogOpen}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && createModeSaving) return;
          setCreateModeDialogOpen(nextOpen);
        }}
        title="新建排序模板"
        description={`为 ${selectedCliName} 新建一个可编辑调用顺序。`}
        className="max-w-lg"
      >
        <div className="space-y-4">
          <Input
            value={createModeName}
            onChange={(event) => setCreateModeName(event.currentTarget.value)}
            placeholder="模板名称"
            aria-label="模板名称"
            maxLength={32}
          />
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button
              onClick={() => setCreateModeDialogOpen(false)}
              variant="secondary"
              disabled={createModeSaving}
            >
              取消
            </Button>
            <Button onClick={createSortMode} variant="primary" disabled={createModeSaving}>
              {createModeSaving ? "创建中…" : "创建"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={renameModeDialogOpen}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && renameModeSaving) return;
          setRenameModeDialogOpen(nextOpen);
        }}
        title="重命名排序模板"
        description={selectedSortMode ? `当前模板：${selectedSortMode.name}` : undefined}
        className="max-w-lg"
      >
        <div className="space-y-4">
          <Input
            value={renameModeName}
            onChange={(event) => setRenameModeName(event.currentTarget.value)}
            placeholder="模板名称"
            aria-label="模板名称"
            maxLength={32}
          />
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button
              onClick={() => setRenameModeDialogOpen(false)}
              variant="secondary"
              disabled={renameModeSaving}
            >
              取消
            </Button>
            <Button onClick={renameSortMode} variant="primary" disabled={renameModeSaving}>
              {renameModeSaving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={!!deleteModeTarget}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && deleteModeDeleting) return;
          if (!nextOpen) setDeleteModeTarget(null);
        }}
        title="确认删除排序模板"
        description={deleteModeTarget ? `将删除：${deleteModeTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            onClick={() => setDeleteModeTarget(null)}
            variant="secondary"
            disabled={deleteModeDeleting}
          >
            取消
          </Button>
          <Button onClick={deleteSortMode} variant="primary" disabled={deleteModeDeleting}>
            {deleteModeDeleting ? "删除中…" : "确认删除"}
          </Button>
        </div>
      </Dialog>

      <Dialog
        open={pendingRouteActivation != null}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) setPendingRouteActivation(null);
        }}
        title={`确认切换 ${selectedCliName} 路由策略？`}
        description={
          pendingRouteActivation
            ? `目前还有 ${pendingRouteActivation.activeSessionCount} 个活跃 Session，切换策略可能导致会话中断，是否确认？`
            : undefined
        }
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            onClick={() => setPendingRouteActivation(null)}
            variant="secondary"
            disabled={activatingRoute}
          >
            取消
          </Button>
          <Button
            onClick={confirmPendingRouteActivation}
            variant="primary"
            disabled={activatingRoute}
          >
            {activatingRoute ? "切换中…" : "确认切换"}
          </Button>
        </div>
      </Dialog>
    </>
  );
}
