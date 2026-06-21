import { isPersistedRequestLogInProgress, requestLogCreatedAtMs } from "./requestLogState";
import type { RequestLogSummary } from "./requestLogs";
import { resolveClaudeModelMappingFromSpecialSettings } from "./requestLogSpecialSettings";
import type { TraceSession } from "./traceStore";

export const REALTIME_TRACE_EXIT_START_MS = 600;
export const REALTIME_TRACE_EXIT_ANIM_MS = 400;
export const REALTIME_TRACE_EXIT_TOTAL_MS =
  REALTIME_TRACE_EXIT_START_MS + REALTIME_TRACE_EXIT_ANIM_MS + 100;

export type ProjectedRealtimeCard = {
  trace: TraceSession;
};

export type ProjectedRequestLogRow = {
  log: RequestLogSummary;
  liveTrace: TraceSession | null;
};

export type RequestActivityProjection = {
  realtimeCards: ProjectedRealtimeCard[];
  requestRows: ProjectedRequestLogRow[];
  visibleRealtimeTraceIds: Set<string>;
  hasPending: boolean;
  hasLiveRealtimeCards: boolean;
  summaryCount: number;
};

export type BuildRequestActivityProjectionInput = {
  requestLogs: RequestLogSummary[];
  traces: TraceSession[];
  nowMs: number;
  realtimeCardLimit: number;
  realtimeCandidateLimit: number;
};

function normalizeTraceId(traceId: string | null | undefined) {
  return traceId?.trim() || null;
}

export function sortRequestLogsForActivity(a: RequestLogSummary, b: RequestLogSummary) {
  const aInProgress = isPersistedRequestLogInProgress(a);
  const bInProgress = isPersistedRequestLogInProgress(b);
  if (aInProgress !== bInProgress) return aInProgress ? -1 : 1;

  const aTsMs = requestLogCreatedAtMs(a);
  const bTsMs = requestLogCreatedAtMs(b);
  if (aTsMs !== bTsMs) return bTsMs - aTsMs;
  return b.id - a.id;
}

export function shouldKeepProjectedRealtimeTraceVisible(trace: TraceSession, nowMs: number) {
  if (!trace.summary) return true;
  return Math.max(0, nowMs - trace.last_seen_ms) < REALTIME_TRACE_EXIT_TOTAL_MS;
}

function mergeTraceWithRequestLog(
  trace: TraceSession,
  requestLog: RequestLogSummary | undefined
): TraceSession {
  if (!requestLog) return trace;

  const requestLogInProgress = isPersistedRequestLogInProgress(requestLog);
  const requestLogTsMs = requestLogCreatedAtMs(requestLog);
  const claudeModelMapping =
    trace.claude_model_mapping ??
    resolveClaudeModelMappingFromSpecialSettings(
      requestLog.special_settings_json,
      requestLog.final_provider_id
    );
  if (!trace.summary && requestLogInProgress) {
    return {
      ...trace,
      session_id: trace.session_id ?? requestLog.session_id ?? null,
      requested_model: trace.requested_model ?? requestLog.requested_model ?? null,
      claude_model_mapping: claudeModelMapping,
      last_seen_ms: Math.max(trace.last_seen_ms, requestLogTsMs),
    };
  }

  const summary = trace.summary;
  const mergedSummary: NonNullable<TraceSession["summary"]> = {
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
    claude_model_mapping: claudeModelMapping,
    summary: mergedSummary,
    last_seen_ms: Math.max(trace.last_seen_ms, requestLogTsMs),
  };
}

export function buildRequestActivityProjection({
  requestLogs,
  traces,
  nowMs,
  realtimeCardLimit,
  realtimeCandidateLimit,
}: BuildRequestActivityProjectionInput): RequestActivityProjection {
  const requestRowsSorted = requestLogs.slice().sort(sortRequestLogsForActivity);
  const logsByTraceId = new Map<string, RequestLogSummary>();
  for (const log of requestRowsSorted) {
    const traceId = normalizeTraceId(log.trace_id);
    if (!traceId || logsByTraceId.has(traceId)) continue;
    logsByTraceId.set(traceId, log);
  }

  const mergedTraceMap = new Map<string, TraceSession>();
  for (const trace of traces) {
    const traceId = normalizeTraceId(trace.trace_id);
    if (!traceId || mergedTraceMap.has(traceId)) continue;
    mergedTraceMap.set(traceId, mergeTraceWithRequestLog(trace, logsByTraceId.get(traceId)));
  }

  const realtimeCandidates = Array.from(mergedTraceMap.values())
    .filter((trace) => shouldKeepProjectedRealtimeTraceVisible(trace, nowMs))
    .sort((a, b) => b.first_seen_ms - a.first_seen_ms)
    .slice(0, realtimeCandidateLimit);

  const realtimeCards = realtimeCandidates.slice(0, realtimeCardLimit).map((trace) => ({ trace }));
  const visibleRealtimeTraceIds = new Set(
    realtimeCards
      .map((card) => normalizeTraceId(card.trace.trace_id))
      .filter((traceId): traceId is string => traceId != null)
  );

  const requestRows = requestRowsSorted
    .filter((log) => {
      const traceId = normalizeTraceId(log.trace_id);
      return !traceId || !visibleRealtimeTraceIds.has(traceId);
    })
    .map((log) => {
      const traceId = normalizeTraceId(log.trace_id);
      return {
        log,
        liveTrace: traceId ? (mergedTraceMap.get(traceId) ?? null) : null,
      };
    });

  return {
    realtimeCards,
    requestRows,
    visibleRealtimeTraceIds,
    hasPending: requestRowsSorted.some((log) => isPersistedRequestLogInProgress(log)),
    hasLiveRealtimeCards: realtimeCards.length > 0,
    summaryCount: requestRowsSorted.length,
  };
}
