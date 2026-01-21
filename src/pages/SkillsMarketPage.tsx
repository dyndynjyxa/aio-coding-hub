// Usage: Discover and install skills from repos. Backend commands: `skills_discover_available`, `skill_install`, `skill_repos_*`, `skills_installed_list`.

import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { CLIS, cliFromKeyOrDefault, enabledFlagForCli, isCliKey } from "../constants/clis";
import { logToConsole } from "../services/consoleLog";
import type { CliKey } from "../services/providers";
import {
  skillInstall,
  skillRepoDelete,
  skillRepoUpsert,
  skillReposList,
  skillsDiscoverAvailable,
  skillsInstalledList,
  type AvailableSkillSummary,
  type InstalledSkillSummary,
  type SkillRepoSummary,
} from "../services/skills";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { Switch } from "../ui/Switch";
import { formatActionFailureToast } from "../utils/errors";

function formatUnixSeconds(ts: number) {
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

type SkillSource = {
  source_git_url: string;
  source_branch: string;
  source_subdir: string;
};

function sourceKey(skill: SkillSource) {
  return `${skill.source_git_url}#${skill.source_branch}:${skill.source_subdir}`;
}

function readCliFromStorage(): CliKey {
  try {
    const raw = localStorage.getItem("skills.activeCli");
    if (isCliKey(raw)) return raw;
  } catch {}
  return "claude";
}

function writeCliToStorage(cli: CliKey) {
  try {
    localStorage.setItem("skills.activeCli", cli);
  } catch {}
}

function shortGitUrl(input: string) {
  const raw = input.trim();
  if (!raw) return raw;
  if (raw.startsWith("git@")) {
    const withoutPrefix = raw.slice("git@".length);
    const parts = withoutPrefix.split(":");
    if (parts.length === 2) {
      return `${parts[0]}/${parts[1].replace(/\.git$/i, "")}`;
    }
    return withoutPrefix.replace(/\.git$/i, "");
  }
  return raw
    .replace(/^https?:\/\//i, "")
    .replace(/\.git$/i, "")
    .replace(/\/+$/, "");
}

function repoKey(skill: Pick<SkillSource, "source_git_url" | "source_branch">) {
  return `${skill.source_git_url}#${skill.source_branch}`;
}

function enabledForCli(skill: InstalledSkillSummary, cli: CliKey) {
  return enabledFlagForCli(skill, cli);
}

type MarketStatus = "not_installed" | "needs_enable" | "enabled";

function statusLabel(status: MarketStatus) {
  if (status === "enabled") return "已启用";
  if (status === "needs_enable") return "未启用";
  return "未安装";
}

export function SkillsMarketPage() {
  const navigate = useNavigate();
  const [activeCli, setActiveCli] = useState<CliKey>(() => readCliFromStorage());
  const currentCli = useMemo(() => cliFromKeyOrDefault(activeCli), [activeCli]);

  const [repos, setRepos] = useState<SkillRepoSummary[]>([]);
  const [installed, setInstalled] = useState<InstalledSkillSummary[]>([]);
  const [available, setAvailable] = useState<AvailableSkillSummary[]>([]);

  const [loading, setLoading] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [installingSource, setInstallingSource] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [repoFilter, setRepoFilter] = useState<string>("all");
  const [onlyActionable, setOnlyActionable] = useState(true);
  const [sortMode, setSortMode] = useState<"actionable" | "name" | "repo">("actionable");

  const [repoDialogOpen, setRepoDialogOpen] = useState(false);
  const [newRepoUrl, setNewRepoUrl] = useState("");
  const [newRepoBranch, setNewRepoBranch] = useState("auto");
  const [repoSaving, setRepoSaving] = useState(false);
  const [repoToggleId, setRepoToggleId] = useState<number | null>(null);
  const [repoDeleteTarget, setRepoDeleteTarget] = useState<SkillRepoSummary | null>(null);
  const [repoDeleting, setRepoDeleting] = useState(false);

  const enabledRepoCount = useMemo(() => repos.filter((r) => r.enabled).length, [repos]);

  useEffect(() => {
    writeCliToStorage(activeCli);
  }, [activeCli]);

  async function refreshBase() {
    setLoading(true);
    try {
      const [repoRows, installedRows] = await Promise.all([
        skillReposList(),
        skillsInstalledList(),
      ]);

      setRepos(repoRows ?? []);
      setInstalled(installedRows ?? []);
    } catch (err) {
      logToConsole("error", "加载 Skill 市场数据失败", { error: String(err) });
      toast("加载失败：请查看控制台日志");
    } finally {
      setLoading(false);
    }
  }

  async function refreshAvailable(refresh: boolean, toastOnSuccess = true) {
    setDiscovering(true);
    try {
      const rows = await skillsDiscoverAvailable(refresh);
      if (!rows) {
        setAvailable([]);
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setAvailable(rows);
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
    } finally {
      setDiscovering(false);
    }
  }

  useEffect(() => {
    void refreshBase();
  }, []);

  useEffect(() => {
    void refreshAvailable(false, false);
  }, [enabledRepoCount]);

  const installedBySource = useMemo(() => {
    const map = new Map<string, InstalledSkillSummary>();
    for (const row of installed) {
      map.set(sourceKey(row), row);
    }
    return map;
  }, [installed]);

  const repoOptions = useMemo(() => {
    const map = new Map<string, { key: string; label: string }>();
    for (const row of available) {
      const key = repoKey(row);
      if (map.has(key)) continue;
      map.set(key, {
        key,
        label: `${shortGitUrl(row.source_git_url)} (${row.source_branch})`,
      });
    }
    return Array.from(map.values()).sort((a, b) => a.label.localeCompare(b.label));
  }, [available]);

  const filteredAvailable = useMemo(() => {
    const q = query.trim().toLowerCase();

    const rows = available.filter((row) => {
      if (repoFilter !== "all" && repoKey(row) !== repoFilter) return false;

      const installedRow = installedBySource.get(sourceKey(row));
      const status: MarketStatus = installedRow
        ? enabledForCli(installedRow, activeCli)
          ? "enabled"
          : "needs_enable"
        : "not_installed";

      if (onlyActionable && status === "enabled") return false;

      if (!q) return true;
      const haystack = [
        row.name,
        row.description,
        row.source_subdir,
        shortGitUrl(row.source_git_url),
        row.source_branch,
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });

    const getStatusRank = (row: AvailableSkillSummary) => {
      const installedRow = installedBySource.get(sourceKey(row));
      const status: MarketStatus = installedRow
        ? enabledForCli(installedRow, activeCli)
          ? "enabled"
          : "needs_enable"
        : "not_installed";
      if (status === "not_installed") return 0;
      if (status === "needs_enable") return 1;
      return 2;
    };

    const sorted = [...rows].sort((a, b) => {
      if (sortMode === "repo") {
        const repoA = shortGitUrl(a.source_git_url);
        const repoB = shortGitUrl(b.source_git_url);
        const repoCmp = repoA.localeCompare(repoB);
        if (repoCmp !== 0) return repoCmp;
        return a.name.localeCompare(b.name);
      }
      if (sortMode === "name") {
        return a.name.localeCompare(b.name);
      }

      const rankA = getStatusRank(a);
      const rankB = getStatusRank(b);
      if (rankA !== rankB) return rankA - rankB;
      return a.name.localeCompare(b.name);
    });

    return sorted;
  }, [activeCli, available, installedBySource, onlyActionable, query, repoFilter, sortMode]);

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
      const next = await skillRepoUpsert({
        repo_id: null,
        git_url: gitUrl,
        branch,
        enabled: true,
      });
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setRepos((prev) => [next, ...prev.filter((r) => r.id !== next.id)]);
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
      const next = await skillRepoUpsert({
        repo_id: repo.id,
        git_url: repo.git_url,
        branch: repo.branch,
        enabled,
      });
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setRepos((prev) => prev.map((r) => (r.id === repo.id ? next : r)));
      toast(enabled ? "仓库已启用" : "仓库已禁用");
    } catch (err) {
      const formatted = formatActionFailureToast("切换仓库", err);
      logToConsole("error", "切换仓库启用状态失败", {
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
    if (!repoDeleteTarget) return;
    if (repoDeleting) return;
    setRepoDeleting(true);
    try {
      const ok = await skillRepoDelete(repoDeleteTarget.id);
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }
      setRepos((prev) => prev.filter((r) => r.id !== repoDeleteTarget.id));
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

  async function installToCurrentCli(skill: AvailableSkillSummary) {
    const key = sourceKey(skill);
    if (installingSource != null) return;

    setInstallingSource(key);
    try {
      const flags = {
        enabled_claude: activeCli === "claude",
        enabled_codex: activeCli === "codex",
        enabled_gemini: activeCli === "gemini",
      };

      const next = await skillInstall({
        git_url: skill.source_git_url,
        branch: skill.source_branch,
        source_subdir: skill.source_subdir,
        ...flags,
      });

      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setInstalled((prev) => [next, ...prev.filter((row) => row.id !== next.id)]);
      setAvailable((prev) =>
        prev.map((row) => (sourceKey(row) === key ? { ...row, installed: true } : row))
      );
      toast("安装成功");
      logToConsole("info", "安装 Skill", { cli: activeCli, skill: next });
    } catch (err) {
      const formatted = formatActionFailureToast("安装", err);
      logToConsole("error", "安装 Skill 失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
        cli: activeCli,
        skill,
      });
      toast(formatted.toast);
    } finally {
      setInstallingSource(null);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Skill 市场</h1>
          <p className="mt-1 text-sm text-slate-600">
            从仓库扫描 `SKILL.md` 并安装到指定 CLI（Claude/Codex/Gemini）。
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
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
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {CLIS.map((cli) => (
          <Button
            key={cli.key}
            onClick={() => setActiveCli(cli.key)}
            variant={activeCli === cli.key ? "primary" : "secondary"}
          >
            {cli.name}
          </Button>
        ))}
        <span className="text-xs text-slate-500">
          已启用仓库：{enabledRepoCount} / {repos.length}
        </span>
      </div>

      <Card className="min-h-[260px]" padding="md">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="text-sm font-semibold">可安装</div>
            <div className="mt-1 text-xs text-slate-500">
              启用仓库后才会出现在这里；安装默认只启用当前 CLI（{currentCli.name}）。
            </div>
          </div>
          <span className="rounded-full bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700">
            {filteredAvailable.length} / {available.length}
          </span>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索 Skill（名称/描述/仓库/目录）"
            className="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-[#0052FF]/30 sm:w-[360px]"
          />

          <select
            value={repoFilter}
            onChange={(e) => setRepoFilter(e.target.value)}
            className="h-10 rounded-lg border border-slate-200 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-[#0052FF]/30"
          >
            <option value="all">全部仓库</option>
            {repoOptions.map((opt) => (
              <option key={opt.key} value={opt.key}>
                {opt.label}
              </option>
            ))}
          </select>

          <select
            value={sortMode}
            onChange={(e) => setSortMode(e.target.value as typeof sortMode)}
            className="h-10 rounded-lg border border-slate-200 bg-white px-3 text-sm outline-none focus:ring-2 focus:ring-[#0052FF]/30"
          >
            <option value="actionable">可操作优先</option>
            <option value="name">按名称</option>
            <option value="repo">按仓库</option>
          </select>

          <div className="flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 py-2">
            <span className="text-xs text-slate-600">仅显示可操作</span>
            <Switch checked={onlyActionable} onCheckedChange={setOnlyActionable} />
          </div>

          {query ? (
            <Button size="sm" variant="ghost" onClick={() => setQuery("")}>
              清空
            </Button>
          ) : null}
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-2">
          {loading ? (
            <div className="text-sm text-slate-600">加载中…</div>
          ) : discovering ? (
            <div className="text-sm text-slate-600">扫描中…</div>
          ) : enabledRepoCount === 0 ? (
            <div className="text-sm text-slate-600">
              暂无启用的仓库。请先点右上角「管理仓库」添加并启用。
            </div>
          ) : filteredAvailable.length === 0 ? (
            <div className="text-sm text-slate-600">
              没有匹配结果。可尝试：清空搜索 / 关闭“仅显示可操作” / 切换仓库 /
              点击右上角「刷新发现」。
            </div>
          ) : (
            filteredAvailable.map((skill) => {
              const key = sourceKey(skill);
              const installing = installingSource === key;
              const installedRow = installedBySource.get(key);
              const status: MarketStatus = installedRow
                ? enabledForCli(installedRow, activeCli)
                  ? "enabled"
                  : "needs_enable"
                : "not_installed";
              const statusTone =
                status === "enabled"
                  ? "bg-emerald-50 text-emerald-700"
                  : status === "needs_enable"
                    ? "bg-amber-50 text-amber-800"
                    : "bg-slate-100 text-slate-600";

              return (
                <div key={key} className="rounded-xl border border-slate-200 bg-white p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <div className="truncate text-sm font-semibold">{skill.name}</div>
                        <span
                          className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${statusTone}`}
                        >
                          {statusLabel(status)}
                        </span>
                      </div>

                      {skill.description ? (
                        <div className="mt-1 text-xs text-slate-500">{skill.description}</div>
                      ) : null}

                      <div className="mt-2 truncate font-mono text-xs text-slate-500">
                        {shortGitUrl(skill.source_git_url)}:{skill.source_subdir}
                      </div>
                    </div>

                    {status === "not_installed" ? (
                      <Button
                        size="sm"
                        variant="primary"
                        disabled={installing}
                        onClick={() => void installToCurrentCli(skill)}
                      >
                        {installing ? "安装中…" : `安装到 ${currentCli.name}`}
                      </Button>
                    ) : status === "needs_enable" ? (
                      <Button size="sm" variant="primary" onClick={() => navigate("/skills")}>
                        去 Skill 启用
                      </Button>
                    ) : (
                      <Button size="sm" variant="secondary" disabled>
                        已启用
                      </Button>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </Card>

      <Dialog
        open={repoDialogOpen}
        title="Skill 仓库"
        description="Git 仓库列表（启用后参与发现）。提示：刷新发现会对缓存仓库执行 fetch/checkout/reset（仅影响 ~/.aio-coding-hub/skill-repos 下的缓存目录）。"
        onOpenChange={setRepoDialogOpen}
      >
        <div className="space-y-4">
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3">
            <div className="text-sm font-semibold">添加仓库</div>
            <div className="mt-2 grid gap-3 sm:grid-cols-3">
              <div className="sm:col-span-2">
                <div className="text-xs font-medium text-slate-600">Git URL</div>
                <input
                  value={newRepoUrl}
                  onChange={(e) => setNewRepoUrl(e.target.value)}
                  placeholder="https://github.com/owner/repo"
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-[#0052FF]/30"
                />
              </div>
              <div>
                <div className="text-xs font-medium text-slate-600">Branch</div>
                <input
                  value={newRepoBranch}
                  onChange={(e) => setNewRepoBranch(e.target.value)}
                  placeholder="auto / main / master"
                  className="mt-1 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-[#0052FF]/30"
                />
                <div className="mt-1 text-[11px] text-slate-500">
                  推荐使用 <span className="font-mono">auto</span>（自动使用仓库默认分支）。
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
              <span className="text-xs text-slate-500">{repos.length} 个</span>
            </div>

            {repos.length === 0 ? (
              <div className="text-sm text-slate-600">
                暂无仓库。添加后点击页面右上角「刷新发现」即可扫描可安装 Skill。
              </div>
            ) : (
              repos.map((repo) => (
                <div key={repo.id} className="rounded-xl border border-slate-200 bg-white p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="break-all text-sm font-medium">{repo.git_url}</div>
                      <div className="mt-1 text-xs text-slate-500">
                        branch: <span className="font-mono">{repo.branch}</span>
                      </div>
                      <div className="mt-1 text-xs text-slate-400">
                        更新 {formatUnixSeconds(repo.updated_at)}
                      </div>
                    </div>

                    <div className="flex shrink-0 items-center gap-2">
                      <span className="text-xs text-slate-600">启用</span>
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
                </div>
              ))
            )}
          </div>
        </div>
      </Dialog>

      <Dialog
        open={repoDeleteTarget != null}
        title="删除仓库"
        description="该操作仅会从本地数据库移除仓库记录，不会删除你的 Git 仓库。"
        onOpenChange={(open) => {
          if (!open) setRepoDeleteTarget(null);
        }}
      >
        <div className="space-y-3">
          <div className="text-sm text-slate-700">确认删除以下仓库？</div>
          <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600">
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
