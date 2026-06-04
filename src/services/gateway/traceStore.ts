import { useSyncExternalStore } from "react";
import { emitListenerSnapshot } from "../../utils/listeners";
import { normalizeClaudeModelMapping, type ClaudeModelMapping } from "./claudeModelMapping";
import type {
  GatewayAttempt,
  GatewayAttemptEvent,
  GatewayRequestEvent,
  GatewayRequestStartEvent,
} from "./gatewayEvents";
import { MAX_ATTEMPTS_PER_TRACE } from "./traceLimits";

export type TraceSummary = GatewayRequestEvent & {
  // Preview/demo traces may carry cost hints that are not part of the runtime gateway event payload.
  cost_usd?: number | null;
  cost_multiplier?: number | null;
};

export type TraceSession = {
  trace_id: string;
  cli_key: string;
  session_id?: string | null;
  method: string;
  path: string;
  query: string | null;
  requested_model?: string | null;
  claude_model_mapping?: ClaudeModelMapping | null;
  first_seen_ms: number;
  last_seen_ms: number;
  attempts: GatewayAttemptEvent[];
  summary?: TraceSummary;
};

export type TraceStoreSnapshot = {
  traces: TraceSession[];
};

type Listener = () => void;

const MAX_TRACES = 50;

/**
 * Traces without summary older than this threshold are considered stale
 * and will be pruned during the next store mutation.
 * Mirrors claude-code-hub's Redis TTL (600s) safety-net strategy.
 */
const STALE_TRACE_TIMEOUT_MS = 5 * 60 * 1000;

type TraceStoreState = {
  traces: TraceSession[];
};

let state: TraceStoreState = {
  traces: [],
};

let snapshot: TraceStoreSnapshot = {
  traces: state.traces,
};

const listeners = new Set<Listener>();

function emit() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

function setState(next: TraceStoreState) {
  state = next;
  snapshot = {
    traces: state.traces,
  };
  emit();
}

function findTraceIndex(traceId: string): number {
  return state.traces.findIndex((trace) => trace.trace_id === traceId);
}

function upsertAttempt(
  attempts: GatewayAttemptEvent[],
  payload: GatewayAttemptEvent
): GatewayAttemptEvent[] {
  const existing = attempts.find((a) => a.attempt_index === payload.attempt_index);
  const mergedPayload =
    existing?.claude_model_mapping && !payload.claude_model_mapping
      ? { ...payload, claude_model_mapping: existing.claude_model_mapping }
      : payload;
  const next = attempts.filter((a) => a.attempt_index !== payload.attempt_index);
  next.push(mergedPayload);
  next.sort((a, b) => a.attempt_index - b.attempt_index);
  return next.slice(-MAX_ATTEMPTS_PER_TRACE);
}

function hasClaudeModelMappingField(payload: GatewayRequestEvent): boolean {
  return Object.prototype.hasOwnProperty.call(payload, "claude_model_mapping");
}

function trimSummaryAttempts<T extends GatewayAttempt | GatewayAttemptEvent>(attempts: T[]): T[] {
  return attempts.length > MAX_ATTEMPTS_PER_TRACE
    ? attempts.slice(-MAX_ATTEMPTS_PER_TRACE)
    : attempts;
}

function trimSummary(payload: GatewayRequestEvent): TraceSummary {
  return {
    ...payload,
    attempts: trimSummaryAttempts(payload.attempts),
  };
}

/**
 * Remove traces stuck in "in progress" (no summary) beyond the stale threshold.
 * Called piggy-back on every store mutation to avoid a dedicated timer.
 */
function pruneStaleTraces(traces: TraceSession[], now: number): TraceSession[] {
  return traces.filter((t) => t.summary || now - t.last_seen_ms < STALE_TRACE_TIMEOUT_MS);
}

function moveTraceToFront(nextTraces: TraceSession[], traceId: string) {
  const index = nextTraces.findIndex((t) => t.trace_id === traceId);
  if (index <= 0) return nextTraces;
  const trace = nextTraces[index];
  nextTraces.splice(index, 1);
  nextTraces.unshift(trace);
  return nextTraces;
}

/**
 * Common upsert logic shared by all three ingest functions.
 * Creates a new TraceSession if not found, otherwise updates the existing one.
 */
function upsertTrace(
  traceId: string,
  createSession: (now: number) => TraceSession,
  updateSession: (existing: TraceSession, now: number) => TraceSession
) {
  const now = Date.now();
  const idx = findTraceIndex(traceId);

  if (idx === -1) {
    const created = createSession(now);
    const nextTraces = pruneStaleTraces([created, ...state.traces], now).slice(0, MAX_TRACES);
    setState({ traces: nextTraces });
    return;
  }

  const existing = state.traces[idx];
  const updated = updateSession(existing, now);

  const nextTraces = pruneStaleTraces(state.traces.slice(), now);
  const prunedIdx = nextTraces.findIndex((t) => t.trace_id === updated.trace_id);
  if (prunedIdx !== -1) {
    nextTraces[prunedIdx] = updated;
  } else {
    nextTraces.unshift(updated);
  }
  moveTraceToFront(nextTraces, updated.trace_id);
  setState({ traces: nextTraces.slice(0, MAX_TRACES) });
}

export function ingestTraceStart(payload: GatewayRequestStartEvent) {
  if (!payload?.trace_id) return;

  upsertTrace(
    payload.trace_id,
    (now) => ({
      trace_id: payload.trace_id,
      cli_key: payload.cli_key,
      session_id: payload.session_id ?? null,
      method: payload.method,
      path: payload.path,
      query: payload.query ?? null,
      requested_model: payload.requested_model ?? null,
      first_seen_ms: now,
      last_seen_ms: now,
      attempts: [],
    }),
    (existing, now) => {
      const nextRequestedModel = payload.requested_model ?? existing.requested_model ?? null;
      const shouldReset = Boolean(existing.summary);
      return {
        ...existing,
        cli_key: payload.cli_key,
        session_id: payload.session_id ?? existing.session_id ?? null,
        method: payload.method,
        path: payload.path,
        query: payload.query ?? null,
        requested_model: nextRequestedModel,
        claude_model_mapping: shouldReset ? null : (existing.claude_model_mapping ?? null),
        last_seen_ms: now,
        ...(shouldReset ? { first_seen_ms: now, attempts: [], summary: undefined } : {}),
      };
    }
  );
}

export function ingestTraceAttempt(payload: GatewayAttemptEvent) {
  if (!payload?.trace_id) return;

  upsertTrace(
    payload.trace_id,
    (now) => ({
      trace_id: payload.trace_id,
      cli_key: payload.cli_key,
      session_id: payload.session_id ?? null,
      method: payload.method,
      path: payload.path,
      query: payload.query ?? null,
      requested_model: payload.requested_model ?? null,
      claude_model_mapping: normalizeClaudeModelMapping(payload.claude_model_mapping),
      first_seen_ms: now,
      last_seen_ms: now,
      attempts: [payload],
    }),
    (existing, now) => {
      const nextRequestedModel = payload.requested_model ?? existing.requested_model ?? null;
      const nextClaudeModelMapping =
        normalizeClaudeModelMapping(payload.claude_model_mapping) ??
        existing.claude_model_mapping ??
        null;
      return {
        ...existing,
        cli_key: payload.cli_key,
        session_id: payload.session_id ?? existing.session_id ?? null,
        method: payload.method,
        path: payload.path,
        query: payload.query ?? null,
        requested_model: nextRequestedModel,
        claude_model_mapping: nextClaudeModelMapping,
        last_seen_ms: now,
        attempts: upsertAttempt(existing.attempts, payload),
      };
    }
  );
}

export function ingestTraceRequest(payload: GatewayRequestEvent) {
  if (!payload?.trace_id) return;
  const summary = trimSummary(payload);

  upsertTrace(
    summary.trace_id,
    (now) => ({
      trace_id: summary.trace_id,
      cli_key: summary.cli_key,
      session_id: summary.session_id ?? null,
      method: summary.method,
      path: summary.path,
      query: summary.query ?? null,
      requested_model: summary.requested_model ?? null,
      claude_model_mapping: normalizeClaudeModelMapping(summary.claude_model_mapping),
      first_seen_ms: now,
      last_seen_ms: now,
      attempts: [],
      summary,
    }),
    (existing, now) => {
      const nextRequestedModel = summary.requested_model ?? existing.requested_model ?? null;
      const normalizedClaudeModelMapping = normalizeClaudeModelMapping(
        summary.claude_model_mapping
      );
      const nextClaudeModelMapping = hasClaudeModelMappingField(summary)
        ? normalizedClaudeModelMapping
        : (normalizedClaudeModelMapping ?? existing.claude_model_mapping ?? null);
      return {
        ...existing,
        cli_key: summary.cli_key,
        session_id: summary.session_id ?? existing.session_id ?? null,
        method: summary.method,
        path: summary.path,
        query: summary.query ?? null,
        requested_model: nextRequestedModel,
        claude_model_mapping: nextClaudeModelMapping,
        last_seen_ms: now,
        summary,
      };
    }
  );
}

export function subscribeTraceStore(listener: Listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useTraceStore(): TraceStoreSnapshot {
  return useSyncExternalStore(
    subscribeTraceStore,
    () => snapshot,
    () => snapshot
  );
}
