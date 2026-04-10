// Usage: Project sessions list. Backend command: `cli_sessions_sessions_list`.

import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { ArrowLeft, Clock, Copy, GitBranch, MessageSquare, Search, Trash2 } from "lucide-react";
import { toast } from "sonner";
import {
  type CliSessionsSource,
  type CliSessionsSessionSummary,
  escapeShellArg,
} from "../services/cli/cliSessions";
import { copyText } from "../services/clipboard";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useCliSessionsProjectsListQuery,
  useCliSessionsSessionDeleteMutation,
  useCliSessionsSessionsListQuery,
} from "../query/cliSessions";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { ConfirmDialog } from "../ui/ConfirmDialog";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Select } from "../ui/Select";
import { Spinner } from "../ui/Spinner";
import { cn } from "../utils/cn";
import { formatRelativeTimeFromUnixSeconds, formatUnixSeconds } from "../utils/formatters";

type SessionSortKey = "recent" | "messages" | "created";

function normalizeSource(raw: string | undefined): CliSessionsSource | null {
  if (raw === "claude" || raw === "codex") return raw;
  return null;
}

function pickSessions(data: CliSessionsSessionSummary[] | null | undefined) {
  return data ?? [];
}

function buildResumeCommand(source: CliSessionsSource, sessionId: string) {
  const escapedId = escapeShellArg(sessionId);
  return source === "claude" ? `claude --resume ${escapedId}` : `codex resume ${escapedId}`;
}

/** 剥离 U+FFFD 替换字符（由后端 lossy UTF-8 解码产生） */
function stripReplacementChars(text: string) {
  return text.replace(/\uFFFD/g, "");
}

function sessionTitle(session: CliSessionsSessionSummary) {
  const raw = session.first_prompt?.trim() || "";
  const clean = stripReplacementChars(raw);
  return clean || session.session_id || "Session";
}

function sessionMatchesQuery(session: CliSessionsSessionSummary, query: string) {
  if (!query) return true;
  const q = query.toLowerCase();
  if (sessionTitle(session).toLowerCase().includes(q)) return true;
  if (session.session_id.toLowerCase().includes(q)) return true;
  if (session.git_branch?.toLowerCase().includes(q)) return true;
  if (session.model_provider?.toLowerCase().includes(q)) return true;
  if (session.cli_version?.toLowerCase().includes(q)) return true;
  return false;
}

function compareSession(
  sortKey: SessionSortKey,
  a: CliSessionsSessionSummary,
  b: CliSessionsSessionSummary
) {
  if (sortKey === "messages") {
    return b.message_count - a.message_count;
  }
  if (sortKey === "created") {
    const aTime = a.created_at ?? -1;
    const bTime = b.created_at ?? -1;
    return bTime - aTime;
  }
  const aTime = a.modified_at ?? -1;
  const bTime = b.modified_at ?? -1;
  return bTime - aTime;
}

export function SessionsProjectPage() {
  const params = useParams();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const source = normalizeSource(params.source);
  const projectId = params.projectId || "";
  const safeSource: CliSessionsSource = source ?? "claude";
  const distro = searchParams.get("distro") ?? undefined;
  const enabled = source != null && projectId.trim().length > 0;

  const projectsQuery = useCliSessionsProjectsListQuery(safeSource, distro);
  const sessionsQuery = useCliSessionsSessionsListQuery(safeSource, projectId, {
    enabled,
    wslDistro: distro,
  });
  const deleteMutation = useCliSessionsSessionDeleteMutation();
  const sessions = useMemo(() => pickSessions(sessionsQuery.data), [sessionsQuery.data]);
  const [filterText, setFilterText] = useState("");
  const [sortKey, setSortKey] = useState<SessionSortKey>("recent");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const filteredSessions = useMemo(() => {
    const q = filterText.trim();
    const next = q ? sessions.filter((s) => sessionMatchesQuery(s, q)) : sessions;
    return [...next].sort((a, b) => compareSession(sortKey, a, b));
  }, [filterText, sessions, sortKey]);
  const project = useMemo(() => {
    return (projectsQuery.data ?? []).find((p) => p?.id === projectId) ?? null;
  }, [projectId, projectsQuery.data]);

  const overview = useMemo(() => {
    const totalSessions = sessions.length;
    const totalMessages = sessions.reduce((sum, s) => sum + s.message_count, 0);
    const lastModified = sessions.reduce<number | null>((acc, s) => {
      if (s.modified_at == null) return acc;
      if (acc == null) return s.modified_at;
      return Math.max(acc, s.modified_at);
    }, null);
    const branches = new Map<string, number>();
    const providers = new Set<string>();
    let sidechains = 0;
    for (const s of sessions) {
      if (s.git_branch) branches.set(s.git_branch, (branches.get(s.git_branch) ?? 0) + 1);
      if (s.model_provider) providers.add(s.model_provider);
      if (s.is_sidechain) sidechains += 1;
    }
    const topBranches = [...branches.entries()].sort((a, b) => b[1] - a[1]).slice(0, 3);
    const providerList = [...providers.values()].slice(0, 5);
    return { totalSessions, totalMessages, lastModified, topBranches, providerList, sidechains };
  }, [sessions]);
  const parentRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: filteredSessions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 100,
    overscan: 10,
  });
  const visibleSelectedCount = useMemo(() => {
    let count = 0;
    for (const session of filteredSessions) {
      if (selectedPaths.has(session.file_path)) {
        count += 1;
      }
    }
    return count;
  }, [filteredSessions, selectedPaths]);
  const allVisibleSelected =
    filteredSessions.length > 0 && visibleSelectedCount === filteredSessions.length;
  const selectedVisibleSessions = useMemo(() => {
    return filteredSessions.filter((session) => selectedPaths.has(session.file_path));
  }, [filteredSessions, selectedPaths]);

  useEffect(() => {
    setSelectedPaths(new Set());
    setShowDeleteDialog(false);
  }, [distro, projectId, source]);

  useEffect(() => {
    const visiblePaths = new Set(filteredSessions.map((session) => session.file_path));
    setSelectedPaths((prev) => {
      let changed = false;
      const next = new Set<string>();
      for (const path of prev) {
        if (visiblePaths.has(path)) {
          next.add(path);
        } else {
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [filteredSessions]);

  function toggleSelect(filePath: string) {
    setSelectedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(filePath)) next.delete(filePath);
      else next.add(filePath);
      return next;
    });
  }

  function toggleSelectAll() {
    if (allVisibleSelected) {
      setSelectedPaths(new Set());
    } else {
      setSelectedPaths(new Set(filteredSessions.map((s) => s.file_path)));
    }
  }

  async function confirmDelete() {
    if (selectedPaths.size === 0 || !source) return;
    try {
      const failedList = await deleteMutation.mutateAsync({
        source,
        filePaths: [...selectedPaths],
        projectId,
        wslDistro: distro,
      });
      const successCount = selectedPaths.size - (failedList?.length ?? 0);
      if (successCount > 0) {
        toast(`已删除 ${successCount} 个会话`);
      }
      if (failedList && failedList.length > 0) {
        toast(`${failedList.length} 个会话删除失败`);
      }
      setSelectedPaths(new Set());
      setShowDeleteDialog(false);
    } catch (err) {
      toast(`删除失败：${String(err)}`);
    }
  }

  function handleSingleDelete(e: React.MouseEvent, filePath: string) {
    e.stopPropagation();
    setSelectedPaths(new Set([filePath]));
    setShowDeleteDialog(true);
  }

  if (source == null) {
    return (
      <ErrorState
        title="无效来源"
        message="source 仅支持 claude / codex"
        onRetry={() => navigate("/sessions", { replace: true })}
      />
    );
  }

  const backUrl = distro
    ? `/sessions?source=${source}&distro=${encodeURIComponent(distro)}`
    : `/sessions?source=${source}`;

  function buildSessionNavUrl(filePath: string) {
    const base = `/sessions/${source}/${encodeURIComponent(projectId)}/session/${encodeURIComponent(filePath)}`;
    return distro ? `${base}?distro=${encodeURIComponent(distro)}` : base;
  }

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title={project?.short_name || projectId}
        subtitle={project?.display_path}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="secondary" onClick={() => navigate(backUrl)}>
              <ArrowLeft className="h-4 w-4" />
              返回项目
            </Button>
            {project?.display_path ? (
              <Button
                variant="ghost"
                onClick={() => void copyText(project.display_path)}
                title="复制项目路径"
              >
                复制路径
              </Button>
            ) : null}
          </div>
        }
      />

      <div className="grid gap-4 lg:flex-1 lg:min-h-0 lg:grid-cols-[360px_1fr] lg:items-stretch lg:overflow-hidden">
        <Card padding="md" className="flex flex-col gap-4 lg:min-h-0">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                项目概览
              </div>
              <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                来源：<span className="font-semibold">{source}</span>
                {distro ? (
                  <span className="ml-2 rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                    WSL: {distro}
                  </span>
                ) : null}
              </div>
            </div>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void sessionsQuery.refetch()}
              className="h-9"
            >
              刷新
            </Button>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
              <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                会话
              </div>
              <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                {overview.totalSessions}
              </div>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
              <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                消息总数
              </div>
              <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                {overview.totalMessages}
              </div>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
              <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                最近更新
              </div>
              <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                {overview.lastModified != null
                  ? formatRelativeTimeFromUnixSeconds(overview.lastModified)
                  : "—"}
              </div>
            </div>
            <div className="rounded-2xl border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900/30">
              <div className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
                Sidechain
              </div>
              <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
                {overview.sidechains}
              </div>
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              分支与 Provider
            </div>
            <div className="mt-2 space-y-2 text-xs text-slate-600 dark:text-slate-400">
              {overview.topBranches.length > 0 ? (
                <div className="flex flex-wrap gap-2">
                  {overview.topBranches.map(([branch, count]) => (
                    <span
                      key={branch}
                      className="inline-flex items-center gap-1 rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300"
                      title={`${count} 个会话`}
                    >
                      <GitBranch className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                      {branch}
                      <span className="text-slate-400 dark:text-slate-500">{count}</span>
                    </span>
                  ))}
                </div>
              ) : (
                <div className="text-slate-500 dark:text-slate-500">暂无分支信息</div>
              )}

              {overview.providerList.length > 0 ? (
                <div className="flex flex-wrap gap-2">
                  {overview.providerList.map((p) => (
                    <span
                      key={p}
                      className="inline-flex items-center rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300"
                    >
                      {p}
                    </span>
                  ))}
                </div>
              ) : (
                <div className="text-slate-500 dark:text-slate-500">暂无 Provider 信息</div>
              )}
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">提示</div>
            <ul className="mt-2 space-y-1 text-xs text-slate-600 dark:text-slate-400">
              <li>点击右侧会话即可进入消息阅览</li>
              <li>每条会话都支持复制恢复命令</li>
              <li>消息页支持分页加载更多内容</li>
            </ul>
          </div>
        </Card>

        <Card padding="sm" className="flex flex-col lg:min-h-0">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="min-w-0">
              <div className="flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
                <MessageSquare className="h-4 w-4 shrink-0 text-accent" />
                <span className="shrink-0">会话</span>
                <span className="shrink-0 text-xs font-medium text-slate-500 dark:text-slate-400">
                  {filteredSessions.length}/{sessions.length}
                </span>
              </div>
              <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                支持按标题 / 分支 / Provider / 版本搜索。
              </div>
            </div>

            <div className="flex items-center gap-2">
              {selectedPaths.size > 0 && (
                <Button
                  size="sm"
                  variant="danger"
                  onClick={() => setShowDeleteDialog(true)}
                  className="h-9"
                >
                  <Trash2 className="h-4 w-4" />
                  删除 ({selectedPaths.size})
                </Button>
              )}
              <Select
                value={sortKey}
                onChange={(e) => setSortKey(e.currentTarget.value as SessionSortKey)}
                className="h-9 w-32 text-xs"
                aria-label="排序"
              >
                <option value="recent">最近更新</option>
                <option value="messages">消息最多</option>
                <option value="created">创建时间</option>
              </Select>
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
                placeholder="搜索会话"
                className="pl-9"
                aria-label="搜索会话"
              />
            </div>
          </div>

          <div className="mt-3 hidden grid-cols-[32px_1fr_90px_140px_120px] gap-3 px-3 text-[11px] font-semibold text-slate-500 dark:text-slate-400 sm:grid">
            <span>
              <input
                type="checkbox"
                checked={allVisibleSelected}
                onChange={toggleSelectAll}
                className="h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                aria-label="全选"
              />
            </span>
            <span>会话</span>
            <span className="text-right">消息</span>
            <span className="text-right">更新</span>
            <span className="text-right">操作</span>
          </div>

          <div
            ref={(node) => {
              if (node) parentRef.current = node;
            }}
            className="mt-2 h-[600px] lg:min-h-0 lg:flex-1 lg:h-auto overflow-auto lg:pr-1 scrollbar-overlay"
          >
            {sessionsQuery.error ? (
              <ErrorState
                title="加载会话失败"
                message={String(sessionsQuery.error)}
                onRetry={() => void sessionsQuery.refetch()}
              />
            ) : sessionsQuery.isLoading ? (
              <div className="flex items-center justify-center py-10">
                <Spinner />
              </div>
            ) : filteredSessions.length === 0 ? (
              <EmptyState
                title={sessions.length === 0 ? "此项目没有会话记录" : "未匹配到会话"}
                variant="dashed"
              />
            ) : (
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  width: "100%",
                  position: "relative",
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const session = filteredSessions[virtualRow.index];
                  const title = sessionTitle(session);
                  const isSelected = selectedPaths.has(session.file_path);
                  const modifiedLabel =
                    session.modified_at != null
                      ? formatRelativeTimeFromUnixSeconds(session.modified_at)
                      : "—";
                  const modifiedTitle =
                    session.modified_at != null ? formatUnixSeconds(session.modified_at) : "—";
                  const createdText =
                    session.created_at != null ? formatUnixSeconds(session.created_at) : "—";

                  return (
                    <div
                      key={virtualRow.key}
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        transform: `translateY(${virtualRow.start}px)`,
                      }}
                      className="px-1 pb-2"
                    >
                      <div
                        role="button"
                        className={cn(
                          "w-full cursor-pointer text-left rounded-2xl border border-slate-200 bg-white px-3 py-3 shadow-card transition",
                          "hover:border-slate-300 hover:bg-slate-50",
                          "dark:border-slate-700 dark:bg-slate-900/40 dark:hover:border-slate-600 dark:hover:bg-slate-900/60",
                          isSelected &&
                            "border-accent/40 bg-accent/5 dark:border-accent/30 dark:bg-accent/5"
                        )}
                        tabIndex={0}
                        onClick={() =>
                          navigate(buildSessionNavUrl(session.file_path), {
                            state: { session },
                          })
                        }
                        onKeyDown={(e) => {
                          if (e.target !== e.currentTarget) return;
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            navigate(buildSessionNavUrl(session.file_path), {
                              state: { session },
                            });
                          }
                        }}
                      >
                        <div className="grid gap-2 sm:grid-cols-[32px_1fr_90px_140px_120px] sm:items-center sm:gap-3">
                          <div
                            className="hidden sm:flex items-center justify-center"
                            onClick={(e) => e.stopPropagation()}
                            onKeyDown={(e) => e.stopPropagation()}
                          >
                            <input
                              type="checkbox"
                              checked={isSelected}
                              onChange={() => toggleSelect(session.file_path)}
                              className="h-4 w-4 rounded border-slate-300 text-accent focus:ring-accent dark:border-slate-600"
                              aria-label={`选择会话 ${title}`}
                            />
                          </div>

                          <div className="min-w-0">
                            <div className="line-clamp-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
                              {title}
                            </div>
                            <div className="mt-1 flex flex-wrap items-center gap-3 text-[11px] text-slate-500 dark:text-slate-400">
                              {session.git_branch ? (
                                <span className="inline-flex items-center gap-1">
                                  <GitBranch className="h-3.5 w-3.5" />
                                  {session.git_branch}
                                </span>
                              ) : null}
                              {session.model_provider ? (
                                <span className="inline-flex items-center gap-1">
                                  <span className="font-semibold">{session.model_provider}</span>
                                </span>
                              ) : null}
                              <span className="text-slate-400 dark:text-slate-500">
                                创建于 {createdText}
                              </span>
                            </div>
                          </div>

                          <div className="flex items-center justify-end gap-1 text-xs text-slate-600 dark:text-slate-300">
                            <span className="font-semibold">{session.message_count}</span>
                          </div>

                          <div
                            className="flex items-center justify-end gap-1 text-xs text-slate-600 dark:text-slate-300"
                            title={modifiedTitle}
                          >
                            <Clock className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500" />
                            <span className="font-semibold">{modifiedLabel}</span>
                          </div>

                          <div className="flex items-center justify-end gap-1">
                            <Button
                              size="sm"
                              variant="primary"
                              onClick={async (e) => {
                                e.stopPropagation();
                                if (!session.session_id.trim()) {
                                  toast("无效 sessionId");
                                  return;
                                }
                                const cmd = buildResumeCommand(source, session.session_id);
                                await copyText(cmd);
                                toast("已复制恢复命令");
                              }}
                              title="复制恢复命令"
                              className="h-8"
                            >
                              <Copy className="h-3.5 w-3.5" />
                              复制
                            </Button>
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={(e) => handleSingleDelete(e, session.file_path)}
                              title="删除会话"
                              className="h-8 text-slate-400 hover:text-rose-500 dark:text-slate-500 dark:hover:text-rose-400"
                            >
                              <Trash2 className="h-3.5 w-3.5" />
                            </Button>
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </Card>
      </div>

      <ConfirmDialog
        open={showDeleteDialog}
        title="确认删除会话"
        description={`将删除 ${selectedPaths.size} 个会话文件，此操作不可撤销。`}
        onClose={() => setShowDeleteDialog(false)}
        onConfirm={() => void confirmDelete()}
        confirmLabel={`确认删除 (${selectedPaths.size})`}
        confirmingLabel="删除中…"
        confirming={deleteMutation.isPending}
        confirmVariant="danger"
      >
        <div className="max-h-40 overflow-auto text-sm text-slate-600 dark:text-slate-400">
          <ul className="space-y-1">
            {selectedVisibleSessions.slice(0, 10).map((s) => (
              <li key={s.file_path} className="truncate">
                {sessionTitle(s)}
              </li>
            ))}
            {selectedPaths.size > 10 && (
              <li className="text-slate-400">...还有 {selectedPaths.size - 10} 个</li>
            )}
          </ul>
        </div>
      </ConfirmDialog>
    </div>
  );
}
