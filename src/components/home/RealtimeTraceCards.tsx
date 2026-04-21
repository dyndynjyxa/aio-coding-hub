// Usage:
// - Render in Home page "概览 / 使用记录" area to show up-to-date in-flight traces.
// - Accepts a list of `TraceSession` candidates; component applies its own visibility + exit animation logic.

import { memo, useEffect, useMemo, useState } from "react";
import { cliBadgeToneStatic, cliShortLabel } from "../../constants/clis";
import { GatewayErrorCodes } from "../../constants/gatewayErrorCodes";
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

export const RealtimeTraceCards = memo(function RealtimeTraceCards({
  folderLookupBySessionKey,
  traces,
  formatUnixSeconds,
  showCustomTooltip,
}: RealtimeTraceCardsProps) {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    if (traces.length === 0) return;
    let timer: number | null = null;
    let active = true;

    const tick = () => {
      if (!active) return false;
      const now = Date.now();
      setNowMs(now);

      return traces.some((trace) => {
        if (!trace.summary) {
          return now - trace.last_seen_ms < STALE_TRACE_TIMEOUT_MS;
        }
        return Math.max(0, now - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
      });
    };

    const stillNeeded = tick();
    if (!stillNeeded) return;

    timer = window.setInterval(() => {
      const needed = tick();
      if (!needed && timer != null) {
        window.clearInterval(timer);
        timer = null;
      }
    }, 250);
    return () => {
      active = false;
      if (timer != null) window.clearInterval(timer);
    };
  }, [traces]);

  const visibleTraces = useMemo(() => {
    const kept = traces.filter((trace) => {
      if (!trace.summary) {
        return nowMs - trace.last_seen_ms < STALE_TRACE_TIMEOUT_MS;
      }
      return Math.max(0, nowMs - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
    });
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

        const modelText =
          trace.requested_model && trace.requested_model.trim()
            ? trace.requested_model.trim()
            : "未知";
        const cliLabel = cliShortLabel(trace.cli_key);
        const cliTone = cliBadgeToneStatic(trace.cli_key);

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
                "group/item relative rounded-lg border transition-colors duration-300 ease-out",
                isInProgress
                  ? "bg-white/90 border-indigo-200/80 shadow-[0_0_0_1px_rgba(99,102,241,0.06),0_2px_12px_rgba(99,102,241,0.1)] glow-pulse-active dark:bg-slate-800/90 dark:border-indigo-600/50 dark:shadow-[0_0_0_1px_rgba(99,102,241,0.12),0_2px_12px_rgba(99,102,241,0.15)]"
                  : "bg-white/80 border-slate-200/60 shadow-sm dark:bg-slate-800/80 dark:border-slate-700/60"
              )}
            >
              <div
                className={cn(
                  "absolute left-0 top-2 bottom-2 w-[3px] rounded-r-full transition-all duration-500 origin-center",
                  isInProgress
                    ? "indicator-shimmer-active shadow-[2px_0_8px_rgba(99,102,241,0.25)]"
                    : statusBadge.isError
                      ? "bg-rose-400 opacity-80"
                      : hasFailover
                        ? "bg-amber-400 opacity-80"
                        : "bg-slate-300/60 opacity-50 dark:bg-slate-500/60"
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
                    className={cn(
                      "inline-flex min-w-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium",
                      cliTone
                    )}
                    title={`${cliLabel} / ${modelText}`}
                  >
                    <CliBrandIcon
                      cliKey={trace.cli_key as CliKey}
                      className="h-2.5 w-2.5 shrink-0 rounded-[3px] object-contain"
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
                    <span className="shrink-0 rounded-md bg-amber-50/80 px-2 py-0.5 text-[11px] font-semibold text-amber-600 ring-1 ring-inset ring-amber-500/10 dark:bg-amber-500/15 dark:text-amber-300 dark:ring-amber-400/20">
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
                      <span className="inline-flex items-center gap-1.5 text-xs font-mono tabular-nums text-indigo-600 dark:text-indigo-300">
                        <Clock className="h-3 w-3 shrink-0" />
                        {formatDurationMs(runningMs)}
                      </span>
                    </span>
                  )}
                </div>

                {isInProgress ? (
                  <div className="grid grid-cols-2 gap-2 text-[11px] lg:grid-cols-[fit-content(180px)_fit-content(96px)_minmax(0,1fr)]">
                    <div className="rounded-md border border-indigo-200/60 bg-indigo-50/70 px-2.5 py-2 dark:border-indigo-500/20 dark:bg-indigo-500/10">
                      <div className="text-slate-400 dark:text-slate-500">当前阶段</div>
                      <div className="mt-1 truncate font-semibold text-indigo-600 dark:text-indigo-300">
                        {liveStageText}
                      </div>
                    </div>
                    <div className="rounded-md border border-slate-200/70 bg-slate-50/80 px-2.5 py-2 dark:border-slate-700/70 dark:bg-slate-800/70">
                      <div className="text-slate-400 dark:text-slate-500">尝试次数</div>
                      <div className="mt-1 truncate font-mono tabular-nums text-slate-700 dark:text-slate-200">
                        {formatInteger(trace.attempts.length)}
                      </div>
                    </div>
                    <div className="col-span-2 rounded-md border border-slate-200/70 bg-slate-50/80 px-2.5 py-2 dark:border-slate-700/70 dark:bg-slate-800/70 lg:col-span-1">
                      <div className="text-slate-400 dark:text-slate-500">当前链路</div>
                      <div
                        className="mt-1 truncate font-medium text-slate-700 dark:text-slate-200"
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
                        <Server className="h-3 w-3 text-slate-400/80 dark:text-slate-500/80 shrink-0" />
                        <span className="truncate font-semibold text-slate-600 dark:text-slate-300">
                          {providerText}
                        </span>
                      </div>
                      <div className="flex items-center h-4">
                        <div className="flex min-w-0 w-full items-center gap-1">
                          {routeLabel && routeTooltipText ? (
                            <span
                              className="cursor-help text-[11px] text-slate-400 dark:text-slate-500"
                              title={routeTooltipText}
                            >
                              {routeLabel}
                            </span>
                          ) : null}
                          {showCostMultiplier ? (
                            <span className="inline-flex shrink-0 items-center text-[11px] font-medium text-slate-500 dark:text-slate-400">
                              {costMultiplierText}
                            </span>
                          ) : null}
                        </div>
                      </div>
                    </div>

                    <div className="grid flex-1 grid-cols-4 gap-x-3 gap-y-0.5 text-slate-500 dark:text-slate-400">
                      <div className="flex items-center gap-1 h-4" title="Input Tokens">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          输入
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                          {formatInteger(displayInputTokens)}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cache Write">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          缓存创建
                        </span>
                        {displayCacheWriteTokens != null ? (
                          <>
                            <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                              {formatInteger(displayCacheWriteTokens)}
                            </span>
                            {cacheWrite.ttl && displayCacheWriteTokens > 0 && (
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
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          首字
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                          {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cost">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          花费
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                          {displayCostText}
                        </span>
                      </div>

                      <div className="flex items-center gap-1 h-4" title="Output Tokens">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          输出
                        </span>
                        <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                          {formatInteger(displayOutputTokens)}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Cache Read">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          缓存读取
                        </span>
                        {displayCacheReadTokens != null ? (
                          <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                            {formatInteger(displayCacheReadTokens)}
                          </span>
                        ) : (
                          <span className="text-slate-300/60 dark:text-slate-600/60">—</span>
                        )}
                      </div>
                      <div className="flex items-center gap-1 h-4" title="Duration">
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          耗时
                        </span>
                        <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
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
                        <span className="text-slate-400/80 dark:text-slate-500/80 shrink-0">
                          速率
                        </span>
                        {displayOutputTokensPerSecond != null ? (
                          <span className="font-mono tabular-nums text-slate-700 dark:text-slate-200 truncate">
                            {formatTokensPerSecondShort(displayOutputTokensPerSecond)}
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
          </div>
        );
      })}
    </>
  );
});
