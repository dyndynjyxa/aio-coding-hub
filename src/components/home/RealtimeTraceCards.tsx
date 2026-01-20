// Usage:
// - Render in Home page "概览 / 使用记录" area to show up-to-date in-flight traces.
// - Accepts a list of `TraceSession` candidates; component applies its own visibility + exit animation logic.

import { useEffect, useMemo, useState } from "react";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import type { TraceSession } from "../../services/traceStore";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatTokensPerSecond,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { Clock, Server, Loader2, Cpu, Terminal, CheckCircle2, XCircle } from "lucide-react";
import {
  computeEffectiveInputTokens,
  computeStatusBadge,
  getErrorCodeLabel,
  SessionReuseBadge,
} from "./HomeLogShared";

export type RealtimeTraceCardsProps = {
  traces: TraceSession[];
  formatUnixSeconds: (ts: number) => string;
  showCustomTooltip: boolean;
};

const REALTIME_TRACE_EXIT_START_MS = 200;
const REALTIME_TRACE_EXIT_ANIM_MS = 700;
const REALTIME_TRACE_EXIT_TOTAL_MS =
  REALTIME_TRACE_EXIT_START_MS + REALTIME_TRACE_EXIT_ANIM_MS + 100;

export function RealtimeTraceCards({
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
        if (!trace.summary) return true;
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
      if (!trace.summary) return true;
      return Math.max(0, nowMs - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
    });
    return kept.slice(0, 5);
  }, [traces, nowMs]);

  return (
    <>
      {visibleTraces.map((trace) => {
        const completedAgeMs = trace.summary ? Math.max(0, nowMs - trace.last_seen_ms) : 0;
        const isExiting = Boolean(trace.summary) && completedAgeMs >= REALTIME_TRACE_EXIT_START_MS;
        const runningMs = trace.summary
          ? trace.summary.duration_ms
          : Math.max(0, nowMs - trace.first_seen_ms);

        const summaryStatus = trace.summary?.status ?? null;
        const summaryErrorCode = trace.summary?.error_code ?? null;
        const isInProgress = !trace.summary;
        const statusBadge = computeStatusBadge({
          status: summaryStatus,
          errorCode: summaryErrorCode,
          inProgress: isInProgress,
        });
        const hasSessionReuse = (trace.attempts ?? []).some(
          (attempt) => attempt.session_reuse === true
        );

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

          const routeLabel = (() => {
            if (segs.length === 0) return null;

            const text = segs
              .map((seg) => {
                const badge =
                  seg.status === "success" ? "✅" : seg.status === "started" ? "⏳" : "❌";
                return `${seg.provider}[${badge}]`;
              })
              .join("->");

            const shouldShow = segs.length > 1 || segs.some((s) => s.status !== "success");
            return shouldShow ? `(${text})` : null;
          })();

          return { providerText, startProvider, endProvider, routeLabel, segments: segs };
        })();

        const providerText = attemptRoute.providerText;

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

        const cacheWrite = (() => {
          const s = trace.summary;
          if (!s)
            return {
              tokens: null as number | null,
              ttl: null as "5m" | null,
            };
          if (s.cache_creation_5m_input_tokens != null) {
            return {
              tokens: s.cache_creation_5m_input_tokens,
              ttl: "5m" as const,
            };
          }
          if (s.cache_creation_input_tokens != null) {
            return { tokens: s.cache_creation_input_tokens, ttl: null };
          }
          return { tokens: null, ttl: null };
        })();

        const ttfbMs = trace.summary
          ? sanitizeTtfbMs(trace.summary.ttfb_ms ?? null, trace.summary.duration_ms)
          : null;

        const effectiveInputTokens = computeEffectiveInputTokens(
          trace.cli_key,
          trace.summary?.input_tokens ?? null,
          trace.summary?.cache_read_input_tokens ?? null
        );

        const outputTokensPerSecond = trace.summary
          ? computeOutputTokensPerSecond(
              trace.summary.output_tokens ?? null,
              trace.summary.duration_ms,
              ttfbMs
            )
          : null;

        return (
          <div
            key={trace.trace_id}
            className={cn(
              "transform overflow-hidden transition-all ease-out motion-reduce:transition-none motion-reduce:transform-none",
              isExiting
                ? "max-h-0 opacity-0 translate-y-1 !mt-0 duration-700"
                : "max-h-[120px] opacity-100 translate-y-0 duration-700"
            )}
          >
            <div
              className={cn(
                "relative transition-all duration-150",
                isInProgress
                  ? "bg-gradient-to-r from-indigo-50/90 to-indigo-50/40"
                  : "bg-gradient-to-r from-emerald-50/70 to-emerald-50/30"
              )}
            >
              {/* Left accent bar */}
              <div
                className={cn(
                  "absolute left-0 top-1 bottom-1 w-0.5 rounded-full",
                  isInProgress ? "bg-indigo-400" : "bg-emerald-400"
                )}
              />
              {isInProgress && (
                <div className="absolute inset-x-0 top-0 h-0.5 overflow-hidden bg-indigo-100">
                  <div className="h-full w-1/3 animate-[loading_1s_ease-in-out_infinite] bg-indigo-500/50" />
                </div>
              )}

              <div className="flex flex-col gap-1 px-3 py-2">
                {/* Row 1: Status + CLI + Model + Time + Badges */}
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className={cn(
                      "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                      statusBadge.tone
                    )}
                    title={statusBadge.title}
                  >
                    {isInProgress ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : summaryStatus && summaryStatus >= 400 ? (
                      <XCircle className="h-3 w-3" />
                    ) : (
                      <CheckCircle2 className="h-3 w-3" />
                    )}
                    {statusBadge.text}
                  </span>

                  <span
                    className={cn(
                      "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                      cliBadgeTone(trace.cli_key)
                    )}
                  >
                    {trace.cli_key === "claude" ? (
                      <Terminal className="h-3 w-3" />
                    ) : (
                      <Cpu className="h-3 w-3" />
                    )}
                    {cliShortLabel(trace.cli_key)}
                  </span>

                  <span className="text-xs font-medium text-slate-800 truncate" title={modelText}>
                    {modelText}
                  </span>

                  {summaryErrorCode && !statusBadge.isErrorOverride && (
                    <span className="rounded bg-amber-50 px-1 py-0.5 text-[10px] font-medium text-amber-700 shrink-0">
                      {getErrorCodeLabel(summaryErrorCode)}
                    </span>
                  )}

                  {hasSessionReuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}

                  <span className="flex items-center gap-1 text-[11px] text-slate-400 ml-auto shrink-0">
                    <Clock className="h-3 w-3" />
                    {formatUnixSeconds(Math.floor(trace.first_seen_ms / 1000))}
                  </span>
                </div>

                {/* Row 2: Provider + Stats Grid (2 rows x 4 cols for alignment) */}
                <div className="flex items-start gap-3 text-[11px]">
                  {/* Provider - left side (2 rows: name + placeholder) */}
                  <div className="flex flex-col gap-y-0.5 w-[90px] shrink-0" title={routeSummary}>
                    <div className="flex items-center gap-1 h-4">
                      <Server className="h-3 w-3 text-slate-400 shrink-0" />
                      <span className="truncate font-medium text-slate-600">{providerText}</span>
                    </div>
                    <div className="h-4" />
                  </div>

                  {/* Stats Grid: 2 rows x 4 cols */}
                  <div className="grid grid-cols-4 gap-x-4 gap-y-0.5 flex-1 text-slate-500">
                    {/* Row 1: 输入 | 缓存创建 | 首字 | 花费 */}
                    <div className="flex items-center gap-1 h-4" title="Input Tokens">
                      <span className="text-slate-400 shrink-0">输入</span>
                      <span className="font-mono tabular-nums text-slate-600">
                        {formatInteger(effectiveInputTokens)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cache Write">
                      <span className="text-slate-400 shrink-0">缓存创建</span>
                      {cacheWrite.tokens ? (
                        <>
                          <span className="font-mono tabular-nums text-slate-600">
                            {formatInteger(cacheWrite.tokens)}
                          </span>
                          {cacheWrite.ttl && (
                            <span className="text-slate-400 text-[10px]">({cacheWrite.ttl})</span>
                          )}
                        </>
                      ) : (
                        <span className="text-slate-300">—</span>
                      )}
                    </div>
                    <div className="flex items-center gap-1 h-4" title="TTFB">
                      <span className="text-slate-400 shrink-0">首字</span>
                      <span className="font-mono tabular-nums text-slate-600">
                        {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cost">
                      <span className="text-slate-400 shrink-0">花费</span>
                      <span className="text-slate-300">—</span>
                    </div>

                    {/* Row 2: 输出 | 缓存读取 | 耗时 | 速率 */}
                    <div className="flex items-center gap-1 h-4" title="Output Tokens">
                      <span className="text-slate-400 shrink-0">输出</span>
                      <span className="font-mono tabular-nums text-slate-600">
                        {formatInteger(trace.summary?.output_tokens ?? null)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Cache Read">
                      <span className="text-slate-400 shrink-0">缓存读取</span>
                      {trace.summary?.cache_read_input_tokens ? (
                        <span className="font-mono tabular-nums text-slate-600">
                          {formatInteger(trace.summary.cache_read_input_tokens)}
                        </span>
                      ) : (
                        <span className="text-slate-300">—</span>
                      )}
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Duration">
                      <span className="text-slate-400 shrink-0">耗时</span>
                      <span
                        className={cn(
                          "font-mono tabular-nums",
                          isInProgress ? "text-indigo-600 font-medium" : "text-slate-600"
                        )}
                      >
                        {formatDurationMs(runningMs)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 h-4" title="Tokens/s">
                      <span className="text-slate-400 shrink-0">速率</span>
                      {outputTokensPerSecond ? (
                        <span className="font-mono tabular-nums text-slate-600">
                          {formatTokensPerSecond(outputTokensPerSecond)}
                        </span>
                      ) : (
                        <span className="text-slate-300">—</span>
                      )}
                    </div>
                  </div>
                </div>
              </div>

              {/* Subtle bottom divider */}
              <div className="absolute inset-x-3 bottom-0 h-px bg-slate-200/50" />
            </div>
          </div>
        );
      })}
    </>
  );
}
