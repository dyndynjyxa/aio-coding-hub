import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

async function importFreshConsoleLog() {
  vi.resetModules();
  return await import("../consoleLog");
}

describe("services/consoleLog", () => {
  it("setConsoleLogMinLevel + shouldLogToConsole", async () => {
    const { setConsoleLogMinLevel, shouldLogToConsole, getConsoleDebugEnabled } =
      await importFreshConsoleLog();

    setConsoleLogMinLevel("info");
    expect(shouldLogToConsole("debug")).toBe(false);
    expect(shouldLogToConsole("info")).toBe(true);
    expect(getConsoleDebugEnabled()).toBe(false);

    setConsoleLogMinLevel("debug");
    expect(shouldLogToConsole("debug")).toBe(true);
    expect(getConsoleDebugEnabled()).toBe(true);
  });

  it("logToConsole redacts sensitive keys and extracts meta", async () => {
    const { clearConsoleLogs, logToConsole, setConsoleLogMinLevel, useConsoleLogs } =
      await importFreshConsoleLog();

    clearConsoleLogs();
    setConsoleLogMinLevel("debug");

    const { result } = renderHook(() => useConsoleLogs());
    expect(result.current).toEqual([]);

    act(() => {
      logToConsole("info", "hello", {
        trace_id: "t-1",
        cli_key: "claude",
        api_key: "SECRET",
        base_url: "https://example.com/private",
        baseUrl: "https://example.com/private2",
        baseOrigin: "http://127.0.0.1:37123",
        authorization: "Bearer SECRET2",
        attempts: [{ provider_name: "P1" }, { providerName: "P2" }, { provider_name: "P1" }],
      });
    });

    await waitFor(() => {
      expect(result.current.length).toBe(1);
    });

    const entry = result.current[0];
    expect(entry.title).toBe("hello");
    expect(entry.details).toEqual(
      expect.objectContaining({
        trace_id: "t-1",
        cli_key: "claude",
        api_key: "[REDACTED]",
        base_url: "[REDACTED]",
        baseUrl: "[REDACTED]",
        baseOrigin: "[REDACTED]",
        authorization: "[REDACTED]",
      })
    );
    expect(entry.meta).toEqual(
      expect.objectContaining({
        trace_id: "t-1",
        cli_key: "claude",
        providers: expect.arrayContaining(["P1", "P2"]),
      })
    );
  });

  it("filters logs below min level and sanitizes circular/deep metadata branches", async () => {
    const { clearConsoleLogs, logToConsole, setConsoleLogMinLevel, useConsoleLogs } =
      await importFreshConsoleLog();

    clearConsoleLogs();
    setConsoleLogMinLevel("error");

    const { result } = renderHook(() => useConsoleLogs());
    const details: Record<string, unknown> & { self?: unknown } = {
      traceId: " trace-1 ",
      cli: " codex ",
      errorCode: " GW_TEST ",
      providers: Array.from({ length: 14 }, (_, index) => `P${index}`),
      attempts: [null, { providerName: "P-extra" }, { provider_name: "" }],
      token: "SECRET",
      deep: { a: { b: { c: { d: { e: { f: { g: "too deep" } } } } } } },
    };
    details.self = details;

    act(() => {
      logToConsole("info", "ignored", details);
    });
    await Promise.resolve();
    expect(result.current).toEqual([]);

    act(() => {
      logToConsole("error", "stored", details);
    });

    await waitFor(() => {
      expect(result.current.length).toBe(1);
    });

    const entry = result.current[0];
    expect(entry.details).toEqual(
      expect.objectContaining({
        token: "[REDACTED]",
        self: "[Circular]",
      })
    );
    expect(entry.meta).toEqual(
      expect.objectContaining({
        trace_id: "trace-1",
        cli_key: "codex",
        error_code: "GW_TEST",
        providers: expect.arrayContaining(["P-extra", "P0", "P10"]),
      })
    );
    expect(entry.meta?.providers).toHaveLength(12);
    expect(JSON.stringify(entry.details)).toContain("[Truncated]");
  });

  it("bounds retained detail strings, arrays, and object keys", async () => {
    const { clearConsoleLogs, logToConsole, setConsoleLogMinLevel, useConsoleLogs } =
      await importFreshConsoleLog();

    clearConsoleLogs();
    setConsoleLogMinLevel("debug");

    const hugeObject = Object.fromEntries(
      Array.from({ length: 120 }, (_, index) => [`key_${index}`, index])
    );
    const { result } = renderHook(() => useConsoleLogs());

    act(() => {
      logToConsole("info", "bounded", {
        hugeString: "x".repeat(5000),
        hugeArray: Array.from({ length: 120 }, (_, index) => index),
        hugeObject,
      });
    });

    await waitFor(() => {
      expect(result.current.length).toBe(1);
    });

    const details = result.current[0].details as {
      hugeString: string;
      hugeArray: unknown[];
      hugeObject: Record<string, unknown>;
    };

    expect(details.hugeString.length).toBeLessThan(5000);
    expect(details.hugeString).toContain("[Truncated]");
    expect(details.hugeArray).toHaveLength(101);
    expect(details.hugeArray[100]).toBe("[Truncated 20 items]");
    expect(Object.keys(details.hugeObject)).toHaveLength(101);
    expect(details.hugeObject.__truncated_keys).toBe("[Truncated]");
  });

  it("uses requestAnimationFrame when scheduling log updates", async () => {
    const originalRequestAnimationFrame = window.requestAnimationFrame;
    const requestAnimationFrameMock = vi.fn((callback: FrameRequestCallback) => {
      callback(1);
      return 1;
    });
    window.requestAnimationFrame = requestAnimationFrameMock;

    try {
      const { clearConsoleLogs, logToConsole, setConsoleLogMinLevel, useConsoleLogs } =
        await importFreshConsoleLog();

      clearConsoleLogs();
      setConsoleLogMinLevel("debug");
      const { result } = renderHook(() => useConsoleLogs());

      act(() => {
        logToConsole("warn", "raf");
      });

      await waitFor(() => expect(result.current).toHaveLength(1));
      expect(requestAnimationFrameMock).toHaveBeenCalled();
    } finally {
      window.requestAnimationFrame = originalRequestAnimationFrame;
    }
  });

  it("isolates console log subscribers when one throws", async () => {
    const consoleWarnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { logToConsole, setConsoleLogMinLevel, subscribeConsoleLogs } =
      await importFreshConsoleLog();

    setConsoleLogMinLevel("debug");
    const throwingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();
    const unsubscribeThrowing = subscribeConsoleLogs(throwingListener);
    const unsubscribeHealthy = subscribeConsoleLogs(healthyListener);

    try {
      logToConsole("info", "subscriber isolation");

      await waitFor(() => {
        expect(throwingListener).toHaveBeenCalledTimes(1);
        expect(healthyListener).toHaveBeenCalledTimes(1);
      });
      expect(consoleWarnSpy).toHaveBeenCalledWith(
        "Console log subscriber failed",
        expect.any(Error)
      );
    } finally {
      unsubscribeThrowing();
      unsubscribeHealthy();
      consoleWarnSpy.mockRestore();
    }
  });
});
