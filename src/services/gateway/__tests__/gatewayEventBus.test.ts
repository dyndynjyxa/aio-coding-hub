import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { clearTauriEventListeners, tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";
import { logToConsole } from "../../consoleLog";

vi.mock("../../consoleLog", () => ({ logToConsole: vi.fn() }));

function rejectedThenable(message: string): PromiseLike<void> {
  return {
    then<TResult1 = void, TResult2 = never>(
      onfulfilled?: ((value: void) => TResult1 | PromiseLike<TResult1>) | null,
      onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null
    ): PromiseLike<TResult1 | TResult2> {
      return Promise.reject(new Error(message)).then(onfulfilled, onrejected);
    },
  };
}

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

  it("does not let a pending disposed entry tear down a newer subscription", async () => {
    let resolveFirstListen!: (unlisten: typeof tauriUnlisten) => void;
    const firstUnlisten = vi.fn();
    const secondUnlisten = vi.fn();
    const delayedListen = new Promise<typeof tauriUnlisten>((resolve) => {
      resolveFirstListen = resolve;
    });

    vi.mocked(tauriListen).mockReturnValueOnce(delayedListen).mockResolvedValueOnce(secondUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const firstHandler = vi.fn();
    const secondHandler = vi.fn();

    const first = subscribeGatewayEvent(gatewayEventNames.request, firstHandler);
    first.unsubscribe();

    const second = subscribeGatewayEvent(gatewayEventNames.request, secondHandler);
    await second.ready;

    resolveFirstListen(firstUnlisten);
    await first.ready;

    const secondCallback = vi.mocked(tauriListen).mock.calls[1]?.[1];
    secondCallback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-new" } });

    expect(tauriListen).toHaveBeenCalledTimes(2);
    expect(firstUnlisten).toHaveBeenCalledTimes(1);
    expect(secondUnlisten).not.toHaveBeenCalled();
    expect(firstHandler).not.toHaveBeenCalled();
    expect(secondHandler).toHaveBeenCalledWith({ trace_id: "t-new" });

    second.unsubscribe();
    expect(secondUnlisten).toHaveBeenCalledTimes(1);
  });

  it("isolates handler failures during shared dispatch", async () => {
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const brokenHandler = vi.fn(() => {
      throw new Error("handler boom");
    });
    const healthyHandler = vi.fn();

    const first = subscribeGatewayEvent(gatewayEventNames.request, brokenHandler);
    const second = subscribeGatewayEvent(gatewayEventNames.request, healthyHandler);
    await Promise.all([first.ready, second.ready]);

    const callback = vi.mocked(tauriListen).mock.calls[0]?.[1];
    expect(() =>
      callback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-3" } })
    ).not.toThrow();

    expect(brokenHandler).toHaveBeenCalledWith({ trace_id: "t-3" });
    expect(healthyHandler).toHaveBeenCalledWith({ trace_id: "t-3" });
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      "网关事件处理失败",
      { event: gatewayEventNames.request, error: "Error: handler boom" },
      "gateway:event_bus"
    );

    first.unsubscribe();
    second.unsubscribe();
  });

  it("logs async handler rejections without breaking shared dispatch", async () => {
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const brokenHandler = vi.fn().mockRejectedValue(new Error("async handler boom"));
    const healthyHandler = vi.fn();

    const first = subscribeGatewayEvent(gatewayEventNames.request, brokenHandler);
    const second = subscribeGatewayEvent(gatewayEventNames.request, healthyHandler);
    await Promise.all([first.ready, second.ready]);

    const callback = vi.mocked(tauriListen).mock.calls[0]?.[1];
    expect(() =>
      callback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-4" } })
    ).not.toThrow();

    await vi.waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "网关事件处理失败",
        { event: gatewayEventNames.request, error: "Error: async handler boom" },
        "gateway:event_bus"
      );
    });
    expect(healthyHandler).toHaveBeenCalledWith({ trace_id: "t-4" });

    first.unsubscribe();
    second.unsubscribe();
  });

  it("logs thenable handler rejections without leaking outside dispatch", async () => {
    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);

    const { subscribeGatewayEvent } = await import("../gatewayEventBus");
    const brokenHandler = vi.fn(() => rejectedThenable("thenable handler boom"));
    const healthyHandler = vi.fn();

    const first = subscribeGatewayEvent(gatewayEventNames.request, brokenHandler);
    const second = subscribeGatewayEvent(gatewayEventNames.request, healthyHandler);
    await Promise.all([first.ready, second.ready]);

    const callback = vi.mocked(tauriListen).mock.calls[0]?.[1];
    expect(() =>
      callback?.({ event: gatewayEventNames.request, payload: { trace_id: "t-5" } })
    ).not.toThrow();

    await vi.waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "网关事件处理失败",
        { event: gatewayEventNames.request, error: "Error: thenable handler boom" },
        "gateway:event_bus"
      );
    });
    expect(healthyHandler).toHaveBeenCalledWith({ trace_id: "t-5" });

    first.unsubscribe();
    second.unsubscribe();
  });
});
