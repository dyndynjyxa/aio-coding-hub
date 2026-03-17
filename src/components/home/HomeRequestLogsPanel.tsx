// Usage:
// - Render as the right side column in `HomeOverviewPanel` to show realtime traces + request logs list.
// - Selection state is controlled by parent; the detail dialog is rendered outside the grid layout.

import { memo, useRef, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
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
  formatUsdRaw,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import {
  buildRequestRouteMeta,
  computeEffectiveInputTokens,
  computeStatusBadge,
  FreeBadge,
  getErrorCodeLabel,
  SessionReuseBadge,
} from "./HomeLogShared";
import { Clock, CheckCircle2, XCircle, Server, RefreshCw, ArrowUpRight } from "lucide-react";
import { RealtimeTraceCards } from "./RealtimeTraceCards";
import { CliBrandIcon } from "./CliBrandIcon";

// Estimated height for each request log card (px): padding + 2 rows of content + margin
const ESTIMATED_LOG_CARD_HEIGHT = 90;

// Threshold below which we skip virtualization (overhead not worth it).
// Set to 30 so the default 50-item HomePage list benefits from virtualization.
const VIRTUALIZATION_THRESHOLD = 30;

// Module-level stable reference: pure function, no need to recreate per render.
const formatUnixSecondsStable = (ts: number) => formatRelativeTimeFromUnixSeconds(ts);

function buildPreviewRequestLogs(nowSec = Math.floor(Date.now() / 1000)): RequestLogSummary[] {
  return [
    {
      id: 900010,
      trace_id: "preview-claude-fast",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      requested_model: "claude-sonnet-4",
      status: 200,
      error_code: null,
      duration_ms: 980,
      ttfb_ms: 180,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 18,
      start_provider_name: "Claude Fast",
      final_provider_id: 18,
      final_provider_name: "Claude Fast",
      route: [{ provider_id: 18, provider_name: "Claude Fast", ok: true, status: 200 }],
      session_reuse: true,
      input_tokens: 86,
      output_tokens: 214,
      total_tokens: 300,
      cache_read_input_tokens: 2048,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.00618,
      cost_multiplier: 1,
      created_at_ms: null,
      created_at: nowSec - 45,
    },
    {
      id: 900009,
      trace_id: "preview-gemini-flash",
      cli_key: "gemini",
      method: "POST",
      path: "/v1/chat/completions",
      requested_model: "gemini-2.5-flash",
      status: 200,
      error_code: null,
      duration_ms: 1380,
      ttfb_ms: 260,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 17,
      start_provider_name: "Gemini Flash",
      final_provider_id: 17,
      final_provider_name: "Gemini Flash",
      route: [{ provider_id: 17, provider_name: "Gemini Flash", ok: true, status: 200 }],
      session_reuse: false,
      input_tokens: 124,
      output_tokens: 328,
      total_tokens: 452,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.00392,
      cost_multiplier: 0.85,
      created_at_ms: null,
      created_at: nowSec - 70,
    },
    {
      id: 900008,
      trace_id: "preview-codex-failover",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      requested_model: "gpt-5.4",
      status: 200,
      error_code: null,
      duration_ms: 5220,
      ttfb_ms: 1260,
      attempt_count: 2,
      has_failover: true,
      start_provider_id: 15,
      start_provider_name: "Codex Pool A",
      final_provider_id: 16,
      final_provider_name: "Codex Pool B",
      route: [
        {
          provider_id: 15,
          provider_name: "Codex Pool A",
          ok: false,
          attempts: 1,
          status: 500,
          error_code: "GW_UPSTREAM_5XX",
        },
        { provider_id: 16, provider_name: "Codex Pool B", ok: true, attempts: 1, status: 200 },
      ],
      session_reuse: false,
      input_tokens: 312,
      output_tokens: 462,
      total_tokens: 774,
      cache_read_input_tokens: 65536,
      cache_creation_input_tokens: 4096,
      cache_creation_5m_input_tokens: 4096,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.0,
      cost_multiplier: 1.15,
      created_at_ms: null,
      created_at: nowSec - 95,
    },
    {
      id: 900001,
      trace_id: "preview-claude",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      requested_model: "claude-sonnet-4",
      status: 200,
      error_code: null,
      duration_ms: 1640,
      ttfb_ms: 320,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 11,
      start_provider_name: "[F]Claude Main",
      final_provider_id: 11,
      final_provider_name: "[F]Claude Main",
      route: [],
      session_reuse: true,
      input_tokens: 138,
      output_tokens: 462,
      total_tokens: 600,
      cache_read_input_tokens: 4096,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.018245,
      cost_multiplier: 1,
      created_at_ms: null,
      created_at: nowSec - 120,
    },
    {
      id: 900002,
      trace_id: "preview-codex",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      requested_model: "gpt-5.4",
      status: 200,
      error_code: null,
      duration_ms: 6420,
      ttfb_ms: 1920,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 12,
      start_provider_name: "[F]CPA-Codex",
      final_provider_id: 12,
      final_provider_name: "[F]CPA-Codex",
      route: [{ provider_id: 12, provider_name: "[F]CPA-Codex", ok: true, status: 200 }],
      session_reuse: true,
      input_tokens: 179,
      output_tokens: 183,
      total_tokens: 362,
      cache_read_input_tokens: 157952,
      cache_creation_input_tokens: null,
      cache_creation_5m_input_tokens: null,
      cache_creation_1h_input_tokens: null,
      cost_usd: null,
      cost_multiplier: 0,
      created_at_ms: null,
      created_at: nowSec - 180,
    },
    {
      id: 900007,
      trace_id: "preview-claude-opus",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      requested_model: "claude-opus-4",
      status: 200,
      error_code: null,
      duration_ms: 8440,
      ttfb_ms: 2200,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 19,
      start_provider_name: "Claude Opus",
      final_provider_id: 19,
      final_provider_name: "Claude Opus",
      route: [{ provider_id: 19, provider_name: "Claude Opus", ok: true, status: 200 }],
      session_reuse: true,
      input_tokens: 420,
      output_tokens: 910,
      total_tokens: 1330,
      cache_read_input_tokens: 32768,
      cache_creation_input_tokens: 8192,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 8192,
      cost_usd: 0.04462,
      cost_multiplier: 1.4,
      created_at_ms: null,
      created_at: nowSec - 255,
    },
    {
      id: 900006,
      trace_id: "preview-codex-timeout",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      requested_model: "gpt-5.4-mini",
      status: 504,
      error_code: "GW_UPSTREAM_TIMEOUT",
      duration_ms: 12040,
      ttfb_ms: 0,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 20,
      start_provider_name: "Codex Timeout",
      final_provider_id: 20,
      final_provider_name: "Codex Timeout",
      route: [{ provider_id: 20, provider_name: "Codex Timeout", ok: false, status: 504 }],
      session_reuse: false,
      input_tokens: 144,
      output_tokens: 0,
      total_tokens: 144,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.0,
      cost_multiplier: 1,
      created_at_ms: null,
      created_at: nowSec - 330,
    },
    {
      id: 900003,
      trace_id: "preview-gemini",
      cli_key: "gemini",
      method: "POST",
      path: "/v1/chat/completions",
      requested_model: "gemini-2.5-pro",
      status: 429,
      error_code: "GW_UPSTREAM_429",
      duration_ms: 2480,
      ttfb_ms: 1100,
      attempt_count: 2,
      has_failover: true,
      start_provider_id: 13,
      start_provider_name: "Gemini Pool",
      final_provider_id: 14,
      final_provider_name: "Gemini Mirror",
      route: [
        {
          provider_id: 13,
          provider_name: "Gemini Pool",
          ok: false,
          attempts: 1,
          status: 429,
          error_code: "GW_UPSTREAM_429",
        },
        { provider_id: 14, provider_name: "Gemini Mirror", ok: false, attempts: 1, status: 429 },
      ],
      session_reuse: false,
      input_tokens: 88,
      output_tokens: 0,
      total_tokens: 88,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.0,
      cost_multiplier: 1.2,
      created_at_ms: null,
      created_at: nowSec - 420,
    },
    {
      id: 900005,
      trace_id: "preview-claude-abort",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      requested_model: "claude-sonnet-4",
      status: 499,
      error_code: "GW_STREAM_ABORTED",
      duration_ms: 2120,
      ttfb_ms: 540,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 21,
      start_provider_name: "Claude Abort",
      final_provider_id: 21,
      final_provider_name: "Claude Abort",
      route: [{ provider_id: 21, provider_name: "Claude Abort", ok: false, status: 499 }],
      session_reuse: true,
      input_tokens: 0,
      output_tokens: 0,
      total_tokens: 0,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.0,
      cost_multiplier: 1,
      created_at_ms: null,
      created_at: nowSec - 560,
    },
    {
      id: 900004,
      trace_id: "preview-gemini-free",
      cli_key: "gemini",
      method: "POST",
      path: "/v1/chat/completions",
      requested_model: "gemini-2.0-flash-exp",
      status: 200,
      error_code: null,
      duration_ms: 1720,
      ttfb_ms: 240,
      attempt_count: 1,
      has_failover: false,
      start_provider_id: 22,
      start_provider_name: "Gemini Free",
      final_provider_id: 22,
      final_provider_name: "Gemini Free",
      route: [{ provider_id: 22, provider_name: "Gemini Free", ok: true, status: 200 }],
      session_reuse: false,
      input_tokens: 102,
      output_tokens: 260,
      total_tokens: 362,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
      cost_usd: null,
      cost_multiplier: 0,
      created_at_ms: null,
      created_at: nowSec - 780,
    },
    {
      id: 900011,
      trace_id: "preview-codex-retry",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      requested_model: "gpt-5.4",
      status: 200,
      error_code: null,
      duration_ms: 3880,
      ttfb_ms: 920,
      attempt_count: 3,
      has_failover: false,
      start_provider_id: 23,
      start_provider_name: "Codex Retry",
      final_provider_id: 23,
      final_provider_name: "Codex Retry",
      route: [
        { provider_id: 23, provider_name: "Codex Retry", ok: true, attempts: 3, status: 200 },
      ],
      session_reuse: true,
      input_tokens: 248,
      output_tokens: 540,
      total_tokens: 788,
      cache_read_input_tokens: 16384,
      cache_creation_input_tokens: 2048,
      cache_creation_5m_input_tokens: 2048,
      cache_creation_1h_input_tokens: 0,
      cost_usd: 0.01134,
      cost_multiplier: 1,
      created_at_ms: null,
      created_at: nowSec - 960,
    },
  ];
}

function buildPreviewTraces(nowMs = Date.now()): TraceSession[] {
  return [
    {
      trace_id: "preview-running-codex",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      query: null,
      requested_model: "gpt-5.4",
      first_seen_ms: nowMs - 18_000,
      last_seen_ms: nowMs,
      attempts: [
        {
          trace_id: "preview-running-codex",
          cli_key: "codex",
          method: "POST",
          path: "/v1/responses",
          query: null,
          attempt_index: 0,
          provider_id: 21,
          provider_name: "[F]CPA-Codex",
          base_url: "https://preview.local",
          outcome: "started",
          status: null,
          attempt_started_ms: nowMs - 18_000,
          attempt_duration_ms: 18_000,
          session_reuse: true,
        } as any,
      ],
    },
  ];
}

function requestLogCreatedAtMs(log: RequestLogSummary) {
  const ms = log.created_at_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return log.created_at * 1000;
}

function mergeTraceWithRequestLog(
  trace: TraceSession,
  requestLog: RequestLogSummary | undefined
): TraceSession {
  if (!requestLog) return trace;

  const summary = trace.summary;
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
    requested_model: trace.requested_model ?? requestLog.requested_model ?? null,
    summary: mergedSummary,
    last_seen_ms: Math.max(trace.last_seen_ms, requestLogCreatedAtMs(requestLog)),
  };
}

type RequestLogCardProps = {
  compactMode: boolean;
  log: RequestLogSummary;
  isSelected: boolean;
  showCustomTooltip: boolean;
  onSelectLogId: (id: number | null) => void;
  formatUnixSeconds: (ts: number) => string;
};

const RequestLogCard = memo(function RequestLogCard({
  compactMode,
  log,
  isSelected,
  showCustomTooltip,
  onSelectLogId,
  formatUnixSeconds,
}: RequestLogCardProps) {
  const statusBadge = computeStatusBadge({
    status: log.status,
    errorCode: log.error_code,
    hasFailover: log.has_failover,
  });

  const providerText =
    log.final_provider_id === 0 ||
    !log.final_provider_name ||
    log.final_provider_name.trim().length === 0 ||
    log.final_provider_name === "Unknown"
      ? "未知"
      : log.final_provider_name;

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
  const cliTone = cliBadgeTone(log.cli_key)
    .replace(/group-hover:bg-white/g, "")
    .replace(/dark:group-hover:bg-slate-800/g, "")
    .replace(/group-hover:border-slate-200/g, "")
    .replace(/dark:group-hover:border-slate-700/g, "");

  const ttfbMs = sanitizeTtfbMs(log.ttfb_ms, log.duration_ms);
  const outputTokensPerSecond = computeOutputTokensPerSecond(
    log.output_tokens,
    log.duration_ms,
    ttfbMs
  );

  const costMultiplier = log.cost_multiplier;
  const isFree = Number.isFinite(costMultiplier) && costMultiplier === 0;
  const showCostMultiplier =
    Number.isFinite(costMultiplier) && costMultiplier >= 0 && Math.abs(costMultiplier - 1) > 0.0001;
  const costMultiplierText = isFree ? "免费" : `x${costMultiplier.toFixed(2)}`;
  const rawCostUsdText = formatUsdRaw(log.cost_usd);

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
            ? "bg-indigo-50/40 border-indigo-200 shadow-sm dark:bg-indigo-900/20 dark:border-indigo-700"
            : "bg-white border-slate-100 hover:bg-slate-50/60 hover:border-slate-200 hover:shadow-sm dark:bg-slate-800 dark:border-slate-700 dark:hover:bg-slate-700/60 dark:hover:border-slate-600"
        )}
      >
        {/* Selection indicator */}
        <div
          className={cn(
            "absolute left-0 top-2 bottom-2 w-1 rounded-r-full transition-all duration-200",
            isSelected
              ? "bg-indigo-500 opacity-100"
              : "bg-slate-300 opacity-0 group-hover/item:opacity-40"
          )}
        />

        <div className={cn("px-3", compactMode ? "py-2" : "py-2.5")}>
          <div className={cn("flex items-center gap-2 min-w-0", compactMode ? "" : "mb-1.5")}>
            <span
              className={cn(
                "inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                statusBadge.tone
              )}
              title={statusBadge.title}
            >
              {statusBadge.isError ? (
                <XCircle className="h-3 w-3" />
              ) : (
                <CheckCircle2 className="h-3 w-3" />
              )}
              {statusBadge.text}
            </span>

            <span
              className={cn(
                "inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                cliTone
              )}
            >
              <CliBrandIcon
                cliKey={log.cli_key}
                className="h-2.5 w-2.5 shrink-0 rounded-[3px] object-contain"
              />
              {cliLabel}
            </span>

            <span
              className="inline-flex min-w-0 items-center rounded-md bg-slate-100/75 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:bg-slate-700/55 dark:text-slate-200"
              title={modelText}
            >
              <span className="truncate">{modelText}</span>
            </span>

            <span
              className="inline-flex min-w-0 items-center rounded-md bg-slate-100/75 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:bg-slate-700/55 dark:text-slate-200"
              title={providerTitle}
            >
              <span className="truncate">{providerText}</span>
            </span>

            {log.session_reuse && <SessionReuseBadge showCustomTooltip={showCustomTooltip} />}
            {isFree && <FreeBadge />}

            {log.error_code && (
              <span className="shrink-0 rounded-md bg-amber-50/70 px-2 py-0.5 text-[11px] font-medium text-amber-600 dark:bg-amber-900/20 dark:text-amber-300">
                {getErrorCodeLabel(log.error_code)}
              </span>
            )}

            <span className="ml-auto flex items-center gap-1.5 text-xs text-slate-400 dark:text-slate-500 shrink-0">
              <Clock className="h-3 w-3" />
              {formatUnixSeconds(log.created_at)}
            </span>
          </div>

          {!compactMode && (
            <div className="flex items-start gap-3 text-[11px]">
              <div className="flex flex-col gap-y-0.5 w-[110px] shrink-0" title={providerTitle}>
                <div className="flex items-center gap-1 h-4">
                  <Server className="h-3 w-3 text-slate-400 dark:text-slate-500 shrink-0" />
                  <span className="truncate font-medium text-slate-600 dark:text-slate-400">
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
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">输入</span>
                  <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                    {formatInteger(effectiveInputTokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Write">
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">缓存创建</span>
                  {cacheWrite.tokens != null ? (
                    <>
                      <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                        {formatInteger(cacheWrite.tokens)}
                      </span>
                      {cacheWrite.ttl && cacheWrite.tokens > 0 && (
                        <span className="text-slate-400 dark:text-slate-500 text-[10px]">
                          ({cacheWrite.ttl})
                        </span>
                      )}
                    </>
                  ) : (
                    <span className="text-slate-300 dark:text-slate-600">—</span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="TTFB">
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">首字</span>
                  <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                    {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                  </span>
                </div>
                <div
                  className="flex items-center gap-1 h-4"
                  title={rawCostUsdText === "—" ? undefined : rawCostUsdText}
                >
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">花费</span>
                  <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                    {rawCostUsdText}
                  </span>
                </div>

                <div className="flex items-center gap-1 h-4" title="Output Tokens">
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">输出</span>
                  <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                    {formatInteger(log.output_tokens)}
                  </span>
                </div>
                <div className="flex items-center gap-1 h-4" title="Cache Read">
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">缓存读取</span>
                  {log.cache_read_input_tokens != null ? (
                    <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                      {formatInteger(log.cache_read_input_tokens)}
                    </span>
                  ) : (
                    <span className="text-slate-300 dark:text-slate-600">—</span>
                  )}
                </div>
                <div className="flex items-center gap-1 h-4" title="Duration">
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">耗时</span>
                  <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                    {formatDurationMs(log.duration_ms)}
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
                  <span className="text-slate-400 dark:text-slate-500 shrink-0">速率</span>
                  {outputTokensPerSecond != null ? (
                    <span className="font-mono tabular-nums text-slate-600 dark:text-slate-300 truncate">
                      {formatTokensPerSecondShort(outputTokensPerSecond)}
                    </span>
                  ) : (
                    <span className="text-slate-300 dark:text-slate-600">—</span>
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
  showOpenLogsPageButton?: boolean;
  requestLogsPreviewEnabled?: boolean;

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
  showOpenLogsPageButton = true,
  requestLogsPreviewEnabled = import.meta.env.DEV,
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
  const [compactMode, setCompactMode] = useState(true);
  const [previewTraces, setPreviewTraces] = useState<TraceSession[]>([]);
  const [previewRequestLogs, setPreviewRequestLogs] = useState<RequestLogSummary[]>([]);
  const displayedTraces = traces.length > 0 ? traces : previewTraces;
  const displayedRequestLogs = requestLogs.length > 0 ? requestLogs : previewRequestLogs;
  const previewActive =
    (traces.length === 0 && previewTraces.length > 0) ||
    (requestLogs.length === 0 && previewRequestLogs.length > 0);
  const realtimeTraceCandidates = useMemo(() => {
    const logsByTraceId = new Map<string, RequestLogSummary>();
    for (const log of displayedRequestLogs) {
      const traceId = log.trace_id?.trim();
      if (!traceId || logsByTraceId.has(traceId)) continue;
      logsByTraceId.set(traceId, log);
    }

    const nowMs = Date.now();
    return displayedTraces
      .map((trace) => mergeTraceWithRequestLog(trace, logsByTraceId.get(trace.trace_id)))
      .filter((t) => nowMs - t.first_seen_ms < 15 * 60 * 1000)
      .sort((a, b) => b.first_seen_ms - a.first_seen_ms)
      .slice(0, 20);
  }, [displayedRequestLogs, displayedTraces]);

  return (
    <Card padding="sm" className="flex flex-col gap-3 lg:col-span-7 h-full">
      <div className="flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div className="flex flex-wrap items-center gap-2">
          <div className="text-sm font-semibold">{title ?? "使用记录（最近 50 条）"}</div>
        </div>

        <div className="flex items-center gap-2">
          <div className="text-xs text-slate-500 dark:text-slate-400">
            {requestLogsAvailable === false
              ? "数据不可用"
              : displayedRequestLogs.length === 0 && requestLogsLoading
                ? "加载中…"
                : requestLogsLoading || requestLogsRefreshing
                  ? `更新中… · 共 ${displayedRequestLogs.length} 条`
                  : `共 ${displayedRequestLogs.length} 条`}
          </div>
          {previewActive && (
            <Button
              onClick={() => {
                setPreviewTraces([]);
                setPreviewRequestLogs([]);
              }}
              variant="ghost"
              size="sm"
              className="h-8 px-2 text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400"
            >
              关闭预览
            </Button>
          )}
          {showOpenLogsPageButton && (
            <Button
              onClick={() => navigate("/logs")}
              variant="ghost"
              size="sm"
              className="h-8 gap-1 px-2 text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400"
              disabled={requestLogsAvailable === false}
              title="打开日志页"
            >
              日志
              <ArrowUpRight className="h-3.5 w-3.5" />
            </Button>
          )}
          <Button
            onClick={onRefreshRequestLogs}
            variant="ghost"
            size="sm"
            className="h-8 gap-1 px-2 text-slate-500 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400"
            disabled={requestLogsAvailable === false || requestLogsLoading || requestLogsRefreshing}
          >
            刷新
            <RefreshCw
              className={cn(
                "h-3.5 w-3.5",
                (requestLogsLoading || requestLogsRefreshing) && "animate-spin"
              )}
            />
          </Button>
          <div className="flex items-center gap-1.5 pl-1">
            <span className="text-xs text-slate-500 dark:text-slate-400">简洁模式</span>
            <Switch
              checked={compactMode}
              onCheckedChange={setCompactMode}
              size="sm"
              aria-label="最近使用记录简洁模式"
            />
          </div>
        </div>
      </div>

      <div className="overflow-hidden flex-1 min-h-0 flex flex-col">
        <RequestLogsList
          realtimeTraceCandidates={realtimeTraceCandidates}
          formatUnixSeconds={formatUnixSecondsStable}
          showCustomTooltip={showCustomTooltip}
          compactMode={compactMode}
          requestLogsPreviewEnabled={requestLogsPreviewEnabled}
          requestLogsAvailable={requestLogsAvailable}
          requestLogs={displayedRequestLogs}
          requestLogsLoading={requestLogsLoading}
          selectedLogId={selectedLogId}
          onSelectLogId={onSelectLogId}
          onEnablePreview={() => {
            setPreviewTraces(buildPreviewTraces());
            setPreviewRequestLogs(buildPreviewRequestLogs());
          }}
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
  requestLogsPreviewEnabled: boolean;
  requestLogsAvailable: boolean | null;
  requestLogs: RequestLogSummary[];
  requestLogsLoading: boolean;
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
  onEnablePreview: () => void;
};

const RequestLogsList = memo(function RequestLogsList({
  realtimeTraceCandidates,
  formatUnixSeconds,
  showCustomTooltip,
  compactMode,
  requestLogsPreviewEnabled,
  requestLogsAvailable,
  requestLogs,
  requestLogsLoading,
  selectedLogId,
  onSelectLogId,
  onEnablePreview,
}: RequestLogsListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const useVirtual = requestLogs.length >= VIRTUALIZATION_THRESHOLD;

  const virtualizer = useVirtualizer({
    count: requestLogs.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_LOG_CARD_HEIGHT,
    overscan: 8,
    enabled: useVirtual,
  });

  const virtualItems = virtualizer.getVirtualItems();

  // Non-virtualized fallback for small lists
  const plainList = !useVirtual && requestLogs.length > 0 && (
    <>
      {requestLogs.map((log) => (
        <RequestLogCard
          compactMode={compactMode}
          key={log.id}
          log={log}
          isSelected={selectedLogId === log.id}
          showCustomTooltip={showCustomTooltip}
          onSelectLogId={onSelectLogId}
          formatUnixSeconds={formatUnixSeconds}
        />
      ))}
    </>
  );

  return (
    <div ref={scrollRef} className="scrollbar-overlay flex-1 overflow-auto pr-1 py-2">
      <RealtimeTraceCards
        traces={realtimeTraceCandidates}
        formatUnixSeconds={formatUnixSeconds}
        showCustomTooltip={showCustomTooltip}
      />

      {requestLogsAvailable === false ? (
        <div className="p-4 text-sm text-slate-600 dark:text-slate-400">数据不可用</div>
      ) : requestLogs.length === 0 ? (
        requestLogsLoading ? (
          <div className="flex items-center justify-center gap-2 p-4 text-sm text-slate-600 dark:text-slate-400">
            <Spinner size="sm" />
            加载中…
          </div>
        ) : (
          <EmptyState
            title="当前没有最近使用记录"
            action={
              requestLogsPreviewEnabled ? (
                <Button variant="secondary" size="sm" onClick={onEnablePreview}>
                  预览记录样式
                </Button>
              ) : undefined
            }
          />
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
            {virtualItems.map((virtualRow) => (
              <div
                key={requestLogs[virtualRow.index].id}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
              >
                <RequestLogCard
                  compactMode={compactMode}
                  log={requestLogs[virtualRow.index]}
                  isSelected={selectedLogId === requestLogs[virtualRow.index].id}
                  showCustomTooltip={showCustomTooltip}
                  onSelectLogId={onSelectLogId}
                  formatUnixSeconds={formatUnixSeconds}
                />
              </div>
            ))}
          </div>
        </div>
      ) : (
        plainList
      )}
    </div>
  );
});
