import { useSyncExternalStore } from "react";
import type {
  GatewayAttemptEvent,
  GatewayRequestEvent,
  GatewayRequestStartEvent,
} from "./gatewayEvents";

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
const MAX_ATTEMPTS_PER_TRACE = 100;

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
  for (const listener of listeners) listener();
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
  const next = attempts.filter((a) => a.attempt_index !== payload.attempt_index);
  next.push(payload);
  next.sort((a, b) => a.attempt_index - b.attempt_index);
  return next.slice(-MAX_ATTEMPTS_PER_TRACE);
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
      first_seen_ms: now,
      last_seen_ms: now,
      attempts: [payload],
    }),
    (existing, now) => {
      const nextRequestedModel = payload.requested_model ?? existing.requested_model ?? null;
      return {
        ...existing,
        cli_key: payload.cli_key,
        session_id: payload.session_id ?? existing.session_id ?? null,
        method: payload.method,
        path: payload.path,
        query: payload.query ?? null,
        requested_model: nextRequestedModel,
        last_seen_ms: now,
        attempts: upsertAttempt(existing.attempts, payload),
      };
    }
  );
}

export function ingestTraceRequest(payload: GatewayRequestEvent) {
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
      summary: payload,
    }),
    (existing, now) => {
      const nextRequestedModel = payload.requested_model ?? existing.requested_model ?? null;
      return {
        ...existing,
        cli_key: payload.cli_key,
        session_id: payload.session_id ?? existing.session_id ?? null,
        method: payload.method,
        path: payload.path,
        query: payload.query ?? null,
        requested_model: nextRequestedModel,
        last_seen_ms: now,
        summary: payload,
      };
    }
  );
}

export function useTraceStore(): TraceStoreSnapshot {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => snapshot,
    () => snapshot
  );
}
