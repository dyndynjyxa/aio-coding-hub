// Usage: Installed/local skills view for a specific workspace.

import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import {
  useSkillImportLocalMutation,
  useSkillLocalDeleteMutation,
  useSkillReturnToLocalMutation,
  useSkillSetEnabledMutation,
  useSkillUninstallMutation,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
} from "../../query/skills";
import { logToConsole } from "../../services/consoleLog";
import type { CliKey } from "../../services/providers/providers";
import {
  type InstalledSkillSummary,
  type LocalSkillSummary,
} from "../../services/workspace/skills";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { formatActionFailureToast } from "../../utils/errors";
import {
  displaySkillName,
  repoPrefixFromGitUrl,
  repositoryWebUrl,
  sourceHint,
} from "../../utils/skillSources";

const TOAST_LOCAL_IMPORT_REQUIRES_ACTIVE = "仅当前工作区可导入本机 Skill。请先切换该工作区为当前。";
const TOAST_RETURN_LOCAL_REQUIRES_ACTIVE = "仅当前工作区可返回本机 Skill。请先切换该工作区为当前。";
const TOAST_DELETE_LOCAL_REQUIRES_ACTIVE = "仅当前工作区可删除本机 Skill。请先切换该工作区为当前。";

function formatUnixSeconds(ts: number) {
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

function pruneSelectionSet<T>(prev: Set<T>, allowed: Set<T>) {
  let changed = false;
  const next = new Set<T>();
  for (const value of prev) {
    if (allowed.has(value)) {
      next.add(value);
    } else {
      changed = true;
    }
  }
  return changed ? next : prev;
}

async function openPathOrReveal(path: string) {
  try {
    await openPath(path);
    return;
  } catch (err) {
    logToConsole("warn", "openPath 失败，尝试 revealItemInDir", {
      error: String(err),
      path,
    });
  }
  await revealItemInDir(path);
}

export type SkillsViewProps = {
  workspaceId: number;
  cliKey: CliKey;
  isActiveWorkspace?: boolean;
  localImportMode?: "single" | "batch_init";
};

export function SkillsView({
  workspaceId,
  cliKey,
  isActiveWorkspace = true,
  localImportMode = "single",
}: SkillsViewProps) {
  const canOperateLocal = isActiveWorkspace;
  const batchInitMode = localImportMode === "batch_init";

  const installedQuery = useSkillsInstalledListQuery(workspaceId);
  const localQuery = useSkillsLocalListQuery(workspaceId, { enabled: canOperateLocal });

  const toggleMutation = useSkillSetEnabledMutation(workspaceId);
  const uninstallMutation = useSkillUninstallMutation(workspaceId);
  const returnToLocalMutation = useSkillReturnToLocalMutation(workspaceId);
  const localDeleteMutation = useSkillLocalDeleteMutation(workspaceId);
  const importMutation = useSkillImportLocalMutation(workspaceId);

  const installed: InstalledSkillSummary[] = installedQuery.data ?? [];
  const localSkills: LocalSkillSummary[] = canOperateLocal ? (localQuery.data ?? []) : [];

  const loading = installedQuery.isFetching;
  const localLoading = canOperateLocal ? localQuery.isFetching : false;
  const togglingSkillId = toggleMutation.isPending
    ? (toggleMutation.variables?.skillId ?? null)
    : null;
  const returningLocalSkillId = returnToLocalMutation.isPending
    ? (returnToLocalMutation.variables ?? null)
    : null;
  const importingLocal = importMutation.isPending;

  const [importTarget, setImportTarget] = useState<LocalSkillSummary | null>(null);
  const [selectedInstalledIds, setSelectedInstalledIds] = useState<Set<number>>(new Set());
  const [selectedLocalDirNames, setSelectedLocalDirNames] = useState<Set<string>>(new Set());
  const [deleteInstalledDialogOpen, setDeleteInstalledDialogOpen] = useState(false);
  const [deleteLocalDialogOpen, setDeleteLocalDialogOpen] = useState(false);
  const [localDeleteTargets, setLocalDeleteTargets] = useState<LocalSkillSummary[]>([]);
  const [deletingInstalled, setDeletingInstalled] = useState(false);
  const [deletingLocal, setDeletingLocal] = useState(false);

  const selectedInstalledSkills = installed.filter((skill) => selectedInstalledIds.has(skill.id));
  const selectedLocalSkills = localSkills.filter((skill) =>
    selectedLocalDirNames.has(skill.dir_name)
  );
  const allInstalledSelected =
    installed.length > 0 && selectedInstalledSkills.length === installed.length;
  const allLocalSelected =
    localSkills.length > 0 && selectedLocalSkills.length === localSkills.length;

  useEffect(() => {
    if (!installedQuery.error) return;
    logToConsole("error", "加载 Skills 数据失败", {
      error: String(installedQuery.error),
      workspace_id: workspaceId,
    });
    toast("加载失败：请查看控制台日志");
  }, [installedQuery.error, workspaceId]);

  useEffect(() => {
    if (!localQuery.error) return;
    logToConsole("error", "扫描本机 Skill 失败", {
      error: String(localQuery.error),
      cli: cliKey,
      workspace_id: workspaceId,
    });
    toast("扫描本机 Skill 失败：请查看控制台日志");
  }, [cliKey, localQuery.error, workspaceId]);

  useEffect(() => {
    setSelectedInstalledIds(new Set());
    setSelectedLocalDirNames(new Set());
    setDeleteInstalledDialogOpen(false);
    setDeleteLocalDialogOpen(false);
    setImportTarget(null);
    setLocalDeleteTargets([]);
    setDeletingInstalled(false);
    setDeletingLocal(false);
  }, [cliKey, workspaceId]);

  useEffect(() => {
    const allowed = new Set((installedQuery.data ?? []).map((skill) => skill.id));
    setSelectedInstalledIds((prev) => pruneSelectionSet(prev, allowed));
  }, [installedQuery.data]);

  useEffect(() => {
    const allowed = new Set(
      (canOperateLocal ? (localQuery.data ?? []) : []).map((skill) => skill.dir_name)
    );
    setSelectedLocalDirNames((prev) => pruneSelectionSet(prev, allowed));
  }, [canOperateLocal, localQuery.data]);

  function toggleInstalledSelection(skillId: number) {
    setSelectedInstalledIds((prev) => {
      const next = new Set(prev);
      if (next.has(skillId)) {
        next.delete(skillId);
      } else {
        next.add(skillId);
      }
      return next;
    });
  }

  function toggleAllInstalledSelection() {
    if (allInstalledSelected) {
      setSelectedInstalledIds(new Set());
      return;
    }
    setSelectedInstalledIds(new Set(installed.map((skill) => skill.id)));
  }

  function toggleLocalSelection(dirName: string) {
    setSelectedLocalDirNames((prev) => {
      const next = new Set(prev);
      if (next.has(dirName)) {
        next.delete(dirName);
      } else {
        next.add(dirName);
      }
      return next;
    });
  }

  function toggleAllLocalSelection() {
    if (allLocalSelected) {
      setSelectedLocalDirNames(new Set());
      return;
    }
    setSelectedLocalDirNames(new Set(localSkills.map((skill) => skill.dir_name)));
  }

  function openInstalledDeleteDialog(skillIds: number[]) {
    setSelectedInstalledIds(new Set(skillIds));
    setDeleteInstalledDialogOpen(true);
  }

  function openLocalDeleteDialog(dirNames: string[]) {
    setImportTarget(null);
    setSelectedLocalDirNames(new Set(dirNames));
    setLocalDeleteTargets(localSkills.filter((skill) => dirNames.includes(skill.dir_name)));
    setDeleteLocalDialogOpen(true);
  }

  async function toggleSkillEnabled(skill: InstalledSkillSummary, enabled: boolean) {
    if (toggleMutation.isPending || deletingInstalled) return;
    try {
      const next = await toggleMutation.mutateAsync({ skillId: skill.id, enabled });
      if (!next) {
        return;
      }
      if (enabled) {
        toast(isActiveWorkspace ? "已启用" : "已启用（非当前工作区，不会同步）");
      } else {
        toast(isActiveWorkspace ? "已禁用" : "已禁用");
      }
    } catch (err) {
      const formatted = formatActionFailureToast("切换启用", err);
      logToConsole("error", "切换 Skill 启用状态失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: cliKey,
        workspace_id: workspaceId,
        skill_id: skill.id,
        enabled,
      });
      toast(formatted.toast);
    }
  }

  async function confirmDeleteInstalledSkills() {
    if (selectedInstalledSkills.length === 0 || deletingInstalled) return;

    const targets = selectedInstalledSkills.map((skill) => ({
      id: skill.id,
      name: skill.name,
    }));
    const failedIds = new Set<number>();
    let successCount = 0;
    let firstFailureToast: string | null = null;

    setDeletingInstalled(true);
    try {
      for (const target of targets) {
        try {
          const ok = await uninstallMutation.mutateAsync(target.id);
          if (ok) {
            successCount += 1;
            continue;
          }
          failedIds.add(target.id);
          firstFailureToast ??= `删除通用技能失败：${target.name}`;
        } catch (err) {
          const formatted = formatActionFailureToast("删除通用技能", err);
          logToConsole("error", "删除通用 Skill 失败", {
            error: formatted.raw,
            error_code: formatted.error_code ?? undefined,
            cli: cliKey,
            workspace_id: workspaceId,
            skill_id: target.id,
          });
          failedIds.add(target.id);
          firstFailureToast ??= formatted.toast;
        }
      }
    } finally {
      setDeletingInstalled(false);
    }

    if (successCount > 0) {
      toast(successCount === 1 ? "已删除通用技能" : `已删除 ${successCount} 个通用技能`);
    }
    if (failedIds.size > 0) {
      if (successCount === 0 && failedIds.size === 1 && firstFailureToast) {
        toast(firstFailureToast);
      } else {
        toast(`${failedIds.size} 个通用技能删除失败`);
      }
    }

    setSelectedInstalledIds(failedIds);
    setDeleteInstalledDialogOpen(false);
  }

  async function returnToLocalSkill(skill: InstalledSkillSummary) {
    if (!canOperateLocal) {
      toast(TOAST_RETURN_LOCAL_REQUIRES_ACTIVE);
      return;
    }
    if (returnToLocalMutation.isPending || deletingInstalled) return;
    const target = skill;
    try {
      const ok = await returnToLocalMutation.mutateAsync(target.id);
      if (!ok) {
        return;
      }
      toast("已返回本机已安装");
      logToConsole("info", "Skill 返回本机已安装", {
        cli: cliKey,
        workspace_id: workspaceId,
        skill: target,
      });
      setSelectedInstalledIds((prev) => {
        if (!prev.has(target.id)) return prev;
        const next = new Set(prev);
        next.delete(target.id);
        return next;
      });
    } catch (err) {
      const formatted = formatActionFailureToast("返回本机", err);
      logToConsole("error", "Skill 返回本机已安装失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: cliKey,
        workspace_id: workspaceId,
        skill: target,
      });
      toast(formatted.toast);
    }
  }

  async function confirmDeleteLocalSkills() {
    if (localDeleteTargets.length === 0 || deletingLocal) return;
    if (!canOperateLocal) {
      toast(TOAST_DELETE_LOCAL_REQUIRES_ACTIVE);
      return;
    }

    const targets = localDeleteTargets.map((skill) => ({
      dirName: skill.dir_name,
      label: skill.name || skill.dir_name,
      path: skill.path,
    }));
    const failedDirNames = new Set<string>();
    let successCount = 0;
    let firstFailureToast: string | null = null;

    setDeletingLocal(true);
    try {
      for (const target of targets) {
        try {
          const ok = await localDeleteMutation.mutateAsync(target.dirName);
          if (ok) {
            successCount += 1;
            continue;
          }
          failedDirNames.add(target.dirName);
          firstFailureToast ??= `删除本机技能失败：${target.label}`;
        } catch (err) {
          const formatted = formatActionFailureToast("删除本机技能", err);
          logToConsole("error", "删除本机 Skill 失败", {
            error: formatted.raw,
            error_code: formatted.error_code ?? undefined,
            cli: cliKey,
            workspace_id: workspaceId,
            dir_name: target.dirName,
            path: target.path,
          });
          failedDirNames.add(target.dirName);
          firstFailureToast ??= formatted.toast;
        }
      }
    } finally {
      setDeletingLocal(false);
    }

    if (successCount > 0) {
      toast(successCount === 1 ? "已删除本机技能" : `已删除 ${successCount} 个本机技能`);
    }
    if (failedDirNames.size > 0) {
      if (successCount === 0 && failedDirNames.size === 1 && firstFailureToast) {
        toast(firstFailureToast);
      } else {
        toast(`${failedDirNames.size} 个本机技能删除失败`);
      }
    }

    setSelectedLocalDirNames(failedDirNames);
    setLocalDeleteTargets(localDeleteTargets.filter((skill) => failedDirNames.has(skill.dir_name)));
    setDeleteLocalDialogOpen(false);
  }

  async function confirmImportLocalSkill() {
    if (!importTarget) return;
    if (importMutation.isPending || deletingLocal) return;
    if (!canOperateLocal) {
      toast(TOAST_LOCAL_IMPORT_REQUIRES_ACTIVE);
      return;
    }
    const target = importTarget;
    try {
      const next = await importMutation.mutateAsync(target.dir_name);
      if (!next) {
        return;
      }

      toast("已导入到技能库");
      logToConsole("info", "导入本机 Skill", {
        cli: cliKey,
        workspace_id: workspaceId,
        imported: next,
      });
      setSelectedLocalDirNames((prev) => {
        if (!prev.has(target.dir_name)) return prev;
        const nextSelected = new Set(prev);
        nextSelected.delete(target.dir_name);
        return nextSelected;
      });
      setImportTarget(null);
    } catch (err) {
      const formatted = formatActionFailureToast("导入", err);
      logToConsole("error", "导入本机 Skill 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: cliKey,
        workspace_id: workspaceId,
        skill: target,
      });
      toast(formatted.toast);
    }
  }

  async function refreshLocalSkills() {
    if (!canOperateLocal || localLoading || deletingLocal) return;
    await localQuery.refetch();
  }

  async function openLocalSkillDir(skill: LocalSkillSummary) {
    try {
      await openPathOrReveal(skill.path);
    } catch (err) {
      logToConsole("error", "打开本机 Skill 目录失败", {
        error: String(err),
        cli: cliKey,
        workspace_id: workspaceId,
        path: skill.path,
      });
      toast("打开目录失败：请查看控制台日志");
    }
  }

  return (
    <>
      <div className="grid h-full gap-4 lg:grid-cols-2">
        <Card className="flex min-h-[240px] flex-col lg:min-h-0" padding="md">
          <div className="flex shrink-0 items-start justify-between gap-3">
            <div className="text-sm font-semibold">通用技能</div>
            <div className="flex flex-wrap items-center justify-end gap-2">
              {installed.length > 0 ? (
                <label className="inline-flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
                  <input
                    type="checkbox"
                    checked={allInstalledSelected}
                    onChange={toggleAllInstalledSelection}
                    disabled={
                      deletingInstalled ||
                      toggleMutation.isPending ||
                      returnToLocalMutation.isPending
                    }
                    className="h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                    aria-label="全选通用技能"
                  />
                  <span>全选</span>
                </label>
              ) : null}
              {selectedInstalledIds.size > 0 ? (
                <Button
                  size="sm"
                  variant="danger"
                  disabled={
                    deletingInstalled || toggleMutation.isPending || returnToLocalMutation.isPending
                  }
                  onClick={() => setDeleteInstalledDialogOpen(true)}
                >
                  删除通用技能 ({selectedInstalledIds.size})
                </Button>
              ) : null}
              <Button
                size="sm"
                variant="secondary"
                aria-label="刷新通用技能"
                onClick={() => void installedQuery.refetch()}
                disabled={loading || deletingInstalled}
              >
                {loading ? "刷新中…" : "刷新"}
              </Button>
              <span className="rounded-full bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                {installed.length}
              </span>
            </div>
          </div>

          <div className="mt-4 min-h-0 flex-1 space-y-2 lg:overflow-y-auto lg:pr-1 scrollbar-overlay">
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
                <Spinner size="sm" />
                加载中…
              </div>
            ) : installed.length === 0 ? (
              <EmptyState title="暂无已安装 Skill。" variant="dashed" />
            ) : (
              installed.map((skill) => {
                const repoPrefix = repoPrefixFromGitUrl(skill.source_git_url);
                const repoUrl = repositoryWebUrl(skill.source_git_url);
                return (
                  <div
                    key={skill.id}
                    className="rounded-xl border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-800"
                  >
                    <div className="flex items-start gap-3">
                      <input
                        type="checkbox"
                        checked={selectedInstalledIds.has(skill.id)}
                        onChange={() => toggleInstalledSelection(skill.id)}
                        disabled={
                          deletingInstalled ||
                          togglingSkillId === skill.id ||
                          returningLocalSkillId === skill.id
                        }
                        className="mt-0.5 h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                        aria-label={`选择通用技能 ${skill.name}`}
                      />

                      <div className="min-w-0 flex-1">
                        {repoPrefix ? (
                          <div className="mb-1 text-[11px] font-medium uppercase tracking-[0.18em] text-slate-400 dark:text-slate-500">
                            {repoPrefix}
                          </div>
                        ) : null}
                        <div className="flex items-center gap-2">
                          <span className="min-w-0 truncate text-sm font-semibold">
                            {displaySkillName(skill.name, skill.source_git_url)}
                          </span>
                          {repoUrl ? (
                            <a
                              href={repoUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="shrink-0 text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                              title={sourceHint(skill)}
                            >
                              <ExternalLink className="h-3.5 w-3.5" />
                            </a>
                          ) : null}
                          <div className="ms-auto flex flex-wrap items-center justify-end gap-2">
                            <span className="text-xs text-slate-600 dark:text-slate-400">启用</span>
                            <Switch
                              checked={skill.enabled}
                              disabled={
                                deletingInstalled ||
                                togglingSkillId === skill.id ||
                                returningLocalSkillId === skill.id
                              }
                              onCheckedChange={(next) => void toggleSkillEnabled(skill, next)}
                            />
                            <Button
                              size="sm"
                              variant="secondary"
                              title={
                                canOperateLocal
                                  ? "将该 Skill 从通用技能返回到本机已安装"
                                  : undefined
                              }
                              disabled={
                                !canOperateLocal ||
                                deletingInstalled ||
                                returningLocalSkillId === skill.id
                              }
                              onClick={() => void returnToLocalSkill(skill)}
                            >
                              返回本机已安装
                            </Button>
                            <Button
                              size="sm"
                              variant="danger"
                              aria-label={`删除通用技能 ${skill.name}`}
                              disabled={
                                deletingInstalled ||
                                togglingSkillId === skill.id ||
                                returningLocalSkillId === skill.id
                              }
                              onClick={() => openInstalledDeleteDialog([skill.id])}
                            >
                              删除
                            </Button>
                          </div>
                        </div>
                        {skill.description ? (
                          <div className="mt-1.5 text-xs text-slate-500 dark:text-slate-400">
                            {skill.description}
                          </div>
                        ) : null}
                        <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                          <span
                            className={cn(
                              "rounded-full px-2 py-1 font-medium",
                              skill.enabled
                                ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
                                : "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-400"
                            )}
                          >
                            {skill.enabled ? "已启用" : "未启用"}
                          </span>
                          <span>更新 {formatUnixSeconds(skill.updated_at)}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </Card>

        <Card className="flex min-h-[240px] flex-col lg:min-h-0" padding="md">
          <div className="flex shrink-0 items-start justify-between gap-3">
            <div className="text-sm font-semibold">本机已安装</div>
            <div className="flex flex-wrap items-center justify-end gap-2">
              {canOperateLocal && localSkills.length > 0 ? (
                <label className="inline-flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
                  <input
                    type="checkbox"
                    checked={allLocalSelected}
                    onChange={toggleAllLocalSelection}
                    disabled={deletingLocal || importingLocal}
                    className="h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                    aria-label="全选本机技能"
                  />
                  <span>全选</span>
                </label>
              ) : null}
              {selectedLocalDirNames.size > 0 ? (
                <Button
                  size="sm"
                  variant="danger"
                  onClick={() => {
                    setLocalDeleteTargets(selectedLocalSkills);
                    setDeleteLocalDialogOpen(true);
                  }}
                  disabled={!canOperateLocal || deletingLocal || importingLocal}
                >
                  删除本机技能 ({selectedLocalDirNames.size})
                </Button>
              ) : null}
              <Button
                size="sm"
                variant="secondary"
                aria-label="刷新本机技能"
                onClick={() => void refreshLocalSkills()}
                disabled={!canOperateLocal || localLoading || deletingLocal}
              >
                {localLoading ? "刷新中…" : "刷新"}
              </Button>
              <span className="rounded-full bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                {canOperateLocal ? (localLoading ? "扫描中…" : `${localSkills.length}`) : "—"}
              </span>
            </div>
          </div>

          <div className="mt-4 min-h-0 flex-1 space-y-2 lg:overflow-y-auto lg:pr-1 scrollbar-overlay">
            {!canOperateLocal ? (
              <EmptyState
                title={`仅当前工作区可扫描/导入本机 Skill（因为会直接读取/写入 ${cliKey} 的真实目录）。`}
                variant="dashed"
              />
            ) : localLoading ? (
              <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
                <Spinner size="sm" />
                扫描中…
              </div>
            ) : localSkills.length === 0 ? (
              <EmptyState title="未发现本机 Skill。" variant="dashed" />
            ) : (
              localSkills.map((skill) => {
                const label = skill.name || skill.dir_name;
                const displayLabel = displaySkillName(label, skill.source_git_url);
                const repoUrl = repositoryWebUrl(skill.source_git_url ?? "");
                const repoPrefix = repoPrefixFromGitUrl(skill.source_git_url ?? "");
                return (
                  <div
                    key={skill.path}
                    className="rounded-xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800"
                  >
                    <div className="flex items-start gap-3">
                      <input
                        type="checkbox"
                        checked={selectedLocalDirNames.has(skill.dir_name)}
                        onChange={() => toggleLocalSelection(skill.dir_name)}
                        disabled={deletingLocal || importingLocal}
                        className="mt-0.5 h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                        aria-label={`选择本机技能 ${label}`}
                      />

                      <div className="min-w-0 flex-1">
                        {repoPrefix ? (
                          <div className="mb-1 text-[11px] font-medium uppercase tracking-[0.18em] text-slate-400 dark:text-slate-500">
                            {repoPrefix}
                          </div>
                        ) : null}
                        <div className="flex items-center gap-2">
                          <span className="min-w-0 truncate text-sm font-semibold">
                            {displayLabel}
                          </span>
                          {repoUrl ? (
                            <a
                              href={repoUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="shrink-0 text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                              title={sourceHint(skill)}
                            >
                              <ExternalLink className="h-3.5 w-3.5" />
                            </a>
                          ) : null}
                          <div className="ms-auto flex flex-wrap items-center justify-end gap-2">
                            {batchInitMode ? null : (
                              <Button
                                size="sm"
                                variant="primary"
                                disabled={deletingLocal || importingLocal}
                                onClick={() => setImportTarget(skill)}
                              >
                                导入技能库
                              </Button>
                            )}
                            <Button
                              size="sm"
                              variant="danger"
                              aria-label={`删除本机技能 ${displayLabel}`}
                              disabled={deletingLocal || importingLocal}
                              onClick={() => openLocalDeleteDialog([skill.dir_name])}
                            >
                              删除
                            </Button>
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => void openLocalSkillDir(skill)}
                            >
                              打开目录
                            </Button>
                          </div>
                        </div>
                        {skill.description ? (
                          <div className="mt-1.5 text-xs text-slate-500 dark:text-slate-400">
                            {skill.description}
                          </div>
                        ) : null}
                        <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                          <span className="min-w-0 truncate font-mono">{skill.path}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </Card>
      </div>

      <ConfirmDialog
        open={deleteInstalledDialogOpen}
        title="确认删除通用技能"
        description={`将删除 ${selectedInstalledSkills.length} 个通用技能，并同步移除受管目录，此操作不可撤销。`}
        onClose={() => setDeleteInstalledDialogOpen(false)}
        onConfirm={() => void confirmDeleteInstalledSkills()}
        confirmLabel="确认删除"
        confirmingLabel="删除中…"
        confirming={deletingInstalled}
        confirmVariant="danger"
        disabled={selectedInstalledSkills.length === 0}
      >
        <div className="max-h-40 overflow-auto text-sm text-slate-600 dark:text-slate-400">
          <ul className="space-y-1">
            {selectedInstalledSkills.slice(0, 10).map((skill) => (
              <li key={skill.id} className="truncate">
                {skill.name}
              </li>
            ))}
            {selectedInstalledSkills.length > 10 ? (
              <li className="text-slate-400">...还有 {selectedInstalledSkills.length - 10} 个</li>
            ) : null}
          </ul>
        </div>
      </ConfirmDialog>

      <ConfirmDialog
        open={deleteLocalDialogOpen}
        title="确认删除本机技能"
        description={`将删除 ${localDeleteTargets.length} 个本机技能目录，此操作不可撤销。`}
        onClose={() => {
          setDeleteLocalDialogOpen(false);
          setLocalDeleteTargets([]);
        }}
        onConfirm={() => void confirmDeleteLocalSkills()}
        confirmLabel="确认删除"
        confirmingLabel="删除中…"
        confirming={deletingLocal}
        confirmVariant="danger"
        disabled={localDeleteTargets.length === 0}
      >
        <div className="max-h-48 space-y-2 overflow-auto text-xs text-slate-600 dark:text-slate-400">
          {localDeleteTargets.slice(0, 10).map((skill) => (
            <div
              key={skill.path}
              className="rounded-xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800"
            >
              <div className="font-medium text-slate-800 dark:text-slate-200">
                {skill.name || skill.dir_name}
              </div>
              <div className="mt-1 break-all font-mono">{skill.path}</div>
            </div>
          ))}
          {localDeleteTargets.length > 10 ? (
            <div className="text-slate-400">...还有 {localDeleteTargets.length - 10} 个</div>
          ) : null}
        </div>
      </ConfirmDialog>

      {batchInitMode ? null : (
        <ConfirmDialog
          open={importTarget != null}
          title="导入到技能库"
          description="导入后该 Skill 会被 AIO 记录并管理，可在其他工作区中启用/禁用。"
          onClose={() => setImportTarget(null)}
          onConfirm={() => void confirmImportLocalSkill()}
          confirmLabel="确认导入"
          confirmingLabel="导入中…"
          confirming={importingLocal}
          disabled={!importTarget}
        >
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400">
            <div className="font-medium text-slate-800 dark:text-slate-200">
              {importTarget?.name || importTarget?.dir_name}
            </div>
            <div className="mt-1 break-all font-mono">{importTarget?.path}</div>
          </div>
        </ConfirmDialog>
      )}
    </>
  );
}
