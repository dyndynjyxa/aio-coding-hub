// Usage: Discover and install skills from repos. Backend commands: `skills_discover_available`, `skill_install_to_local`, `skill_repos_*`, `skills_installed_list`, `skills_local_list`.

import { ChevronDown, ChevronRight, ExternalLink } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { CLIS, cliFromKeyOrDefault, isCliKey } from "../constants/clis";
import { useSettingsQuery } from "../query/settings";
import {
  useSkillInstallToLocalMutation,
  useSkillRepoDeleteMutation,
  useSkillRepoUpsertMutation,
  useSkillReposListQuery,
  useSkillsDiscoverAvailableMutation,
  useSkillsDiscoverAvailableQuery,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
} from "../query/skills";
import { useWorkspacesListQuery } from "../query/workspaces";
import { logToConsole } from "../services/consoleLog";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cliPriorityOrder";
import type { CliKey } from "../services/providers";
import type {
  AvailableSkillSummary,
  InstalledSkillSummary,
  SkillRepoSummary,
} from "../services/skills";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { EmptyState } from "../ui/EmptyState";
import { Spinner } from "../ui/Spinner";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import { formatActionFailureToast } from "../utils/errors";
import {
  normalizeRepoPath,
  repoKey,
  repoPrefixFromGitUrl,
  repositoryWebUrl,
  sourceHint,
  sourceKey,
} from "../utils/skillSources";

function formatUnixSeconds(ts: number) {
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

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

function statusLabel(status: MarketStatus) {
  if (status === "local_installed") return "已装到当前 CLI";
  if (status === "enabled") return "已在通用技能";
  if (status === "needs_enable") return "通用技能未启用";
  return "未安装";
}

function statusTone(status: MarketStatus) {
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

export function SkillsMarketPage() {
  const navigate = useNavigate();
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;
  const [activeCli, setActiveCli] = useState<CliKey | null>(() => readCliFromStorage());
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
  const installToLocalMutation = useSkillInstallToLocalMutation(activeWorkspaceId ?? 0);

  const loading =
    reposQuery.isLoading ||
    workspacesQuery.isLoading ||
    installedQuery.isLoading ||
    (Boolean(activeWorkspaceId) && localQuery.isLoading);
  const discovering = discoverMutation.isPending || availableQuery.isFetching;

  const [query, setQuery] = useState("");
  const [repoFilter, setRepoFilter] = useState<string>("all");
  const [onlyActionable, setOnlyActionable] = useState(true);
  const [expandedRepos, setExpandedRepos] = useState<Set<string>>(new Set());
  const [installingRepoKey, setInstallingRepoKey] = useState<string | null>(null);
  const [installingSources, setInstallingSources] = useState<Set<string>>(new Set());
  const installBusy = installingRepoKey != null || installingSources.size > 0;

  const [repoDialogOpen, setRepoDialogOpen] = useState(false);
  const [newRepoUrl, setNewRepoUrl] = useState("");
  const [newRepoBranch, setNewRepoBranch] = useState("auto");
  const [repoSaving, setRepoSaving] = useState(false);
  const [repoToggleId, setRepoToggleId] = useState<number | null>(null);
  const [repoDeleteTarget, setRepoDeleteTarget] = useState<SkillRepoSummary | null>(null);
  const [repoDeleting, setRepoDeleting] = useState(false);

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
    const map = new Map<string, { key: string; label: string }>();
    for (const row of available) {
      const key = repoKey(row);
      if (map.has(key)) continue;
      const repoPath = normalizeRepoPath(row.source_git_url) || row.source_git_url;
      map.set(key, {
        key,
        label: `${repoPath} (${row.source_branch})`,
      });
    }
    return Array.from(map.values()).sort((a, b) => a.label.localeCompare(b.label));
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
  }, [available, onlyActionable, query, repoFilter, getStatus]);

  const groupedAvailable = useMemo(() => {
    const map = new Map<string, RepoGroup>();

    for (const skill of filteredAvailable) {
      const key = repoKey(skill);
      const group = map.get(key) ?? {
        key,
        gitUrl: skill.source_git_url,
        branch: skill.source_branch,
        repoPath: normalizeRepoPath(skill.source_git_url) || skill.source_git_url,
        repoPrefix: repoPrefixFromGitUrl(skill.source_git_url) ?? "仓库",
        skills: [] as AvailableSkillSummary[],
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
      map.set(key, group);
    }

    return Array.from(map.values())
      .map((group) => ({
        ...group,
        skills: [...group.skills].sort((a, b) => {
          const rank = statusRank(getStatus(a)) - statusRank(getStatus(b));
          if (rank !== 0) return rank;
          return a.name.localeCompare(b.name);
        }),
      }))
      .sort((a, b) => {
        if (a.installableCount !== b.installableCount) {
          return b.installableCount - a.installableCount;
        }
        return a.repoPath.localeCompare(b.repoPath);
      });
  }, [filteredAvailable, getStatus]);

  useEffect(() => {
    setExpandedRepos((prev) => {
      const allowed = new Set(groupedAvailable.map((group) => group.key));
      const next = new Set(Array.from(prev).filter((key) => allowed.has(key)));

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
    } catch (err) {
      const formatted = formatActionFailureToast("刷新发现", err);
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
    } catch (err) {
      const formatted = formatActionFailureToast("添加仓库", err);
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
    } catch (err) {
      const formatted = formatActionFailureToast("切换仓库", err);
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
      const ok = await repoDeleteMutation.mutateAsync(repoDeleteTarget.id);
      if (!ok) return;

      toast("仓库已删除");
      logToConsole("info", "删除 Skill 仓库", repoDeleteTarget);
      setRepoDeleteTarget(null);
    } catch (err) {
      const formatted = formatActionFailureToast("删除仓库", err);
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
    } catch (err) {
      const formatted = formatActionFailureToast("安装到当前 CLI", err);
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
        } catch (err) {
          failedCount += 1;
          const formatted = formatActionFailureToast("安装到当前 CLI", err);
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
    setExpandedRepos((prev) => {
      const next = new Set(prev);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      return next;
    });
  }

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">Skill 市场</h1>
          <Button onClick={() => navigate("/skills")} variant="secondary">
            返回 Skill
          </Button>
          <Button onClick={() => setRepoDialogOpen(true)} variant="secondary">
            管理仓库
          </Button>
          <Button
            onClick={() => void refreshAvailable(true)}
            variant="primary"
            disabled={discovering}
          >
            {discovering ? "刷新中…" : "刷新发现"}
          </Button>
        </div>

        <TabList
          ariaLabel="CLI 选择"
          items={orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name }))}
          value={effectiveCli}
          onChange={setActiveCli}
        />
      </div>

      <Card
        padding="md"
        className="border-slate-200/80 bg-[radial-gradient(circle_at_top_left,_rgba(14,165,233,0.08),_transparent_45%),linear-gradient(135deg,rgba(255,255,255,0.98),rgba(248,250,252,0.96))] dark:border-slate-700 dark:bg-slate-900"
      >
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="max-w-3xl">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              按仓库浏览，默认直接安装到当前 CLI
            </div>
            <div className="mt-1 text-xs leading-6 text-slate-600 dark:text-slate-400">
              现在市场页默认把技能装进 <span className="font-medium">{currentCli.name}</span>{" "}
              的本机目录，不会先进入通用技能区。需要统一管理时，再去 Skill 页面导入到通用技能。
            </div>
            {!activeWorkspaceId ? (
              <div className="mt-2 text-xs text-amber-700 dark:text-amber-400">
                当前还没有激活的 workspace，安装前请先去 Workspaces 页面设置当前工作区。
              </div>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2 text-xs">
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-slate-700 shadow-sm dark:bg-slate-800 dark:text-slate-300">
              已启用仓库 {enabledRepoCount} / {repos.length}
            </span>
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-slate-700 shadow-sm dark:bg-slate-800 dark:text-slate-300">
              当前 CLI {currentCli.name}
            </span>
            <span className="rounded-full bg-white/80 px-3 py-1.5 text-slate-700 shadow-sm dark:bg-slate-800 dark:text-slate-300">
              发现技能 {available.length}
            </span>
          </div>
        </div>
      </Card>

      <Card
        className="min-h-0 flex flex-1 flex-col overflow-hidden"
        padding="md"
        data-testid="skills-market-list-card"
      >
        <div className="flex flex-wrap items-center gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索技能、仓库、目录"
            className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100 sm:w-[320px]"
          />

          <select
            value={repoFilter}
            onChange={(e) => setRepoFilter(e.target.value)}
            className="h-10 rounded-xl border border-slate-200 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100"
          >
            <option value="all">全部仓库</option>
            {repoOptions.map((option) => (
              <option key={option.key} value={option.key}>
                {option.label}
              </option>
            ))}
          </select>

          <div className="flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-600 dark:bg-slate-800">
            <span className="text-xs text-slate-600 dark:text-slate-400">仅显示可安装</span>
            <Switch checked={onlyActionable} onCheckedChange={setOnlyActionable} />
          </div>

          {query ? (
            <Button size="sm" variant="ghost" onClick={() => setQuery("")}>
              清空
            </Button>
          ) : null}
        </div>

        <div
          className="mt-4 min-h-0 flex-1 overflow-y-auto pr-1 scrollbar-overlay"
          data-testid="skills-market-scroll-region"
        >
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
              <Spinner size="sm" />
              加载中…
            </div>
          ) : discovering ? (
            <div className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
              <Spinner size="sm" />
              扫描中…
            </div>
          ) : enabledRepoCount === 0 ? (
            <EmptyState
              title="暂无启用的仓库"
              description="先添加并启用仓库，再点击右上角“刷新发现”。"
            />
          ) : groupedAvailable.length === 0 ? (
            <EmptyState
              title="没有匹配的仓库或技能"
              description="可以试试清空搜索、切换仓库，或者关闭“仅显示可安装”。"
            />
          ) : (
            <div className="space-y-3">
              {groupedAvailable.map((group) => {
                const expanded = expandedRepos.has(group.key);
                const repoUrl = repositoryWebUrl(group.gitUrl);

                return (
                  <section
                    key={group.key}
                    className="rounded-2xl border border-slate-200/80 bg-[linear-gradient(180deg,rgba(248,250,252,0.96),rgba(255,255,255,0.92))] p-4 dark:border-slate-700 dark:bg-slate-900"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-4">
                      <div className="min-w-0 flex-1">
                        <button
                          type="button"
                          onClick={() => toggleRepoExpanded(group.key)}
                          className="flex min-w-0 items-start gap-3 text-left"
                        >
                          <span className="mt-0.5 rounded-full border border-slate-200 bg-white p-1 text-slate-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                            {expanded ? (
                              <ChevronDown className="h-4 w-4" />
                            ) : (
                              <ChevronRight className="h-4 w-4" />
                            )}
                          </span>
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="truncate text-base font-semibold text-slate-900 dark:text-slate-100">
                                {group.repoPrefix}
                              </span>
                              <span className="rounded-full bg-slate-900 px-2.5 py-1 text-[11px] font-medium text-white dark:bg-slate-100 dark:text-slate-900">
                                {group.skills.length} 个技能
                              </span>
                              {group.installableCount > 0 ? (
                                <span className="rounded-full bg-sky-50 px-2.5 py-1 text-[11px] font-medium text-sky-700 dark:bg-sky-900/30 dark:text-sky-300">
                                  可安装 {group.installableCount}
                                </span>
                              ) : null}
                              {group.localCount > 0 ? (
                                <span className="rounded-full bg-sky-50 px-2.5 py-1 text-[11px] font-medium text-sky-700 dark:bg-sky-900/30 dark:text-sky-300">
                                  本机 {group.localCount}
                                </span>
                              ) : null}
                              {group.enabledCount > 0 ? (
                                <span className="rounded-full bg-emerald-50 px-2.5 py-1 text-[11px] font-medium text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400">
                                  通用已启用 {group.enabledCount}
                                </span>
                              ) : null}
                            </div>
                            <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                              <span className="font-mono">{group.repoPath}</span>
                              <span>branch: {group.branch}</span>
                            </div>
                          </div>
                        </button>
                      </div>

                      <div className="flex flex-wrap items-center justify-end gap-2">
                        {repoUrl ? (
                          <a
                            href={repoUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-slate-200 bg-white text-slate-500 transition hover:text-slate-900 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:text-slate-100"
                            title={group.repoPath}
                          >
                            <ExternalLink className="h-4 w-4" />
                          </a>
                        ) : null}
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => toggleRepoExpanded(group.key)}
                        >
                          {expanded ? "收起" : "展开"}
                        </Button>
                        <Button
                          size="sm"
                          variant="primary"
                          disabled={group.installableCount === 0 || installBusy}
                          onClick={() => void installWholeRepo(group)}
                        >
                          {installingRepoKey === group.key ? "安装中…" : `安装本仓库全部技能`}
                        </Button>
                      </div>
                    </div>

                    {!expanded ? (
                      <div className="mt-3 flex flex-wrap gap-2">
                        {group.skills.slice(0, 4).map((skill) => (
                          <span
                            key={sourceKey(skill)}
                            className="rounded-full border border-slate-200 bg-white px-3 py-1 text-xs text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300"
                          >
                            {skill.name}
                          </span>
                        ))}
                        {group.skills.length > 4 ? (
                          <span className="rounded-full border border-dashed border-slate-300 px-3 py-1 text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
                            还有 {group.skills.length - 4} 个
                          </span>
                        ) : null}
                      </div>
                    ) : (
                      <div className="mt-4 space-y-2">
                        {group.skills.map((skill) => {
                          const key = sourceKey(skill);
                          const status = getStatus(skill);
                          const installing = installingSources.has(key);

                          return (
                            <div
                              key={key}
                              className="rounded-xl border border-slate-200 bg-white/90 px-3 py-3 dark:border-slate-700 dark:bg-slate-800"
                            >
                              <div className="flex flex-wrap items-start gap-3">
                                <div className="min-w-0 flex-1">
                                  <div className="flex flex-wrap items-center gap-2">
                                    <span className="min-w-0 truncate text-sm font-semibold text-slate-900 dark:text-slate-100">
                                      {skill.name}
                                    </span>
                                    <span
                                      className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${statusTone(status)}`}
                                    >
                                      {statusLabel(status)}
                                    </span>
                                    {repoUrl ? (
                                      <a
                                        href={repoUrl}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                                        title={sourceHint(skill)}
                                      >
                                        <ExternalLink className="h-3.5 w-3.5" />
                                      </a>
                                    ) : null}
                                  </div>
                                  {skill.description ? (
                                    <div className="mt-1.5 text-xs text-slate-500 dark:text-slate-400">
                                      {skill.description}
                                    </div>
                                  ) : null}
                                  <div className="mt-2 truncate font-mono text-[11px] text-slate-400 dark:text-slate-500">
                                    {skill.source_subdir}
                                  </div>
                                </div>

                                <div className="ms-auto flex shrink-0 flex-wrap items-center justify-end gap-2">
                                  {status === "not_installed" ? (
                                    <Button
                                      size="sm"
                                      variant="primary"
                                      disabled={installBusy}
                                      onClick={() => void installSingleSkill(skill)}
                                    >
                                      {installing ? "安装中…" : `安装到 ${currentCli.name}`}
                                    </Button>
                                  ) : status === "needs_enable" ? (
                                    <Button
                                      size="sm"
                                      variant="secondary"
                                      onClick={() => navigate("/skills")}
                                    >
                                      去通用技能
                                    </Button>
                                  ) : status === "local_installed" ? (
                                    <Button
                                      size="sm"
                                      variant="secondary"
                                      onClick={() => navigate("/skills")}
                                    >
                                      查看本机已安装
                                    </Button>
                                  ) : (
                                    <Button size="sm" variant="secondary" disabled>
                                      已在通用技能
                                    </Button>
                                  )}
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </section>
                );
              })}
            </div>
          )}
        </div>
      </Card>

      <Dialog
        open={repoDialogOpen}
        title="Skill 仓库"
        description="启用后的仓库会参与发现。刷新发现只会更新 ~/.aio-coding-hub/skill-repos 下的缓存副本，不会动你的原始仓库。"
        onOpenChange={setRepoDialogOpen}
      >
        <div className="space-y-4">
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800">
            <div className="text-sm font-semibold">添加仓库</div>
            <div className="mt-2 grid gap-3 sm:grid-cols-3">
              <div className="sm:col-span-2">
                <div className="text-xs font-medium text-slate-600 dark:text-slate-400">
                  Git URL
                </div>
                <input
                  value={newRepoUrl}
                  onChange={(e) => setNewRepoUrl(e.target.value)}
                  placeholder="https://github.com/owner/repo"
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100"
                />
              </div>
              <div>
                <div className="text-xs font-medium text-slate-600 dark:text-slate-400">Branch</div>
                <input
                  value={newRepoBranch}
                  onChange={(e) => setNewRepoBranch(e.target.value)}
                  placeholder="auto / main / master"
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-accent/30 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100"
                />
                <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                  推荐使用 <span className="font-mono">auto</span>。
                </div>
              </div>
            </div>
            <div className="mt-3 flex justify-end">
              <Button onClick={() => void addRepo()} variant="primary" disabled={repoSaving}>
                {repoSaving ? "添加中…" : "添加仓库"}
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-semibold">仓库列表</div>
              <span className="text-xs text-slate-500 dark:text-slate-400">{repos.length} 个</span>
            </div>

            {repos.length === 0 ? (
              <EmptyState
                title="暂无仓库"
                description="添加后点击页面右上角“刷新发现”即可扫描技能。"
              />
            ) : (
              repos.map((repo) => {
                const repoUrl = repositoryWebUrl(repo.git_url);
                return (
                  <div
                    key={repo.id}
                    className="rounded-xl border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-800"
                  >
                    <div className="flex items-center gap-2">
                      <span className="min-w-0 truncate text-sm font-medium">{repo.git_url}</span>
                      {repoUrl ? (
                        <a
                          href={repoUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="shrink-0 text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
                          title={repo.git_url}
                        >
                          <ExternalLink className="h-3.5 w-3.5" />
                        </a>
                      ) : null}
                      <div className="ms-auto flex items-center gap-2">
                        <span className="text-xs text-slate-600 dark:text-slate-400">启用</span>
                        <Switch
                          checked={repo.enabled}
                          disabled={repoToggleId === repo.id || repoDeleting}
                          onCheckedChange={(next) => void toggleRepoEnabled(repo, next)}
                        />
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={repoDeleting}
                          onClick={() => setRepoDeleteTarget(repo)}
                        >
                          删除
                        </Button>
                      </div>
                    </div>
                    <div className="mt-1.5 flex items-center gap-3 text-xs text-slate-500 dark:text-slate-400">
                      <span>
                        branch: <span className="font-mono">{repo.branch}</span>
                      </span>
                      <span>更新 {formatUnixSeconds(repo.updated_at)}</span>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </Dialog>

      <Dialog
        open={repoDeleteTarget != null}
        title="删除仓库"
        description="这只会移除本地记录，不会删除你的 Git 仓库。"
        onOpenChange={(open) => {
          if (!open) setRepoDeleteTarget(null);
        }}
      >
        <div className="space-y-3">
          <div className="text-sm text-slate-700 dark:text-slate-300">确认删除以下仓库？</div>
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400">
            <div className="break-all font-mono">{repoDeleteTarget?.git_url}</div>
            <div className="mt-1">
              branch: <span className="font-mono">{repoDeleteTarget?.branch}</span>
            </div>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              variant="secondary"
              onClick={() => setRepoDeleteTarget(null)}
              disabled={repoDeleting}
            >
              取消
            </Button>
            <Button
              variant="primary"
              onClick={() => void confirmDeleteRepo()}
              disabled={repoDeleting}
            >
              {repoDeleting ? "删除中…" : "确认删除"}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
