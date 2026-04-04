// Usage: Session viewer entry (projects list). Backend commands: `cli_sessions_projects_list`.

import { useEffect, useMemo, useState, useRef } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Clock, FolderOpen, Hash, Search } from "lucide-react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { toast } from "sonner";
import type { CliSessionsSource, CliSessionsProjectSummary } from "../services/cliSessions";
import { copyText } from "../services/clipboard";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { Spinner } from "../ui/Spinner";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { useCliSessionsProjectsListQuery } from "../query/cliSessions";
import { useSettingsQuery } from "../query/settings";
import { useWslDetectionQuery } from "../query/wsl";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cliPriorityOrder";
import { cliShortLabel } from "../constants/clis";
import { cn } from "../utils/cn";
import { isWindowsRuntime } from "../utils/platform";
import { formatRelativeTimeFromUnixSeconds, formatUnixSeconds } from "../utils/formatters";

type ProjectSortKey = "recent" | "sessions" | "name";

function normalizeSource(raw: string | null): CliSessionsSource | null {
  if (raw === "claude" || raw === "codex") return raw;
  return null;
}

function pickProjects(data: CliSessionsProjectSummary[] | null | undefined) {
  return data ?? [];
}

function sourceDirHint(source: CliSessionsSource, distro?: string) {
  if (distro) {
    if (source === "claude") return `\\\\wsl$\\${distro}\\~/.claude/projects`;
    return `\\\\wsl$\\${distro}\\~/.codex/sessions`;
  }
  if (source === "claude") return "~/.claude/projects";
  return "$CODEX_HOME/sessions 或 ~/.codex/sessions";
}

function projectDisplayName(project: CliSessionsProjectSummary) {
  return project.short_name?.trim() || project.id;
}

function projectMatchesQuery(project: CliSessionsProjectSummary, query: string) {
  if (!query) return true;
  const q = query.toLowerCase();
  if (projectDisplayName(project).toLowerCase().includes(q)) return true;
  if (project.display_path.toLowerCase().includes(q)) return true;
  if (project.model_provider?.toLowerCase().includes(q)) return true;
  return false;
}

function compareProject(
  sortKey: ProjectSortKey,
  a: CliSessionsProjectSummary,
  b: CliSessionsProjectSummary
) {
  if (sortKey === "sessions") {
    return b.session_count - a.session_count;
  }
  if (sortKey === "name") {
    return projectDisplayName(a).localeCompare(projectDisplayName(b));
  }
  const aTime = a.last_modified ?? -1;
  const bTime = b.last_modified ?? -1;
  return bTime - aTime;
}

export function SessionsPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const isWindows = isWindowsRuntime();
  const settingsQuery = useSettingsQuery();
  const orderedSourceTabs = getOrderedClis(settingsQuery.data?.cli_priority_order, [
    "claude",
    "codex",
  ])
    .map((cli) => normalizeSource(cli.key))
    .filter((cliKey): cliKey is CliSessionsSource => cliKey != null)
    .map((cliKey) => ({
      key: cliKey,
      label: cliShortLabel(cliKey),
    }));
  const defaultSource =
    normalizeSource(
      pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, ["claude", "codex"])
    ) ?? "claude";
  const source = normalizeSource(searchParams.get("source")) ?? defaultSource;
  const distroParam = searchParams.get("distro")?.trim() ?? "";
  const [filterText, setFilterText] = useState("");
  const [sortKey, setSortKey] = useState<ProjectSortKey>("recent");
  const parentRef = useRef<HTMLDivElement>(null);

  const wslDetection = useWslDetectionQuery({ enabled: isWindows });
  const wslDistros = useMemo(
    () => (wslDetection.data?.detected ? wslDetection.data.distros : []),
    [wslDetection.data]
  );
  const showEnvSelector = isWindows && wslDistros.length > 0;
  const activeDistro =
    !isWindows || !distroParam
      ? undefined
      : !wslDetection.isFetched || wslDistros.includes(distroParam)
        ? distroParam
        : undefined;

  const projectsQuery = useCliSessionsProjectsListQuery(source, activeDistro);
  const projects = useMemo(() => pickProjects(projectsQuery.data), [projectsQuery.data]);
  const filteredProjects = useMemo(() => {
    const q = filterText.trim();
    const next = q ? projects.filter((p) => projectMatchesQuery(p, q)) : projects;
    return [...next].sort((a, b) => compareProject(sortKey, a, b));
  }, [filterText, projects, sortKey]);

  const rowVirtualizer = useVirtualizer({
    count: filteredProjects.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 80,
    overscan: 5,
  });

  const stats = useMemo(() => {
    const totalProjects = projects.length;
    const totalSessions = projects.reduce((sum, p) => sum + p.session_count, 0);
    const lastModified = projects.reduce<number | null>((acc, p) => {
      if (p.last_modified == null) return acc;
      if (acc == null) return p.last_modified;
      return Math.max(acc, p.last_modified);
    }, null);
    return { totalProjects, totalSessions, lastModified };
  }, [projects]);

  const loading = projectsQuery.isLoading;
  const available: boolean | null = loading ? null : projectsQuery.data != null;

  useEffect(() => {
    setFilterText("");
    setSortKey("recent");
  }, [activeDistro, source]);

  function updateSearchParams(nextSource: CliSessionsSource, nextDistro: string) {
    const params: Record<string, string> = { source: nextSource };
    if (nextDistro) params.distro = nextDistro;
    setSearchParams(params, { replace: true });
  }

  function handleSourceChange(next: CliSessionsSource) {
    updateSearchParams(next, distroParam);
  }

  function handleDistroChange(next: string) {
    updateSearchParams(source, next);
  }

  const envLabel = activeDistro ? `WSL: ${activeDistro}` : "Windows";

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title="Session 会话"
        subtitle="从本地会话文件读取（项目 → 会话 → 消息）"
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {showEnvSelector ? (
              <Select
                value={activeDistro ?? ""}
                onChange={(e) => handleDistroChange(e.currentTarget.value)}
                className="h-9 w-44 text-xs"
                aria-label="运行环境"
              >
                <option value="">Windows</option>
                {wslDistros.map((d) => (
                  <option key={d} value={d}>
                    WSL: {d}
                  </option>
                ))}
              </Select>
            ) : null}
            <TabList
              ariaLabel="来源切换"
              items={orderedSourceTabs}
              value={source}
              onChange={handleSourceChange}
            />
          </div>
        }
      />

      {projectsQuery.error ? (
        <ErrorState
          title="加载项目失败"
          message={String(projectsQuery.error)}
          onRetry={() => void projectsQuery.refetch()}
        />
      ) : available === null ? (
        <div className="flex items-center justify-center py-10">
          <Spinner />
        </div>
      ) : projects.length === 0 ? (
        <EmptyState
          title="未找到任何项目"
          description={
            activeDistro
              ? `请确认 WSL ${activeDistro} 中 ${source === "claude" ? "~/.claude/projects" : "~/.codex/sessions"} 目录存在并且包含会话文件。`
              : source === "claude"
                ? "请确认 ~/.claude/projects 目录存在并且包含会话文件。"
                : "请确认 $CODEX_HOME/sessions 或 ~/.codex/sessions 目录存在并且包含会话文件。"
          }
          variant="dashed"
        />
      ) : (
        <div className="grid gap-4 lg:flex-1 lg:min-h-0 lg:grid-cols-[360px_1fr] lg:items-stretch lg:overflow-hidden">
          <Card padding="md" className="flex flex-col gap-4 lg:min-h-0">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">概览</div>
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  当前来源：<span className="font-semibold">{source}</span>
                  {activeDistro ? (
                    <span className="ml-2 rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                      {envLabel}
                    </span>
                  ) : null}
                </div>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => void copyText(sourceDirHint(source, activeDistro))}
                className="h-9"
                title="复制数据源路径提示"
              >
                复制路径提示
              </Button>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
                <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                  项目
                </div>
                <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                  {stats.totalProjects}
                </div>
              </div>
              <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
                <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                  会话总数
                </div>
                <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                  {stats.totalSessions}
                </div>
              </div>
              <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
                <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                  最近更新
                </div>
                <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                  {stats.lastModified != null
                    ? formatRelativeTimeFromUnixSeconds(stats.lastModified)
                    : "—"}
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">数据源</div>
              <div className="mt-2 text-xs text-slate-600 dark:text-slate-400">
                <div className="flex items-start justify-between gap-2">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">目录</span>
                  <span className="min-w-0 text-right font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    {sourceDirHint(source, activeDistro)}
                  </span>
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                操作提示
              </div>
              <ul className="mt-2 space-y-1 text-xs text-slate-600 dark:text-slate-400">
                <li>1) 选择项目进入会话列表</li>
                <li>2) 在会话列表中复制恢复命令或查看消息</li>
                <li>3) 消息页支持"加载更多"</li>
              </ul>
            </div>
          </Card>

          <Card padding="sm" className="flex flex-col lg:h-full">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <div className="flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
                  <FolderOpen className="h-4 w-4 shrink-0 text-accent" />
                  <span className="shrink-0">项目</span>
                  <span className="shrink-0 text-xs font-medium text-slate-500 dark:text-slate-400">
                    {filteredProjects.length}/{projects.length}
                  </span>
                </div>
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  在左侧查看数据源与统计;点击项目进入会话列表。
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Select
                  value={sortKey}
                  onChange={(e) => setSortKey(e.currentTarget.value as ProjectSortKey)}
                  className="h-9 w-32 text-xs"
                  aria-label="排序"
                >
                  <option value="recent">最近更新</option>
                  <option value="sessions">会话最多</option>
                  <option value="name">名称 A→Z</option>
                </Select>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => void projectsQuery.refetch()}
                  className="h-9"
                >
                  刷新
                </Button>
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
                  placeholder="搜索项目名 / 路径 / Provider"
                  className="pl-9"
                  aria-label="搜索项目"
                />
              </div>
            </div>

            <div className="mt-3 hidden grid-cols-[1fr_110px_140px] gap-3 px-3 text-[11px] font-semibold text-slate-500 dark:text-slate-400 sm:grid">
              <span>项目</span>
              <span className="text-right">会话</span>
              <span className="text-right">最近更新</span>
            </div>

            <div
              ref={(node) => {
                if (node) parentRef.current = node;
              }}
              className="mt-2 lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1 scrollbar-overlay"
            >
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  position: "relative",
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualItem) => {
                  const project = filteredProjects[virtualItem.index];
                  const modifiedLabel =
                    project.last_modified != null
                      ? formatRelativeTimeFromUnixSeconds(project.last_modified)
                      : "—";
                  const modifiedTitle =
                    project.last_modified != null ? formatUnixSeconds(project.last_modified) : "—";

                  const navUrl = activeDistro
                    ? `/sessions/${source}/${encodeURIComponent(project.id)}?distro=${encodeURIComponent(activeDistro)}`
                    : `/sessions/${source}/${encodeURIComponent(project.id)}`;

                  return (
                    <div
                      key={virtualItem.key}
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        transform: `translateY(${virtualItem.start}px)`,
                        paddingBottom: "4px",
                      }}
                    >
                      <button
                        type="button"
                        onClick={() => {
                          if (!project.id.trim()) {
                            toast("无效项目 ID");
                            return;
                          }
                          navigate(navUrl);
                        }}
                        className={cn(
                          "w-full rounded-2xl border border-slate-200 bg-white px-3 py-3 text-left shadow-card transition",
                          "hover:border-slate-300 hover:bg-slate-50",
                          "dark:border-slate-700 dark:bg-slate-900/40 dark:hover:border-slate-600 dark:hover:bg-slate-900/60"
                        )}
                      >
                        <div className="grid gap-2 sm:grid-cols-[1fr_110px_140px] sm:items-center sm:gap-3">
                          <div className="min-w-0">
                            <div className="flex items-center gap-2">
                              <div className="truncate text-sm font-semibold text-slate-900 dark:text-slate-100">
                                {projectDisplayName(project)}
                              </div>
                              {project.model_provider ? (
                                <span className="shrink-0 rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                                  {project.model_provider}
                                </span>
                              ) : null}
                            </div>
                            <div
                              className="mt-1 truncate text-xs text-slate-500 dark:text-slate-400"
                              title={project.display_path}
                            >
                              {project.display_path}
                            </div>
                          </div>

                          <div className="flex items-center gap-1 text-xs text-slate-600 dark:text-slate-300 sm:justify-end">
                            <Hash className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                            <span className="font-semibold">{project.session_count}</span>
                          </div>

                          <div
                            className="flex items-center gap-1 text-xs text-slate-600 dark:text-slate-300 sm:justify-end"
                            title={modifiedTitle}
                          >
                            <Clock className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                            <span className="font-semibold">{modifiedLabel}</span>
                          </div>
                        </div>
                      </button>
                    </div>
                  );
                })}
              </div>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
