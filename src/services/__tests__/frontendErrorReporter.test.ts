import { beforeEach, describe, expect, it, vi } from "vitest";

let invokeCalls: unknown[][] = [];
let logToConsoleCalls: unknown[][] = [];
let invokeImpl: (...args: unknown[]) => Promise<unknown> = (...args) => {
  invokeCalls.push(args);
  return Promise.resolve(true);
};

vi.mock("../app/frontendErrorReport", () => ({
  appFrontendErrorReport: ((...args: unknown[]) => {
    return invokeImpl(...args);
  }) as typeof import("../app/frontendErrorReport").appFrontendErrorReport,
}));

vi.mock("../consoleLog", () => ({
  logToConsole: ((...args: unknown[]) => {
    logToConsoleCalls.push(args);
  }) as typeof import("../consoleLog").logToConsole,
}));

// Track event listeners added during tests so we can clean them up
const trackedListeners: { type: string; listener: EventListenerOrEventListenerObject }[] = [];
const origAddEventListener = window.addEventListener.bind(window);
const origRemoveEventListener = window.removeEventListener.bind(window);

describe("services/frontendErrorReporter", () => {
  beforeEach(() => {
    // Remove all tracked listeners from previous tests
    for (const { type, listener } of trackedListeners) {
      origRemoveEventListener(type, listener);
    }
    trackedListeners.length = 0;

    // Intercept addEventListener to track new listeners
    window.addEventListener = ((
      type: string,
      listener: EventListenerOrEventListenerObject,
      ...rest: unknown[]
    ) => {
      trackedListeners.push({ type, listener });
      return origAddEventListener(type, listener, ...(rest as []));
    }) as typeof window.addEventListener;

    vi.resetModules();
    invokeCalls = [];
    logToConsoleCalls = [];
    invokeImpl = (...args) => {
      invokeCalls.push(args);
      return Promise.resolve(true);
    };

    delete (window as any).location;
    Object.defineProperty(window, "location", {
      configurable: true,
      value: {
        href: "http://localhost/#/",
      },
    });

    delete (window as any).navigator;
    Object.defineProperty(window, "navigator", {
      configurable: true,
      value: {
        userAgent: "test-agent",
      },
    });
  });

  it("installs global handlers and reports window error once in dedup window", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    mod.installGlobalErrorReporting();
    mod.installGlobalErrorReporting();

    window.dispatchEvent(new ErrorEvent("error", { message: "boom" }));
    window.dispatchEvent(new ErrorEvent("error", { message: "boom" }));

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(logToConsoleCalls).toHaveLength(1);
    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "error",
        message: "boom",
      }),
    ]);
  });

  it("reports render errors", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    mod.reportRenderError(new Error("render failed"), { componentStack: "at Test" });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "render",
        message: "render failed",
      }),
    ]);
  });

  it("reports unhandled rejection with Error reason", async () => {
    // jsdom lacks PromiseRejectionEvent, polyfill it for this test
    if (typeof globalThis.PromiseRejectionEvent === "undefined") {
      (globalThis as any).PromiseRejectionEvent = class extends Event {
        readonly reason: unknown;
        readonly promise: Promise<unknown>;
        constructor(type: string, init: { reason?: unknown; promise: Promise<unknown> }) {
          super(type);
          this.reason = init.reason;
          this.promise = init.promise;
        }
      };
    }

    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();
    mod.installGlobalErrorReporting();

    const error = new Error("promise failed");
    window.dispatchEvent(
      new PromiseRejectionEvent("unhandledrejection", {
        reason: error,
        promise: Promise.reject(error).catch(() => {}),
      })
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "unhandledrejection",
        message: "promise failed",
      }),
    ]);
    // details_json should contain reason_type and reason
    const payload = invokeCalls[0][0] as Record<string, unknown>;
    const details = JSON.parse(payload.detailsJson as string);
    expect(details.reason_type).toBe("object");
    expect(details.reason).toBe("promise failed");
  });

  it("reports unhandled rejection with non-Error reason", async () => {
    // jsdom lacks PromiseRejectionEvent, polyfill it for this test
    if (typeof globalThis.PromiseRejectionEvent === "undefined") {
      (globalThis as any).PromiseRejectionEvent = class extends Event {
        readonly reason: unknown;
        readonly promise: Promise<unknown>;
        constructor(type: string, init: { reason?: unknown; promise: Promise<unknown> }) {
          super(type);
          this.reason = init.reason;
          this.promise = init.promise;
        }
      };
    }

    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();
    mod.installGlobalErrorReporting();

    window.dispatchEvent(
      new PromiseRejectionEvent("unhandledrejection", {
        reason: "string rejection reason",
        promise: Promise.reject("string rejection reason").catch(() => {}),
      })
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "unhandledrejection",
        message: "string rejection reason",
      }),
    ]);
    const payload = invokeCalls[0][0] as Record<string, unknown>;
    // stack should be null for non-Error reasons
    expect(payload.stack).toBeNull();
    const details = JSON.parse(payload.detailsJson as string);
    expect(details.reason_type).toBe("string");
    expect(details.reason).toBe("string rejection reason");
  });

  it("dedup overflow clears map and re-adds", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    // Send 201 unique errors to trigger the overflow (MAX_DEDUP_KEYS = 200)
    for (let i = 0; i < 201; i++) {
      mod.reportRenderError(new Error(`unique-error-${i}`));
    }
    await new Promise((resolve) => setTimeout(resolve, 0));

    // All 201 should be sent (each is unique, so none deduped)
    expect(invokeCalls).toHaveLength(201);

    // Now re-send the first error; since the map was cleared at overflow,
    // error #0 is no longer in the dedup map, so it should be sent again.
    invokeCalls = [];
    mod.reportRenderError(new Error("unique-error-0"));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "render",
        message: "unique-error-0",
      }),
    ]);
  });

  it("swallows error when frontend error report service rejects", async () => {
    invokeImpl = (...args) => {
      invokeCalls.push(args);
      return Promise.reject(new Error("invoke failed"));
    };

    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    // This should not throw even though the service rejects
    mod.reportRenderError(new Error("some error"));
    await new Promise((resolve) => setTimeout(resolve, 0));

    // The invoke was attempted
    expect(invokeCalls).toHaveLength(1);
    // logToConsole was called before the invoke
    expect(logToConsoleCalls).toHaveLength(1);
  });

  it("normalizeMessage returns fallback for empty value", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    mod.reportRenderError("");
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "render",
        message: "Unknown frontend error",
      }),
    ]);
  });

  it("safeToString handles non-string non-Error values", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    // Pass a number -- not a string and not an Error, exercises String(value) path
    mod.reportRenderError(42);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "render",
        message: "42",
      }),
    ]);
  });

  it("safeToString returns fallback when String() throws", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    // Create an object whose toString throws
    const badObj = {
      toString() {
        throw new Error("cannot stringify");
      },
    };

    mod.reportRenderError(badObj);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    // safeToString catches the throw and returns "[unstringifiable]"
    expect(invokeCalls[0]).toEqual([
      expect.objectContaining({
        source: "render",
        message: "[unstringifiable]",
      }),
    ]);
  });

  it("normalizeStack returns null for non-string and empty string", async () => {
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    // reportRenderError with a non-Error value triggers normalizeStack(null)
    mod.reportRenderError("some string error");
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    const payload = invokeCalls[0][0] as Record<string, unknown>;
    // stack should be null because "some string error" is not an Error (no .stack)
    expect(payload.stack).toBeNull();
  });

  it("buildSharedMeta populates href and user_agent from globals", async () => {
    // location and navigator are set in beforeEach
    const mod = await import("../frontendErrorReporter");
    mod.__testResetFrontendErrorReporterState();

    mod.reportRenderError(new Error("meta test"));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(invokeCalls).toHaveLength(1);
    const payload = invokeCalls[0][0] as Record<string, unknown>;
    expect(payload.href).toBe("http://localhost/#/");
    expect(payload.userAgent).toBe("test-agent");
  });
});
