import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";

async function importFreshCacheAnomalyMonitor() {
  vi.resetModules();
  const mod = await import("../cacheAnomalyMonitor");
  return mod;
}

function requestStart(traceId: string, model: string) {
  return {
    trace_id: traceId,
    cli_key: "claude",
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: model,
    ts: 0,
  } as any;
}

function requestEvent(traceId: string, opts: { create: number; read: number; input: number }) {
  return {
    trace_id: traceId,
    cli_key: "claude",
    method: "POST",
    path: "/v1/messages",
    query: null,
    status: 200,
    error_category: null,
    error_code: null,
    duration_ms: 100,
    attempts: [
      {
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "success",
        status: 200,
      },
    ],
    input_tokens: opts.input,
    cache_read_input_tokens: opts.read,
    cache_creation_input_tokens: opts.create,
  } as any;
}

describe("services/gateway/cacheAnomalyMonitor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("defaults disabled and notifies subscribers (including idempotent sets)", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const mod = await importFreshCacheAnomalyMonitor();

    expect(mod.getCacheAnomalyMonitorEnabled()).toBe(false);

    const { result } = renderHook(() => mod.useCacheAnomalyMonitorEnabled());
    expect(result.current).toBe(false);

    // Idempotent set should be a no-op.
    mod.setCacheAnomalyMonitorEnabled(false);
    expect(mod.getCacheAnomalyMonitorEnabled()).toBe(false);

    act(() => mod.setCacheAnomalyMonitorEnabled(true));
    expect(mod.getCacheAnomalyMonitorEnabled()).toBe(true);
    expect(result.current).toBe(true);

    act(() => mod.setCacheAnomalyMonitorEnabled(false));
    expect(mod.getCacheAnomalyMonitorEnabled()).toBe(false);
    expect(result.current).toBe(false);

    vi.useRealTimers();
  });

  it("toggle enabled state and ignore non-caching models", async () => {
    const {
      getCacheAnomalyMonitorEnabled,
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
    } = await importFreshCacheAnomalyMonitor();

    expect(getCacheAnomalyMonitorEnabled()).toBe(false);
    setCacheAnomalyMonitorEnabled(true);
    expect(getCacheAnomalyMonitorEnabled()).toBe(true);

    // Haiku is treated as non-caching and should be ignored.
    ingestCacheAnomalyRequestStart(requestStart("t-haiku", "claude-3-haiku"));

    setCacheAnomalyMonitorEnabled(false);
    expect(getCacheAnomalyMonitorEnabled()).toBe(false);
  });

  it("covers ingest early-return and normalization branches without emitting alerts", async () => {
    vi.useFakeTimers();
    const baseTimeMs = 1_700_000_000_000;
    vi.setSystemTime(baseTimeMs);

    const mod = await importFreshCacheAnomalyMonitor();
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    // Disabled: ingest calls should no-op
    mod.setCacheAnomalyMonitorEnabled(false);
    expect(() => mod.ingestCacheAnomalyRequestStart({} as any)).not.toThrow();
    expect(() => mod.ingestCacheAnomalyRequest({} as any)).not.toThrow();

    mod.setCacheAnomalyMonitorEnabled(true);

    // Invalid request_start payloads
    mod.ingestCacheAnomalyRequestStart({
      trace_id: "",
      cli_key: "claude",
      method: "GET",
      path: "/v1/test",
      query: null,
      ts: 0,
    } as any);
    mod.ingestCacheAnomalyRequestStart({
      trace_id: "t-unsupported",
      cli_key: "gemini",
      method: "GET",
      path: "/v1/test",
      query: null,
      ts: 0,
    } as any);

    // Long model name normalization (slice > 200)
    mod.ingestCacheAnomalyRequestStart({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      requested_model: "x".repeat(250),
      ts: 0,
    } as any);

    // Invalid request payloads
    mod.ingestCacheAnomalyRequest({
      trace_id: "",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [],
    } as any);

    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "gemini",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [],
    } as any);

    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: null,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [],
    } as any);

    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 500,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [],
    } as any);

    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: "E",
      duration_ms: 1,
      attempts: [],
    } as any);

    // Success request but no attempts => pickFinalProvider returns null
    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [],
      input_tokens: "foo",
      cache_read_input_tokens: -1,
      cache_creation_input_tokens: 1,
    } as any);

    // Invalid provider_id => early return
    mod.ingestCacheAnomalyRequest({
      trace_id: "t-long-model",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [
        {
          provider_id: -1,
          provider_name: "  ",
          base_url: "",
          outcome: "failed",
          status: 200,
        },
      ],
      input_tokens: "foo",
      cache_read_input_tokens: -1,
      cache_creation_input_tokens: 1,
    } as any);

    // denomTokens <= 0 => early return (also provider name fallback from whitespace)
    mod.ingestCacheAnomalyRequestStart({
      trace_id: "t-zero",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      requested_model: "claude-3-opus",
      ts: 0,
    } as any);
    mod.ingestCacheAnomalyRequest({
      trace_id: "t-zero",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 1,
      attempts: [
        {
          provider_id: 1,
          provider_name: "   ",
          base_url: "https://p1",
          outcome: "failed",
          status: 200,
        },
      ],
      input_tokens: 0,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
    } as any);

    // trace model prune path (expire start entries that never received a request)
    mod.ingestCacheAnomalyRequestStart({
      trace_id: "t-expire",
      cli_key: "claude",
      method: "POST",
      path: "/v1/messages",
      query: null,
      requested_model: "claude-3-opus",
      ts: 0,
    } as any);

    vi.setSystemTime(baseTimeMs + 10 * 60_000 + 1);
    mod.ingestCacheAnomalyRequestStart(requestStart("t-prune-driver", "claude-3-opus"));
    mod.ingestCacheAnomalyRequest(
      requestEvent("t-prune-driver", {
        input: 1000,
        read: 100,
        create: 0,
      })
    );

    vi.useRealTimers();
  });

  it("disables monitor when self-check detects inconsistent window sums", async () => {
    vi.useFakeTimers();

    const {
      getCacheAnomalyMonitorEnabled,
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    setCacheAnomalyMonitorEnabled(true);
    expect(getCacheAnomalyMonitorEnabled()).toBe(true);

    // 1) Add sample at minute 30 (eval will run once and set lastEvalMs).
    vi.setSystemTime(1_800_000);
    ingestCacheAnomalyRequestStart(requestStart("t-30", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t-30", {
        input: 1000,
        read: 50,
        create: 10,
      })
    );

    // 2) Move time backwards far enough to collide ring buffer bucket index 30 (minute -30).
    // This overwrites ring bucket minute=30 without updating lastEvalMs (eval interval gate).
    vi.setSystemTime(-1_800_000);
    ingestCacheAnomalyRequestStart(requestStart("t--30", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t--30", {
        input: 1000,
        read: 50,
        create: 10,
      })
    );

    // 3) Move forward to minute 31 and trigger evaluation again -> self-check should fail.
    vi.setSystemTime(1_860_001);
    ingestCacheAnomalyRequestStart(requestStart("t-31", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t-31", {
        input: 1000,
        read: 50,
        create: 10,
      })
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(getCacheAnomalyMonitorEnabled()).toBe(false);
    expect(tauriInvoke).toHaveBeenCalledWith(
      "notice_send",
      expect.objectContaining({ level: "error", title: "缓存异常监测已关闭" })
    );

    vi.useRealTimers();
  });

  it("handles codex effective input and provider/model fallbacks", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    setCacheAnomalyMonitorEnabled(true);

    ingestCacheAnomalyRequestStart({
      trace_id: "t-codex",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      query: null,
      requested_model: null,
      ts: 0,
    } as any);

    ingestCacheAnomalyRequest({
      trace_id: "t-codex",
      cli_key: "codex",
      method: "POST",
      path: "/v1/responses",
      query: null,
      status: 200,
      error_category: null,
      error_code: null,
      duration_ms: 10,
      attempts: [
        {
          provider_id: 2,
          provider_name: "  ",
          base_url: "",
          outcome: "failed",
          status: 500,
        },
        {
          provider_id: 2,
          provider_name: "",
          base_url: "",
          outcome: "failed",
          status: 500,
        },
      ],
      input_tokens: 100,
      cache_read_input_tokens: 200,
      cache_creation_input_tokens: 1,
      cache_creation_5m_input_tokens: 3,
      cache_creation_1h_input_tokens: 4,
    } as any);

    vi.useRealTimers();
  });

  it("emits alert for creation-without-read and handles notice_send failure", async () => {
    vi.useFakeTimers();
    const baseTimeMs = 1_700_000_000_000;
    vi.setSystemTime(baseTimeMs);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    // Force notice_send to fail so safeNoticeSend catches the error path.
    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string) => {
      if (cmd === "notice_send") {
        throw new Error("boom");
      }
      return null as any;
    });

    const core = await import("@tauri-apps/api/core");
    expect(core.invoke).toBe(tauriInvoke);

    setCacheAnomalyMonitorEnabled(true);

    // Build enough samples without triggering evaluation (nowMs=0 => eval gated).
    for (let i = 0; i < 10; i += 1) {
      const traceId = `t-${i}`;
      ingestCacheAnomalyRequestStart(requestStart(traceId, "claude-3-opus"));
      ingestCacheAnomalyRequest(
        requestEvent(traceId, {
          input: 400,
          read: 0,
          create: 100,
        })
      );
    }

    // Advance time so evaluation runs and triggers the "creation but no read" alert.
    vi.setSystemTime(baseTimeMs + 60_001);
    ingestCacheAnomalyRequestStart(requestStart("t-final", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t-final", {
        input: 400,
        read: 0,
        create: 100,
      })
    );

    // emitAlert is fire-and-forget; allow microtasks to flush.
    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(tauriInvoke).toHaveBeenCalledWith(
      "notice_send",
      expect.objectContaining({ level: "warning" })
    );

    vi.useRealTimers();
  });

  it("skips ingesting requests for non-caching models (haiku)", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);
    setCacheAnomalyMonitorEnabled(true);

    ingestCacheAnomalyRequestStart(requestStart("t-haiku", "claude-3-haiku"));
    ingestCacheAnomalyRequest(
      requestEvent("t-haiku", {
        input: 1000,
        read: 100,
        create: 100,
      })
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(tauriInvoke).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("emits alert for high cache create share", async () => {
    vi.useFakeTimers();
    const baseTimeMs = 1_700_000_000_000;
    vi.setSystemTime(baseTimeMs);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);
    setCacheAnomalyMonitorEnabled(true);

    // Build enough samples without triggering evaluation repeatedly.
    for (let i = 0; i < 10; i += 1) {
      const traceId = `t-create-share-${i}`;
      ingestCacheAnomalyRequestStart(requestStart(traceId, "claude-3-opus"));
      ingestCacheAnomalyRequest(
        requestEvent(traceId, {
          input: 390,
          read: 10,
          create: 390,
        })
      );
    }

    vi.setSystemTime(baseTimeMs + 60_001);
    ingestCacheAnomalyRequestStart(requestStart("t-create-share-final", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t-create-share-final", {
        input: 390,
        read: 10,
        create: 390,
      })
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(tauriInvoke).toHaveBeenCalledWith(
      "notice_send",
      expect.objectContaining({ level: "warning" })
    );

    vi.useRealTimers();
  });

  it("emits alert for create/read imbalance", async () => {
    vi.useFakeTimers();
    const baseTimeMs = 1_700_000_000_000;
    vi.setSystemTime(baseTimeMs);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);
    setCacheAnomalyMonitorEnabled(true);

    for (let i = 0; i < 10; i += 1) {
      const traceId = `t-create-ratio-${i}`;
      ingestCacheAnomalyRequestStart(requestStart(traceId, "claude-3-opus"));
      ingestCacheAnomalyRequest(
        requestEvent(traceId, {
          input: 300,
          read: 100,
          create: 300,
        })
      );
    }

    vi.setSystemTime(baseTimeMs + 60_001);
    ingestCacheAnomalyRequestStart(requestStart("t-create-ratio-final", "claude-3-opus"));
    ingestCacheAnomalyRequest(
      requestEvent("t-create-ratio-final", {
        input: 300,
        read: 100,
        create: 300,
      })
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(tauriInvoke).toHaveBeenCalledWith(
      "notice_send",
      expect.objectContaining({ level: "warning" })
    );

    vi.useRealTimers();
  });

  it("emits alert for hit-rate big drop", async () => {
    vi.useFakeTimers();
    const baseTimeMs = 1_700_000_000_000;
    vi.setSystemTime(baseTimeMs);

    const {
      setCacheAnomalyMonitorEnabled,
      ingestCacheAnomalyRequestStart,
      ingestCacheAnomalyRequest,
    } = await importFreshCacheAnomalyMonitor();

    vi.mocked(tauriInvoke).mockResolvedValue(true as any);
    setCacheAnomalyMonitorEnabled(true);

    for (let minute = 0; minute < 60; minute += 1) {
      vi.setSystemTime(baseTimeMs + minute * 60_000);
      const traceId = `t-drop-${minute}`;
      ingestCacheAnomalyRequestStart(requestStart(traceId, "claude-3-opus"));
      ingestCacheAnomalyRequest(
        requestEvent(traceId, {
          input: 400,
          read: minute < 45 ? 80 : 1,
          create: 0,
        })
      );
    }

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(tauriInvoke).toHaveBeenCalledWith(
      "notice_send",
      expect.objectContaining({ level: "warning" })
    );

    vi.useRealTimers();
  });
});
