import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { clearTauriEventListeners, tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/gateway/gatewayEvents", () => {
  it("cleans up successful listeners when one subscription fails", async () => {
    setTauriRuntime();
    vi.resetModules();
    clearTauriEventListeners();

    const unlistenFns = Array.from({ length: 4 }, () => vi.fn());
    vi.mocked(tauriListen)
      .mockResolvedValueOnce(unlistenFns[0])
      .mockResolvedValueOnce(unlistenFns[1])
      .mockRejectedValueOnce(new Error("listen boom"))
      .mockResolvedValueOnce(unlistenFns[2])
      .mockResolvedValueOnce(unlistenFns[3]);

    const { listenGatewayEvents } = await import("../gatewayEvents");

    await expect(listenGatewayEvents()).rejects.toThrow("listen boom");

    expect(tauriListen).toHaveBeenCalledTimes(5);
    unlistenFns.forEach((fn) => expect(fn).toHaveBeenCalledTimes(1));
  });

  it("bounds non-transition circuit dedup without clearing hot entries", async () => {
    vi.resetModules();
    const { shouldLogCircuitNonTransition } = await import("../gatewayEvents");

    const dedup = new Map<string, number>();
    expect(shouldLogCircuitNonTransition(dedup, "same", 0)).toBe(true);
    expect(shouldLogCircuitNonTransition(dedup, "same", 100)).toBe(false);
    expect(shouldLogCircuitNonTransition(dedup, "same", 3000)).toBe(true);

    const withExpired = new Map<string, number>([
      ["old", 0],
      ["fresh", 9500],
    ]);
    expect(shouldLogCircuitNonTransition(withExpired, "next", 10_000)).toBe(true);
    expect(withExpired.has("old")).toBe(false);
    expect(withExpired.has("fresh")).toBe(true);
    expect(withExpired.has("next")).toBe(true);

    const full = new Map<string, number>();
    for (let index = 0; index < 500; index += 1) {
      full.set(`key-${index}`, 10_000 + index);
    }

    expect(shouldLogCircuitNonTransition(full, "key-new", 11_000)).toBe(true);

    expect(full.size).toBe(500);
    expect(full.has("key-0")).toBe(false);
    expect(full.has("key-1")).toBe(true);
    expect(full.has("key-new")).toBe(true);
  });

  it("registers listeners and handles payload branches", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(0);

    const { setConsoleLogMinLevel } = await import("../../consoleLog");
    setConsoleLogMinLevel("debug");

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    expect(tauriListen).toHaveBeenCalledTimes(5);

    const handlerFor = (eventName: string) =>
      vi.mocked(tauriListen).mock.calls.find((call) => call[0] === eventName)?.[1];

    const requestStart = handlerFor(gatewayEventNames.requestStart);
    requestStart?.({ payload: null } as any);
    requestStart?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        requested_model: "claude-3",
        ts: 0,
      },
    } as any);

    const attempt = handlerFor(gatewayEventNames.attempt);
    attempt?.({ payload: null } as any);
    attempt?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "started",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 0,
      },
    } as any);
    attempt?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "failed",
        status: 500,
        attempt_started_ms: 0,
        attempt_duration_ms: 12,
        circuit_state_before: "OPEN",
        circuit_state_after: "CLOSED",
        circuit_failure_count: 1,
        circuit_failure_threshold: 5,
      },
    } as any);

    const request = handlerFor(gatewayEventNames.request);
    request?.({ payload: null } as any);
    request?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        status: 500,
        error_category: "upstream",
        error_code: "E",
        duration_ms: 1000,
        ttfb_ms: 200,
        attempts: [],
        input_tokens: 1,
        output_tokens: 1,
        total_tokens: 2,
      },
    } as any);
    request?.({
      payload: {
        trace_id: "t2",
        cli_key: "claude",
        method: "POST",
        path: "/v1/ok",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 1000,
        ttfb_ms: 999,
        attempts: [],
        output_tokens: 5,
      },
    } as any);

    const log = handlerFor(gatewayEventNames.log);
    log?.({ payload: null } as any);
    log?.({
      payload: {
        level: "nope",
        error_code: "GW_PORT_IN_USE",
        message: "x",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);

    const circuit = handlerFor(gatewayEventNames.circuit);
    circuit?.({ payload: null } as any);
    circuit?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        prev_state: "CLOSED",
        next_state: "OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: 123,
        cooldown_until: null,
        reason: "FAILURE_THRESHOLD_REACHED",
        ts: 0,
      },
    } as any);

    // Non-transition + dedup.
    circuit?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        prev_state: "OPEN",
        next_state: "OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: 123,
        cooldown_until: null,
        reason: "SKIP_OPEN",
        ts: 0,
      },
    } as any);
    circuit?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        prev_state: "OPEN",
        next_state: "OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: 123,
        cooldown_until: null,
        reason: "SKIP_OPEN",
        ts: 0,
      },
    } as any);

    unlisten();
    vi.useRealTimers();
  });

  it("accepts valid trace payloads and drops invalid payloads observably", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(0);
    clearTauriEventListeners();

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { setConsoleLogMinLevel, useConsoleLogs } = await import("../../consoleLog");
    setConsoleLogMinLevel("debug");

    const { useTraceStore } = await import("../traceStore");
    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const traceResult = renderHook(() => useTraceStore()).result;
    const consoleResult = renderHook(() => useConsoleLogs()).result;

    const handlerFor = (eventName: string) =>
      vi
        .mocked(tauriListen)
        .mock.calls.slice()
        .reverse()
        .find((call) => call[0] === eventName)?.[1];

    act(() => {
      handlerFor(gatewayEventNames.requestStart)?.({
        payload: {
          trace_id: "valid-trace",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
          query: null,
          requested_model: "claude-3",
          ts: 0,
        },
      } as any);
    });

    expect(traceResult.current.traces).toHaveLength(1);
    expect(traceResult.current.traces[0]?.trace_id).toBe("valid-trace");

    act(() => {
      handlerFor(gatewayEventNames.requestStart)?.({
        payload: {
          trace_id: "invalid-trace",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
          query: null,
          ts: "bad-ts",
        },
      } as any);
    });

    act(() => {
      handlerFor(gatewayEventNames.requestStart)?.({
        payload: {
          trace_id: "invalid-null-method",
          cli_key: "claude",
          method: null,
          path: "/v1/messages",
          query: null,
          requested_model: "claude-3",
          ts: 0,
        },
      } as any);
    });

    expect(traceResult.current.traces.map((trace) => trace.trace_id)).toEqual(["valid-trace"]);

    act(() => {
      vi.runOnlyPendingTimers();
    });

    expect(consoleResult.current.some((entry) => entry.eventType === "gateway:event_guard")).toBe(
      true
    );

    const guardEntry = consoleResult.current.find(
      (entry) => entry.eventType === "gateway:event_guard"
    );
    expect(guardEntry?.level).toBe("warn");
    expect(guardEntry?.details).toMatchObject({ event: gatewayEventNames.requestStart });

    unlisten();
    vi.useRealTimers();
  });

  it("accepts valid Claude model mapping payloads and drops invalid ones", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(0);
    clearTauriEventListeners();

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { useTraceStore } = await import("../traceStore");
    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const traceResult = renderHook(() => useTraceStore()).result;

    const handlerFor = (eventName: string) =>
      vi
        .mocked(tauriListen)
        .mock.calls.slice()
        .reverse()
        .find((call) => call[0] === eventName)?.[1];

    act(() => {
      handlerFor(gatewayEventNames.attempt)?.({
        payload: {
          trace_id: "mapping-trace",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
          query: null,
          requested_model: "claude-sonnet",
          attempt_index: 1,
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://provider-a.example",
          outcome: "started",
          status: null,
          attempt_started_ms: 0,
          attempt_duration_ms: 0,
          claude_model_mapping: {
            requestedModel: "claude-sonnet",
            effectiveModel: "gpt-5.4",
            mappingKind: "sonnet",
            providerId: 1,
            providerName: "Provider A",
            applied: true,
          },
        },
      } as any);
    });

    expect(traceResult.current.traces[0]?.claude_model_mapping?.effectiveModel).toBe("gpt-5.4");

    act(() => {
      handlerFor(gatewayEventNames.request)?.({
        payload: {
          trace_id: "invalid-mapping-trace",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
          query: null,
          requested_model: "claude-sonnet",
          status: 200,
          error_category: null,
          error_code: null,
          duration_ms: 10,
          ttfb_ms: null,
          attempts: [],
          input_tokens: null,
          output_tokens: null,
          total_tokens: null,
          cache_read_input_tokens: null,
          cache_creation_input_tokens: null,
          cache_creation_5m_input_tokens: null,
          cache_creation_1h_input_tokens: null,
          claude_model_mapping: {
            requestedModel: "claude-sonnet",
            effectiveModel: "gpt-5.4",
            mappingKind: "sonnet",
            providerId: "bad",
            providerName: "Provider A",
            applied: true,
          },
        },
      } as any);
    });

    expect(traceResult.current.traces.map((trace) => trace.trace_id)).toEqual(["mapping-trace"]);

    unlisten();
    vi.useRealTimers();
  });
});
