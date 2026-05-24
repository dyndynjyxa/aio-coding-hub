// Usage: Rendered by ProvidersPage when `view === "sortModes"`.

import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { Plus } from "lucide-react";
import { CLIS } from "../../constants/clis";
import type { CliKey, ProviderSummary } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { providerBaseUrlSummary } from "./baseUrl";
import { ProviderOrderItem, SortableProviderOrderItem } from "./SortableProviderOrderItem";
import { useSortModesDataModel } from "./useSortModesDataModel";

export type SortModesViewProps = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
  providers: ProviderSummary[];
  providersLoading: boolean;
};

export function SortModesView({
  activeCli,
  setActiveCli,
  providers,
  providersLoading,
}: SortModesViewProps) {
  const model = useSortModesDataModel({
    activeCli,
    setActiveCli,
    providers,
    providersLoading,
  });

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );
  const defaultEnabledProviders = model.providers.filter((provider) => provider.enabled);
  const enabledModeRows = model.modeProviders.filter((row) => row.enabled);
  const orderCountLabel =
    model.activeModeId == null
      ? String(defaultEnabledProviders.length)
      : `${enabledModeRows.length}/${model.modeProviders.length}`;

  return (
    <>
      <div className="flex flex-col gap-4 lg:min-h-0 lg:flex-1">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={() => model.selectEditingMode(null)}
              variant={model.activeModeId == null ? "primary" : "secondary"}
              size="sm"
            >
              Default
            </Button>
            {model.sortModes.map((mode) => (
              <Button
                key={mode.id}
                onClick={() => model.selectEditingMode(mode.id)}
                variant={model.activeModeId === mode.id ? "primary" : "secondary"}
                size="sm"
              >
                {mode.name}
              </Button>
            ))}
            <span className="text-xs text-muted-foreground">
              {model.sortModesLoading ? "加载中…" : `共 ${model.sortModes.length + 1} 个`}
            </span>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={() => void model.refreshSortModes()}
              variant="secondary"
              size="sm"
              disabled={model.sortModesLoading}
            >
              刷新
            </Button>
            <Button onClick={() => model.setCreateModeDialogOpen(true)} variant="primary" size="sm">
              新建排序模板
            </Button>
            {model.selectedMode ? (
              <>
                <Button
                  onClick={() => model.setRenameModeDialogOpen(true)}
                  variant="secondary"
                  size="sm"
                >
                  重命名
                </Button>
                <Button
                  onClick={() => model.setDeleteModeTarget(model.selectedMode)}
                  variant="secondary"
                  size="sm"
                  className="hover:!bg-rose-50 hover:!text-rose-600"
                >
                  删除
                </Button>
              </>
            ) : null}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          {CLIS.map((cli) => (
            <Button
              key={cli.key}
              onClick={() => model.setActiveCli(cli.key)}
              variant={model.activeCli === cli.key ? "primary" : "secondary"}
              size="sm"
            >
              {cli.name}
            </Button>
          ))}
          <span className="text-xs text-muted-foreground">选择要配置的 CLI</span>
        </div>

        <div className="grid gap-4 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_420px] xl:grid-cols-[minmax(0,1fr)_480px]">
          <section className="flex flex-col rounded-lg border border-border bg-card p-3 lg:min-h-0">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <div className="text-sm font-semibold text-foreground">
                  供应商 · {model.currentCli.name}
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  左侧管理模板成员；右侧调整调用顺序和启用状态。
                </div>
              </div>
            </div>

            <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay">
              {model.providersLoading ? (
                <div className="text-sm text-muted-foreground">加载中…</div>
              ) : model.providers.length === 0 ? (
                <div className="text-sm text-muted-foreground">
                  暂无 Provider。请先在「供应商」视图添加。
                </div>
              ) : (
                <div className="space-y-2">
                  {model.providers.map((provider) => {
                    const modeSelected = model.activeModeId != null;
                    const modeUnavailable = model.modeProvidersAvailable === false;
                    const modeRow =
                      model.modeProviders.find((row) => row.provider_id === provider.id) ?? null;
                    const modeDisabled =
                      !modeSelected ||
                      modeUnavailable ||
                      model.modeProvidersLoading ||
                      model.modeProvidersSaving;
                    const inMode = Boolean(modeSelected && !modeUnavailable && modeRow);
                    const buttonTitle = !modeSelected
                      ? "请选择一个自定义排序模板后再加入"
                      : model.modeProvidersLoading
                        ? "右侧列表加载中…"
                        : undefined;

                    return (
                      <div
                        key={provider.id}
                        className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2.5 shadow-sm"
                      >
                        <div className="min-w-0">
                          <div className="flex min-w-0 items-center gap-2">
                            <div className="truncate text-sm font-semibold text-foreground">
                              {provider.name}
                            </div>
                            {!provider.enabled ? (
                              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
                                Default 关闭
                              </span>
                            ) : null}
                            {modeRow ? (
                              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
                                {modeRow.enabled ? "模板启用" : "模板关闭"}
                              </span>
                            ) : null}
                          </div>
                          <div className="truncate text-xs text-muted-foreground">
                            {providerBaseUrlSummary(provider)}
                          </div>
                        </div>
                        {modeSelected ? (
                          <div className="flex shrink-0 items-center gap-2">
                            {inMode ? (
                              <Button
                                onClick={() => model.removeProviderFromMode(provider.id)}
                                variant="secondary"
                                size="sm"
                                className="hover:!bg-rose-50 hover:!text-rose-600"
                                disabled={modeDisabled}
                              >
                                移除
                              </Button>
                            ) : (
                              <Button
                                onClick={() => model.addProviderToMode(provider.id)}
                                variant="primary"
                                size="sm"
                                className="font-semibold shadow-sm"
                                disabled={modeDisabled}
                                title={buttonTitle}
                              >
                                <Plus className="h-3.5 w-3.5" />
                                加入
                              </Button>
                            )}
                          </div>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </section>

          <aside
            aria-label="排序模板调用顺序"
            className="flex flex-col rounded-lg border border-border bg-card p-3 lg:min-h-0"
          >
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <div className="text-sm font-semibold text-foreground">
                  调用顺序 · {model.selectedMode ? model.selectedMode.name : "Default"}
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  {model.activeModeId == null
                    ? "调用顺序按照从上到下依次调用；Default 顺序请在「供应商」视图调整。"
                    : "开启项按照从上到下依次调用；关闭项不参与调用。"}
                </div>
              </div>
              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
                {orderCountLabel}
              </span>
            </div>

            <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay">
              {model.activeModeId == null ? (
                defaultEnabledProviders.length === 0 ? (
                  <div className="text-sm text-muted-foreground">
                    Default 当前没有已启用的 Provider。
                  </div>
                ) : (
                  <div className="space-y-2">
                    {defaultEnabledProviders.map((provider, index) => (
                      <ProviderOrderItem key={provider.id} provider={provider} index={index} />
                    ))}
                  </div>
                )
              ) : model.modeProvidersLoading ? (
                <div className="text-sm text-muted-foreground">加载中…</div>
              ) : model.modeProviders.length === 0 ? (
                <div className="space-y-2">
                  <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                    当前排序模板在 {model.currentCli.name} 下未配置 Provider；若激活将导致无可用
                    Provider。
                  </div>
                  <div className="text-sm text-muted-foreground">
                    请从左侧供应商列表点击「加入」。
                  </div>
                </div>
              ) : (
                <div className="space-y-2">
                  {enabledModeRows.length === 0 ? (
                    <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                      当前排序模板没有已启用 Provider；若激活将导致无可用 Provider。
                    </div>
                  ) : null}
                  <DndContext
                    sensors={sensors}
                    collisionDetection={closestCenter}
                    onDragEnd={model.handleModeDragEnd}
                  >
                    <SortableContext
                      items={model.modeProviders.map((row) => row.provider_id)}
                      strategy={verticalListSortingStrategy}
                    >
                      <div className="space-y-2">
                        {model.modeProviders.map((row, index) => {
                          const provider = model.providersById[row.provider_id] ?? null;
                          const providerLabel = provider?.name?.trim()
                            ? provider.name
                            : `Provider #${row.provider_id}`;

                          return (
                            <SortableProviderOrderItem
                              key={row.provider_id}
                              provider={provider}
                              providerId={row.provider_id}
                              index={index}
                              disabled={model.modeProvidersSaving}
                              showProviderDisabledBadge={false}
                              trailing={
                                <>
                                  <div
                                    className="flex shrink-0 items-center gap-1.5"
                                    onPointerDown={(event) => event.stopPropagation()}
                                  >
                                    <span className="text-[11px] text-muted-foreground">启用</span>
                                    <Switch
                                      checked={row.enabled}
                                      onCheckedChange={(checked) =>
                                        void model.setModeProviderEnabled(row.provider_id, checked)
                                      }
                                      size="sm"
                                      disabled={model.modeProvidersSaving}
                                      aria-label={`启用 ${providerLabel}`}
                                    />
                                  </div>
                                  <Button
                                    onClick={() => model.removeProviderFromMode(row.provider_id)}
                                    onPointerDown={(event) => event.stopPropagation()}
                                    variant="secondary"
                                    size="sm"
                                    className="h-7 px-2 text-[11px] hover:!bg-rose-50 hover:!text-rose-600"
                                    disabled={model.modeProvidersSaving}
                                  >
                                    移除
                                  </Button>
                                </>
                              }
                            />
                          );
                        })}
                      </div>
                    </SortableContext>
                  </DndContext>
                </div>
              )}
            </div>
          </aside>
        </div>
      </div>

      <Dialog
        open={model.createModeDialogOpen}
        onOpenChange={(open) => model.setCreateModeDialogOpen(open)}
        title="新建排序模板"
        description="Default 为系统内置模板；自定义排序模板用于保存可切换的 Provider 路由顺序副本（不改默认顺序）。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <FormField label="名称" hint="例如：工作 / 生活">
            <Input
              value={model.createModeName}
              onChange={(event) => model.setCreateModeName(event.currentTarget.value)}
              placeholder="工作"
            />
          </FormField>

          <div className="flex items-center justify-end gap-2 border-t border-border pt-3 dark:border-border">
            <Button
              onClick={() => model.setCreateModeDialogOpen(false)}
              variant="secondary"
              disabled={model.createModeSaving}
            >
              取消
            </Button>
            <Button
              onClick={() => void model.createSortMode()}
              variant="primary"
              disabled={model.createModeSaving}
            >
              {model.createModeSaving ? "创建中…" : "创建"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={model.renameModeDialogOpen}
        onOpenChange={(open) => model.setRenameModeDialogOpen(open)}
        title={model.selectedMode ? `重命名排序模板：${model.selectedMode.name}` : "重命名排序模板"}
        description="仅支持重命名自定义排序模板；Default 为系统内置模板。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <FormField label="名称">
            <Input
              value={model.renameModeName}
              onChange={(event) => model.setRenameModeName(event.currentTarget.value)}
            />
          </FormField>

          <div className="flex items-center justify-end gap-2 border-t border-border pt-3 dark:border-border">
            <Button
              onClick={() => model.setRenameModeDialogOpen(false)}
              variant="secondary"
              disabled={model.renameModeSaving}
            >
              取消
            </Button>
            <Button
              onClick={() => void model.renameSortMode()}
              variant="primary"
              disabled={model.renameModeSaving || !model.selectedMode}
            >
              {model.renameModeSaving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={!!model.deleteModeTarget}
        onOpenChange={(open) => {
          if (!open && model.deleteModeDeleting) return;
          if (!open) model.setDeleteModeTarget(null);
        }}
        title="确认删除排序模板"
        description={model.deleteModeTarget ? `将删除：${model.deleteModeTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            onClick={() => model.setDeleteModeTarget(null)}
            variant="secondary"
            disabled={model.deleteModeDeleting}
          >
            取消
          </Button>
          <Button
            onClick={() => void model.deleteSortMode()}
            variant="primary"
            disabled={model.deleteModeDeleting}
          >
            {model.deleteModeDeleting ? "删除中…" : "确认删除"}
          </Button>
        </div>
      </Dialog>
    </>
  );
}
