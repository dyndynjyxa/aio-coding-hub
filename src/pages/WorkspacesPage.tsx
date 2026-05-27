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

type NoticeTone = "success" | "warning" | "danger" | "neutral";

const NOTICE_TONE_CLASS: Record<NoticeTone, string> = {
  success:
    "border-emerald-200 bg-emerald-50 text-emerald-900 dark:border-emerald-800 dark:bg-emerald-900/20 dark:text-emerald-300",
  warning:
    "border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-300",
  danger:
    "border-rose-200 bg-rose-50 text-rose-800 dark:border-rose-800 dark:bg-rose-900/20 dark:text-rose-300",
  neutral: "border-line-subtle bg-surface-inset text-secondary-foreground",
};

const WORKSPACE_ITEM_BASE_CLASS =
  "rounded-2xl border p-4 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/35";

const DIFF_BOX_CLASS = "rounded-xl border border-line-subtle bg-surface-inset p-3";

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
      : "border-line-subtle bg-surface-inset text-muted-foreground";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-bold shrink-0 whitespace-nowrap",
        toneClass
      )}
    >
      {children}
    </span>
  );
}

function Notice({
  children,
  tone,
  className,
}: {
  children: ReactNode;
  tone: NoticeTone;
  className?: string;
}) {
  return (
    <div className={cn("rounded-xl border px-3 py-2 text-sm", NOTICE_TONE_CLASS[tone], className)}>
      {children}
    </div>
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
          <div className="flex items-center justify-between gap-3 w-full pb-3 border-b border-line-subtle">
            <div className="flex items-center gap-2 text-sm font-bold text-foreground">
              <Layers className="h-4 w-4 shrink-0 text-accent" />
              <span>工作区</span>
              <span className="text-xs font-medium text-muted-foreground">({items.length} 个)</span>
            </div>
            <Button
              variant="primary"
              size="sm"
              onClick={openCreateDialog}
              className="h-8 gap-1.5 px-3 font-semibold rounded-lg shadow-sm"
            >
              <Plus className="h-4 w-4" />
              新建
            </Button>
          </div>

          <div className="mt-3">
            <div className="relative">
              <div className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-muted-foreground">
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

          <div className="mt-3 space-y-3 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay">
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground px-1">
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
                      WORKSPACE_ITEM_BASE_CLASS,
                      isActive
                        ? "border-state-selected-border bg-state-selected shadow-[0_1px_2px_rgba(15,23,42,0.04)]"
                        : isSelected
                          ? "border-line bg-surface-inset"
                          : "border-line-subtle bg-surface-panel hover:border-line hover:bg-state-hover"
                    )}
                    aria-current={isActive ? "true" : undefined}
                    role="button"
                    tabIndex={0}
                    onClick={() => setSelectedWorkspaceId(workspace.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") setSelectedWorkspaceId(workspace.id);
                    }}
                  >
                    <div className="flex flex-col gap-3">
                      {/* Top Row: Name and Status Badge */}
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex items-center gap-2 min-w-0 flex-1">
                          <Layers className="h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
                          <div className="truncate text-sm font-bold text-foreground">
                            {workspace.name}
                          </div>
                        </div>
                        {isActive ? (
                          <Badge tone="active">当前</Badge>
                        ) : (
                          <Badge tone="neutral">可用</Badge>
                        )}
                      </div>

                      {/* Bottom Row: Action bar */}
                      <div className="flex items-center justify-between border-t border-line-subtle/50 pt-2.5 mt-0.5">
                        <div className="flex items-center">
                          {isActive ? (
                            <div className="flex items-center gap-1.5 text-[11px] font-semibold text-emerald-600 dark:text-emerald-400">
                              <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 shadow-[0_0_4px] shadow-emerald-500/70" />
                              <span>当前网关已启用</span>
                            </div>
                          ) : (
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={(e) => {
                                e.stopPropagation();
                                setSelectedWorkspaceId(workspace.id);
                                openSwitchDialog(workspace.id);
                              }}
                              className="h-7 px-2 text-[11px] font-bold gap-1 rounded-md"
                              title="对比当前工作区与目标工作区的差异，并确认切换"
                            >
                              <ArrowRightLeft className="h-3 w-3" />
                              <span>对比切换</span>
                            </Button>
                          )}
                        </div>

                        <div className="flex items-center gap-1">
                          <Button
                            size="icon"
                            variant="ghost"
                            className="h-7 w-7 text-muted-foreground/70 hover:text-foreground hover:bg-state-hover rounded-md"
                            aria-label="重命名"
                            title="重命名"
                            onClick={(e) => {
                              e.stopPropagation();
                              openRenameDialog(workspace);
                            }}
                          >
                            <Pencil className="h-3.5 w-3.5" />
                          </Button>
                          <Button
                            size="icon"
                            variant="ghost"
                            className="h-7 w-7 text-muted-foreground/45 hover:text-destructive hover:bg-destructive/10 disabled:opacity-20 disabled:hover:bg-transparent rounded-md"
                            aria-label="删除"
                            title={isActive ? "请先切换当前工作区再删除" : "删除"}
                            disabled={isActive}
                            onClick={(e) => {
                              e.stopPropagation();
                              openDeleteDialog(workspace);
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
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
                    <div className="truncate text-lg font-semibold text-foreground">
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
                    : "lg:overflow-y-auto scrollbar-overlay lg:pr-1"
                )}
              >
                {rightTab === "overview" ? (
                  <div className="space-y-4">
                    {selectedWorkspace.id === activeWorkspaceId ? (
                      <Notice tone="success">
                        <div className="font-medium">当前工作区</div>
                        <div className="mt-1 text-xs text-emerald-900/80 dark:text-emerald-400/80">
                          对 Prompts/MCP/Skills 的修改会立即生效。
                        </div>
                      </Notice>
                    ) : (
                      <Notice tone="warning">
                        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                          <div>
                            <div className="font-medium text-foreground">该工作区尚未生效</div>
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
                      </Notice>
                    )}

                    {applyReport && applyReport.to_workspace_id === selectedWorkspace.id ? (
                      <Notice tone="neutral">
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
                      </Notice>
                    ) : null}

                    <div className="grid gap-3 sm:grid-cols-3">
                      <Card padding="sm">
                        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          Prompts
                        </div>
                        <div className="mt-2 text-sm text-secondary-foreground">
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
                        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          MCP
                        </div>
                        <div className="mt-2 text-sm text-secondary-foreground">
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
                        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          Skills
                        </div>
                        <div className="mt-2 text-sm text-secondary-foreground">
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
                      <Notice tone="neutral" className="mb-3">
                        非当前工作区：启用/停用仅写入数据库，不会同步到 CLI。
                      </Notice>
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
            <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              对比范围
            </div>
            <div className="mt-2 text-sm text-secondary-foreground">
              当前：
              {(() => {
                const fromId = preview?.from_workspace_id ?? activeWorkspaceId;
                if (!fromId) return "（未设置）";
                return workspaceById.get(fromId)?.name ?? `#${fromId}`;
              })()}
              <span className="mx-2 text-muted-foreground">→</span>
              目标：{switchTarget?.name ?? "—"}
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              仅展示 Prompts/MCP/Skills 的差异。确认无误后再切换为当前。
            </div>
          </Card>

          {previewLoading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Spinner size="sm" />
              生成对比中…
            </div>
          ) : !preview ? (
            <div className="text-sm text-muted-foreground">暂无对比数据。</div>
          ) : (
            <div className="space-y-3">
              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Prompts
                </div>
                <div className="mt-2 text-sm text-secondary-foreground">
                  {preview.prompts.will_change ? (
                    <span className="rounded-full bg-amber-50 dark:bg-amber-900/30 px-2 py-0.5 text-[11px] font-medium text-amber-800 dark:text-amber-400">
                      将变更
                    </span>
                  ) : (
                    <span className="rounded-full bg-secondary px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                      不变
                    </span>
                  )}
                </div>
                <div className="mt-2 grid gap-3 sm:grid-cols-2">
                  <div className={DIFF_BOX_CLASS}>
                    <div className="text-xs font-medium text-muted-foreground">当前</div>
                    <div className="mt-1 text-sm font-semibold text-foreground">
                      {preview.prompts.from_enabled?.name ?? "（未启用）"}
                    </div>
                    <div className="mt-1 text-xs text-muted-foreground">
                      {preview.prompts.from_enabled?.excerpt ?? "—"}
                    </div>
                  </div>
                  <div className={DIFF_BOX_CLASS}>
                    <div className="text-xs font-medium text-muted-foreground">目标</div>
                    <div className="mt-1 text-sm font-semibold text-foreground">
                      {preview.prompts.to_enabled?.name ?? "（未启用）"}
                    </div>
                    <div className="mt-1 text-xs text-muted-foreground">
                      {preview.prompts.to_enabled?.excerpt ?? "—"}
                    </div>
                  </div>
                </div>
              </Card>

              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  MCP
                </div>
                <div className="mt-2 text-sm text-secondary-foreground">
                  +{preview.mcp.added.length} / -{preview.mcp.removed.length}
                </div>
                {preview.mcp.added.length || preview.mcp.removed.length ? (
                  <div className="mt-2 grid gap-3 sm:grid-cols-2">
                    <div className={DIFF_BOX_CLASS}>
                      <div className="text-xs font-medium text-muted-foreground">新增</div>
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
                    <div className={DIFF_BOX_CLASS}>
                      <div className="text-xs font-medium text-muted-foreground">移除</div>
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
                  <div className="mt-2 text-xs text-muted-foreground">无变化</div>
                )}
              </Card>

              <Card padding="sm">
                <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Skills
                </div>
                <div className="mt-2 text-sm text-secondary-foreground">
                  +{preview.skills.added.length} / -{preview.skills.removed.length}
                </div>
                {preview.skills.added.length || preview.skills.removed.length ? (
                  <div className="mt-2 grid gap-3 sm:grid-cols-2">
                    <div className={DIFF_BOX_CLASS}>
                      <div className="text-xs font-medium text-muted-foreground">新增</div>
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
                    <div className={DIFF_BOX_CLASS}>
                      <div className="text-xs font-medium text-muted-foreground">移除</div>
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
                  <div className="mt-2 text-xs text-muted-foreground">无变化</div>
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

          <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line-subtle pt-3">
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
              <label className="flex items-center gap-2 text-sm text-secondary-foreground">
                <input
                  type="radio"
                  name="create-mode"
                  checked={createMode === "clone_active"}
                  onChange={() => setCreateMode("clone_active")}
                />
                从当前工作区克隆
              </label>
              <label className="flex items-center gap-2 text-sm text-secondary-foreground">
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

          {createError ? <Notice tone="danger">{createError}</Notice> : null}

          <div className="flex items-center justify-end gap-2 border-t border-line-subtle pt-3">
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

          {renameError ? <Notice tone="danger">{renameError}</Notice> : null}

          <div className="flex items-center justify-end gap-2 border-t border-line-subtle pt-3">
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
          <Notice tone="warning">
            删除会移除此工作区下的 Prompts/MCP/Skills 配置（DB）。不会删除任何未托管的 CLI 文件。
          </Notice>
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
