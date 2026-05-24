import { render } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import { useGatewayQuerySync } from "../useGatewayQuerySync";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { tauriListen, tauriUnlisten } from "../../test/mocks/tauri";
import { gatewayKeys, providerLimitUsageKeys, usageKeys } from "../../query/keys";

function Harness() {
  useGatewayQuerySync();
  return null;
}

describe("hooks/useGatewayQuerySync", () => {
  it("throttles invalidations for gateway events and cleans up listeners", async () => {
    vi.useFakeTimers();
    setTauriRuntime();

    const handlers = new Map<string, (event: any) => void>();
    vi.mocked(tauriListen).mockImplementation(async (event: string, handler: any) => {
      handlers.set(event, handler);
      return tauriUnlisten;
    });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    const { unmount } = render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    // wait for dynamic import("@tauri-apps/api/event") + listen registrations
    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(handlers.has(gatewayEventNames.circuit)).toBe(true);
    expect(handlers.has(gatewayEventNames.status)).toBe(true);
    expect(handlers.has(gatewayEventNames.requestSignal)).toBe(true);
    expect(handlers.has(gatewayEventNames.requestStart)).toBe(false);
    expect(handlers.has(gatewayEventNames.attempt)).toBe(false);
    expect(handlers.has(gatewayEventNames.request)).toBe(false);

    // Circuit invalidation throttled at 500ms.
    const circuitHandler = handlers.get(gatewayEventNames.circuit)!;
    circuitHandler({ payload: null });
    circuitHandler({ payload: null }); // should be ignored while timer is set
    vi.advanceTimersByTime(499);
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: gatewayKeys.circuits() });
    vi.advanceTimersByTime(1);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuits() });

    // Status invalidation throttled at 300ms.
    const statusHandler = handlers.get(gatewayEventNames.status)!;
    statusHandler({ payload: null });
    statusHandler({ payload: null });
    vi.advanceTimersByTime(300);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.status() });

    // Usage invalidation only reacts to request completion signals.
    const requestHandler = handlers.get(gatewayEventNames.requestSignal)!;
    requestHandler({
      payload: {
        phase: "start",
        trace_id: "t-1",
        cli_key: "claude",
        session_id: null,
        requested_model: null,
        ts: 1,
      },
    });
    vi.advanceTimersByTime(1000);
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: usageKeys.all });

    requestHandler({
      payload: {
        phase: "complete",
        trace_id: "t-1",
        cli_key: "claude",
        session_id: null,
        requested_model: "m".repeat(5000),
        ts: 2,
      },
    });
    requestHandler({
      payload: {
        phase: "complete",
        trace_id: "t-1",
        cli_key: "claude",
        session_id: null,
        requested_model: null,
        ts: 2,
      },
    });
    vi.advanceTimersByTime(1000);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: usageKeys.all });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: providerLimitUsageKeys.all });

    unmount();
    expect(tauriUnlisten).toHaveBeenCalled();

    vi.useRealTimers();
  });

  it("drops queued invalidations on unmount", async () => {
    vi.useFakeTimers();
    setTauriRuntime();

    const handlers = new Map<string, (event: any) => void>();
    vi.mocked(tauriListen).mockImplementation(async (event: string, handler: any) => {
      handlers.set(event, handler);
      return tauriUnlisten;
    });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    const { unmount } = render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    handlers.get(gatewayEventNames.circuit)?.({ payload: null });
    unmount();

    vi.advanceTimersByTime(500);
    expect(invalidateSpy).not.toHaveBeenCalled();

    vi.useRealTimers();
  });

  it("coalesces request-signal usage invalidations while a prior invalidation is in flight", async () => {
    vi.useFakeTimers();
    setTauriRuntime();

    const handlers = new Map<string, (event: any) => void>();
    vi.mocked(tauriListen).mockImplementation(async (event: string, handler: any) => {
      handlers.set(event, handler);
      return tauriUnlisten;
    });

    const client = createTestQueryClient();
    const usageResolvers: Array<() => void> = [];
    const invalidateSpy = vi.spyOn(client, "invalidateQueries").mockImplementation((filters) => {
      if (filters?.queryKey === usageKeys.all) {
        return new Promise<void>((resolve) => {
          usageResolvers.push(resolve);
        }) as ReturnType<typeof client.invalidateQueries>;
      }
      return Promise.resolve() as ReturnType<typeof client.invalidateQueries>;
    });

    render(
      <QueryClientProvider client={client}>
        <Harness />
      </QueryClientProvider>
    );

    await vi.runAllTimersAsync();
    await Promise.resolve();

    const requestHandler = handlers.get(gatewayEventNames.requestSignal)!;
    const emitComplete = (traceId: string) =>
      requestHandler({
        payload: {
          phase: "complete",
          trace_id: traceId,
          cli_key: "codex",
          session_id: null,
          requested_model: null,
          ts: 1,
        },
      });
    const countInvalidationsFor = (queryKey: readonly unknown[]) =>
      invalidateSpy.mock.calls.filter(([filters]) => filters?.queryKey === queryKey).length;

    emitComplete("t-1");
    await vi.advanceTimersByTimeAsync(1000);
    await Promise.resolve();

    expect(countInvalidationsFor(usageKeys.all)).toBe(1);
    expect(countInvalidationsFor(providerLimitUsageKeys.all)).toBe(1);

    emitComplete("t-2");
    emitComplete("t-3");
    emitComplete("t-4");
    await vi.advanceTimersByTimeAsync(1000);
    await Promise.resolve();

    expect(countInvalidationsFor(usageKeys.all)).toBe(1);
    expect(countInvalidationsFor(providerLimitUsageKeys.all)).toBe(1);

    usageResolvers.shift()?.();
    for (let i = 0; i < 5; i += 1) {
      await Promise.resolve();
    }

    expect(countInvalidationsFor(usageKeys.all)).toBe(2);
    expect(countInvalidationsFor(providerLimitUsageKeys.all)).toBe(2);

    vi.useRealTimers();
  });
});
