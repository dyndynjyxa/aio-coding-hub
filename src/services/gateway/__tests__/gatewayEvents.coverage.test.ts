import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

const logToConsole = vi.fn();
const shouldLogToConsole = vi.fn();

vi.mock("../../consoleLog", () => ({
  logToConsole,
  shouldLogToConsole,
}));

vi.mock("../traceStore", () => ({
  ingestTraceAttempt: vi.fn(),
  ingestTraceRequest: vi.fn(),
  ingestTraceStart: vi.fn(),
}));

vi.mock("../cacheAnomalyMonitor", () => ({
  ingestCacheAnomalyRequest: vi.fn(),
  ingestCacheAnomalyRequestStart: vi.fn(),
}));

describe("services/gatewayEvents (coverage)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("skips debug logging when console min-level is above debug", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.useFakeTimers();
    vi.setSystemTime(0);

    shouldLogToConsole.mockReturnValue(false);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const handlerFor = (eventName: string) =>
      vi.mocked(tauriListen).mock.calls.find((call) => call[0] === eventName)?.[1];

    handlerFor("gateway:request_start")?.({
      payload: { trace_id: "t1", cli_key: "claude", method: null, path: null, query: null, ts: 0 },
    } as any);

    expect(logToConsole).not.toHaveBeenCalled();

    unlisten();
    vi.useRealTimers();
  });

  it("covers log/circuit/request_start/request branches (normalization + fallbacks)", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.useFakeTimers();
    vi.setSystemTime(0);

    shouldLogToConsole.mockReturnValue(true);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const handlerFor = (eventName: string) =>
      vi.mocked(tauriListen).mock.calls.find((call) => call[0] === eventName)?.[1];

    // request_start: method/path fallbacks
    handlerFor("gateway:request_start")?.({
      payload: {
        trace_id: "t-start",
        cli_key: "claude",
        method: null,
        path: null,
        query: null,
        ts: 0,
      },
    } as any);

    // attempt: outcome success branch + provider/method/path/status fallbacks
    handlerFor("gateway:attempt")?.({
      payload: {
        trace_id: "t-attempt",
        cli_key: "claude",
        method: null,
        path: null,
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "",
        base_url: "https://p1",
        outcome: "success",
        status: null,
        attempt_started_ms: 0,
        attempt_duration_ms: 12,
        circuit_state_before: "CLOSED",
        circuit_state_after: "CLOSED",
        circuit_failure_count: null,
        circuit_failure_threshold: null,
      },
    } as any);

    // request: attempts fallback + output_tokens null path in computeOutputTokensPerSecond
    handlerFor("gateway:request")?.({
      payload: {
        trace_id: "t-request",
        cli_key: "claude",
        method: null,
        path: null,
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 10,
        ttfb_ms: 1,
        attempts: undefined,
        output_tokens: null,
      },
    } as any);

    // log: normalize log level (warn) + non-port-in-use title branch
    handlerFor("gateway:log")?.({
      payload: {
        level: "warn",
        error_code: "GW_SOMETHING",
        message: "x",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);

    // circuit: reason fallbacks + additional reason branches
    handlerFor("gateway:circuit")?.({
      payload: {
        trace_id: "t-c1",
        cli_key: "claude",
        provider_id: 1,
        provider_name: "",
        base_url: "https://p1",
        prev_state: "CLOSED",
        next_state: "OPEN",
        failure_count: 1,
        failure_threshold: 5,
        open_until: 123,
        cooldown_until: null,
        reason: "OPEN_EXPIRED",
        ts: 0,
      },
    } as any);

    handlerFor("gateway:circuit")?.({
      payload: {
        trace_id: "t-c2",
        cli_key: "claude",
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        prev_state: "BAD",
        next_state: null,
        failure_count: 1,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
        reason: "   ",
        ts: 0,
      },
    } as any);

    handlerFor("gateway:circuit")?.({
      payload: {
        trace_id: "t-c3",
        cli_key: "claude",
        provider_id: 2,
        provider_name: "P2",
        base_url: "https://p2",
        prev_state: "OPEN",
        next_state: "OPEN",
        failure_count: 1,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
        reason: "SKIP_COOLDOWN",
        ts: 0,
      },
    } as any);

    handlerFor("gateway:circuit")?.({
      payload: {
        trace_id: "t-c4",
        cli_key: "claude",
        provider_id: 3,
        provider_name: "P3",
        base_url: "https://p3",
        prev_state: "OPEN",
        next_state: "OPEN",
        failure_count: 1,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
        reason: "SOME_REASON",
        ts: 0,
      },
    } as any);

    expect(logToConsole).toHaveBeenCalled();

    unlisten();
    vi.useRealTimers();
  });

  it("forwards gateway log events according to console min-level", async () => {
    setTauriRuntime();
    vi.resetModules();

    shouldLogToConsole.mockImplementation((level: string) => level !== "debug");
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const logHandler = vi
      .mocked(tauriListen)
      .mock.calls.find((call) => call[0] === "gateway:log")?.[1];

    logHandler?.({
      payload: {
        level: "info",
        error_code: "GW_INFO",
        message: "info",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);
    logHandler?.({
      payload: {
        level: "debug",
        error_code: "GW_DEBUG",
        message: "debug",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);
    logHandler?.({
      payload: {
        level: "warn",
        error_code: "GW_WARN",
        message: "warn",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);
    logHandler?.({
      payload: {
        level: "error",
        error_code: "GW_ERROR",
        message: "error",
        requested_port: 1,
        bound_port: 2,
        base_url: "http://x",
      },
    } as any);

    const gatewayLogCalls = logToConsole.mock.calls.filter((call) => call[3] === "gateway:log");
    expect(gatewayLogCalls).toHaveLength(3);
    expect(gatewayLogCalls.map((call) => call[0])).toEqual(["info", "warn", "error"]);

    unlisten();
  });

  it("covers request output tokens/sec edge cases", async () => {
    setTauriRuntime();
    vi.resetModules();

    shouldLogToConsole.mockReturnValue(true);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const request = vi
      .mocked(tauriListen)
      .mock.calls.find((call) => call[0] === "gateway:request")?.[1];

    request?.({
      payload: {
        trace_id: "t1",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 0,
        ttfb_ms: 1,
        attempts: [],
        output_tokens: 10,
      },
    } as any);

    request?.({
      payload: {
        trace_id: "t2",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 10,
        ttfb_ms: null,
        attempts: [],
        output_tokens: 10,
      },
    } as any);

    request?.({
      payload: {
        trace_id: "t3",
        cli_key: "claude",
        method: "GET",
        path: "/v1/test",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 10,
        ttfb_ms: 10,
        attempts: [],
        output_tokens: 10,
      },
    } as any);

    const payloads = logToConsole.mock.calls
      .filter((call) => call[0] === "debug" && String(call[1]).includes("网关请求"))
      .map((call) => call[2] as any);

    expect(payloads.some((p) => p?.trace_id === "t1" && p?.output_tokens_per_second === null)).toBe(
      true
    );
    expect(payloads.some((p) => p?.trace_id === "t2" && p?.output_tokens_per_second === null)).toBe(
      true
    );
    expect(payloads.some((p) => p?.trace_id === "t3" && p?.output_tokens_per_second != null)).toBe(
      true
    );

    unlisten();
  });

  it("skips logging for high-frequency started attempt events", async () => {
    setTauriRuntime();
    vi.resetModules();

    shouldLogToConsole.mockReturnValue(true);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const attempt = vi
      .mocked(tauriListen)
      .mock.calls.find((call) => call[0] === "gateway:attempt")?.[1];

    attempt?.({
      payload: {
        trace_id: "t-started",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        attempt_index: 1,
        provider_id: 1,
        provider_name: "P1",
        base_url: "https://p1",
        outcome: "started",
        status: null,
        attempt_started_ms: 1,
        attempt_duration_ms: 0,
      },
    } as any);

    const attemptDebugLogs = logToConsole.mock.calls.filter((call) =>
      String(call[1]).includes("故障切换尝试")
    );
    expect(attemptDebugLogs).toHaveLength(0);

    unlisten();
  });

  it("deduplicates non-transition circuit logs inside window", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.useFakeTimers();
    vi.setSystemTime(0);

    shouldLogToConsole.mockReturnValue(true);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const circuit = vi
      .mocked(tauriListen)
      .mock.calls.find((call) => call[0] === "gateway:circuit")?.[1];

    const payload = {
      trace_id: "t-circuit",
      cli_key: "claude",
      provider_id: 7,
      provider_name: "P7",
      base_url: "https://p7",
      prev_state: "OPEN",
      next_state: "OPEN",
      failure_count: 1,
      failure_threshold: 5,
      open_until: null,
      cooldown_until: null,
      reason: "SKIP_OPEN",
      ts: 0,
    };

    circuit?.({ payload } as any);
    circuit?.({ payload } as any);

    let skippedLogs = logToConsole.mock.calls.filter(
      (call) => call[0] === "debug" && String(call[1]).includes("Provider 跳过")
    );
    expect(skippedLogs).toHaveLength(1);

    vi.setSystemTime(3001);
    circuit?.({ payload } as any);

    skippedLogs = logToConsole.mock.calls.filter(
      (call) => call[0] === "debug" && String(call[1]).includes("Provider 跳过")
    );
    expect(skippedLogs).toHaveLength(2);

    unlisten();
    vi.useRealTimers();
  });

  it("clears circuit dedup map when it grows too large", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.useFakeTimers();
    vi.setSystemTime(0);

    shouldLogToConsole.mockReturnValue(true);
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { listenGatewayEvents } = await import("../gatewayEvents");
    const unlisten = await listenGatewayEvents();

    const circuit = vi
      .mocked(tauriListen)
      .mock.calls.find((call) => call[0] === "gateway:circuit")?.[1];

    for (let i = 0; i < 501; i += 1) {
      circuit?.({
        payload: {
          trace_id: `t-${i}`,
          cli_key: "claude",
          provider_id: i,
          provider_name: "P",
          base_url: "https://p",
          prev_state: "OPEN",
          next_state: "OPEN",
          failure_count: 1,
          failure_threshold: 5,
          open_until: null,
          cooldown_until: null,
          reason: "SKIP_OPEN",
          ts: 0,
        },
      } as any);
    }

    expect(logToConsole).toHaveBeenCalled();

    unlisten();
    vi.useRealTimers();
  });
});
