// Usage: Manage prompt templates. Backend commands: `prompts_*`, `prompt_*` (incl. default sync via `prompts_default_sync_from_files`).

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Pencil, Trash2 } from "lucide-react";
import { CLIS, cliLongLabel } from "../constants/clis";
import { logToConsole } from "../services/consoleLog";
import {
  promptDelete,
  promptSetEnabled,
  promptUpsert,
  promptsList,
  type PromptSummary,
} from "../services/prompts";
import type { CliKey } from "../services/providers";
import { startupSyncDefaultPromptsFromFilesOncePerSession } from "../services/startup";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { FormField } from "../ui/FormField";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Textarea } from "../ui/Textarea";
import { Switch } from "../ui/Switch";
import { cn } from "../utils/cn";
import { formatUnknownError } from "../utils/errors";

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
  if (normalized.length <= 120) return normalized;
  return `${normalized.slice(0, 120)}…`;
}

function formatPromptSaveToast(raw: string) {
  const msg = raw.trim();

  if (msg.includes("DB_CONSTRAINT:") && msg.includes("prompt") && msg.includes("name=")) {
    return "保存失败：名称重复（同一 CLI 下名称必须唯一）";
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

export function PromptsPage() {
  const [activeCli, setActiveCli] = useState<CliKey>("claude");
  const [items, setItems] = useState<PromptSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [togglingId, setTogglingId] = useState<number | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<PromptSummary | null>(null);

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<PromptSummary | null>(null);
  const [name, setName] = useState("");
  const [content, setContent] = useState("");

  const cliLabel = useMemo(() => {
    return cliLongLabel(activeCli);
  }, [activeCli]);

  async function refresh(cliKey: CliKey) {
    setLoading(true);
    try {
      const next = await promptsList(cliKey);
      if (!next) {
        setItems([]);
        return;
      }
      setItems(next);
    } catch (err) {
      logToConsole("error", "加载提示词失败", {
        error: String(err),
        cli: cliKey,
      });
      toast("加载失败：请查看控制台日志");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      await startupSyncDefaultPromptsFromFilesOncePerSession();
      if (cancelled) return;
      await refresh(activeCli);
    })();
    return () => {
      cancelled = true;
    };
  }, [activeCli]);

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
    setSaving(true);
    try {
      const next = await promptUpsert({
        prompt_id: editTarget?.id ?? null,
        cli_key: activeCli,
        name,
        content,
        enabled: editTarget?.enabled ?? false,
      });

      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      logToConsole(editTarget ? "info" : "info", editTarget ? "更新提示词" : "新增提示词", {
        id: next.id,
        cli: next.cli_key,
        enabled: next.enabled,
      });

      toast(editTarget ? "提示词已更新" : "提示词已新增");
      setDialogOpen(false);
      setEditTarget(null);
      await refresh(activeCli);
    } catch (err) {
      const msg = formatUnknownError(err);
      logToConsole("error", "保存提示词失败", { error: msg, cli: activeCli });
      toast(formatPromptSaveToast(msg));
    } finally {
      setSaving(false);
    }
  }

  async function toggleEnabled(target: PromptSummary, enabled: boolean) {
    if (togglingId != null) return;
    setTogglingId(target.id);
    try {
      const next = await promptSetEnabled(target.id, enabled);
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setItems((prev) =>
        prev.map((p) => {
          if (p.id === next.id) return next;
          if (enabled) return { ...p, enabled: false };
          return p;
        })
      );

      logToConsole("info", "切换提示词启用状态", {
        id: next.id,
        cli: next.cli_key,
        enabled: next.enabled,
      });
      toast(next.enabled ? `已启用并同步到 ${promptFileHint(next.cli_key)}` : "已停用");
    } catch (err) {
      logToConsole("error", "切换提示词启用状态失败", {
        error: String(err),
        id: target.id,
      });
      toast(`操作失败：${String(err)}`);
    } finally {
      setTogglingId(null);
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    const target = deleteTarget;
    setSaving(true);
    try {
      const ok = await promptDelete(target.id);
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setItems((prev) => prev.filter((p) => p.id !== target.id));
      logToConsole("info", "删除提示词", { id: target.id, cli: target.cli_key });
      toast("已删除");
      setDeleteTarget(null);
    } catch (err) {
      logToConsole("error", "删除提示词失败", { error: String(err), id: target.id });
      toast(`删除失败：${String(err)}`);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="提示词"
        actions={
          <>
            <Button
              onClick={() => {
                setEditTarget(null);
                setDialogOpen(true);
              }}
              variant="primary"
            >
              添加提示词
            </Button>
          </>
        }
      />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2">
          {CLIS.map((cli) => (
            <Button
              key={cli.key}
              onClick={() => setActiveCli(cli.key)}
              variant={activeCli === cli.key ? "primary" : "secondary"}
              size="sm"
            >
              {cli.name}
            </Button>
          ))}
        </div>
        <span className="text-xs text-slate-500">
          {loading ? "加载中…" : `共 ${items.length} 条`}
        </span>
      </div>

      {loading ? (
        <div className="text-sm text-slate-600">加载中…</div>
      ) : items.length === 0 ? (
        <div className="text-sm text-slate-600">
          暂无提示词。点击右上角「添加提示词」创建第一条（{cliLabel}）。
        </div>
      ) : (
        <div className="space-y-2">
          {items.map((p) => (
            <Card key={p.id} padding="sm">
              <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-sm font-semibold text-slate-900">{p.name}</div>
                    <span
                      className={cn(
                        "rounded-full px-2 py-0.5 text-xs font-medium",
                        p.enabled ? "bg-emerald-50 text-emerald-700" : "bg-slate-100 text-slate-600"
                      )}
                    >
                      {p.enabled ? "已启用" : "未启用"}
                    </span>
                  </div>

                  <div className="mt-2 text-sm text-slate-700">
                    <span className="text-xs text-slate-500">预览：</span>{" "}
                    <span className="break-words font-mono text-xs">
                      {previewContent(p.content)}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-1">
                  <Switch
                    checked={p.enabled}
                    disabled={togglingId === p.id}
                    onCheckedChange={(checked) => void toggleEnabled(p, checked)}
                    size="sm"
                  />
                  <div className="mx-1 h-4 w-px bg-slate-200" />
                  <Button
                    onClick={() => {
                      setEditTarget(p);
                      setDialogOpen(true);
                    }}
                    variant="ghost"
                    size="icon"
                    title="编辑"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>

                  <Button
                    onClick={() => setDeleteTarget(p)}
                    variant="ghost"
                    size="icon"
                    title="删除"
                    className="text-red-600 hover:bg-red-50 hover:text-red-700"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      <Dialog
        open={dialogOpen}
        title={editTarget ? "编辑提示词" : "添加提示词"}
        description={`当前 CLI：${cliLabel}（选择在列表中通过“启用”开关完成）`}
        onOpenChange={(open) => {
          setDialogOpen(open);
          if (!open) setEditTarget(null);
        }}
      >
        <div className="grid gap-4">
          <FormField label="名称" hint="例如：默认系统提示词">
            <Input
              type="text"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="例如：默认系统提示词"
            />
          </FormField>

          <FormField label="内容">
            <Textarea
              value={content}
              onChange={(e) => setContent(e.currentTarget.value)}
              placeholder="输入提示词内容（支持多行）"
              rows={12}
              mono
              className="text-xs"
            />
          </FormField>

          <div className="flex flex-wrap items-center gap-2">
            <Button onClick={save} variant="primary" disabled={saving}>
              {saving ? "保存中…" : "保存"}
            </Button>
            <Button
              onClick={() => {
                setDialogOpen(false);
                setEditTarget(null);
              }}
              variant="secondary"
              disabled={saving}
            >
              取消
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={Boolean(deleteTarget)}
        title="确认删除"
        description={deleteTarget ? `将删除「${deleteTarget.name}」且不可恢复。` : undefined}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
        className="max-w-xl"
      >
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={confirmDelete} variant="primary" disabled={saving}>
            {saving ? "删除中…" : "确认删除"}
          </Button>
          <Button onClick={() => setDeleteTarget(null)} variant="secondary" disabled={saving}>
            取消
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
