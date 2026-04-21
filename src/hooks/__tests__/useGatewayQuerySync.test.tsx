import { render } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import { useGatewayQuerySync } from "../useGatewayQuerySync";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { tauriListen, tauriUnlisten } from "../../test/mocks/tauri";
import { gatewayKeys, usageKeys } from "../../query/keys";

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
    requestHandler({ payload: { phase: "start" } });
    vi.advanceTimersByTime(1000);
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: usageKeys.all });

    requestHandler({ payload: { phase: "complete" } });
    requestHandler({ payload: { phase: "complete" } });
    vi.advanceTimersByTime(1000);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: usageKeys.all });

    unmount();
    expect(tauriUnlisten).toHaveBeenCalled();

    vi.useRealTimers();
  });
});
