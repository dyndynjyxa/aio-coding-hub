// Usage: Prompt templates view for a specific workspace.

import { Pencil, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  usePromptDeleteMutation,
  usePromptSetEnabledMutation,
  usePromptUpsertMutation,
  usePromptsListQuery,
} from "../../query/prompts";
import { logToConsole } from "../../services/consoleLog";
import type { PromptSummary } from "../../services/workspace/prompts";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Dialog } from "../../ui/Dialog";
import { EmptyState } from "../../ui/EmptyState";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { Textarea } from "../../ui/Textarea";
import { cn } from "../../utils/cn";
import { formatUnknownError } from "../../utils/errors";

function promptFileHint(cliKey: CliKey) {
  switch (cliKey) {
    case "claude":
      return "~/.claude/CLAUDE.md";
    case "codex":
      return "~/.codex/AGENTS.md";
    case "gemini":
      return "~/.gemini/GEMINI.md";
    default:
      return "~";
  }
}

function previewContent(content: string) {
  const normalized = content.replace(/\s+/g, " ").trim();
  if (!normalized) return "空内容";
  if (normalized.length <= 120) return normalized;
  return `${normalized.slice(0, 120)}…`;
}

function formatPromptSaveToast(raw: string) {
  const msg = raw.trim();

  if (msg.includes("DB_CONSTRAINT:") && msg.includes("prompt") && msg.includes("name=")) {
    return "保存失败：名称重复（同一工作区下名称必须唯一）";
  }
  if (/SEC_INVALID_INPUT:\s*prompt name is required/i.test(msg)) {
    return "保存失败：名称不能为空";
  }
  if (/SEC_INVALID_INPUT:\s*prompt content is required/i.test(msg)) {
    return "保存失败：内容不能为空";
  }
  if (msg.startsWith("DB_CONSTRAINT:")) {
    return "保存失败：数据库约束冲突（请检查名称是否重复）";
  }

  return `保存失败：${msg || "未知错误"}`;
}

export type PromptsViewProps = {
  workspaceId: number;
  cliKey: CliKey;
  isActiveWorkspace?: boolean;
};

export function PromptsView({ workspaceId, cliKey, isActiveWorkspace = true }: PromptsViewProps) {
  const promptsQuery = usePromptsListQuery(workspaceId);
  const upsertMutation = usePromptUpsertMutation(workspaceId);
  const toggleMutation = usePromptSetEnabledMutation(workspaceId);
  const deleteMutation = usePromptDeleteMutation(workspaceId);

  const items: PromptSummary[] = promptsQuery.data ?? [];
  const loading = promptsQuery.isFetching;
  const saving = upsertMutation.isPending || toggleMutation.isPending || deleteMutation.isPending;
  const [togglingId, setTogglingId] = useState<number | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PromptSummary | null>(null);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<PromptSummary | null>(null);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");

  const fileHint = useMemo(() => promptFileHint(cliKey), [cliKey]);

  useEffect(() => {
    if (!promptsQuery.error) return;
    logToConsole("error", "加载提示词失败", {
      error: String(promptsQuery.error),
      workspace_id: workspaceId,
    });
    toast("加载失败：请查看控制台日志");
  }, [promptsQuery.error, workspaceId]);

  useEffect(() => {
    if (!dialogOpen) return;
    if (editTarget) {
      setName(editTarget.name);
      setContent(editTarget.content);
      return;
    }
    setName("");
    setContent("");
  }, [dialogOpen, editTarget]);

  async function save() {
    if (saving) return;
    try {
      const next = await upsertMutation.mutateAsync({
        promptId: editTarget?.id ?? null,
        name,
        content,
        enabled: editTarget?.enabled ?? false,
      });

      if (!next) {
        return;
      }

      logToConsole(editTarget ? "info" : "info", editTarget ? "更新提示词" : "新增提示词", {
        id: next.id,
        workspace_id: workspaceId,
        enabled: next.enabled,
      });

      toast(editTarget ? "提示词已更新" : "提示词已新增");
      setDialogOpen(false);
      setEditTarget(null);
    } catch (err) {
      const msg = formatUnknownError(err);
      logToConsole("error", "保存提示词失败", { error: msg, workspace_id: workspaceId });
      toast(formatPromptSaveToast(msg));
    }
  }

  async function toggleEnabled(target: PromptSummary, enabled: boolean) {
    if (togglingId != null) return;
    setTogglingId(target.id);
    try {
      const next = await toggleMutation.mutateAsync({ promptId: target.id, enabled });
      if (!next) {
        return;
      }

      logToConsole("info", "切换提示词启用状态", {
        id: next.id,
        workspace_id: workspaceId,
        enabled: next.enabled,
      });

      if (next.enabled) {
        toast(
          isActiveWorkspace ? `已启用并同步到 ${fileHint}` : "已启用（非当前工作区，不会同步）"
        );
      } else {
        toast(isActiveWorkspace ? "已停用并同步" : "已停用");
      }
    } catch (err) {
      logToConsole("error", "切换提示词启用状态失败", {
        error: String(err),
        id: target.id,
        workspace_id: workspaceId,
      });
      toast(`操作失败：${String(err)}`);
    } finally {
      setTogglingId(null);
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    if (saving) return;
    const target = deleteTarget;
    try {
      const ok = await deleteMutation.mutateAsync(target.id);
      if (!ok) {
        return;
      }
      logToConsole("info", "删除提示词", { id: target.id, workspace_id: workspaceId });
      toast("已删除");
      setDeleteTarget(null);
    } catch (err) {
      logToConsole("error", "删除提示词失败", { error: String(err), id: target.id });
      toast(`删除失败：${String(err)}`);
    }
  }

  return (
    <>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {loading ? "加载中…" : `共 ${items.length} 条`}
          </span>
          {isActiveWorkspace ? (
            <span className="text-xs text-slate-500 dark:text-slate-400">
              启用后会写入 {fileHint}
            </span>
          ) : (
            <span className="text-xs text-slate-500 dark:text-slate-400">
              非当前工作区：仅写入数据库，不触发同步
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            onClick={() => {
              setEditTarget(null);
              setDialogOpen(true);
            }}
            variant="primary"
          >
            新增提示词
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
          <Spinner size="sm" />
          加载中…
        </div>
      ) : items.length === 0 ? (
        <EmptyState title="暂无提示词" description="点击右上角「新增提示词」创建第一条。" />
      ) : (
        <div className="space-y-2">
          {items.map((prompt) => (
            <Card key={prompt.id} padding="md">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <div className="truncate text-base font-semibold text-slate-900 dark:text-slate-100 leading-tight">
                      {prompt.name}
                    </div>
                    {prompt.enabled ? (
                      <span className="rounded-full bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400">
                        已启用
                      </span>
                    ) : (
                      <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:text-slate-400">
                        未启用
                      </span>
                    )}
                  </div>
                  <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                    {previewContent(prompt.content)}
                  </div>
                </div>

                <div className="flex shrink-0 items-center gap-3">
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-600 dark:text-slate-400">启用</span>
                    <Switch
                      checked={prompt.enabled}
                      disabled={togglingId === prompt.id}
                      onCheckedChange={(next) => void toggleEnabled(prompt, next)}
                    />
                  </div>

                  <div className="h-8 w-px bg-slate-200 dark:bg-slate-600" />

                  <div className="flex items-center gap-1">
                    <Button
                      onClick={() => {
                        setEditTarget(prompt);
                        setDialogOpen(true);
                      }}
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8 p-0 text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 dark:text-slate-400 dark:hover:text-indigo-400 dark:hover:bg-indigo-900/30"
                      aria-label="Edit"
                      title="编辑"
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      onClick={() => setDeleteTarget(prompt)}
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8 p-0 text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:text-rose-400 dark:hover:bg-rose-900/30"
                      aria-label="Delete"
                      title="删除"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      <Dialog
        open={dialogOpen}
        title={editTarget ? "编辑提示词" : "新增提示词"}
        description={
          isActiveWorkspace ? `启用后会同步到 ${fileHint}` : "非当前工作区：保存不会触发同步"
        }
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditTarget(null);
        }}
        className="max-w-3xl"
      >
        <div className="grid gap-4">
          <FormField label="名称">
            <Input value={name} onChange={(e) => setName(e.currentTarget.value)} />
          </FormField>
          <FormField label="内容">
            <Textarea
              value={content}
              onChange={(e) => setContent(e.currentTarget.value)}
              rows={10}
            />
          </FormField>

          <div className="flex items-center justify-end gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button onClick={() => setDialogOpen(false)} variant="secondary">
              取消
            </Button>
            <Button
              onClick={() => void save()}
              variant="primary"
              disabled={saving || !name.trim()}
              className={cn(saving ? "opacity-80" : "")}
            >
              {saving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={deleteTarget != null}
        title="确认删除提示词"
        description={deleteTarget ? `将删除：${deleteTarget.name}` : undefined}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button onClick={() => setDeleteTarget(null)} variant="secondary">
              取消
            </Button>
            <Button
              onClick={() => void confirmDelete()}
              variant="danger"
              disabled={!deleteTarget || saving}
            >
              {saving ? "删除中…" : "确认删除"}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
