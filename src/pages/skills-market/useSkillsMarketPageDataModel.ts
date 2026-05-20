// Usage: Data-model hook for the skills market page.

import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { CLIS, cliFromKeyOrDefault, isCliKey } from "../../constants/clis";
import { useSettingsQuery } from "../../query/settings";
import {
  useSkillInstallToLocalMutation,
  useSkillRepoDeleteMutation,
  useSkillRepoUpsertMutation,
  useSkillReposListQuery,
  useSkillsDiscoverAvailableMutation,
  useSkillsDiscoverAvailableQuery,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
} from "../../query/skills";
import { useWorkspacesListQuery } from "../../query/workspaces";
import { logToConsole } from "../../services/consoleLog";
import { getOrderedClis, pickDefaultCliByPriority } from "../../services/cli/cliPriorityOrder";
import type { CliKey } from "../../services/providers/providers";
import type {
  AvailableSkillSummary,
  InstalledSkillSummary,
  SkillRepoSummary,
} from "../../services/workspace/skills";
import { formatActionFailureToast } from "../../utils/errors";
import {
  normalizeRepoPath,
  repoKey,
  repoPrefixFromGitUrl,
  repositoryWebUrl,
  sourceHint,
  sourceKey,
} from "../../utils/skillSources";

type MarketStatus = "not_installed" | "local_installed" | "needs_enable" | "enabled";

type RepoGroup = {
  key: string;
  gitUrl: string;
  branch: string;
  repoPath: string;
  repoPrefix: string;
  skills: AvailableSkillSummary[];
  installableCount: number;
  localCount: number;
  needsEnableCount: number;
  enabledCount: number;
};

function readCliFromStorage(): CliKey | null {
  try {
    const raw = localStorage.getItem("skills.activeCli");
    if (isCliKey(raw)) return raw;
  } catch {}
  return null;
}

function writeCliToStorage(cli: CliKey) {
  try {
    localStorage.setItem("skills.activeCli", cli);
  } catch {}
}

export function formatUnixSeconds(ts: number) {
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

export function statusLabel(status: MarketStatus) {
  if (status === "local_installed") return "已装到当前 CLI";
  if (status === "enabled") return "已在通用技能";
  if (status === "needs_enable") return "通用技能未启用";
  return "未安装";
}

export function statusTone(status: MarketStatus) {
  if (status === "local_installed") {
    return "bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-300";
  }
  if (status === "enabled") {
    return "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400";
  }
  if (status === "needs_enable") {
    return "bg-amber-50 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400";
  }
  return "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-400";
}

function statusRank(status: MarketStatus) {
  if (status === "not_installed") return 0;
  if (status === "local_installed") return 1;
  if (status === "needs_enable") return 2;
  return 3;
}

export function useSkillsMarketPageDataModel() {
  const navigate = useNavigate();
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;

  const [activeCli, setActiveCli] = useState<CliKey | null>(() => readCliFromStorage());
  const [query, setQuery] = useState("");
  const [repoFilter, setRepoFilter] = useState<string>("all");
  const [onlyActionable, setOnlyActionable] = useState(true);
  const [expandedRepos, setExpandedRepos] = useState<Set<string>>(new Set());
  const [installingRepoKey, setInstallingRepoKey] = useState<string | null>(null);
  const [installingSources, setInstallingSources] = useState<Set<string>>(new Set());
  const [repoDialogOpen, setRepoDialogOpen] = useState(false);
  const [newRepoUrl, setNewRepoUrl] = useState("");
  const [newRepoBranch, setNewRepoBranch] = useState("auto");
  const [repoSaving, setRepoSaving] = useState(false);
  const [repoToggleId, setRepoToggleId] = useState<number | null>(null);
  const [repoDeleteTarget, setRepoDeleteTarget] = useState<SkillRepoSummary | null>(null);
  const [repoDeleting, setRepoDeleting] = useState(false);

  const effectiveCli = activeCli ?? defaultCli;
  const currentCli = useMemo(() => cliFromKeyOrDefault(effectiveCli), [effectiveCli]);

  const reposQuery = useSkillReposListQuery();
  const repos = useMemo(() => reposQuery.data ?? [], [reposQuery.data]);
  const enabledRepoCount = useMemo(() => repos.filter((repo) => repo.enabled).length, [repos]);

  const workspacesQuery = useWorkspacesListQuery(effectiveCli);
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;

  const installedQuery = useSkillsInstalledListQuery(activeWorkspaceId);
  const localQuery = useSkillsLocalListQuery(activeWorkspaceId, {
    enabled: Boolean(activeWorkspaceId),
  });
  const availableQuery = useSkillsDiscoverAvailableQuery(false, {
    enabled: enabledRepoCount > 0,
  });

  const installed = useMemo(
    () => (activeWorkspaceId ? (installedQuery.data ?? []) : []),
    [activeWorkspaceId, installedQuery.data]
  );
  const localSkills = useMemo(
    () => (activeWorkspaceId ? (localQuery.data ?? []) : []),
    [activeWorkspaceId, localQuery.data]
  );
  const available = useMemo(() => availableQuery.data ?? [], [availableQuery.data]);

  const discoverMutation = useSkillsDiscoverAvailableMutation();
  const repoUpsertMutation = useSkillRepoUpsertMutation();
  const repoDeleteMutation = useSkillRepoDeleteMutation();
  const installToLocalMutation = useSkillInstallToLocalMutation(activeWorkspaceId);

  const loading =
    reposQuery.isLoading ||
    workspacesQuery.isLoading ||
    installedQuery.isLoading ||
    (Boolean(activeWorkspaceId) && localQuery.isLoading);
  const discovering = discoverMutation.isPending || availableQuery.isFetching;
  const installBusy = installingRepoKey != null || installingSources.size > 0;

  useEffect(() => {
    writeCliToStorage(effectiveCli);
  }, [effectiveCli]);

  const installedBySource = useMemo(() => {
    const map = new Map<string, InstalledSkillSummary>();
    for (const row of installed) {
      map.set(sourceKey(row), row);
    }
    return map;
  }, [installed]);

  const localBySource = useMemo(() => {
    const map = new Map<string, (typeof localSkills)[number]>();
    for (const row of localSkills) {
      if (!row.source_git_url || !row.source_branch || !row.source_subdir) continue;

      map.set(
        sourceKey({
          source_git_url: row.source_git_url,
          source_branch: row.source_branch,
          source_subdir: row.source_subdir,
        }),
        row
      );
    }
    return map;
  }, [localSkills]);

  const getStatus = useCallback(
    (skill: AvailableSkillSummary): MarketStatus => {
      const key = sourceKey(skill);
      if (localBySource.has(key)) return "local_installed";

      const installedRow = installedBySource.get(key);
      if (!installedRow) return "not_installed";
      return installedRow.enabled ? "enabled" : "needs_enable";
    },
    [installedBySource, localBySource]
  );

  const repoOptions = useMemo(() => {
    const repoOptionMap = new Map<string, { key: string; label: string }>();

    for (const row of available) {
      const key = repoKey(row);
      if (repoOptionMap.has(key)) continue;

      const repoPath = normalizeRepoPath(row.source_git_url) || row.source_git_url;
      repoOptionMap.set(key, {
        key,
        label: `${repoPath} (${row.source_branch})`,
      });
    }

    return Array.from(repoOptionMap.values()).sort((left, right) =>
      left.label.localeCompare(right.label)
    );
  }, [available]);

  const filteredAvailable = useMemo(() => {
    const loweredQuery = query.trim().toLowerCase();

    return available.filter((row) => {
      if (repoFilter !== "all" && repoKey(row) !== repoFilter) return false;

      const status = getStatus(row);
      if (onlyActionable && status !== "not_installed") return false;
      if (!loweredQuery) return true;

      const haystack = [
        row.name,
        row.description,
        row.source_subdir,
        normalizeRepoPath(row.source_git_url),
        repoPrefixFromGitUrl(row.source_git_url),
        row.source_branch,
      ]
        .join(" ")
        .toLowerCase();

      return haystack.includes(loweredQuery);
    });
  }, [available, getStatus, onlyActionable, query, repoFilter]);

  const groupedAvailable = useMemo(() => {
    const groupMap = new Map<string, RepoGroup>();

    for (const skill of filteredAvailable) {
      const key = repoKey(skill);
      const group = groupMap.get(key) ?? {
        key,
        gitUrl: skill.source_git_url,
        branch: skill.source_branch,
        repoPath: normalizeRepoPath(skill.source_git_url) || skill.source_git_url,
        repoPrefix: repoPrefixFromGitUrl(skill.source_git_url) ?? "仓库",
        skills: [],
        installableCount: 0,
        localCount: 0,
        needsEnableCount: 0,
        enabledCount: 0,
      };

      const status = getStatus(skill);
      if (status === "not_installed") group.installableCount += 1;
      if (status === "local_installed") group.localCount += 1;
      if (status === "needs_enable") group.needsEnableCount += 1;
      if (status === "enabled") group.enabledCount += 1;

      group.skills.push(skill);
      groupMap.set(key, group);
    }

    return Array.from(groupMap.values())
      .map((group) => ({
        ...group,
        skills: [...group.skills].sort((left, right) => {
          const rank = statusRank(getStatus(left)) - statusRank(getStatus(right));
          if (rank !== 0) return rank;
          return left.name.localeCompare(right.name);
        }),
      }))
      .sort((left, right) => {
        if (left.installableCount !== right.installableCount) {
          return right.installableCount - left.installableCount;
        }
        return left.repoPath.localeCompare(right.repoPath);
      });
  }, [filteredAvailable, getStatus]);

  useEffect(() => {
    setExpandedRepos((previous) => {
      const allowed = new Set(groupedAvailable.map((group) => group.key));
      const next = new Set(Array.from(previous).filter((key) => allowed.has(key)));

      if (repoFilter !== "all" && allowed.has(repoFilter)) {
        next.add(repoFilter);
      } else if (next.size === 0 && groupedAvailable[0]) {
        next.add(groupedAvailable[0].key);
      }

      return next;
    });
  }, [groupedAvailable, repoFilter]);

  async function refreshAvailable(refresh: boolean, toastOnSuccess = true) {
    try {
      const rows = await discoverMutation.mutateAsync(refresh);
      if (!rows) return;

      logToConsole("info", refresh ? "刷新 Skill 发现（下载更新）" : "扫描 Skill（缓存）", {
        refresh,
        count: rows.length,
      });
      if (toastOnSuccess) toast(`已发现 ${rows.length} 个 Skill`);
    } catch (error) {
      const formatted = formatActionFailureToast("刷新发现", error);
      logToConsole("error", "刷新 Skill 发现失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        refresh,
      });
      toast(formatted.toast);
    }
  }

  async function addRepo() {
    if (repoSaving) return;

    const gitUrl = newRepoUrl.trim();
    const branch = newRepoBranch.trim() || "auto";
    if (!gitUrl) {
      toast("请填写 Git URL");
      return;
    }

    setRepoSaving(true);
    try {
      const next = await repoUpsertMutation.mutateAsync({
        repoId: null,
        gitUrl,
        branch,
        enabled: true,
      });
      if (!next) return;

      setNewRepoUrl("");
      setNewRepoBranch(branch);
      toast("仓库已添加");
      logToConsole("info", "添加 Skill 仓库", next);
    } catch (error) {
      const formatted = formatActionFailureToast("添加仓库", error);
      logToConsole("error", "添加 Skill 仓库失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
      });
      toast(formatted.toast);
    } finally {
      setRepoSaving(false);
    }
  }

  async function toggleRepoEnabled(repo: SkillRepoSummary, enabled: boolean) {
    if (repoToggleId != null) return;

    setRepoToggleId(repo.id);
    try {
      const next = await repoUpsertMutation.mutateAsync({
        repoId: repo.id,
        gitUrl: repo.git_url,
        branch: repo.branch,
        enabled,
      });
      if (!next) return;
      toast(enabled ? "仓库已启用" : "仓库已禁用");
    } catch (error) {
      const formatted = formatActionFailureToast("切换仓库", error);
      logToConsole("error", "切换 Skill 仓库启用状态失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        repo_id: repo.id,
        enabled,
      });
      toast(formatted.toast);
    } finally {
      setRepoToggleId(null);
    }
  }

  async function confirmDeleteRepo() {
    if (!repoDeleteTarget || repoDeleting) return;

    setRepoDeleting(true);
    try {
      const deleted = await repoDeleteMutation.mutateAsync(repoDeleteTarget.id);
      if (!deleted) return;

      toast("仓库已删除");
      logToConsole("info", "删除 Skill 仓库", repoDeleteTarget);
      setRepoDeleteTarget(null);
    } catch (error) {
      const formatted = formatActionFailureToast("删除仓库", error);
      logToConsole("error", "删除 Skill 仓库失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        repo: repoDeleteTarget,
      });
      toast(formatted.toast);
    } finally {
      setRepoDeleting(false);
    }
  }

  async function installSkillToCurrentCli(skill: AvailableSkillSummary, silent = false) {
    if (!activeWorkspaceId) {
      toast("未找到当前工作区（workspace）。请先在 Workspaces 页面创建并设为当前。");
      return null;
    }

    const next = await installToLocalMutation.mutateAsync({
      gitUrl: skill.source_git_url,
      branch: skill.source_branch,
      sourceSubdir: skill.source_subdir,
    });
    if (!next) return null;

    logToConsole("info", "安装 Skill 到当前 CLI", {
      cli: effectiveCli,
      workspace_id: activeWorkspaceId,
      source: sourceHint(skill),
      local_skill: next,
    });
    if (!silent) {
      toast(`已安装到 ${currentCli.name}`);
    }
    return next;
  }

  async function installSingleSkill(skill: AvailableSkillSummary) {
    if (installingRepoKey || installingSources.size > 0) return;

    const key = sourceKey(skill);
    setInstallingSources(new Set([key]));
    try {
      await installSkillToCurrentCli(skill);
    } catch (error) {
      const formatted = formatActionFailureToast("安装到当前 CLI", error);
      logToConsole("error", "安装 Skill 到当前 CLI 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: effectiveCli,
        skill,
      });
      toast(formatted.toast);
    } finally {
      setInstallingSources(new Set());
    }
  }

  async function installWholeRepo(group: RepoGroup) {
    if (installingRepoKey || installingSources.size > 0) return;

    const targets = group.skills.filter((skill) => getStatus(skill) === "not_installed");
    if (targets.length === 0) {
      toast("这个仓库下没有可安装的技能");
      return;
    }

    setInstallingRepoKey(group.key);
    setInstallingSources(new Set(targets.map(sourceKey)));

    let successCount = 0;
    let failedCount = 0;

    try {
      for (const skill of targets) {
        try {
          const next = await installSkillToCurrentCli(skill, true);
          if (next) successCount += 1;
        } catch (error) {
          failedCount += 1;
          const formatted = formatActionFailureToast("安装到当前 CLI", error);
          logToConsole("error", "批量安装 Skill 到当前 CLI 失败", {
            error: formatted.raw,
            error_code: formatted.error_code ?? undefined,
            cli: effectiveCli,
            repo: group.repoPath,
            skill,
          });
        }
      }
    } finally {
      setInstallingRepoKey(null);
      setInstallingSources(new Set());
    }

    if (successCount > 0) {
      toast(
        successCount === 1
          ? `已安装 1 个技能到 ${currentCli.name}`
          : `已安装 ${successCount} 个技能到 ${currentCli.name}`
      );
    }
    if (failedCount > 0) {
      toast(failedCount === 1 ? "有 1 个技能安装失败" : `有 ${failedCount} 个技能安装失败`);
    }
  }

  function toggleRepoExpanded(groupKey: string) {
    setExpandedRepos((previous) => {
      const next = new Set(previous);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      return next;
    });
  }

  return {
    navigate,
    orderedCliTabs,
    effectiveCli,
    setActiveCli,
    currentCli,
    repos,
    enabledRepoCount,
    activeWorkspaceId,
    loading,
    discovering,
    query,
    setQuery,
    repoFilter,
    setRepoFilter,
    onlyActionable,
    setOnlyActionable,
    expandedRepos,
    installingRepoKey,
    installingSources,
    installBusy,
    repoDialogOpen,
    setRepoDialogOpen,
    newRepoUrl,
    setNewRepoUrl,
    newRepoBranch,
    setNewRepoBranch,
    repoSaving,
    repoToggleId,
    repoDeleteTarget,
    setRepoDeleteTarget,
    repoDeleting,
    repoOptions,
    available,
    groupedAvailable,
    refreshAvailable,
    addRepo,
    toggleRepoEnabled,
    confirmDeleteRepo,
    installSingleSkill,
    installWholeRepo,
    toggleRepoExpanded,
    getStatus,
    statusLabel,
    statusTone,
    repositoryWebUrl,
    sourceHint,
    sourceKey,
    formatUnixSeconds,
  };
}
