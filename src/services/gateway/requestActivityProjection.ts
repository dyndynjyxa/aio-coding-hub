// 契约：请求"进行中"判定的单一真值来源 = activeRequests 注册表成员身份，本模块是唯一判定处。
// requestLogState 只负责持久化终态（completed / interrupted）的分类，不判定进行中。

import type { ActiveRequest } from "./activeRequests";
import {
  requestLogActivityState,
  requestLogCreatedAtMs,
  type PersistedRequestLogActivityState,
} from "./requestLogState";
import type { RequestLogSummary } from "./requestLogs";
import { resolveClaudeModelMappingFromSpecialSettings } from "./requestLogSpecialSettings";
import { MAX_ATTEMPTS_PER_TRACE } from "./traceLimits";
import type { TraceSession, TraceSummary } from "./traceStore";

export const REALTIME_TRACE_EXIT_START_MS = 600;
const REALTIME_TRACE_EXIT_ANIM_MS = 400;
const REALTIME_TRACE_EXIT_TOTAL_MS =
  REALTIME_TRACE_EXIT_START_MS + REALTIME_TRACE_EXIT_ANIM_MS + 100;

export type ActiveTraceSession = Omit<TraceSession, "summary"> & {
  summary?: undefined;
};

export type SettlingTraceSession = Omit<TraceSession, "summary"> & {
  summary: TraceSummary;
};

export type ProjectedRealtimeCard =
  | {
      kind: "active";
      trace: ActiveTraceSession;
      activeRequest: ActiveRequest;
    }
  | {
      kind: "settling";
      trace: SettlingTraceSession;
      activeRequest: null;
    };

export type ProjectedRequestLogRow = {
  log: RequestLogSummary;
  liveTrace: TraceSession | null;
  activityState: PersistedRequestLogActivityState;
};

export type RequestActivityProjection = {
  realtimeCards: ProjectedRealtimeCard[];
  requestRows: ProjectedRequestLogRow[];
  summaryCount: number;
};

export type BuildRequestActivityProjectionInput = {
  requestLogs: RequestLogSummary[];
  activeRequests?: ActiveRequest[];
  traces: TraceSession[];
  nowMs: number;
  realtimeCardLimit: number;
};

export type ActiveRequestSnapshotItem = ActiveRequest;

type RequestActivitySourceIndex = {
  activeByTraceId: Map<string, ActiveRequest>;
  requestRowsSorted: RequestLogSummary[];
  logsByTraceId: Map<string, RequestLogSummary>;
  mergedTraceMap: Map<string, TraceSession>;
};

function normalizeTraceId(traceId: string | null | undefined) {
  return traceId?.trim() || null;
}

function isRequestLogTerminal(log: RequestLogSummary) {
  return log.status != null || log.error_code != null;
}

function requestLogActivitySortRank(log: RequestLogSummary) {
  return requestLogActivityState(log) === "interrupted" ? 1 : 0;
}

function sortRequestLogsForActivity(a: RequestLogSummary, b: RequestLogSummary) {
  const aRank = requestLogActivitySortRank(a);
  const bRank = requestLogActivitySortRank(b);
  if (aRank !== bRank) return aRank - bRank;

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

  const requestLogTsMs = requestLogCreatedAtMs(requestLog);
  const claudeModelMapping =
    trace.claude_model_mapping ??
    resolveClaudeModelMappingFromSpecialSettings(
      requestLog.special_settings_json,
      requestLog.final_provider_id
    );
  const enrichedTrace = {
    ...trace,
    session_id: trace.session_id ?? requestLog.session_id ?? null,
    requested_model: trace.requested_model ?? requestLog.requested_model ?? null,
    claude_model_mapping: claudeModelMapping,
    last_seen_ms: Math.max(trace.last_seen_ms, requestLogTsMs),
  };

  if (!trace.summary && !isRequestLogTerminal(requestLog)) {
    return enrichedTrace;
  }

  const summary = trace.summary;
  const mergedSummary: TraceSummary = {
    trace_id: trace.trace_id,
    cli_key: trace.cli_key,
    session_id: summary?.session_id ?? trace.session_id ?? requestLog.session_id ?? null,
    method: trace.method,
    path: trace.path,
    query: trace.query,
    requested_model:
      summary?.requested_model ?? trace.requested_model ?? requestLog.requested_model ?? null,
    claude_model_mapping: summary?.claude_model_mapping ?? claudeModelMapping ?? null,
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
    effective_input_tokens:
      summary?.effective_input_tokens ?? requestLog.effective_input_tokens ?? null,
    cost_usd: summary?.cost_usd ?? requestLog.cost_usd ?? null,
    cost_multiplier: summary?.cost_multiplier ?? requestLog.cost_multiplier ?? null,
  };

  return {
    ...enrichedTrace,
    summary: mergedSummary,
  };
}

function traceFromActiveRequest(activeRequest: ActiveRequest): ActiveTraceSession {
  const createdAtMs = Number.isFinite(activeRequest.created_at_ms)
    ? Math.max(0, activeRequest.created_at_ms)
    : 0;
  return {
    trace_id: activeRequest.trace_id,
    cli_key: activeRequest.cli_key,
    session_id: activeRequest.session_id ?? null,
    method: activeRequest.method,
    path: activeRequest.path,
    query: activeRequest.query ?? null,
    requested_model: activeRequest.requested_model ?? null,
    claude_model_mapping: activeRequest.current_attempt?.claude_model_mapping ?? null,
    first_seen_ms: createdAtMs,
    last_seen_ms: Math.max(createdAtMs, activeRequest.last_activity_ms ?? 0),
    attempts: activeRequest.current_attempt ? [activeRequest.current_attempt] : [],
  };
}

function mergeTraceWithActiveRequestProgress(
  trace: TraceSession,
  activeRequest: ActiveRequest
): TraceSession {
  if (trace.summary) return trace;

  const currentAttempt = activeRequest.current_attempt;
  const traceAttempts = trace.attempts ?? [];
  const latestAttemptIndex = traceAttempts.reduce(
    (latest, attempt) => Math.max(latest, attempt.attempt_index),
    -1
  );
  const attempts = (() => {
    if (!currentAttempt || currentAttempt.attempt_index <= latestAttemptIndex) {
      return traceAttempts;
    }
    const merged = [...traceAttempts, currentAttempt].sort(
      (a, b) => a.attempt_index - b.attempt_index
    );
    return merged.slice(-MAX_ATTEMPTS_PER_TRACE);
  })();

  return {
    ...trace,
    session_id: trace.session_id ?? activeRequest.session_id ?? null,
    requested_model: trace.requested_model ?? activeRequest.requested_model ?? null,
    claude_model_mapping:
      trace.claude_model_mapping ?? currentAttempt?.claude_model_mapping ?? null,
    last_seen_ms: Math.max(trace.last_seen_ms, activeRequest.last_activity_ms ?? 0),
    attempts,
  };
}

function buildRequestActivitySourceIndex(input: {
  requestLogs: RequestLogSummary[];
  activeRequests: ActiveRequest[];
  traces: TraceSession[];
}): RequestActivitySourceIndex {
  const activeByTraceId = new Map<string, ActiveRequest>();
  for (const activeRequest of input.activeRequests) {
    const traceId = normalizeTraceId(activeRequest.trace_id);
    if (!traceId || activeByTraceId.has(traceId)) continue;
    activeByTraceId.set(traceId, activeRequest);
  }

  const requestRowsSorted = input.requestLogs.slice().sort(sortRequestLogsForActivity);
  const logsByTraceId = new Map<string, RequestLogSummary>();
  for (const log of requestRowsSorted) {
    const traceId = normalizeTraceId(log.trace_id);
    if (!traceId || logsByTraceId.has(traceId)) continue;
    logsByTraceId.set(traceId, log);
  }

  const mergedTraceMap = new Map<string, TraceSession>();
  for (const trace of input.traces) {
    const traceId = normalizeTraceId(trace.trace_id);
    if (!traceId || mergedTraceMap.has(traceId)) continue;
    mergedTraceMap.set(traceId, mergeTraceWithRequestLog(trace, logsByTraceId.get(traceId)));
  }

  for (const [traceId, activeRequest] of activeByTraceId) {
    const existingTrace = mergedTraceMap.get(traceId);
    if (existingTrace) {
      mergedTraceMap.set(
        traceId,
        mergeTraceWithActiveRequestProgress(existingTrace, activeRequest)
      );
      continue;
    }
    const log = logsByTraceId.get(traceId);
    if (log && isRequestLogTerminal(log)) continue;
    mergedTraceMap.set(
      traceId,
      mergeTraceWithRequestLog(traceFromActiveRequest(activeRequest), log)
    );
  }

  return {
    activeByTraceId,
    requestRowsSorted,
    logsByTraceId,
    mergedTraceMap,
  };
}

function projectRealtimeCard(
  traceId: string,
  trace: TraceSession,
  activeByTraceId: Map<string, ActiveRequest>,
  nowMs: number
): ProjectedRealtimeCard | null {
  if (trace.summary) {
    if (Math.max(0, nowMs - trace.last_seen_ms) >= REALTIME_TRACE_EXIT_TOTAL_MS) return null;
    return {
      kind: "settling",
      trace: { ...trace, summary: trace.summary },
      activeRequest: null,
    };
  }

  const activeRequest = activeByTraceId.get(traceId);
  if (!activeRequest) return null;
  return {
    kind: "active",
    trace: { ...trace, summary: undefined },
    activeRequest,
  };
}

function sortRealtimeCards(a: ProjectedRealtimeCard, b: ProjectedRealtimeCard) {
  if (a.trace.first_seen_ms !== b.trace.first_seen_ms) {
    return b.trace.first_seen_ms - a.trace.first_seen_ms;
  }
  return b.trace.trace_id.localeCompare(a.trace.trace_id);
}

function selectRealtimeCards(
  index: RequestActivitySourceIndex,
  nowMs: number,
  realtimeCardLimit: number
) {
  const activeCards: ProjectedRealtimeCard[] = [];
  const settlingCards: ProjectedRealtimeCard[] = [];

  for (const [traceId, trace] of index.mergedTraceMap) {
    const card = projectRealtimeCard(traceId, trace, index.activeByTraceId, nowMs);
    if (!card) continue;
    if (card.kind === "active") activeCards.push(card);
    else settlingCards.push(card);
  }

  activeCards.sort(sortRealtimeCards);
  settlingCards.sort(sortRealtimeCards);
  const settlingBudget = Math.max(0, realtimeCardLimit - activeCards.length);
  return [...activeCards, ...settlingCards.slice(0, settlingBudget)].sort(sortRealtimeCards);
}

export function shouldTickRequestActivityClock({
  requestLogs,
  activeRequests = [],
  traces,
  nowMs,
}: Pick<
  BuildRequestActivityProjectionInput,
  "requestLogs" | "activeRequests" | "traces" | "nowMs"
>) {
  const index = buildRequestActivitySourceIndex({ requestLogs, activeRequests, traces });
  for (const [traceId, trace] of index.mergedTraceMap) {
    if (projectRealtimeCard(traceId, trace, index.activeByTraceId, nowMs)) return true;
  }
  return false;
}

export function buildRequestActivityProjection({
  requestLogs,
  activeRequests = [],
  traces,
  nowMs,
  realtimeCardLimit,
}: BuildRequestActivityProjectionInput): RequestActivityProjection {
  const index = buildRequestActivitySourceIndex({ requestLogs, activeRequests, traces });
  const realtimeCards = selectRealtimeCards(index, nowMs, realtimeCardLimit);
  const visibleRealtimeTraceIds = new Set(
    realtimeCards
      .map((card) => normalizeTraceId(card.trace.trace_id))
      .filter((traceId): traceId is string => traceId != null)
  );

  const requestRows: ProjectedRequestLogRow[] = [];
  for (const log of index.requestRowsSorted) {
    const traceId = normalizeTraceId(log.trace_id);
    if (traceId && visibleRealtimeTraceIds.has(traceId)) continue;
    requestRows.push({
      log,
      liveTrace: traceId ? (index.mergedTraceMap.get(traceId) ?? null) : null,
      activityState: requestLogActivityState(log),
    });
  }

  const summaryTraceIds = new Set<string>();
  for (const log of index.requestRowsSorted) {
    const traceId = normalizeTraceId(log.trace_id);
    if (traceId) summaryTraceIds.add(traceId);
  }
  for (const activeRequest of index.activeByTraceId.values()) {
    const traceId = normalizeTraceId(activeRequest.trace_id);
    if (traceId) summaryTraceIds.add(traceId);
  }

  return {
    realtimeCards,
    requestRows,
    summaryCount: summaryTraceIds.size,
  };
}
