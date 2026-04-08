// Usage:
// - Render as the right side column in `HomeOverviewPanel` to show realtime traces + request logs list.
// - Selection state is controlled by parent; the detail dialog is rendered outside the grid layout.

import { memo, useRef, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cliBadgeToneStatic, cliShortLabel } from "../../constants/clis";
import { useNowMs } from "../../hooks/useNowMs";
import { useCliSessionsFolderLookupByIdsQuery } from "../../query/cliSessions";
import type {
  CliSessionsFolderLookupEntry,
  CliSessionsFolderLookupInput,
  CliSessionsSource,
} from "../../services/cliSessions";
import type { RequestLogSummary } from "../../services/requestLogs";
import type { TraceSession } from "../../services/traceStore";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { Tooltip } from "../../ui/Tooltip";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatRelativeTimeFromUnixSeconds,
  formatTokensPerSecond,
  formatTokensPerSecondShort,
  formatUsd,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import {
  buildRequestLogAuditMeta,
  buildRequestRouteMeta,
  computeEffectiveInputTokens,
  computeStatusBadge,
  FolderBadge,
  FreeBadge,
  getErrorCodeLabel,
  isPersistedRequestLogInProgress,
  resolveLiveTraceDurationMs,
  resolveLiveTraceProvider,
  SessionReuseBadge,
} from "./HomeLogShared";
import {
  Clock,
  CheckCircle2,
  XCircle,
  Server,
  RefreshCw,
  ArrowUpRight,
  Loader2,
} from "lucide-react";
import { RealtimeTraceCards } from "./RealtimeTraceCards";
import { CliBrandIcon } from "./CliBrandIcon";
import {
  buildPreviewRequestLogs,
  buildPreviewSessionFolderLookups,
  buildPreviewTraces,
} from "./previewData";

// Estimated height for each request log card (px): padding + 2 rows of content + margin
const ESTIMATED_LOG_CARD_HEIGHT = 90;

// Threshold below which we skip virtualization (overhead not worth it).
// Set to 30 so the default 50-item HomePage list benefits from virtualization.
const VIRTUALIZATION_THRESHOLD = 30;

// Module-level stable reference: pure function, no need to recreate per render.
const formatUnixSecondsStable = (ts: number) => formatRelativeTimeFromUnixSeconds(ts);

function isFolderLookupCliKey(cliKey: string): cliKey is CliSessionsSource {
  return cliKey === "claude" || cliKey === "codex";
}

function sessionFolderLookupKey(cliKey: string, sessionId: string | null | undefined) {
  const normalized = sessionId?.trim();
  if (!normalized) return null;
  return `${cliKey}:${normalized}`;
}

function requestLogCreatedAtMs(log: RequestLogSummary) {
  const ms = log.created_at_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return log.created_at * 1000;
}

function sortRequestLogsForDisplay(a: RequestLogSummary, b: RequestLogSummary) {
  const aInProgress = isPersistedRequestLogInProgress(a);
  const bInProgress = isPersistedRequestLogInProgress(b);
  if (aInProgress !== bInProgress) return aInProgress ? -1 : 1;

  const aTsMs = requestLogCreatedAtMs(a);
  const bTsMs = requestLogCreatedAtMs(b);
  if (aTsMs !== bTsMs) return bTsMs - aTsMs;
  return b.id - a.id;
}

function mergeTraceWithRequestLog(
  trace: TraceSession,
  requestLog: RequestLogSummary | undefined
): TraceSession {
  if (!requestLog) return trace;

  const summary = trace.summary;
  const requestLogInProgress = isPersistedRequestLogInProgress(requestLog);
  if (!summary && requestLogInProgress) {
    return {
      ...trace,
      session_id: trace.session_id ?? requestLog.session_id ?? null,
      requested_model: trace.requested_model ?? requestLog.requested_model ?? null,
      last_seen_ms: Math.max(trace.last_seen_ms, requestLogCreatedAtMs(requestLog)),
    };
  }

  const mergedSummary = {
    trace_id: trace.trace_id,
    cli_key: trace.cli_key,
    method: trace.method,
    path: trace.path,
    query: trace.query,
    status: summary?.status ?? requestLog.status ?? null,
    error_category: summary?.error_category ?? null,
    error_code: summary?.error_code ?? requestLog.error_code ?? null,
    duration_ms: summary?.duration_ms ?? requestLog.duration_ms ?? 0,
    ttfb_ms: summary?.ttfb_ms ?? requestLog.ttfb_ms ?? null,
    attempts: summary?.attempts ?? [],
    input_tokens: summary?.input_tokens ?? requestLog.input_tokens ?? null,
    output_tokens: summary?.output_tokens ?? requestLog.output_tokens ?? null,
    total_tokens: summary?.total_tokens ?? requestLog.total_tokens ?? null,
    cache_read_input_tokens:
      summary?.cache_read_input_tokens ?? requestLog.cache_read_input_tokens ?? null,
    cache_creation_input_tokens:
      summary?.cache_creation_input_tokens ?? requestLog.cache_creation_input_tokens ?? null,
    cache_creation_5m_input_tokens:
      summary?.cache_creation_5m_input_tokens ?? requestLog.cache_creation_5m_input_tokens ?? null,
    cache_creation_1h_input_tokens:
      summary?.cache_creation_1h_input_tokens ?? requestLog.cache_creation_1h_input_tokens ?? null,
    cost_usd: summary?.cost_usd ?? requestLog.cost_usd ?? null,
    cost_multiplier: summary?.cost_multiplier ?? requestLog.cost_multiplier ?? null,
  };

  return {
    ...trace,
    session_id: trace.session_id ?? requestLog.session_id ?? null,
    requested_model: trace.requested_model ?? requestLog.requested_model ?? null,
    summary: mergedSummary,
    last_seen_ms: Math.max(trace.last_seen_ms, requestLogCreatedAtMs(requestLog)),
  };
}

function createRealtimeTraceFromInProgressLog(log: RequestLogSummary): TraceSession {
  const createdAtMs = requestLogCreatedAtMs(log);
  return {
    trace_id: log.trace_id,
    cli_key: log.cli_key,
    session_id: log.session_id ?? null,
    method: log.method,
    path: log.path,
    query: null,
    requested_model: log.requested_model ?? null,
    first_seen_ms: createdAtMs,
    last_seen_ms: createdAtMs,
    attempts: [],
  };
}

type RequestLogCardProps = {
  compactMode: boolean;
  log: RequestLogSummary;
  liveTrace?: TraceSession;
  nowMs: number;
  isSelected: boolean;
  sessionFolder?: CliSessionsFolderLookupEntry | null;
  showCustomTooltip: boolean;
  onSelectLogId: (id: number | null) => void;
  formatUnixSeconds: (ts: number) => string;
};

const RequestLogCard = memo(function RequestLogCard({
  compactMode,
  log,
  liveTrace,
  nowMs,
  isSelected,
  sessionFolder,
  showCustomTooltip,
  onSelectLogId,
  formatUnixSeconds,
}: RequestLogCardProps) {
  const auditMeta = buildRequestLogAuditMeta(log);
  const isInProgress = isPersistedRequestLogInProgress(log);
  const liveProvider = resolveLiveTraceProvider(liveTrace);
  const displayDurationMs =
    isInProgress && liveTrace
      ? (resolveLiveTraceDurationMs(liveTrace, nowMs) ?? log.duration_ms)
      : log.duration_ms;
  const statusBadge = computeStatusBadge({
    status: log.status,
    errorCode: log.error_code,
    inProgress: isInProgress,
    hasFailover: log.has_failover,
  });

  const providerText =
    (isInProgress && liveProvider ? liveProvider.providerName : null) ??
    auditMeta.providerFallbackText ??
    (log.final_provider_id === 0 ||
    !log.final_provider_name ||
    log.final_provider_name.trim().length === 0 ||
    log.final_provider_name === "Unknown"
      ? "未知"
      : log.final_provider_name);

  const routeMeta = buildRequestRouteMeta({
    route: log.route,
    status: log.status,
    hasFailover: log.has_failover,
    attemptCount: log.attempt_count,
  });

  const providerTitle = providerText;

  const modelText =
    log.requested_model && log.requested_model.trim() ? log.requested_model.trim() : "未知";

  const cliLabel = cliShortLabel(log.cli_key);
  const cliTone = cliBadgeToneStatic(log.cli_key);
  const compactTextClass = compactMode ? "whitespace-normal break-all" : "truncate";

  const ttfbMs = sanitizeTtfbMs(log.ttfb_ms, displayDurationMs);
  const outputTokensPerSecond = computeOutputTokensPerSecond(
    log.output_tokens,
    displayDurationMs,
    ttfbMs
  );

  const costMultiplier = log.cost_multiplier;
  const isFree = Number.isFinite(costMultiplier) && costMultiplier === 0;
  const showCostMultiplier =
    Number.isFinite(costMultiplier) && costMultiplier >= 0 && Math.abs(costMultiplier - 1) > 0.0001;
  const costMultiplierText = isFree ? "免费" : `x${costMultiplier.toFixed(2)}`;
  const costUsdText = formatUsd(log.cost_usd);

  const cacheWrite = (() => {
    // 优先展示有值的 TTL 桶；若都为 0，则仍展示 0 而不是 "—"。
    if (log.cache_creation_5m_input_tokens != null && log.cache_creation_5m_input_tokens > 0) {
      return { tokens: log.cache_creation_5m_input_tokens, ttl: "5m" as const };
    }
    if (log.cache_creation_1h_input_tokens != null && log.cache_creation_1h_input_tokens > 0) {
      return { tokens: log.cache_creation_1h_input_tokens, ttl: "1h" as const };
    }
    if (log.cache_creation_input_tokens != null && log.cache_creation_input_tokens > 0) {
      return { tokens: log.cache_creation_input_tokens, ttl: null };
    }
    if (log.cache_creation_5m_input_tokens != null) {
      return { tokens: log.cache_creation_5m_input_tokens, ttl: "5m" as const };
    }
    if (log.cache_creation_1h_input_tokens != null) {
      return { tokens: log.cache_creation_1h_input_tokens, ttl: "1h" as const };
    }
    if (log.cache_creation_input_tokens != null) {
      return { tokens: log.cache_creation_input_tokens, ttl: null };
    }
    return { tokens: null as number | null, ttl: null as "5m" | "1h" | null };
  })();

  const effectiveInputTokens = computeEffectiveInputTokens(
    log.cli_key,
    log.input_tokens,
    log.cache_read_input_tokens
  );

  return (
    <button type="button" onClick={() => onSelectLogId(log.id)} className="w-full text-left group">
      <div
        className={cn(
          "relative transition-all duration-200 group/item mx-2 my-1.5 rounded-lg border",
          isSelected
            ? "bg-indigo-50/50 border-indigo-200/80 shadow-[0_0_0_1px_rgba(99,102,241,0.08),0_2px_8px_rgba(99,102,241,0.1)] dark:bg-indigo-950/30 dark:border-indigo-600/40 dark:shadow-[0_0_0_1px_rgba(99,102,241,0.15),0_2px_8px_rgba(99,102,241,0.12)]"
            : auditMeta.muted
              ? "bg-slate-50/85 border-slate-200/70 hover:bg-slate-100/85 hover:border-slate-300/70 dark:bg-slate-800/70 dark:border-slate-700/80 dark:hover:bg-slate-800/90 dark:hover:border-slate-600/80"
              : "bg-white/80 border-slate-200/60 hover:bg-slate-50/80 hover:border-slate-300/60 hover:shadow-[0_2px_8px_rgba(15,23,42,0.06)] dark:bg-slate-800/80 dark:border-slate-700/60 dark:hover:bg-slate-750/80 dark:hover:border-slate-600/60 dark:hover:shadow-[0_2px_8px_rgba(0,0,0,0.2)]"
        )}
      >
        {/* Selection indicator */}
        <div
          className={cn(
            "absolute left-0 top-2 bottom-2 w-[3px] rounded-r-full transition-all duration-200",
            isSelected
              ? "bg-gradient-to-b from-indigo-500 to-indigo-400 opacity-100 shadow-[2px_0_8px_rgba(99,102,241,0.2)]"
              : "bg-slate-300 opacity-0 group-hover/item:opacity-50 dark:bg-slate-500"
          )}
        />

        <div className={cn("px-3", compactMode ? "py-2" : "py-2.5")}>
          <div
            className={cn(
              compactMode
                ? "grid grid-cols-[minmax(0,1fr)_auto] items-start gap-x-2 gap-y-1"
                : "flex items-center gap-2 min-w-0 mb-1.5"
            )}
          >
            <div
              className={cn(
                "min-w-0",
                compactMode ? "flex flex-wrap items-start gap-2" : "contents"
              )}
            >
              <span
                className={cn(
                  "inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-md px-1.5 py-0.5 text-[11px] font-medium",
                  statusBadge.tone
                )}
                title={statusBadge.title}
              >
                {isInProgress ? (
                  <Loader2 className="h-3 w-3 shrink-0 animate-spin" />
                ) : statusBadge.isError ? (
                  <XCircle className="h-3 w-3 shrink-0" />
                ) : (
                  <CheckCircle2 className="h-3 w-3 shrink-0" />
                )}
                <span className="flex-1 text-center">{statusBadge.text}</span>
              </span>

              <span
                className={cn(
                  "inline-flex min-w-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium",
                  cliTone
                )}
                title={`${cliLabel} / ${modelText}`}
              >
                <CliBrandIcon
                  cliKey={log.cli_key}
                  className="h-2.5 w-2.5 shrink-0 rounded-[3px] object-contain"
                />
                <span className="shrink-0">{cliLabel} /</span>
                <span className={compactTextClass}>{modelText}</span>
              </span>

              {sessionFolder && (
                <FolderBadge
                  folderName={sessionFolder.folder_name}
                  folderPath={sessionFolder.folder_path}
                  allowWrap={compactMode}
                />
              )}

              {compactMode && (
                <span
                  className="inline-flex min-w-0 items-center gap-1 rounded-md bg-slate-100/75 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:bg-slate-700/55 dark:text-slate-200"
                  title={providerTitle}
                >
                  <Server className="h-3 w-3 shrink-0 text-slate-400 dark:text-slate-500" />
                  <span className={compactTextClass}>{providerText}</span>
                </span>
              )}

              {log.session_reuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}

              {isFree && <FreeBadge />}

              {log.error_code && (
                <span className="shrink-0 whitespace-nowrap rounded-md bg-amber-50/80 px-2 py-0.5 text-[11px] font-semibold text-amber-600 ring-1 ring-inset ring-amber-500/10 dark:bg-amber-500/15 dark:text-amber-300 dark:ring-amber-400/20">
                  {getErrorCodeLabel(log.error_code)}
                </span>
              )}

              {auditMeta.tags.map((tag) => (
                <span
                  key={tag.label}
                  className={cn(
                    "shrink-0 whitespace-nowrap rounded-md px-2 py-0.5 text-[11px] font-semibold",
                    tag.className
                  )}
                  title={tag.title}
                >
                  {tag.label}
                </span>
              ))}
            </div>

            <span
              className={cn(
                "flex shrink-0 items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500 whitespace-nowrap",
                compactMode ? "self-start" : "ml-auto w-[150px] justify-end"
              )}
            >
              <Clock className="h-3 w-3 shrink-0" />
              {formatUnixSeconds(log.created_at)}
            </span>
          </div>

          {!compactMode && auditMeta.summary ? (
            <div className="mb-1.5 text-[11px] text-slate-500 dark:text-slate-400">
              {auditMeta.summary}
            </div>
          ) : null}

          {!compactMode && (
            <div className="flex items-start gap-3 text-[11px]">
              <div className="flex flex-col gap-y-0.5 w-[110px] shrink-0" title={providerTitle}>
                <div className="flex items-center gap-1 h-4">
                  <Server className="h-3 w-3 text-slate-400/80 dark:text-slate-500/80 shrink-0" />
                  <span className="truncate font-semibold text-slate-600 dark:text-slate-300">
                    {providerText}
                  </span>
                </div>
                <div className="flex items-center h-4">
                  <div className="flex items-center gap-1 min-w-0 w-full">
                    {routeMeta.hasRoute && routeMeta.tooltipText ? (
                      showCustomTooltip ? (
                        <Tooltip
                          content={routeMeta.tooltipContent}
                          contentClassName="max-w-[400px] break-words"
                          placement="top"
                        >
                          <span className="text-[11px] text-slate-400 dark:text-slate-500 hover:text-indigo-600 dark:hover:text-indigo-400 cursor-help">
                            {routeMeta.label}
                          </span>
                        </Tooltip>
                      ) : (
                        <span
                          className="text-[11px] text-slate-400 dark:text-slate-500 cursor-help"
                          title={routeMeta.tooltipText}
                        >
                          {routeMeta.label}
                        </span>
                      )
                    ) : null}

                    {showCostMultiplier ? (
                      <span className="inline-flex items-center text-[11px] font-medium text-slate-500 dark:text-slate-400 shrink-0">
                        {costMultiplierText}
                      </span>
                    ) : null}
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-4 gap-x-3 gap-y-0.5 flex-1 text-slate-500 dark:text-slate-400">
                <div className="flex items-center gap-1 h-4" title="Input Tokens">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">输入</span>
                  <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                    {formatInteger(effectiveInputTokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Write">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                    缓存创建
                  </span>
                  {cacheWrite.tokens != null ? (
                    <>
                      <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                        {formatInteger(cacheWrite.tokens)}
                      </span>
                      {cacheWrite.ttl && cacheWrite.tokens > 0 && (
                        <span className="text-slate-400/70 dark:text-slate-500/70 text-[10px]">
                          ({cacheWrite.ttl})
                        </span>
                      )}
                    </>
                  ) : (
                    <span className="text-slate-300/60 dark:text-slate-600/60">—</span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="TTFB">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">首字</span>
                  <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                    {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                  </span>
                </div>
                <div
                  className="flex items-center gap-1 h-4"
                  title={costUsdText === "—" ? undefined : costUsdText}
                >
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">花费</span>
                  <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                    {costUsdText}
                  </span>
                </div>

                <div className="flex items-center gap-1 h-4" title="Output Tokens">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">输出</span>
                  <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                    {formatInteger(log.output_tokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Read">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                    缓存读取
                  </span>
                  {log.cache_read_input_tokens != null ? (
                    <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                      {formatInteger(log.cache_read_input_tokens)}
                    </span>
                  ) : (
                    <span className="text-slate-300/60 dark:text-slate-600/60">—</span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="Duration">
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">耗时</span>
                  <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                    {formatDurationMs(displayDurationMs)}
                  </span>
                </div>
                <div
                  className="flex items-center gap-1 h-4"
                  title={
                    outputTokensPerSecond != null
                      ? formatTokensPerSecond(outputTokensPerSecond)
                      : undefined
                  }
                >
                  <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">速率</span>
                  {outputTokensPerSecond != null ? (
                    <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                      {formatTokensPerSecondShort(outputTokensPerSecond)}
                    </span>
                  ) : (
                    <span className="text-slate-300/60 dark:text-slate-600/60">—</span>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </button>
  );
});

export type HomeRequestLogsPanelProps = {
  showCustomTooltip: boolean;
  title?: string;
  showSummaryText?: boolean;
  summaryTextOverride?: string;
  showOpenLogsPageButton?: boolean;
  showRefreshButton?: boolean;
  showCompactModeToggle?: boolean;
  compactModeOverride?: boolean;
  emptyStateTitle?: string;
  devPreviewEnabled?: boolean;

  traces: TraceSession[];

  requestLogs: RequestLogSummary[];
  requestLogsLoading: boolean;
  requestLogsRefreshing: boolean;
  requestLogsAvailable: boolean | null;
  onRefreshRequestLogs: () => void;

  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

export function HomeRequestLogsPanel({
  showCustomTooltip,
  title,
  showSummaryText = true,
  summaryTextOverride,
  showOpenLogsPageButton = true,
  showRefreshButton = true,
  showCompactModeToggle = true,
  compactModeOverride,
  emptyStateTitle = "当前没有最近使用记录",
  devPreviewEnabled = false,
  traces,
  requestLogs,
  requestLogsLoading,
  requestLogsRefreshing,
  requestLogsAvailable,
  onRefreshRequestLogs,
  selectedLogId,
  onSelectLogId,
}: HomeRequestLogsPanelProps) {
  const navigate = useNavigate();
  const [compactMode, setCompactMode] = useState(() => {
    try {
      const stored = localStorage.getItem("home_request_logs_compact");
      return stored == null ? true : stored === "true";
    } catch {
      return true;
    }
  });
  const handleCompactModeChange = (next: boolean) => {
    setCompactMode(next);
    try {
      localStorage.setItem("home_request_logs_compact", String(next));
    } catch {
      // ignore
    }
  };
  const effectiveCompactMode = compactModeOverride ?? compactMode;
  const previewTraces = useMemo(
    () => (devPreviewEnabled && traces.length === 0 ? buildPreviewTraces() : []),
    [devPreviewEnabled, traces.length]
  );
  const previewRequestLogs = useMemo(
    () => (devPreviewEnabled && requestLogs.length === 0 ? buildPreviewRequestLogs() : []),
    [devPreviewEnabled, requestLogs.length]
  );
  const previewSessionFolderLookups = useMemo(
    () => (devPreviewEnabled ? buildPreviewSessionFolderLookups() : []),
    [devPreviewEnabled]
  );
  const displayedTraces = traces.length > 0 ? traces : previewTraces;
  const displayedRequestLogs = requestLogs.length > 0 ? requestLogs : previewRequestLogs;
  const sortedRequestLogs = useMemo(() => {
    if (displayedRequestLogs.length <= 1) return displayedRequestLogs;
    return displayedRequestLogs.slice().sort(sortRequestLogsForDisplay);
  }, [displayedRequestLogs]);
  const summaryText =
    summaryTextOverride ??
    (requestLogsAvailable === false
      ? "数据不可用"
      : sortedRequestLogs.length === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsLoading || requestLogsRefreshing
          ? `更新中… · 共 ${sortedRequestLogs.length} 条`
          : `共 ${sortedRequestLogs.length} 条`);
  const realtimeTraceCandidates = useMemo(() => {
    const logsByTraceId = new Map<string, RequestLogSummary>();
    for (const log of sortedRequestLogs) {
      const traceId = log.trace_id?.trim();
      if (!traceId) continue;
      if (!logsByTraceId.has(traceId)) {
        logsByTraceId.set(traceId, log);
      }
    }

    const mergedTraceMap = new Map<string, TraceSession>();
    for (const trace of displayedTraces) {
      const traceId = trace.trace_id?.trim();
      if (!traceId || mergedTraceMap.has(traceId)) continue;
      mergedTraceMap.set(traceId, mergeTraceWithRequestLog(trace, logsByTraceId.get(traceId)));
    }
    for (const log of sortedRequestLogs) {
      if (!isPersistedRequestLogInProgress(log)) continue;
      const traceId = log.trace_id?.trim();
      if (!traceId || mergedTraceMap.has(traceId)) continue;
      mergedTraceMap.set(traceId, createRealtimeTraceFromInProgressLog(log));
    }

    const nowMs = Date.now();
    return Array.from(mergedTraceMap.values())
      .filter((t) => nowMs - t.first_seen_ms < 15 * 60 * 1000)
      .sort((a, b) => b.first_seen_ms - a.first_seen_ms)
      .slice(0, 20);
  }, [displayedTraces, sortedRequestLogs]);
  const sessionFolderLookupItems = useMemo(() => {
    const seen = new Set<string>();
    const out: CliSessionsFolderLookupInput[] = [];

    const pushIfNeeded = (cliKey: string, sessionId: string | null | undefined) => {
      if (!isFolderLookupCliKey(cliKey)) return;
      const normalized = sessionId?.trim();
      if (!normalized) return;
      const key = `${cliKey}:${normalized}`;
      if (seen.has(key)) return;
      seen.add(key);
      out.push({ source: cliKey, session_id: normalized });
    };

    for (const log of sortedRequestLogs) {
      pushIfNeeded(log.cli_key, log.session_id);
    }
    for (const trace of realtimeTraceCandidates) {
      pushIfNeeded(trace.cli_key, trace.session_id);
    }

    return out;
  }, [realtimeTraceCandidates, sortedRequestLogs]);
  const sessionFolderLookupQuery = useCliSessionsFolderLookupByIdsQuery(sessionFolderLookupItems);
  const sessionFolderLookupBySessionKey = useMemo(() => {
    const map = new Map<string, CliSessionsFolderLookupEntry>();
    for (const item of sessionFolderLookupQuery.data ?? []) {
      const key = sessionFolderLookupKey(item.source, item.session_id);
      if (!key) continue;
      map.set(key, item);
    }
    for (const item of previewSessionFolderLookups) {
      const key = sessionFolderLookupKey(item.source, item.session_id);
      if (!key || map.has(key)) continue;
      map.set(key, item);
    }
    return map;
  }, [previewSessionFolderLookups, sessionFolderLookupQuery.data]);
  const liveTracesByTraceId = useMemo(() => {
    const map = new Map<string, TraceSession>();
    for (const trace of realtimeTraceCandidates) {
      const traceId = trace.trace_id?.trim();
      if (!traceId || map.has(traceId)) continue;
      map.set(traceId, trace);
    }
    return map;
  }, [realtimeTraceCandidates]);
  const hasLiveInProgressRequestLogs = useMemo(
    () =>
      sortedRequestLogs.some((log) => {
        if (!isPersistedRequestLogInProgress(log)) return false;
        return liveTracesByTraceId.has(log.trace_id);
      }),
    [liveTracesByTraceId, sortedRequestLogs]
  );
  const nowMs = useNowMs(hasLiveInProgressRequestLogs, 250);

  return (
    <Card padding="sm" className="flex flex-col gap-3 lg:col-span-7 h-full">
      <div className="flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div className="flex flex-wrap items-center gap-2">
          <div className="text-sm font-semibold">{title ?? "最近代理记录"}</div>
        </div>

        <div className="flex items-center gap-2">
          {showSummaryText ? (
            <div className="text-xs text-slate-500 dark:text-slate-400">{summaryText}</div>
          ) : null}
          {showOpenLogsPageButton && (
            <Button
              onClick={() => navigate("/logs")}
              variant="ghost"
              size="sm"
              className="h-8 gap-1 px-2 text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400"
              disabled={requestLogsAvailable === false}
              title="打开代理记录页"
            >
              代理记录
              <ArrowUpRight className="h-3.5 w-3.5" />
            </Button>
          )}
          {showRefreshButton ? (
            <Button
              onClick={onRefreshRequestLogs}
              variant="ghost"
              size="sm"
              className="h-8 gap-1 px-2 text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400"
              disabled={
                requestLogsAvailable === false || requestLogsLoading || requestLogsRefreshing
              }
            >
              刷新
              <RefreshCw
                className={cn(
                  "h-3.5 w-3.5",
                  (requestLogsLoading || requestLogsRefreshing) && "animate-spin"
                )}
              />
            </Button>
          ) : null}
          {showCompactModeToggle ? (
            <div className="flex items-center gap-1.5 pl-1">
              <span className="text-xs text-slate-500 dark:text-slate-400">简洁模式</span>
              <Switch
                checked={effectiveCompactMode}
                onCheckedChange={handleCompactModeChange}
                size="sm"
                aria-label="最近使用记录简洁模式"
              />
            </div>
          ) : null}
        </div>
      </div>

      <div className="overflow-hidden flex-1 min-h-0 flex flex-col">
        <RequestLogsList
          realtimeTraceCandidates={realtimeTraceCandidates}
          formatUnixSeconds={formatUnixSecondsStable}
          showCustomTooltip={showCustomTooltip}
          compactMode={effectiveCompactMode}
          folderLookupBySessionKey={sessionFolderLookupBySessionKey}
          tracesByTraceId={liveTracesByTraceId}
          nowMs={nowMs}
          requestLogsAvailable={requestLogsAvailable}
          requestLogs={sortedRequestLogs}
          requestLogsLoading={requestLogsLoading}
          emptyStateTitle={emptyStateTitle}
          selectedLogId={selectedLogId}
          onSelectLogId={onSelectLogId}
        />
      </div>
    </Card>
  );
}

// Inner list component that conditionally applies virtualization
type RequestLogsListProps = {
  realtimeTraceCandidates: TraceSession[];
  formatUnixSeconds: (ts: number) => string;
  showCustomTooltip: boolean;
  compactMode: boolean;
  folderLookupBySessionKey: Map<string, CliSessionsFolderLookupEntry>;
  tracesByTraceId: Map<string, TraceSession>;
  nowMs: number;
  requestLogsAvailable: boolean | null;
  requestLogs: RequestLogSummary[];
  requestLogsLoading: boolean;
  emptyStateTitle: string;
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

const RequestLogsList = memo(function RequestLogsList({
  realtimeTraceCandidates,
  formatUnixSeconds,
  showCustomTooltip,
  compactMode,
  folderLookupBySessionKey,
  tracesByTraceId,
  nowMs,
  requestLogsAvailable,
  requestLogs,
  requestLogsLoading,
  emptyStateTitle,
  selectedLogId,
  onSelectLogId,
}: RequestLogsListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const renderedRequestLogs = useMemo(() => {
    if (realtimeTraceCandidates.length === 0) return requestLogs;
    const realtimeTraceIds = new Set(
      realtimeTraceCandidates.map((trace) => trace.trace_id?.trim()).filter(Boolean)
    );
    return requestLogs.filter((log) => {
      if (!isPersistedRequestLogInProgress(log)) return true;
      const traceId = log.trace_id?.trim();
      if (!traceId) return true;
      return !realtimeTraceIds.has(traceId);
    });
  }, [realtimeTraceCandidates, requestLogs]);
  const useVirtual = renderedRequestLogs.length >= VIRTUALIZATION_THRESHOLD;

  const virtualizer = useVirtualizer({
    count: renderedRequestLogs.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_LOG_CARD_HEIGHT,
    overscan: 8,
    enabled: useVirtual,
  });

  const virtualItems = virtualizer.getVirtualItems();

  // Non-virtualized fallback for small lists
  const plainList = !useVirtual && renderedRequestLogs.length > 0 && (
    <>
      {renderedRequestLogs.map((log) => {
        const trace = tracesByTraceId.get(log.trace_id);
        const liveNow = trace && isPersistedRequestLogInProgress(log) ? nowMs : 0;
        const sessionFolder = (() => {
          const key = sessionFolderLookupKey(log.cli_key, log.session_id ?? trace?.session_id);
          return key ? (folderLookupBySessionKey.get(key) ?? null) : null;
        })();
        return (
          <RequestLogCard
            compactMode={compactMode}
            key={log.id}
            log={log}
            liveTrace={trace}
            nowMs={liveNow}
            isSelected={selectedLogId === log.id}
            sessionFolder={sessionFolder}
            showCustomTooltip={showCustomTooltip}
            onSelectLogId={onSelectLogId}
            formatUnixSeconds={formatUnixSeconds}
          />
        );
      })}
    </>
  );

  return (
    <div ref={scrollRef} className="scrollbar-overlay flex-1 overflow-auto pr-1 py-2">
      {/* Wrapper isolates trace exit animations from the log list below,
          preventing layout shifts when multiple traces collapse simultaneously. */}
      <div className="will-change-[height]">
        <RealtimeTraceCards
          folderLookupBySessionKey={folderLookupBySessionKey}
          traces={realtimeTraceCandidates}
          formatUnixSeconds={formatUnixSeconds}
          showCustomTooltip={showCustomTooltip}
        />
      </div>

      {requestLogsAvailable === false ? (
        <div className="p-4 text-sm text-slate-600 dark:text-slate-400">数据不可用</div>
      ) : renderedRequestLogs.length === 0 ? (
        requestLogsLoading ? (
          <div className="flex items-center justify-center gap-2 p-4 text-sm text-slate-600 dark:text-slate-400">
            <Spinner size="sm" />
            加载中…
          </div>
        ) : (
          <EmptyState title={emptyStateTitle} />
        )
      ) : useVirtual ? (
        <div
          style={{
            height: virtualizer.getTotalSize(),
            width: "100%",
            position: "relative",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              transform: `translateY(${virtualItems[0]?.start ?? 0}px)`,
            }}
          >
            {virtualItems.map((virtualRow) => {
              const vLog = renderedRequestLogs[virtualRow.index];
              const vTrace = tracesByTraceId.get(vLog.trace_id);
              const vNow = vTrace && isPersistedRequestLogInProgress(vLog) ? nowMs : 0;
              const sessionFolder = (() => {
                const key = sessionFolderLookupKey(
                  vLog.cli_key,
                  vLog.session_id ?? vTrace?.session_id
                );
                return key ? (folderLookupBySessionKey.get(key) ?? null) : null;
              })();
              return (
                <div key={vLog.id} data-index={virtualRow.index} ref={virtualizer.measureElement}>
                  <RequestLogCard
                    compactMode={compactMode}
                    log={vLog}
                    liveTrace={vTrace}
                    nowMs={vNow}
                    isSelected={selectedLogId === vLog.id}
                    sessionFolder={sessionFolder}
                    showCustomTooltip={showCustomTooltip}
                    onSelectLogId={onSelectLogId}
                    formatUnixSeconds={formatUnixSeconds}
                  />
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        plainList
      )}
    </div>
  );
});
