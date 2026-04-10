// Usage: Rendered by ProvidersPage when `view === "sortModes"`.

import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CLIS } from "../../constants/clis";
import type { CliKey, ProviderSummary } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { providerBaseUrlSummary } from "./baseUrl";
import { SortableModeProviderRow } from "./SortableModeProviderRow";
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
            <span className="text-xs text-slate-500 dark:text-slate-400">
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
          <span className="text-xs text-slate-500 dark:text-slate-400">选择要配置的 CLI</span>
        </div>

        <div className="grid gap-4 lg:min-h-0 lg:flex-1 lg:grid-cols-2">
          <Card padding="sm" className="flex flex-col lg:min-h-0">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <div className="text-sm font-semibold">默认顺序 · {model.currentCli.name}</div>
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  默认顺序来自「供应商」视图拖拽（基础顺序）；Default
                  路由仍受「供应商」启用开关影响。
                </div>
              </div>
            </div>

            <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1">
              {model.providersLoading ? (
                <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
              ) : model.providers.length === 0 ? (
                <div className="text-sm text-slate-600 dark:text-slate-400">
                  暂无 Provider。请先在「供应商」视图添加。
                </div>
              ) : (
                <div className="space-y-2">
                  {model.providers.map((provider) => {
                    const modeSelected = model.activeModeId != null;
                    const modeUnavailable = model.modeProvidersAvailable === false;
                    const modeDisabled =
                      !modeSelected ||
                      modeUnavailable ||
                      model.modeProvidersLoading ||
                      model.modeProvidersSaving;
                    const inMode =
                      modeSelected && !modeUnavailable && model.modeProviderIdSet.has(provider.id);
                    const buttonText = inMode
                      ? "已加入"
                      : model.modeProvidersLoading
                        ? "加载中…"
                        : "加入";
                    const buttonTitle = !modeSelected
                      ? "请选择一个自定义排序模板后再加入"
                      : model.modeProvidersLoading
                        ? "右侧列表加载中…"
                        : undefined;

                    return (
                      <Card
                        key={provider.id}
                        padding="sm"
                        className="flex items-center justify-between gap-3 shadow-none"
                      >
                        <div className="min-w-0">
                          <div className="flex min-w-0 items-center gap-2">
                            <div className="truncate text-sm font-semibold">{provider.name}</div>
                            {!provider.enabled ? (
                              <span className="shrink-0 rounded-full bg-slate-100 px-2 py-0.5 font-mono text-[10px] text-slate-600 dark:bg-slate-700 dark:text-slate-400">
                                Default 关闭
                              </span>
                            ) : null}
                          </div>
                          <div className="truncate text-xs text-slate-500 dark:text-slate-400">
                            {providerBaseUrlSummary(provider)}
                          </div>
                        </div>
                        <Button
                          onClick={() => model.addProviderToMode(provider.id)}
                          variant="secondary"
                          size="sm"
                          disabled={modeDisabled || inMode}
                          title={buttonTitle}
                        >
                          {buttonText}
                        </Button>
                      </Card>
                    );
                  })}
                </div>
              )}
            </div>
          </Card>

          <Card padding="sm" className="flex flex-col lg:min-h-0">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <div className="text-sm font-semibold">
                  编辑模板：{model.selectedMode ? model.selectedMode.name : "未选择"} ·{" "}
                  {model.currentCli.name}
                </div>
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  {model.activeModeId == null
                    ? "请选择一个自定义排序模板进行编辑；Default 的顺序请在「供应商」视图调整。"
                    : "严格子集：激活后仅使用该列表中「已启用」的 Provider 参与路由（不受「供应商」启用开关影响）。"}
                </div>
              </div>
            </div>

            <div className="mt-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1">
              {model.activeModeId == null ? (
                <div className="text-sm text-slate-600 dark:text-slate-400">
                  请选择一个自定义排序模板进行编辑。
                </div>
              ) : model.modeProvidersLoading ? (
                <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
              ) : model.modeProviders.length === 0 ? (
                <div className="space-y-2">
                  <div className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                    当前排序模板在 {model.currentCli.name} 下未配置 Provider；若激活将导致无可用
                    Provider。
                  </div>
                  <div className="text-sm text-slate-600 dark:text-slate-400">
                    请从左侧「默认顺序」列表点击「加入」。
                  </div>
                </div>
              ) : (
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
                      {model.modeProviders.map((row) => (
                        <SortableModeProviderRow
                          key={row.provider_id}
                          providerId={row.provider_id}
                          provider={model.providersById[row.provider_id] ?? null}
                          modeEnabled={row.enabled}
                          disabled={model.modeProvidersSaving}
                          onToggleEnabled={model.setModeProviderEnabled}
                          onRemove={model.removeProviderFromMode}
                        />
                      ))}
                    </div>
                  </SortableContext>
                </DndContext>
              )}
            </div>
          </Card>
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

          <div className="flex items-center justify-end gap-2 border-t border-slate-100 pt-3 dark:border-slate-700">
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

          <div className="flex items-center justify-end gap-2 border-t border-slate-100 pt-3 dark:border-slate-700">
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
