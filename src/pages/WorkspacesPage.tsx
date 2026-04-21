// Usage: Workspaces configuration center (profiles). All edits are scoped to selected workspace; only active workspace triggers real sync.

import { ArrowRightLeft, Layers, Pencil, Plus, Search, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { cliLongLabel } from "../constants/clis";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { EmptyState } from "../ui/EmptyState";
import { FormField } from "../ui/FormField";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Spinner } from "../ui/Spinner";
import { TabList } from "../ui/TabList";
import { cn } from "../utils/cn";
import { McpServersView } from "./mcp/McpServersView";
import { PromptsView } from "./prompts/PromptsView";
import { SkillsView } from "./skills/SkillsView";
import {
  WORKSPACES_RIGHT_TABS,
  useWorkspacesPageDataModel,
} from "./workspaces/useWorkspacesPageDataModel";

function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "active";
}) {
  const toneClass =
    tone === "active"
      ? "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
      : "border-slate-200 bg-white text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-medium",
        toneClass
      )}
    >
      {children}
    </span>
  );
}

export function WorkspacesPage() {
  const model = useWorkspacesPageDataModel();
  const {
    orderedCliTabs,
    effectiveCli,
    setActiveCli,
    items,
    loading,
    selectedWorkspaceId,
    setSelectedWorkspaceId,
    filterText,
    setFilterText,
    filtered,
    activeWorkspaceId,
    selectedWorkspace,
    rightTab,
    setRightTab,
    overviewLoading,
    overviewStats,
    applyReport,
    applying,
    rollbackToPrevious,
    openCreateDialog,
    openRenameDialog,
    openDeleteDialog,
    openSwitchDialog,
    createOpen,
    setCreateOpen,
    createName,
    setCreateName,
    createMode,
    setCreateMode,
    createError,
    createWorkspace,
    renameOpen,
    setRenameOpen,
    renameTarget,
    renameName,
    setRenameName,
    renameError,
    renameWorkspace,
    setRenameTargetId,
    deleteOpen,
    setDeleteOpen,
    deleteTarget,
    deleteWorkspace,
    setDeleteTargetId,
    switchOpen,
    setSwitchOpen,
    setSwitchTargetId,
    switchTarget,
    switchConfirm,
    setSwitchConfirm,
    previewQuery,
    preview,
    previewLoading,
    workspaceById,
    applySwitchTarget,
  } = model;

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title="工作区"
        actions={
          <>
            <TabList
              ariaLabel="目标 CLI"
              items={orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name }))}
              value={effectiveCli}
              onChange={setActiveCli}
            />
          </>
        }
      />

      <div className="grid gap-4 lg:flex-1 lg:min-h-0 lg:grid-cols-[360px_1fr] lg:items-stretch lg:overflow-hidden">
        <Card padding="sm" className="flex flex-col lg:min-h-0">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center justify-between gap-3">
                <div className="flex min-w-0 items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
                  <Layers className="h-4 w-4 shrink-0 text-accent" />
                  <span className="shrink-0">工作区</span>
                  <span className="shrink-0 text-xs font-medium text-slate-500 dark:text-slate-400">
                    {items.length} 个
                  </span>
                </div>
                <Button variant="primary" onClick={openCreateDialog}>
                  <Plus className="h-4 w-4" />
                  新建
                </Button>
              </div>
              <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                同一 CLI 下名称不可重复。
              </div>
            </div>
          </div>

          <div className="mt-3">
            <div className="relative">
              <div className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-slate-400">
                <Search className="h-4 w-4" aria-hidden="true" />
              </div>
              <Input
                value={filterText}
                onChange={(e) => setFilterText(e.currentTarget.value)}
                placeholder="搜索"
                className="pl-9"
                aria-label="搜索工作区"
              />
            </div>
          </div>

          <div className="mt-3 space-y-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1">
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400 px-1">
                <Spinner size="sm" />
                加载中…
              </div>
            ) : filtered.length === 0 ? (
              <EmptyState title="暂无工作区" variant="dashed" />
            ) : (
              filtered.map((workspace) => {
                const isActive = workspace.id === activeWorkspaceId;
                const isSelected = workspace.id === selectedWorkspaceId;

                return (
                  <div
                    key={workspace.id}
                    className={cn(
                      "rounded-2xl border p-4 transition",
                      isActive
                        ? "border-accent/30 bg-accent/[0.03] shadow-sm dark:border-accent/40 dark:bg-accent/10"
                        : isSelected
                          ? "border-slate-300 bg-slate-50 dark:border-slate-600 dark:bg-slate-700"
                          : "border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700"
                    )}
                    aria-current={isActive ? "true" : undefined}
                    role="button"
                    tabIndex={0}
                    onClick={() => setSelectedWorkspaceId(workspace.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") setSelectedWorkspaceId(workspace.id);
                    }}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <div className="truncate text-sm font-semibold text-slate-900 dark:text-slate-100">
                            {workspace.name}
                          </div>
                          {isActive ? (
                            <Badge tone="active">当前</Badge>
                          ) : (
                            <Badge tone="neutral">可用</Badge>
                          )}
                        </div>
                      </div>

                      <div className="flex shrink-0 items-center gap-1.5">
                        {isActive ? null : (
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedWorkspaceId(workspace.id);
                              openSwitchDialog(workspace.id);
                            }}
                            className="h-8"
                            title="对比当前工作区与目标工作区的差异，并确认切换"
                          >
                            <ArrowRightLeft className="h-4 w-4" />
                            切换…
                          </Button>
                        )}
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label="重命名"
                          title="重命名"
                          onClick={(e) => {
                            e.stopPropagation();
                            openRenameDialog(workspace);
                          }}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          size="icon"
                          variant="danger"
                          aria-label="删除"
                          title={isActive ? "请先切换当前工作区再删除" : "删除"}
                          disabled={isActive}
                          onClick={(e) => {
                            e.stopPropagation();
                            openDeleteDialog(workspace);
                          }}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </Card>

        <div className="flex flex-col gap-4 lg:min-h-0 lg:flex-1 lg:overflow-hidden">
          {selectedWorkspace ? (
            <Card padding="md" className="lg:min-h-0 lg:flex lg:flex-1 lg:flex-col">
              <div className="shrink-0 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <div className="truncate text-lg font-semibold text-slate-900 dark:text-slate-100">
                      {selectedWorkspace.name}
                    </div>
                    {selectedWorkspace.id === activeWorkspaceId ? (
                      <Badge tone="active">当前</Badge>
                    ) : (
                      <Badge tone="neutral">非当前</Badge>
                    )}
                    <Badge tone="neutral">{cliLongLabel(selectedWorkspace.cli_key)}</Badge>
                  </div>
                </div>

                <TabList
                  ariaLabel="配置分类"
                  items={WORKSPACES_RIGHT_TABS}
                  value={rightTab}
                  onChange={setRightTab}
                  className="w-full sm:w-auto"
                />
              </div>

              <div
                className={cn(
                  "mt-4 min-h-0 flex-1",
                  rightTab === "skills"
                    ? "lg:overflow-hidden"
                    : "lg:overflow-y-auto custom-scrollbar lg:pr-1"
                )}
              >
                {rightTab === "overview" ? (
                  <div className="space-y-4">
                    {selectedWorkspace.id === activeWorkspaceId ? (
                      <div className="rounded-xl border border-emerald-200 dark:border-emerald-700 bg-emerald-50 dark:bg-emerald-900/30 px-3 py-2 text-sm text-emerald-900 dark:text-emerald-400">
                        <div className="font-medium">当前工作区</div>
                        <div className="mt-1 text-xs text-emerald-900/80 dark:text-emerald-400/80">
                          对 Prompts/MCP/Skills 的修改会立即生效。
                        </div>
                      </div>
                    ) : (
                      <div className="rounded-xl border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-3 py-2 text-sm text-amber-900 dark:text-amber-400">
                        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                          <div>
                            <div className="font-medium text-slate-900 dark:text-slate-100">
                              该工作区尚未生效
                            </div>
                            <div className="mt-1 text-xs text-amber-900/80 dark:text-amber-400/80">
                              修改会先保存，切换后才会写入对应 CLI 配置（仅 AIO 托管部分）。
                            </div>
                          </div>
                          <Button
                            size="sm"
                            variant="primary"
                            disabled={
                              !selectedWorkspace || selectedWorkspace.id === activeWorkspaceId
                            }
                            onClick={() => openSwitchDialog(selectedWorkspace.id)}
                          >
                            <ArrowRightLeft className="h-4 w-4" />
                            切换…
                          </Button>
                        </div>
                      </div>
                    )}

                    {applyReport && applyReport.to_workspace_id === selectedWorkspace.id ? (
                      <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-300">
                        已切换为当前工作区（
                        {new Date(applyReport.applied_at * 1000).toLocaleString()}）
                        {applyReport.from_workspace_id ? (
                          <Button
                            size="sm"
                            variant="secondary"
                            className="ml-2"
                            disabled={applying}
                            onClick={() => void rollbackToPrevious()}
                          >
                            回滚到上一个
                          </Button>
                        ) : null}
                      </div>
                    ) : null}

                    <div className="grid gap-3 sm:grid-cols-3">
                      <Card padding="sm">
                        <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                          Prompts
                        </div>
                        <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                          {overviewLoading ? (
                            "加载中…"
                          ) : overviewStats ? (
                            <>
                              已启用 {overviewStats.prompts.enabled} / 共{" "}
                              {overviewStats.prompts.total}
                            </>
                          ) : (
                            "—"
                          )}
                        </div>
                        <div className="mt-3">
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => setRightTab("prompts")}
                          >
                            去配置
                          </Button>
                        </div>
                      </Card>

                      <Card padding="sm">
                        <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                          MCP
                        </div>
                        <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                          {overviewLoading ? (
                            "加载中…"
                          ) : overviewStats ? (
                            <>
                              已启用 {overviewStats.mcp.enabled} / 共 {overviewStats.mcp.total}
                            </>
                          ) : (
                            "—"
                          )}
                        </div>
                        <div className="mt-3">
                          <Button size="sm" variant="secondary" onClick={() => setRightTab("mcp")}>
                            去配置
                          </Button>
                        </div>
                      </Card>

                      <Card padding="sm">
                        <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                          Skills
                        </div>
                        <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                          {overviewLoading ? (
                            "加载中…"
                          ) : overviewStats ? (
                            <>
                              已启用 {overviewStats.skills.enabled} / 共{" "}
                              {overviewStats.skills.total}
                            </>
                          ) : (
                            "—"
                          )}
                        </div>
                        <div className="mt-3">
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => setRightTab("skills")}
                          >
                            去配置
                          </Button>
                        </div>
                      </Card>
                    </div>
                  </div>
                ) : rightTab === "prompts" ? (
                  <PromptsView
                    workspaceId={selectedWorkspace.id}
                    cliKey={selectedWorkspace.cli_key}
                    isActiveWorkspace={selectedWorkspace.id === activeWorkspaceId}
                  />
                ) : rightTab === "mcp" ? (
                  <>
                    {selectedWorkspace.id === activeWorkspaceId ? null : (
                      <div className="mb-3 rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-300">
                        非当前工作区：启用/停用仅写入数据库，不会同步到 CLI。
                      </div>
                    )}
                    <McpServersView workspaceId={selectedWorkspace.id} />
                  </>
                ) : (
                  <SkillsView
                    workspaceId={selectedWorkspace.id}
                    cliKey={selectedWorkspace.cli_key}
                    isActiveWorkspace={selectedWorkspace.id === activeWorkspaceId}
                  />
                )}
              </div>
            </Card>
          ) : (
            <EmptyState title="请选择一个工作区" variant="dashed" />
          )}
        </div>
      </div>

      <Dialog
        open={switchOpen}
        onOpenChange={(open) => {
          setSwitchOpen(open);
          if (!open) {
            setSwitchConfirm("");
            setSwitchTargetId(null);
          }
        }}
        title="对比并切换"
        description={
          switchTarget ? `将切换为当前：${switchTarget.name}` : "对比当前工作区与目标工作区的差异"
        }
        className="max-w-3xl"
      >
        <div className="space-y-3">
          <Card padding="sm">
            <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
              对比范围
            </div>
            <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
              当前：
              {(() => {
                const fromId = preview?.from_workspace_id ?? activeWorkspaceId;
                if (!fromId) return "（未设置）";
                return workspaceById.get(fromId)?.name ?? `#${fromId}`;
              })()}
              <span className="mx-2 text-slate-400 dark:text-slate-500">→</span>
              目标：{switchTarget?.name ?? "—"}
            </div>
            <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
              仅展示 Prompts/MCP/Skills 的差异。确认无误后再切换为当前。
            </div>
          </Card>

          {previewLoading ? (
            <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
              <Spinner size="sm" />
              生成对比中…
            </div>
          ) : !preview ? (
            <div className="text-sm text-slate-600 dark:text-slate-400">暂无对比数据。</div>
          ) : (
            <div className="space-y-3">
              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                  Prompts
                </div>
                <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                  {preview.prompts.will_change ? (
                    <span className="rounded-full bg-amber-50 dark:bg-amber-900/30 px-2 py-0.5 text-[11px] font-medium text-amber-800 dark:text-amber-400">
                      将变更
                    </span>
                  ) : (
                    <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:text-slate-400">
                      不变
                    </span>
                  )}
                </div>
                <div className="mt-2 grid gap-3 sm:grid-cols-2">
                  <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                    <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                      当前
                    </div>
                    <div className="mt-1 text-sm font-semibold text-slate-900 dark:text-slate-100">
                      {preview.prompts.from_enabled?.name ?? "（未启用）"}
                    </div>
                    <div className="mt-1 text-xs text-slate-600 dark:text-slate-400">
                      {preview.prompts.from_enabled?.excerpt ?? "—"}
                    </div>
                  </div>
                  <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                    <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                      目标
                    </div>
                    <div className="mt-1 text-sm font-semibold text-slate-900 dark:text-slate-100">
                      {preview.prompts.to_enabled?.name ?? "（未启用）"}
                    </div>
                    <div className="mt-1 text-xs text-slate-600 dark:text-slate-400">
                      {preview.prompts.to_enabled?.excerpt ?? "—"}
                    </div>
                  </div>
                </div>
              </Card>

              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                  MCP
                </div>
                <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                  +{preview.mcp.added.length} / -{preview.mcp.removed.length}
                </div>
                {preview.mcp.added.length || preview.mcp.removed.length ? (
                  <div className="mt-2 grid gap-3 sm:grid-cols-2">
                    <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                      <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                        新增
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {preview.mcp.added.map((k) => (
                          <span
                            key={k}
                            className="rounded-full bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400"
                          >
                            {k}
                          </span>
                        ))}
                      </div>
                    </div>
                    <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                      <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                        移除
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {preview.mcp.removed.map((k) => (
                          <span
                            key={k}
                            className="rounded-full bg-rose-50 dark:bg-rose-900/30 px-2 py-0.5 text-[11px] font-medium text-rose-700 dark:text-rose-400"
                          >
                            {k}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="mt-2 text-xs text-slate-500 dark:text-slate-400">无变化</div>
                )}
              </Card>

              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
                  Skills
                </div>
                <div className="mt-2 text-sm text-slate-700 dark:text-slate-300">
                  +{preview.skills.added.length} / -{preview.skills.removed.length}
                </div>
                {preview.skills.added.length || preview.skills.removed.length ? (
                  <div className="mt-2 grid gap-3 sm:grid-cols-2">
                    <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                      <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                        新增
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {preview.skills.added.map((k) => (
                          <span
                            key={k}
                            className="rounded-full bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400"
                          >
                            {k}
                          </span>
                        ))}
                      </div>
                    </div>
                    <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3">
                      <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                        移除
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {preview.skills.removed.map((k) => (
                          <span
                            key={k}
                            className="rounded-full bg-rose-50 dark:bg-rose-900/30 px-2 py-0.5 text-[11px] font-medium text-rose-700 dark:text-rose-400"
                          >
                            {k}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="mt-2 text-xs text-slate-500 dark:text-slate-400">无变化</div>
                )}
              </Card>
            </div>
          )}

          <FormField label="输入 APPLY 以确认切换">
            <Input
              value={switchConfirm}
              onChange={(e) => setSwitchConfirm(e.currentTarget.value)}
            />
          </FormField>

          <div className="flex flex-wrap items-center justify-between gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button
              variant="secondary"
              onClick={() => void previewQuery.refetch()}
              disabled={previewLoading}
            >
              刷新对比
            </Button>
            <div className="flex items-center gap-2">
              <Button onClick={() => setSwitchOpen(false)} variant="secondary">
                取消
              </Button>
              <Button
                onClick={() => void applySwitchTarget()}
                variant="primary"
                disabled={
                  !switchTarget ||
                  switchTarget.id === activeWorkspaceId ||
                  switchConfirm.trim().toUpperCase() !== "APPLY" ||
                  applying
                }
              >
                {applying ? "切换中…" : "确认切换"}
              </Button>
            </div>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={createOpen}
        onOpenChange={(open) => setCreateOpen(open)}
        title={`新建工作区（${cliLongLabel(effectiveCli)}）`}
        description="默认空白创建：Prompt/MCP/Skills 均为未启用状态。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <FormField label="名称">
            <Input value={createName} onChange={(e) => setCreateName(e.currentTarget.value)} />
          </FormField>

          <FormField label="创建方式">
            <div className="grid gap-2">
              <label className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
                <input
                  type="radio"
                  name="create-mode"
                  checked={createMode === "clone_active"}
                  onChange={() => setCreateMode("clone_active")}
                />
                从当前工作区克隆
              </label>
              <label className="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
                <input
                  type="radio"
                  name="create-mode"
                  checked={createMode === "blank"}
                  onChange={() => setCreateMode("blank")}
                />
                空白创建（推荐）
              </label>
            </div>
          </FormField>

          {createError ? (
            <div className="rounded-xl border border-rose-200 dark:border-rose-700 bg-rose-50 dark:bg-rose-900/30 px-3 py-2 text-sm text-rose-800 dark:text-rose-400">
              {createError}
            </div>
          ) : null}

          <div className="flex items-center justify-end gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button onClick={() => setCreateOpen(false)} variant="secondary">
              取消
            </Button>
            <Button
              onClick={() => void createWorkspace()}
              variant="primary"
              disabled={!!createError}
            >
              创建
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={renameOpen}
        onOpenChange={(open) => {
          setRenameOpen(open);
          if (!open) setRenameTargetId(null);
        }}
        title={renameTarget ? `重命名：${renameTarget.name}` : "重命名工作区"}
        description="名称在同一 CLI 下必须唯一。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <FormField label="名称">
            <Input value={renameName} onChange={(e) => setRenameName(e.currentTarget.value)} />
          </FormField>

          {renameError ? (
            <div className="rounded-xl border border-rose-200 dark:border-rose-700 bg-rose-50 dark:bg-rose-900/30 px-3 py-2 text-sm text-rose-800 dark:text-rose-400">
              {renameError}
            </div>
          ) : null}

          <div className="flex items-center justify-end gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button onClick={() => setRenameOpen(false)} variant="secondary">
              取消
            </Button>
            <Button
              onClick={() => void renameWorkspace()}
              variant="primary"
              disabled={!!renameError || !renameTarget}
            >
              保存
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={deleteOpen}
        onOpenChange={(open) => {
          setDeleteOpen(open);
          if (!open) setDeleteTargetId(null);
        }}
        title="确认删除工作区"
        description={deleteTarget ? `将删除：${deleteTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="rounded-xl border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-3 py-2 text-sm text-amber-900 dark:text-amber-400">
            删除会移除此工作区下的 Prompts/MCP/Skills 配置（DB）。不会删除任何未托管的 CLI 文件。
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button onClick={() => setDeleteOpen(false)} variant="secondary">
              取消
            </Button>
            <Button
              onClick={() => void deleteWorkspace()}
              variant="danger"
              disabled={!deleteTarget}
            >
              确认删除
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
