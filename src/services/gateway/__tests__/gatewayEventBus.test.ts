import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { clearTauriEventListeners, tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";

describe("services/gateway/gatewayEventBus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
    clearTauriEventListeners();
  });

  it("retries listen setup after an initialization failure", async () => {
    vi.mocked(tauriListen)
      .mockRejectedValueOnce(new Error("listen boom"))
      .mockResolvedValue(tauriUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const handler = vi.fn();

    const first = subscribeGatewayEvent(gatewayEventNames.request, handler);
    await expect(first.ready).rejects.toThrow("listen boom");
    first.unsubscribe();

    const second = subscribeGatewayEvent(gatewayEventNames.request, handler);
    await second.ready;
    const callback =
      vi.mocked(tauriListen).mock.calls[vi.mocked(tauriListen).mock.calls.length - 1]?.[1];
    callback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-1" } });

    expect(tauriListen).toHaveBeenCalledTimes(2);
    expect(handler).toHaveBeenCalledWith({ trace_id: "t-1" });

    second.unsubscribe();
  });

  it("drops pending disposed entries so a later subscription can reinitialize", async () => {
    let resolveListen!: (unlisten: typeof tauriUnlisten) => void;
    const delayedListen = new Promise<typeof tauriUnlisten>((resolve) => {
      resolveListen = resolve;
    });

    vi.mocked(tauriListen).mockReturnValueOnce(delayedListen).mockResolvedValueOnce(tauriUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const handler = vi.fn();

    const pending = subscribeGatewayEvent(gatewayEventNames.request, handler);
    pending.unsubscribe();

    resolveListen(tauriUnlisten);
    await pending.ready;

    const next = subscribeGatewayEvent(gatewayEventNames.request, handler);
    await next.ready;
    const callback =
      vi.mocked(tauriListen).mock.calls[vi.mocked(tauriListen).mock.calls.length - 1]?.[1];
    callback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-2" } });

    expect(tauriListen).toHaveBeenCalledTimes(2);
    expect(handler).toHaveBeenCalledWith({ trace_id: "t-2" });

    next.unsubscribe();
  });
});
