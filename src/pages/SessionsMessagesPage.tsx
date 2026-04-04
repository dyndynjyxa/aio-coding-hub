// Usage: Session messages viewer. Backend command: `cli_sessions_messages_get`.

import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, ArrowLeft, Copy, Loader2 } from "lucide-react";
import { toast } from "sonner";
import {
  type CliSessionsDisplayContentBlock,
  type CliSessionsSource,
  type CliSessionsSessionSummary,
  escapeShellArg,
} from "../services/cliSessions";
import { copyText } from "../services/clipboard";
import { useCliSessionsMessagesInfiniteQuery } from "../query/cliSessions";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { EmptyState } from "../ui/EmptyState";
import { ErrorState } from "../ui/ErrorState";
import { PageHeader } from "../ui/PageHeader";
import { Spinner } from "../ui/Spinner";
import { Switch } from "../ui/Switch";
import { cn } from "../utils/cn";
import {
  formatIsoDateTime,
  formatRelativeTimeFromUnixSeconds,
  formatUnixSeconds,
} from "../utils/formatters";

function normalizeSource(raw: string | undefined): CliSessionsSource | null {
  if (raw === "claude" || raw === "codex") return raw;
  return null;
}

function safeDecodeURIComponent(raw: string) {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function buildResumeCommand(source: CliSessionsSource, sessionId: string) {
  const escapedId = escapeShellArg(sessionId);
  return source === "claude" ? `claude --resume ${escapedId}` : `codex resume ${escapedId}`;
}

function renderBlock(block: CliSessionsDisplayContentBlock, key: string) {
  if (block.type === "text") {
    return (
      <pre key={key} className="whitespace-pre-wrap break-words text-sm leading-relaxed">
        {block.text}
      </pre>
    );
  }

  if (block.type === "thinking") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          思考过程
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.thinking}
        </pre>
      </details>
    );
  }

  if (block.type === "reasoning") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          推理过程
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.text}
        </pre>
      </details>
    );
  }

  if (block.type === "tool_use") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          Tool 调用：{block.name}
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.input}
        </pre>
      </details>
    );
  }

  if (block.type === "tool_result") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          Tool 结果{block.is_error ? "（错误）" : ""}
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.content}
        </pre>
      </details>
    );
  }

  if (block.type === "function_call") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          Function 调用：{block.name}
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.arguments}
        </pre>
      </details>
    );
  }

  if (block.type === "function_call_output") {
    return (
      <details key={key} className="rounded-lg border border-slate-200 dark:border-slate-700">
        <summary className="cursor-pointer select-none px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-300">
          Function 输出
        </summary>
        <pre className="whitespace-pre-wrap break-words px-3 pb-3 text-xs text-slate-600 dark:text-slate-300">
          {block.output}
        </pre>
      </details>
    );
  }

  return null;
}

type MessageSide = "left" | "right" | "center";

function messageSide(roleRaw: string): MessageSide {
  const role = roleRaw.trim().toLowerCase();
  if (role === "user") return "right";
  if (role === "system") return "center";
  return "left";
}

function senderLabel(source: CliSessionsSource, roleRaw: string) {
  const role = roleRaw.trim().toLowerCase();
  if (role === "user") return "你";
  if (role === "assistant") return source === "claude" ? "Claude Code" : "Codex";
  if (role === "system") return "System";
  if (role.startsWith("tool")) return "Tool";
  return roleRaw || "unknown";
}

function avatarTextForRole(roleRaw: string) {
  const role = roleRaw.trim().toLowerCase();
  if (role === "user") return "我";
  if (role === "assistant") return "AI";
  if (role === "system") return "SYS";
  if (role.startsWith("tool")) return "TL";
  return "AI";
}

export function SessionsMessagesPage() {
  const params = useParams();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const source = normalizeSource(params.source);
  const projectId = params.projectId || "";
  const safeSource: CliSessionsSource = source ?? "claude";
  const distro = searchParams.get("distro") ?? undefined;

  const rawFilePath = params["*"] || "";
  const filePath = rawFilePath ? safeDecodeURIComponent(rawFilePath) : "";

  const location = useLocation();
  const sessionFromState = location.state?.session as CliSessionsSessionSummary | undefined;
  const session = sessionFromState ?? null;

  const containerRef = useRef<HTMLDivElement>(null);
  const [showTimestamp, setShowTimestamp] = useState<boolean>(true);
  const [showModel, setShowModel] = useState<boolean>(false);

  const enabled = source != null && session != null && filePath.trim().length > 0;
  const messagesQuery = useCliSessionsMessagesInfiniteQuery(safeSource, filePath, {
    enabled,
    fromEnd: false,
    wslDistro: distro,
  });
  const allMessages = useMemo(() => {
    return messagesQuery.data?.pages.flatMap((page) => page?.messages ?? []) ?? [];
  }, [messagesQuery.data]);
  const rowVirtualizer = useVirtualizer({
    count: allMessages.length,
    getScrollElement: () => containerRef.current,
    getItemKey: (index) => allMessages[index]?.uuid ?? String(index),
    estimateSize: () => 150,
    overscan: 8,
  });
  const total = messagesQuery.data?.pages[0]?.total ?? 0;
  const hasMore = messagesQuery.hasNextPage ?? false;
  const loading = messagesQuery.isLoading;
  const loadingMore = messagesQuery.isFetchingNextPage;
  const error = messagesQuery.error ? String(messagesQuery.error) : null;
  const canReachSessionEnd = !hasMore;
  const jumpBottomTitle = canReachSessionEnd ? "滚动到会话末尾" : "滚动到已加载底部";
  const jumpBottomLabel = canReachSessionEnd ? "到会话末尾" : "到已加载底部";

  const handleFetchNextPage = async () => {
    if (!hasMore || loading || loadingMore) return;
    await messagesQuery.fetchNextPage();
  };

  const scrollToBottom = () => {
    const el = containerRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  };

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.scrollTop = 0;
  }, [source, filePath]);

  if (source == null) {
    return (
      <ErrorState
        title="无效来源"
        message="source 仅支持 claude / codex"
        onRetry={() => navigate("/sessions", { replace: true })}
      />
    );
  }

  if (!session) {
    return (
      <ErrorState
        title="会话信息缺失"
        message="无法获取会话元数据。请从会话列表页进入。"
        onRetry={() => {
          const backUrl = distro
            ? `/sessions/${source}/${encodeURIComponent(projectId)}?distro=${encodeURIComponent(distro)}`
            : `/sessions/${source}/${encodeURIComponent(projectId)}`;
          navigate(backUrl, { replace: true });
        }}
      />
    );
  }

  const backUrl = distro
    ? `/sessions/${source}/${encodeURIComponent(projectId)}?distro=${encodeURIComponent(distro)}`
    : `/sessions/${source}/${encodeURIComponent(projectId)}`;

  const title = session?.first_prompt?.trim() || session?.session_id || "Session";
  const subtitleParts: string[] = [];
  if (session?.session_id) subtitleParts.push(`Session ID：${session.session_id}`);
  if (session?.git_branch) subtitleParts.push(`分支：${session.git_branch}`);
  if (session?.model_provider) subtitleParts.push(`Provider：${session.model_provider}`);
  if (distro) subtitleParts.push(`WSL: ${distro}`);
  const subtitle = subtitleParts.length > 0 ? subtitleParts.join(" · ") : undefined;
  const canCopyResume = Boolean(session?.session_id?.trim());
  const loadedCount = allMessages.length;
  const globalStartIndex = 0;

  return (
    <div className="flex min-h-0 flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title={title}
        subtitle={subtitle}
        actions={
          <Button variant="secondary" onClick={() => navigate(backUrl)}>
            <ArrowLeft className="h-4 w-4" />
            返回会话
          </Button>
        }
      />

      {error ? (
        <ErrorState
          title="加载消息失败"
          message={error}
          onRetry={() => void messagesQuery.refetch()}
        />
      ) : null}

      <div className="grid gap-4 lg:flex-1 lg:min-h-0 lg:grid-cols-[360px_1fr] lg:items-stretch lg:overflow-hidden">
        <Card
          padding="md"
          className="flex flex-col gap-4 lg:min-h-0 lg:overflow-auto scrollbar-overlay"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                会话信息
              </div>
              <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {total > 0 ? (
                  <span>
                    已加载 {loadedCount}/{total} 条消息{hasMore ? "（可加载更多）" : ""}
                  </span>
                ) : (
                  <span>—</span>
                )}
              </div>
            </div>

            <Button
              size="sm"
              variant="secondary"
              onClick={scrollToBottom}
              className="h-9"
              title={jumpBottomTitle}
            >
              <ArrowDown className="h-4 w-4" />
              {jumpBottomLabel}
            </Button>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">快速操作</div>
            <div className="mt-3 flex flex-wrap gap-2">
              <Button
                variant="primary"
                disabled={!canCopyResume}
                onClick={async () => {
                  if (!session?.session_id?.trim()) return;
                  const cmd = buildResumeCommand(source, session.session_id);
                  await copyText(cmd);
                  toast("已复制恢复命令");
                }}
                title="复制恢复命令"
              >
                <Copy className="h-4 w-4" />
                复制恢复命令
              </Button>
              {session?.session_id ? (
                <Button
                  variant="secondary"
                  onClick={() => void copyText(session.session_id)}
                  title="复制 Session ID"
                >
                  复制 Session ID
                </Button>
              ) : null}
              {filePath ? (
                <Button
                  variant="ghost"
                  onClick={() => void copyText(filePath)}
                  title="复制文件路径"
                >
                  复制文件路径
                </Button>
              ) : null}
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">显示选项</div>
            <div className="mt-3 space-y-3 text-xs text-slate-600 dark:text-slate-400">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <label
                    htmlFor="switch-show-timestamp"
                    className="font-semibold text-slate-700 dark:text-slate-200 cursor-pointer"
                  >
                    显示时间
                  </label>
                  <div className="mt-0.5 text-[11px] text-slate-500 dark:text-slate-500">
                    消息头部展示时间戳
                  </div>
                </div>
                <Switch
                  id="switch-show-timestamp"
                  checked={showTimestamp}
                  onCheckedChange={setShowTimestamp}
                  size="sm"
                />
              </div>
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <label
                    htmlFor="switch-show-model"
                    className="font-semibold text-slate-700 dark:text-slate-200 cursor-pointer"
                  >
                    显示模型
                  </label>
                  <div className="mt-0.5 text-[11px] text-slate-500 dark:text-slate-500">
                    消息头部展示模型名
                  </div>
                </div>
                <Switch
                  id="switch-show-model"
                  checked={showModel}
                  onCheckedChange={setShowModel}
                  size="sm"
                />
              </div>
            </div>
          </div>

          <div className="rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">元信息</div>
            <div className="mt-3 space-y-2 text-xs text-slate-600 dark:text-slate-400">
              <div className="flex items-start justify-between gap-3">
                <span className="shrink-0 text-slate-500 dark:text-slate-500">来源</span>
                <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300">
                  {source}
                </span>
              </div>
              {distro ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">环境</span>
                  <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    WSL: {distro}
                  </span>
                </div>
              ) : null}
              {session?.git_branch ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">分支</span>
                  <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    {session.git_branch}
                  </span>
                </div>
              ) : null}
              {session?.created_at != null ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">创建</span>
                  <span className="text-right font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    {formatUnixSeconds(session.created_at)}
                  </span>
                </div>
              ) : null}
              {session?.modified_at != null ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">更新</span>
                  <span
                    className="text-right font-mono text-[11px] text-slate-700 dark:text-slate-300"
                    title={formatUnixSeconds(session.modified_at)}
                  >
                    {formatRelativeTimeFromUnixSeconds(session.modified_at)}
                  </span>
                </div>
              ) : null}
              {session?.cli_version ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">CLI 版本</span>
                  <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300">
                    {session.cli_version}
                  </span>
                </div>
              ) : null}
              {session?.cwd ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">CWD</span>
                  <span
                    className="min-w-0 truncate text-right font-mono text-[11px] text-slate-700 dark:text-slate-300"
                    title={session.cwd}
                  >
                    {session.cwd}
                  </span>
                </div>
              ) : null}
              {session?.project_path ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">项目路径</span>
                  <span
                    className="min-w-0 truncate text-right font-mono text-[11px] text-slate-700 dark:text-slate-300"
                    title={session.project_path}
                  >
                    {session.project_path}
                  </span>
                </div>
              ) : null}
              {filePath ? (
                <div className="flex items-start justify-between gap-3">
                  <span className="shrink-0 text-slate-500 dark:text-slate-500">文件</span>
                  <span
                    className="min-w-0 truncate text-right font-mono text-[11px] text-slate-700 dark:text-slate-300"
                    title={filePath}
                  >
                    {filePath}
                  </span>
                </div>
              ) : null}
            </div>
          </div>
        </Card>

        <Card padding="none" className="flex min-h-0 flex-1 flex-col">
          <div className="flex items-center justify-between border-b border-slate-200 px-4 py-3 text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
            <div>
              {total > 0 ? (
                <span>
                  {hasMore ? "可加载更多" : "已到会话末尾"} · 已加载 {loadedCount}/{total}
                </span>
              ) : (
                <span>—</span>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={!hasMore || loading || loadingMore}
                onClick={() => void handleFetchNextPage()}
                title="加载更多消息"
                className="h-9"
              >
                {loadingMore ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
                加载更多
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={scrollToBottom}
                className="h-9"
                title={jumpBottomTitle}
              >
                <ArrowDown className="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div
            ref={containerRef}
            className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6 scrollbar-overlay"
            style={{ overflowAnchor: "none" }}
          >
            {loading ? (
              <div className="flex items-center justify-center py-10">
                <Spinner />
              </div>
            ) : allMessages.length === 0 ? (
              <EmptyState title="此会话没有可显示的消息" variant="dashed" />
            ) : (
              <>
                <div className="py-2 text-center text-[11px] text-slate-400 dark:text-slate-500">
                  — 会话开始 —
                </div>
                <div
                  className="mx-auto w-full max-w-4xl"
                  style={{
                    height: `${rowVirtualizer.getTotalSize()}px`,
                    position: "relative",
                  }}
                >
                  {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                    const msg = allMessages[virtualRow.index];
                    const idx = virtualRow.index;
                    const role = (msg.role || "unknown").trim() || "unknown";
                    const side = messageSide(role);
                    const timeText =
                      showTimestamp && msg.timestamp ? formatIsoDateTime(msg.timestamp) : null;
                    const modelText = showModel && msg.model ? msg.model : null;
                    const messageKey = `${msg.uuid ?? "m"}:${idx}`;
                    const globalIndex = globalStartIndex + idx + 1;
                    const sender = senderLabel(source, role);

                    const avatarText = avatarTextForRole(role);
                    const avatarClass =
                      side === "right"
                        ? "border-slate-200 bg-white text-slate-900 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                        : side === "left"
                          ? "bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
                          : "border-fuchsia-200 bg-fuchsia-50 text-fuchsia-700 dark:border-fuchsia-700 dark:bg-fuchsia-900/30 dark:text-fuchsia-300";

                    const bubbleClass =
                      side === "right"
                        ? "border-accent/20 bg-gradient-to-br from-accent/10 to-accent-secondary/5 text-slate-900 dark:border-accent/30 dark:from-accent/20 dark:to-accent-secondary/10 dark:text-slate-100"
                        : side === "center"
                          ? "border-fuchsia-200 bg-fuchsia-50 text-fuchsia-800 dark:border-fuchsia-700 dark:bg-fuchsia-900/30 dark:text-fuchsia-200"
                          : role.toLowerCase().startsWith("tool")
                            ? "border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-200"
                            : "border-slate-200 bg-white text-slate-900 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-100";

                    return (
                      <div
                        key={virtualRow.key}
                        data-index={virtualRow.index}
                        ref={rowVirtualizer.measureElement}
                        style={{
                          position: "absolute",
                          top: 0,
                          left: 0,
                          width: "100%",
                          transform: `translateY(${virtualRow.start}px)`,
                        }}
                        className="pb-3"
                      >
                        <div
                          className={cn(
                            "flex gap-3",
                            side === "right" ? "justify-end" : "justify-start"
                          )}
                        >
                          {side === "left" ? (
                            <div
                              className={cn(
                                "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border text-[10px] font-extrabold shadow-sm",
                                avatarClass
                              )}
                              aria-hidden="true"
                              title={sender}
                            >
                              {avatarText}
                            </div>
                          ) : null}

                          <div
                            className={cn(
                              "min-w-0",
                              side === "center" ? "w-full max-w-3xl" : "max-w-[85%]"
                            )}
                          >
                            <div
                              className={cn(
                                "rounded-2xl border px-4 py-3 shadow-card",
                                bubbleClass,
                                side === "center" ? "mx-auto" : null
                              )}
                            >
                              <div
                                className={cn(
                                  "mb-2 flex flex-wrap items-center justify-between gap-2 text-[11px]",
                                  side === "right"
                                    ? "text-slate-600 dark:text-slate-300"
                                    : "text-slate-500 dark:text-slate-400"
                                )}
                              >
                                <div className="flex min-w-0 items-center gap-2">
                                  <span className="shrink-0 font-mono text-[10px] opacity-70">
                                    #{globalIndex}
                                  </span>
                                  <span className="truncate font-semibold">{sender}</span>
                                  {modelText ? (
                                    <span className="truncate font-mono text-[10px] opacity-70">
                                      {modelText}
                                    </span>
                                  ) : null}
                                </div>
                                {timeText ? (
                                  <span className="shrink-0 font-mono text-[10px] opacity-70">
                                    {timeText}
                                  </span>
                                ) : null}
                              </div>

                              <div className="flex flex-col gap-2">
                                {msg.content.map((block, blockIdx) =>
                                  renderBlock(block, `${messageKey}:b:${blockIdx}`)
                                )}
                              </div>
                            </div>
                          </div>

                          {side === "right" ? (
                            <div
                              className={cn(
                                "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border text-[11px] font-extrabold shadow-sm",
                                avatarClass
                              )}
                              aria-hidden="true"
                              title="你"
                            >
                              {avatarText}
                            </div>
                          ) : null}
                        </div>
                      </div>
                    );
                  })}
                </div>
                {!hasMore ? (
                  <div className="py-2 text-center text-[11px] text-slate-400 dark:text-slate-500">
                    — 会话结束 —
                  </div>
                ) : null}
              </>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
