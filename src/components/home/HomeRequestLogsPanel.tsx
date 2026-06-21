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
} from "../../services/cli/cliSessions";
import {
  isPersistedRequestLogInProgress,
  requestLogCreatedAtMs,
} from "../../services/gateway/requestLogState";
import {
  buildRequestActivityProjection,
  type ProjectedRealtimeCard,
  type ProjectedRequestLogRow,
} from "../../services/gateway/requestActivityProjection";
import type { RequestLogSummary } from "../../services/gateway/requestLogs";
import type { TraceSession } from "../../services/gateway/traceStore";
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
  FastModeBadge,
  formatClaudeModelMappingText,
  FolderBadge,
  FreeBadge,
  getErrorCodeLabel,
  hasPriorityServiceTierSpecialSetting,
  resolveClaudeModelMappingFromSpecialSettings,
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
  const persistedRunningMs = (() => {
    const createdAtMs = requestLogCreatedAtMs(log);
    if (createdAtMs <= 0) return log.duration_ms;
    return Math.max(0, nowMs - createdAtMs);
  })();
  const displayDurationMs =
    isInProgress && liveTrace
      ? (resolveLiveTraceDurationMs(liveTrace, nowMs) ?? persistedRunningMs)
      : isInProgress
        ? persistedRunningMs
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

  const modelText = formatClaudeModelMappingText(
    log.requested_model,
    resolveClaudeModelMappingFromSpecialSettings(log.special_settings_json, log.final_provider_id)
  );

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

  // Codex fast mode (priority service tier) detection
  const isPriorityServiceTier =
    log.cli_key === "codex" && hasPriorityServiceTierSpecialSetting(log.special_settings_json);

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
          "relative transition-all duration-300 ease-out group/item mx-2 my-1.5 rounded-lg border",
          isSelected
            ? "bg-state-selected/65 border-state-selected-border/80 shadow-request-log-selected"
            : auditMeta.muted
              ? "bg-surface-inset/60 border-border/30 opacity-75 hover:opacity-100 hover:bg-surface-inset/90 hover:border-border/60 dark:bg-surface-inset/40 dark:border-border/30"
              : "bg-secondary/35 border-border/40 hover:bg-secondary/65 hover:-translate-y-[0.5px] hover:shadow-request-log-hover dark:bg-secondary/45 dark:border-border/40 dark:hover:bg-secondary/80 dark:hover:border-border/60 dark:hover:shadow-request-log-hover-dark"
        )}
      >
        {/* Selection indicator */}
        <div
          className={cn(
            "absolute left-0 top-3 bottom-3 w-[2.5px] rounded-full transition-all duration-300",
            isSelected
              ? "bg-gradient-to-b from-page-accent to-page-secondary shadow-page-accent-soft opacity-100"
              : "bg-muted opacity-0 group-hover/item:opacity-30 dark:bg-muted"
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
                  className="inline-flex min-w-0 items-center gap-1 rounded-md bg-muted/65 px-2 py-0.5 text-[11px] font-medium text-muted-foreground border border-border/40 dark:bg-muted/40 dark:border-border/20 shadow-pill-subtle"
                  title={providerTitle}
                >
                  <Server className="h-3 w-3 shrink-0 text-muted-foreground/60" />
                  <span className={compactTextClass}>{providerText}</span>
                </span>
              )}

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
                "flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground whitespace-nowrap",
                compactMode ? "self-start" : "ml-auto w-[150px] justify-end"
              )}
            >
              {log.session_reuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}
              <span className="flex items-center gap-1 w-[64px] justify-end shrink-0 select-none">
                <Clock className="h-3 w-3 shrink-0" />
                <span>{formatUnixSeconds(log.created_at)}</span>
              </span>
            </span>
          </div>

          {!compactMode && auditMeta.summary ? (
            <div className="mb-1.5 text-[11px] text-muted-foreground">{auditMeta.summary}</div>
          ) : null}

          {!compactMode && (
            <div className="flex items-start gap-3 text-[11px]">
              <div className="flex flex-col gap-y-0.5 w-[110px] shrink-0" title={providerTitle}>
                <div className="flex items-center gap-1 h-4">
                  <Server className="h-3 w-3 text-muted-foreground/60 shrink-0" />
                  <span className="truncate font-semibold text-foreground/85">{providerText}</span>
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
                          <span className="text-[11px] text-muted-foreground hover:text-indigo-600 dark:hover:text-indigo-400 cursor-help">
                            {routeMeta.label}
                          </span>
                        </Tooltip>
                      ) : (
                        <span
                          className="text-[11px] text-muted-foreground cursor-help"
                          title={routeMeta.tooltipText}
                        >
                          {routeMeta.label}
                        </span>
                      )
                    ) : null}

                    {showCostMultiplier ? (
                      <span className="inline-flex items-center text-[11px] font-medium text-muted-foreground shrink-0">
                        {costMultiplierText}
                      </span>
                    ) : null}
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-4 gap-x-3 gap-y-0.5 flex-1 text-muted-foreground">
                <div className="flex items-center gap-1 h-4" title="Input Tokens">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    输入
                  </span>
                  <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                    {formatInteger(effectiveInputTokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Write">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    缓存创建
                  </span>
                  {cacheWrite.tokens != null ? (
                    <>
                      <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                        {formatInteger(cacheWrite.tokens)}
                      </span>
                      {cacheWrite.ttl && cacheWrite.tokens > 0 && (
                        <span className="text-[10px] font-medium text-muted-foreground/60">
                          ({cacheWrite.ttl})
                        </span>
                      )}
                    </>
                  ) : (
                    <span className="text-muted-foreground/40 text-xs font-mono select-none">
                      —
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="TTFB">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    首字
                  </span>
                  <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                    {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                  </span>
                </div>
                <div
                  className="flex items-center gap-1 h-4"
                  title={costUsdText === "—" ? undefined : costUsdText}
                >
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    花费
                  </span>
                  <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                    {costUsdText}
                  </span>
                  {isPriorityServiceTier && <FastModeBadge showCustomTooltip={showCustomTooltip} />}
                </div>

                <div className="flex items-center gap-1 h-4" title="Output Tokens">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    输出
                  </span>
                  <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                    {formatInteger(log.output_tokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Read">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    缓存读取
                  </span>
                  {log.cache_read_input_tokens != null ? (
                    <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                      {formatInteger(log.cache_read_input_tokens)}
                    </span>
                  ) : (
                    <span className="text-muted-foreground/40 text-xs font-mono select-none">
                      —
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="Duration">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    耗时
                  </span>
                  <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
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
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/75 select-none shrink-0">
                    速率
                  </span>
                  {outputTokensPerSecond != null ? (
                    <span className="font-mono tabular-nums text-xs font-semibold text-foreground/90 truncate">
                      {formatTokensPerSecondShort(outputTokensPerSecond)}
                    </span>
                  ) : (
                    <span className="text-muted-foreground/40 text-xs font-mono select-none">
                      —
                    </span>
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
  const clockEnabled = useMemo(
    () =>
      displayedTraces.length > 0 ||
      displayedRequestLogs.some((log) => isPersistedRequestLogInProgress(log)),
    [displayedRequestLogs, displayedTraces.length]
  );
  const nowMs = useNowMs(clockEnabled, 250);
  const activityProjection = useMemo(
    () =>
      buildRequestActivityProjection({
        requestLogs: displayedRequestLogs,
        traces: displayedTraces,
        nowMs,
        realtimeCardLimit: 5,
        realtimeCandidateLimit: 20,
      }),
    [displayedRequestLogs, displayedTraces, nowMs]
  );
  const summaryText =
    summaryTextOverride ??
    (requestLogsAvailable === false
      ? "数据不可用"
      : activityProjection.summaryCount === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsLoading || requestLogsRefreshing
          ? `更新中… · 共 ${activityProjection.summaryCount} 条`
          : `共 ${activityProjection.summaryCount} 条`);
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

    for (const row of activityProjection.requestRows) {
      pushIfNeeded(row.log.cli_key, row.log.session_id ?? row.liveTrace?.session_id);
    }
    for (const card of activityProjection.realtimeCards) {
      pushIfNeeded(card.trace.cli_key, card.trace.session_id);
    }

    return out;
  }, [activityProjection]);
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

  return (
    <Card padding="sm" className="flex flex-col gap-3 lg:col-span-7 h-full">
      <div className="flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div className="flex flex-wrap items-center gap-2">
          <div className="text-sm font-semibold">{title ?? "最近代理记录"}</div>
        </div>

        <div className="flex items-center gap-2">
          {showSummaryText ? (
            <div className="text-xs text-muted-foreground">{summaryText}</div>
          ) : null}
          {showOpenLogsPageButton && (
            <Button
              onClick={() => navigate("/logs")}
              variant="ghost"
              size="sm"
              className="h-8 gap-1 px-2 text-muted-foreground hover:text-indigo-600 dark:hover:text-indigo-400"
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
              className="h-8 gap-1 px-2 text-muted-foreground hover:text-indigo-600 dark:hover:text-indigo-400"
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
              <span className="text-xs text-muted-foreground">简洁模式</span>
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
          realtimeCards={activityProjection.realtimeCards}
          formatUnixSeconds={formatUnixSecondsStable}
          showCustomTooltip={showCustomTooltip}
          compactMode={effectiveCompactMode}
          folderLookupBySessionKey={sessionFolderLookupBySessionKey}
          nowMs={nowMs}
          requestLogsAvailable={requestLogsAvailable}
          requestRows={activityProjection.requestRows}
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
  realtimeCards: ProjectedRealtimeCard[];
  formatUnixSeconds: (ts: number) => string;
  showCustomTooltip: boolean;
  compactMode: boolean;
  folderLookupBySessionKey: Map<string, CliSessionsFolderLookupEntry>;
  nowMs: number;
  requestLogsAvailable: boolean | null;
  requestRows: ProjectedRequestLogRow[];
  requestLogsLoading: boolean;
  emptyStateTitle: string;
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

const RequestLogsList = memo(function RequestLogsList({
  realtimeCards,
  formatUnixSeconds,
  showCustomTooltip,
  compactMode,
  folderLookupBySessionKey,
  nowMs,
  requestLogsAvailable,
  requestRows,
  requestLogsLoading,
  emptyStateTitle,
  selectedLogId,
  onSelectLogId,
}: RequestLogsListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const hasRealtimeCards = realtimeCards.length > 0;
  const useVirtual = requestRows.length >= VIRTUALIZATION_THRESHOLD;

  const virtualizer = useVirtualizer({
    count: requestRows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_LOG_CARD_HEIGHT,
    overscan: 8,
    enabled: useVirtual,
  });

  const virtualItems = virtualizer.getVirtualItems();

  // Non-virtualized fallback for small lists
  const plainList = !useVirtual && requestRows.length > 0 && (
    <>
      {requestRows.map((row) => {
        const { log, liveTrace: trace } = row;
        const liveNow = isPersistedRequestLogInProgress(log) ? nowMs : 0;
        const sessionFolder = (() => {
          const key = sessionFolderLookupKey(log.cli_key, log.session_id ?? trace?.session_id);
          return key ? (folderLookupBySessionKey.get(key) ?? null) : null;
        })();
        return (
          <RequestLogCard
            compactMode={compactMode}
            key={log.id}
            log={log}
            liveTrace={trace ?? undefined}
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
          cards={realtimeCards}
          nowMs={nowMs}
          formatUnixSeconds={formatUnixSeconds}
          showCustomTooltip={showCustomTooltip}
        />
      </div>

      {requestLogsAvailable === false ? (
        <div className="p-4 text-sm text-muted-foreground">数据不可用</div>
      ) : requestRows.length === 0 ? (
        requestLogsLoading ? (
          <div className="flex items-center justify-center gap-2 p-4 text-sm text-muted-foreground">
            <Spinner size="sm" />
            加载中…
          </div>
        ) : hasRealtimeCards ? null : (
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
              const vRow = requestRows[virtualRow.index];
              const vLog = vRow.log;
              const vTrace = vRow.liveTrace;
              const vNow = isPersistedRequestLogInProgress(vLog) ? nowMs : 0;
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
                    liveTrace={vTrace ?? undefined}
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
