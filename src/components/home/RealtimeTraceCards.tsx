// Usage:
// - Render in Home page "概览 / 使用记录" area to show up-to-date in-flight traces.
// - Accepts a list of `TraceSession` candidates; component applies its own visibility + exit animation logic.

import { memo, useEffect, useMemo, useState } from "react";
import { cliShortLabel } from "../../constants/clis";
import { GatewayErrorCodes } from "../../constants/gatewayErrorCodes";
import { useNowMs } from "../../hooks/useNowMs";
import type { CliSessionsFolderLookupEntry } from "../../services/cli/cliSessions";
import type { CliKey } from "../../services/providers/providers";
import type { TraceSession } from "../../services/gateway/traceStore";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatTokensPerSecond,
  formatTokensPerSecondShort,
  formatUsd,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { Clock, Server, Loader2, CheckCircle2, XCircle } from "lucide-react";
import {
  computeEffectiveInputTokens,
  computeStatusBadge,
  FolderBadge,
  formatClaudeModelMappingText,
  FreeBadge,
  getErrorCodeLabel,
  SessionReuseBadge,
} from "./HomeLogShared";
import { CliBrandIcon } from "./CliBrandIcon";

export type RealtimeTraceCardsProps = {
  folderLookupBySessionKey: Map<string, CliSessionsFolderLookupEntry>;
  traces: TraceSession[];
  formatUnixSeconds: (ts: number) => string;
  showCustomTooltip: boolean;
};

function sessionFolderLookupKey(cliKey: string, sessionId: string | null | undefined) {
  const normalized = sessionId?.trim();
  if (!normalized) return null;
  return `${cliKey}:${normalized}`;
}

const REALTIME_TRACE_EXIT_START_MS = 600;
const REALTIME_TRACE_EXIT_ANIM_MS = 400;
const REALTIME_TRACE_EXIT_TOTAL_MS =
  REALTIME_TRACE_EXIT_START_MS + REALTIME_TRACE_EXIT_ANIM_MS + 100;

/**
 * UI-level safety net: hide traces stuck in "in progress" beyond this threshold.
 * Works independently of traceStore pruning for defense-in-depth.
 */
const STALE_TRACE_TIMEOUT_MS = 5 * 60 * 1000;

/**
 * When multiple traces complete within this window, they batch-exit together
 * to avoid staggered collapse animations that feel chaotic.
 */
const BATCH_EXIT_WINDOW_MS = 500;
const LIVE_METRIC_CARD_BASE = "h-full min-w-0 rounded-xl border px-2.5 py-1.5 transition-all duration-300 hover:scale-[1.01] hover:shadow-[0_4px_12px_rgba(59,130,246,0.04)] dark:hover:shadow-[0_4px_16px_rgba(0,0,0,0.3)]";
const LIVE_METRIC_CARD_SURFACE =
  "border-slate-200 bg-slate-100/50 shadow-[inset_0_1px_1px_rgba(255,255,255,0.9),0_1px_2px_rgba(0,0,0,0.01)] dark:border-[#2b3547] dark:bg-[#1f2633]/60 dark:shadow-[inset_0_1px_1px_rgba(255,255,255,0.03)]";
const LIVE_METRIC_LABEL = "leading-3 text-[10px] font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400";
const LIVE_METRIC_VALUE = "mt-1.5 truncate leading-none font-bold font-mono tracking-tight text-slate-700 dark:text-slate-200";
const LIVE_METRIC_STAGE_VALUE =
  "mt-1.5 truncate font-extrabold leading-none text-blue-600 dark:text-sky-400 drop-shadow-[0_0_8px_rgba(56,189,248,0.2)]";

function shouldKeepRealtimeTraceVisible(trace: TraceSession, nowMs: number): boolean {
  if (!trace.summary) {
    return nowMs - trace.last_seen_ms < STALE_TRACE_TIMEOUT_MS;
  }
  return Math.max(0, nowMs - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
}

function shouldUseRealtimeTraceClock(traces: TraceSession[], nowMs: number): boolean {
  return traces.some((trace) => shouldKeepRealtimeTraceVisible(trace, nowMs));
}

export const RealtimeTraceCards = memo(function RealtimeTraceCards({
  folderLookupBySessionKey,
  traces,
  formatUnixSeconds,
  showCustomTooltip,
}: RealtimeTraceCardsProps) {
  const [clockEnabled, setClockEnabled] = useState(() =>
    shouldUseRealtimeTraceClock(traces, Date.now())
  );
  const clockNowMs = useNowMs(clockEnabled, 250);
  const nowMs = clockEnabled ? clockNowMs : Date.now();

  useEffect(() => {
    const nextEnabled = shouldUseRealtimeTraceClock(traces, Date.now());
    setClockEnabled((current) => (current === nextEnabled ? current : nextEnabled));
  }, [traces]);

  useEffect(() => {
    if (!clockEnabled) return;
    if (shouldUseRealtimeTraceClock(traces, clockNowMs)) return;
    setClockEnabled(false);
  }, [clockEnabled, clockNowMs, traces]);

  const visibleTraces = useMemo(() => {
    const kept = traces.filter((trace) => shouldKeepRealtimeTraceVisible(trace, nowMs));
    return kept.slice(0, 5);
  }, [traces, nowMs]);

  // Compute a batch-aligned exit threshold: if multiple traces completed within
  // BATCH_EXIT_WINDOW_MS of each other, they all exit when the earliest one would.
  const batchExitThresholdMs = useMemo(() => {
    const completedTraces = visibleTraces.filter((t) => t.summary);
    if (completedTraces.length <= 1) return null;
    // Find earliest completion
    const earliestLastSeen = Math.min(...completedTraces.map((t) => t.last_seen_ms));
    const latestLastSeen = Math.max(...completedTraces.map((t) => t.last_seen_ms));
    // If completions are within the batch window, align exits to the earliest
    if (latestLastSeen - earliestLastSeen <= BATCH_EXIT_WINDOW_MS) {
      return earliestLastSeen + REALTIME_TRACE_EXIT_START_MS;
    }
    return null;
  }, [visibleTraces]);

  return (
    <>
      {visibleTraces.map((trace) => {
        const completedAgeMs = trace.summary ? Math.max(0, nowMs - trace.last_seen_ms) : 0;
        const isExiting =
          Boolean(trace.summary) &&
          (batchExitThresholdMs != null
            ? nowMs >= batchExitThresholdMs
            : completedAgeMs >= REALTIME_TRACE_EXIT_START_MS);
        const runningMs = trace.summary
          ? trace.summary.duration_ms
          : Math.max(0, nowMs - trace.first_seen_ms);

        const summaryStatus = trace.summary?.status ?? null;
        const summaryErrorCode = trace.summary?.error_code ?? null;
        const isInProgress = !trace.summary;

        const attemptRoute = (() => {
          const sortedAttempts = (trace.attempts ?? [])
            .slice()
            .sort((a, b) => a.attempt_index - b.attempt_index);

          type RouteSeg = { provider: string; status: "success" | "started" | "failed" };
          const segs: RouteSeg[] = [];

          for (const attempt of sortedAttempts) {
            const raw = attempt.provider_name?.trim();
            if (!raw || raw === "Unknown") continue;

            const status: RouteSeg["status"] =
              attempt.outcome === "success"
                ? "success"
                : attempt.outcome === "started"
                  ? "started"
                  : "failed";

            const last = segs[segs.length - 1];
            if (last?.provider === raw) {
              if (last.status === status) continue;
              if (last.status === "success") continue;
              if (status === "success") {
                last.status = "success";
                continue;
              }
              if (last.status === "started") continue;
              if (status === "started") {
                last.status = "started";
                continue;
              }
              continue;
            }

            segs.push({ provider: raw, status });
          }

          const startProvider = segs[0]?.provider ?? null;
          const endProvider = segs[segs.length - 1]?.provider ?? null;
          const providerText = endProvider ?? "未知";

          return { providerText, startProvider, endProvider, segments: segs };
        })();

        const hasFailover =
          attemptRoute.segments.length > 1 ||
          attemptRoute.segments.some((s) => s.status === "failed");

        const statusBadge = computeStatusBadge({
          status: summaryStatus,
          errorCode: summaryErrorCode,
          inProgress: isInProgress,
          hasFailover,
        });
        const isClientAbort =
          statusBadge.isClientAbort ||
          summaryStatus === 499 ||
          summaryErrorCode === GatewayErrorCodes.REQUEST_ABORTED ||
          summaryErrorCode === GatewayErrorCodes.STREAM_ABORTED;
        const hasSessionReuse = (trace.attempts ?? []).some(
          (attempt) => attempt.session_reuse === true
        );
        const latestAttempt = (trace.attempts ?? [])
          .slice()
          .sort((a, b) => b.attempt_index - a.attempt_index)[0];

        const providerText = attemptRoute.providerText;
        const sessionFolder = (() => {
          const key = sessionFolderLookupKey(trace.cli_key, trace.session_id);
          return key ? (folderLookupBySessionKey.get(key) ?? null) : null;
        })();

        const routeSummary = (() => {
          if (!attemptRoute.startProvider && !attemptRoute.endProvider) return "—";
          if (!attemptRoute.startProvider) return attemptRoute.endProvider ?? "—";
          if (!attemptRoute.endProvider) return attemptRoute.startProvider;
          const routeSegCount = attemptRoute.segments.length;
          const extra = routeSegCount > 2 ? ` +${routeSegCount - 2}` : "";
          return attemptRoute.startProvider === attemptRoute.endProvider
            ? attemptRoute.startProvider
            : `${attemptRoute.startProvider} → ${attemptRoute.endProvider}${extra}`;
        })();

        const modelText = formatClaudeModelMappingText(
          trace.requested_model,
          trace.claude_model_mapping
        );
        const cliLabel = cliShortLabel(trace.cli_key);

        const cacheWrite = (() => {
          const s = trace.summary;
          if (!s)
            return {
              tokens: null as number | null,
              ttl: null as "5m" | "1h" | null,
            };
          // 优先 5m，其次 1h，最后用 cache_creation_input_tokens 汇总
          if (s.cache_creation_5m_input_tokens != null && s.cache_creation_5m_input_tokens > 0) {
            return { tokens: s.cache_creation_5m_input_tokens, ttl: "5m" as const };
          }
          if (s.cache_creation_1h_input_tokens != null && s.cache_creation_1h_input_tokens > 0) {
            return { tokens: s.cache_creation_1h_input_tokens, ttl: "1h" as const };
          }
          if (s.cache_creation_input_tokens != null && s.cache_creation_input_tokens > 0) {
            return { tokens: s.cache_creation_input_tokens, ttl: null };
          }
          if (s.cache_creation_5m_input_tokens != null) {
            return { tokens: s.cache_creation_5m_input_tokens, ttl: "5m" as const };
          }
          if (s.cache_creation_1h_input_tokens != null) {
            return { tokens: s.cache_creation_1h_input_tokens, ttl: "1h" as const };
          }
          if (s.cache_creation_input_tokens != null) {
            return { tokens: s.cache_creation_input_tokens, ttl: null };
          }
          return { tokens: null as number | null, ttl: null as "5m" | "1h" | null };
        })();

        const ttfbMs = trace.summary
          ? sanitizeTtfbMs(trace.summary.ttfb_ms ?? null, trace.summary.duration_ms)
          : null;

        const effectiveInputTokens = computeEffectiveInputTokens(
          trace.cli_key,
          trace.summary?.input_tokens ?? null,
          trace.summary?.cache_read_input_tokens ?? null
        );
        const displayInputTokens = effectiveInputTokens ?? (isClientAbort ? 0 : null);
        const displayOutputTokens = trace.summary?.output_tokens ?? (isClientAbort ? 0 : null);
        const displayCacheReadTokens =
          trace.summary?.cache_read_input_tokens ?? (isClientAbort ? 0 : null);
        const displayCacheWriteTokens = cacheWrite.tokens ?? (isClientAbort ? 0 : null);
        const displayCostUsd = trace.summary?.cost_usd ?? (isClientAbort ? 0 : null);
        const displayCostText = displayCostUsd == null ? "—" : formatUsd(displayCostUsd);
        const costMultiplier =
          typeof trace.summary?.cost_multiplier === "number" ? trace.summary.cost_multiplier : null;
        const isFree = costMultiplier === 0;
        const showCostMultiplier =
          costMultiplier != null && costMultiplier >= 0 && Math.abs(costMultiplier - 1) > 0.0001;
        const costMultiplierText = isFree
          ? "免费"
          : costMultiplier != null
            ? `x${costMultiplier.toFixed(2)}`
            : null;

        const outputTokensPerSecond = trace.summary
          ? computeOutputTokensPerSecond(displayOutputTokens, trace.summary.duration_ms, ttfbMs)
          : null;
        const displayOutputTokensPerSecond =
          outputTokensPerSecond ?? (isClientAbort && displayOutputTokens === 0 ? 0 : null);
        const routeLabel = (() => {
          if (attemptRoute.segments.length === 0) return null;
          if (isInProgress) return "链路[进行中]";
          if (hasFailover) return `链路[降级*${attemptRoute.segments.length}]`;
          return "链路";
        })();
        const routeTooltipText =
          routeSummary !== "—"
            ? routeSummary
            : attemptRoute.segments.length > 0
              ? attemptRoute.segments.map((seg) => seg.provider).join(" → ")
              : null;
        const providerTitle = providerText;
        const liveStageText = (() => {
          if (!isInProgress) return null;
          if (!latestAttempt) return "等待首个尝试";
          if (hasFailover) return "切换处理中";
          if (latestAttempt.outcome === "started") return "处理中";
          return "等待结果";
        })();
        const liveRouteText =
          routeSummary !== "—"
            ? routeSummary
            : latestAttempt?.provider_name?.trim() || providerText || "等待 provider";
        return (
          <div
            key={trace.trace_id}
            className={cn(
              "transform overflow-hidden transition-all ease-out motion-reduce:transition-none motion-reduce:transform-none",
              isExiting
                ? "max-h-0 opacity-0 scale-y-95 !mt-0 !mb-0 duration-400 ease-in"
                : "max-h-[220px] opacity-100 scale-y-100 duration-300 ease-out my-1.5 mx-2"
            )}
          >
            <div
              className={cn(
                "group/item relative rounded-lg border transition-all duration-300 ease-out",
                isInProgress
                  ? "border-blue-200/50 bg-gradient-to-br from-white/95 to-slate-50/95 shadow-[0_8px_30px_rgba(219,234,254,0.15),inset_0_1px_1px_rgba(255,255,255,0.7)] dark:border-slate-800/60 dark:bg-gradient-to-br dark:from-[#1b212c]/90 dark:to-[#10141d]/90 dark:shadow-[0_8px_30px_rgba(0,0,0,0.4),inset_0_1px_1px_rgba(255,255,255,0.03)] hover:border-blue-400/50 dark:hover:border-blue-900/40 hover:shadow-[0_8px_30px_rgba(59,130,246,0.15)] dark:hover:shadow-[0_8px_30px_rgba(30,58,138,0.3)] hover:scale-[1.005]"
                  : "bg-gradient-to-br from-white/95 to-slate-50/95 border-slate-200/50 shadow-sm dark:bg-gradient-to-br dark:from-[#1b212c]/90 dark:to-[#10141d]/90 dark:border-slate-800/60 hover:shadow-[0_8px_20px_-4px_rgba(148,163,184,0.15)] dark:hover:shadow-[0_8px_24px_rgba(0,0,0,0.3)] hover:border-slate-300/60 dark:hover:border-slate-700/60 hover:from-white dark:hover:from-[#1f2633]/90 dark:hover:to-[#121721]/90 hover:scale-[1.002]"
              )}
            >
              <div
                className={cn(
                  "absolute left-0 top-2 bottom-2 w-[3.5px] rounded-r-full transition-all duration-500 origin-center",
                  isInProgress
                    ? "bg-gradient-to-b from-blue-500 via-sky-400 to-blue-600 shadow-[0_0_10px_rgba(59,130,246,0.6)]"
                    : statusBadge.isError
                      ? "bg-gradient-to-b from-rose-500 to-pink-500 shadow-[0_0_8px_rgba(244,63,94,0.4)]"
                      : hasFailover
                        ? "bg-gradient-to-b from-amber-500 to-yellow-500 shadow-[0_0_8px_rgba(245,158,11,0.4)]"
                        : "bg-slate-300 dark:bg-slate-700 opacity-40"
                )}
              />

              <div className="px-3 py-2.5">
                <div className="mb-1.5 flex min-w-0 items-center gap-2">
                  <span
                    className={cn(
                      "inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium",
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
                    <span className="flex-1 text-center truncate">{statusBadge.text}</span>
                  </span>

                  <span
                    className="inline-flex min-w-0 items-center gap-1 rounded-md bg-slate-100/80 px-2 py-0.5 text-[11px] font-medium text-slate-600 border border-slate-200/50 dark:bg-slate-800/80 dark:text-slate-300 dark:border-slate-700/50 shadow-[0_1px_2px_rgba(0,0,0,0.02)]"
                    title={`${cliLabel} / ${modelText}`}
                  >
                    <CliBrandIcon
                      cliKey={trace.cli_key as CliKey}
                      className="h-2.5 w-2.5 shrink-0 rounded-[3px] object-contain opacity-80"
                    />
                    <span className="shrink-0">{cliLabel} /</span>
                    <span className="truncate">{modelText}</span>
                  </span>

                  {sessionFolder && (
                    <FolderBadge
                      folderName={sessionFolder.folder_name}
                      folderPath={sessionFolder.folder_path}
                    />
                  )}

                  {hasSessionReuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}

                  {isFree && <FreeBadge />}

                  {summaryErrorCode && (
                    <span className="shrink-0 rounded-md bg-amber-50/80 px-2 py-0.5 text-[11px] font-semibold text-amber-600 ring-1 ring-inset ring-amber-500/10 dark:bg-amber-500/15 dark:text-amber-300 dark:ring-amber-400/20 shadow-[0_1px_2px_rgba(0,0,0,0.02)] border border-amber-500/10 dark:border-amber-400/10">
                      {getErrorCodeLabel(summaryErrorCode)}
                    </span>
                  )}

                  {!isInProgress ? (
                    <span className="ml-auto flex w-[150px] shrink-0 items-center justify-end gap-1.5 text-xs text-slate-400 dark:text-slate-500 whitespace-nowrap">
                      <Clock className="h-3 w-3 shrink-0" />
                      {formatUnixSeconds(Math.floor(trace.first_seen_ms / 1000))}
                    </span>
                  ) : (
                    <span className="ml-auto flex shrink-0 items-center gap-2 whitespace-nowrap">
                      <span className="inline-flex items-center gap-1.5 text-xs font-mono font-semibold tabular-nums text-slate-500 dark:text-slate-400">
                        <Clock className="h-3 w-3 shrink-0 text-blue-500/80 dark:text-blue-400/80" />
                        {formatDurationMs(runningMs)}
                      </span>
                    </span>
                  )}
                </div>

                {isInProgress ? (
                  <div className="grid grid-cols-1 gap-2 text-[11px] sm:grid-cols-9">
                    <div
                      className={cn(
                        LIVE_METRIC_CARD_BASE,
                        LIVE_METRIC_CARD_SURFACE,
                        "sm:col-span-3"
                      )}
                    >
                      <div className={LIVE_METRIC_LABEL}>当前阶段</div>
                      <div className={LIVE_METRIC_STAGE_VALUE}>{liveStageText}</div>
                    </div>
                    <div
                      className={cn(
                        LIVE_METRIC_CARD_BASE,
                        LIVE_METRIC_CARD_SURFACE,
                        "sm:col-span-3"
                      )}
                    >
                      <div className={LIVE_METRIC_LABEL}>尝试次数</div>
                      <div className={cn(LIVE_METRIC_VALUE, "font-mono tabular-nums")}>
                        {formatInteger(trace.attempts.length)}
                      </div>
                    </div>
                    <div
                      className={cn(
                        LIVE_METRIC_CARD_BASE,
                        LIVE_METRIC_CARD_SURFACE,
                        "sm:col-span-3"
                      )}
                    >
                      <div className={LIVE_METRIC_LABEL}>当前链路</div>
                      <div
                        className={cn(LIVE_METRIC_VALUE, "font-medium")}
                        title={routeTooltipText ?? liveRouteText}
                      >
                        {liveRouteText}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-start gap-3 text-[11px]">
                    <div
                      className="flex w-[110px] shrink-0 flex-col gap-y-0.5"
                      title={providerTitle}
                    >
                      <div className="flex items-center gap-1 h-4">
                        <Server className="h-3 w-3 text-muted-foreground/80 dark:text-muted-foreground/80 shrink-0" />
                        <span className="truncate font-semibold text-muted-foreground dark:text-secondary-foreground">
                          {providerText}
                        </span>
                      </div>
                      <div className="flex items-center h-4">
                        <div className="flex min-w-0 w-full items-center gap-1">
                          {routeLabel && routeTooltipText ? (
                            <span
                              className="cursor-help text-[11px] text-muted-foreground"
                              title={routeTooltipText}
                            >
                              {routeLabel}
                            </span>
                          ) : null}
                          {showCostMultiplier ? (
                            <span className="inline-flex shrink-0 items-center text-[11px] font-medium text-muted-foreground">
                              {costMultiplierText}
                            </span>
                          ) : null}
                        </div>
                      </div>
                    </div>

                    <div className="grid flex-1 grid-cols-4 gap-x-3 gap-y-0.5 text-slate-500 dark:text-slate-400">
                      <div className="flex items-center gap-1 h-4" title="Input Tokens">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          输入
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                          {formatInteger(displayInputTokens)}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cache Write">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          缓存创建
                        </span>
                        {displayCacheWriteTokens != null ? (
                          <>
                            <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                              {formatInteger(displayCacheWriteTokens)}
                            </span>
                            {cacheWrite.ttl && displayCacheWriteTokens > 0 && (
                              <span className="text-slate-400/70 dark:text-slate-500/70 text-[10px]">
                                ({cacheWrite.ttl})
                              </span>
                            )}
                          </>
                        ) : (
                          <span className="text-slate-300 dark:text-slate-700">
                            —
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-1 h-4" title="TTFB">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          首字
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                          {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cost">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          花费
                        </span>
                        <span className="font-mono tabular-nums text-slate-800 dark:text-slate-100 font-bold truncate">
                          {displayCostText}
                        </span>
                      </div>

                      <div className="flex items-center gap-1 h-4" title="Output Tokens">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          输出
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                          {formatInteger(displayOutputTokens)}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cache Read">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          缓存读取
                        </span>
                        {displayCacheReadTokens != null ? (
                          <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                            {formatInteger(displayCacheReadTokens)}
                          </span>
                        ) : (
                          <span className="text-slate-300 dark:text-slate-700">
                            —
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Duration">
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          耗时
                        </span>
                        <span className="font-mono tabular-nums text-slate-500 dark:text-slate-400 font-medium truncate">
                          {formatDurationMs(runningMs)}
                        </span>
                      </div>
                      <div
                        className="flex items-center gap-1 h-4"
                        title={
                          displayOutputTokensPerSecond != null
                            ? formatTokensPerSecond(displayOutputTokensPerSecond)
                            : undefined
                        }
                      >
                        <span className="text-slate-400 dark:text-slate-500 font-medium shrink-0">
                          速率
                        </span>
                        {displayOutputTokensPerSecond != null ? (
                          <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 font-semibold truncate">
                            {formatTokensPerSecondShort(displayOutputTokensPerSecond)}
                          </span>
                        ) : (
                          <span className="text-slate-300 dark:text-slate-700">
                            —
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </>
  );
});
