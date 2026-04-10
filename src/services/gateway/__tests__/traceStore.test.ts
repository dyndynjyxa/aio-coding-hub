import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

async function importFreshTraceStore() {
  vi.resetModules();
  return await import("../traceStore");
}

describe("services/gateway/traceStore", () => {
  it("ingestTraceStart creates traces and resets completed traces", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceRequest, useTraceStore } = await importFreshTraceStore();

    const { result } = renderHook(() => useTraceStore());
    expect(result.current.traces).toEqual([]);

    act(() => {
      ingestTraceStart({
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        requested_model: "claude-3",
        ts: 0,
      });
    });
    expect(result.current.traces[0]?.trace_id).toBe("t1");
    expect(result.current.traces[0]?.summary).toBeUndefined();

    act(() => {
      ingestTraceRequest({
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 12,
        attempts: [],
      });
    });
    expect(result.current.traces[0]?.summary?.status).toBe(200);

    vi.setSystemTime(1000);
    act(() => {
      ingestTraceStart({
        trace_id: "t1",
        cli_key: "claude",
        method: "POST",
        path: "/v1/again",
        query: "x=1",
        requested_model: "claude-3-opus",
        ts: 1,
      });
    });
    expect(result.current.traces[0]?.method).toBe("POST");
    expect(result.current.traces[0]?.path).toBe("/v1/again");
    expect(result.current.traces[0]?.summary).toBeUndefined();

    vi.useRealTimers();
  });

  it("ingestTraceAttempt upserts attempts and moves trace to front", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceAttempt, useTraceStore } = await importFreshTraceStore();

    const { result } = renderHook(() => useTraceStore());

    act(() => {
      ingestTraceAttempt({
        trace_id: "tA",
        cli_key: "codex",
        method: "GET",
        path: "/x",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      });
    });
    expect(result.current.traces[0]?.trace_id).toBe("tA");
    expect(result.current.traces[0]?.attempts).toHaveLength(1);

    // Upsert same index replaces.
    act(() => {
      ingestTraceAttempt({
        trace_id: "tA",
        cli_key: "codex",
        method: "GET",
        path: "/x",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "failed",
        status: 500,
        attempt_started_ms: 0,
        attempt_duration_ms: 12,
      });
    });
    expect(result.current.traces[0]?.attempts).toHaveLength(1);
    expect(result.current.traces[0]?.attempts[0]?.status).toBe(500);

    // New trace moves to front.
    vi.setSystemTime(1000);
    act(() => {
      ingestTraceAttempt({
        trace_id: "tB",
        cli_key: "claude",
        method: "POST",
        path: "/y",
        query: null,
        attempt_index: 1,
        provider_id: 2,
        provider_name: "P2",
        base_url: "https://p2",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      });
    });
    expect(result.current.traces[0]?.trace_id).toBe("tB");
    expect(result.current.traces[1]?.trace_id).toBe("tA");

    vi.useRealTimers();
  });

  it("ingestTraceAttempt backfills requested_model when request_start is missing", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceAttempt, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    act(() => {
      ingestTraceAttempt({
        trace_id: "t-model-from-attempt",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-opus-4-6",
        attempt_index: 1,
        provider_id: 2,
        provider_name: "SSAiCode",
        base_url: "https://provider.example",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      });
    });

    expect(result.current.traces[0]?.requested_model).toBe("claude-opus-4-6");

    vi.useRealTimers();
  });

  it("ingestTraceRequest creates new trace when trace_id not found", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(5000);

    const { ingestTraceRequest, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    expect(result.current.traces).toEqual([]);

    act(() => {
      ingestTraceRequest({
        trace_id: "new-req",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 50,
        attempts: [],
      });
    });

    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("new-req");
    expect(result.current.traces[0]?.summary).toBeDefined();
    expect(result.current.traces[0]?.summary?.status).toBe(200);
    expect(result.current.traces[0]?.summary?.duration_ms).toBe(50);
    expect(result.current.traces[0]?.attempts).toEqual([]);

    vi.useRealTimers();
  });

  it("ingestTraceRequest updates existing trace with summary", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceRequest, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    act(() => {
      ingestTraceStart({
        trace_id: "existing-req",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-3-opus",
        ts: 0,
      });
    });

    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.summary).toBeUndefined();

    vi.setSystemTime(100);
    act(() => {
      ingestTraceRequest({
        trace_id: "existing-req",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 100,
        attempts: [],
      });
    });

    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("existing-req");
    expect(result.current.traces[0]?.summary).toBeDefined();
    expect(result.current.traces[0]?.summary?.status).toBe(200);

    vi.useRealTimers();
  });

  it("ingestTraceRequest backfills requested_model when summary arrives first", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceRequest, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    act(() => {
      ingestTraceRequest({
        trace_id: "summary-first",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-opus-4-6",
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 50,
        attempts: [],
      });
    });

    expect(result.current.traces[0]?.requested_model).toBe("claude-opus-4-6");

    vi.useRealTimers();
  });

  it("preserves and backfills session_id across realtime event updates", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceAttempt, ingestTraceRequest, useTraceStore } =
      await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    act(() => {
      ingestTraceStart({
        trace_id: "t-session",
        cli_key: "codex",
        session_id: "session-from-start",
        method: "POST",
        path: "/v1/responses",
        query: null,
        ts: 0,
      });
    });
    expect(result.current.traces[0]?.session_id).toBe("session-from-start");

    act(() => {
      ingestTraceAttempt({
        trace_id: "t-session",
        cli_key: "codex",
        method: "POST",
        path: "/v1/responses",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      });
    });
    expect(result.current.traces[0]?.session_id).toBe("session-from-start");

    act(() => {
      ingestTraceRequest({
        trace_id: "t-session-2",
        cli_key: "claude",
        session_id: "session-from-summary",
        method: "POST",
        path: "/v1/messages",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 50,
        attempts: [],
      });
    });
    expect(
      result.current.traces.find((trace) => trace.trace_id === "t-session-2")?.session_id
    ).toBe("session-from-summary");

    vi.useRealTimers();
  });

  it("ignores payloads with missing trace_id", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceAttempt, ingestTraceRequest, useTraceStore } =
      await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    expect(result.current.traces).toEqual([]);

    // null/undefined payloads
    act(() => {
      ingestTraceStart(null as never);
      ingestTraceAttempt(undefined as never);
      ingestTraceRequest(null as never);
    });
    expect(result.current.traces).toEqual([]);

    // payloads with empty trace_id
    act(() => {
      ingestTraceStart({
        trace_id: "",
        cli_key: "claude",
        method: "GET",
        path: "/",
        query: null,
        ts: 0,
      });
      ingestTraceAttempt({
        trace_id: "",
        cli_key: "claude",
        method: "GET",
        path: "/",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P",
        base_url: "https://p",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      });
      ingestTraceRequest({
        trace_id: "",
        cli_key: "claude",
        method: "GET",
        path: "/",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 0,
        attempts: [],
      });
    });
    expect(result.current.traces).toEqual([]);

    vi.useRealTimers();
  });

  it("pruneStaleTraces removes old traces without summary", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    // Create a trace at time 0 (no summary = "in progress")
    act(() => {
      ingestTraceStart({
        trace_id: "stale-trace",
        cli_key: "claude",
        method: "GET",
        path: "/v1/old",
        query: null,
        requested_model: "claude-3",
        ts: 0,
      });
    });
    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("stale-trace");

    // Advance time past STALE_TRACE_TIMEOUT_MS (5 minutes = 300000ms)
    vi.setSystemTime(300_001);

    // Ingest another trace; pruneStaleTraces runs and removes the stale one
    act(() => {
      ingestTraceStart({
        trace_id: "fresh-trace",
        cli_key: "claude",
        method: "POST",
        path: "/v1/new",
        query: null,
        requested_model: "claude-3",
        ts: 300_001,
      });
    });

    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("fresh-trace");
  });

  it("ingestTraceRequest prune-then-upsert: stale trace pruned and re-inserted via idx === -1 branch", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceRequest, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    // Create a trace at time 0 (no summary)
    act(() => {
      ingestTraceStart({
        trace_id: "will-be-pruned",
        cli_key: "claude",
        method: "GET",
        path: "/v1/stale",
        query: null,
        requested_model: "claude-3",
        ts: 0,
      });
    });
    expect(result.current.traces).toHaveLength(1);

    // Advance past stale threshold
    vi.setSystemTime(300_001);

    // ingestTraceRequest for the same trace_id:
    // 1. findTraceIndex finds the trace (idx !== -1)
    // 2. pruneStaleTraces removes it (no summary + stale)
    // 3. prunedIdx === -1 => re-inserted via unshift
    act(() => {
      ingestTraceRequest({
        trace_id: "will-be-pruned",
        cli_key: "claude",
        method: "GET",
        path: "/v1/stale",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 300_001,
        attempts: [],
      });
    });

    // The trace should exist with summary (re-inserted after pruning)
    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("will-be-pruned");
    expect(result.current.traces[0]?.summary).toBeDefined();
    expect(result.current.traces[0]?.summary?.status).toBe(200);

    vi.useRealTimers();
  });

  it("moveTraceToFront returns early when trace is already at front or not found", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { ingestTraceStart, ingestTraceAttempt, useTraceStore } = await importFreshTraceStore();
    const { result } = renderHook(() => useTraceStore());

    // Create a single trace
    act(() => {
      ingestTraceStart({
        trace_id: "only-trace",
        cli_key: "claude",
        method: "GET",
        path: "/v1/single",
        query: null,
        requested_model: "claude-3",
        ts: 0,
      });
    });
    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("only-trace");

    // Update the same trace (already at front, moveTraceToFront index === 0 => returns early)
    vi.setSystemTime(100);
    act(() => {
      ingestTraceAttempt({
        trace_id: "only-trace",
        cli_key: "claude",
        method: "GET",
        path: "/v1/single",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "started",
        status: null,
        attempt_started_ms: 100,
        attempt_duration_ms: 0,
      });
    });

    // Trace is still at front, only one trace
    expect(result.current.traces).toHaveLength(1);
    expect(result.current.traces[0]?.trace_id).toBe("only-trace");
    expect(result.current.traces[0]?.attempts).toHaveLength(1);

    vi.useRealTimers();
  });
});
