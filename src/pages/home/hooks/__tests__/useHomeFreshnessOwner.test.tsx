import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../../../constants/gatewayEvents";
import { useWindowForeground } from "../../../../hooks/useWindowForeground";
import { logToConsole } from "../../../../services/consoleLog";
import { subscribeGatewayEvent } from "../../../../services/gateway/gatewayEventBus";
import { useHomeFreshnessOwner } from "../useHomeFreshnessOwner";

vi.mock("../../../../hooks/useWindowForeground", () => ({
  useWindowForeground: vi.fn(),
}));

vi.mock("../../../../services/gateway/gatewayEventBus", () => ({
  subscribeGatewayEvent: vi.fn(),
}));

vi.mock("../../../../services/consoleLog", () => ({
  logToConsole: vi.fn(),
}));

describe("pages/home/hooks/useHomeFreshnessOwner", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
    vi.mocked(subscribeGatewayEvent).mockReturnValue({
      ready: Promise.resolve(),
      unsubscribe: vi.fn(),
    });
  });

  it("coalesces duplicate complete signals into one request logs refresh", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);
    let eventHandler:
      | ((payload: {
          trace_id: string;
          cli_key: string;
          phase: "start" | "complete";
          ts: number;
        }) => void)
      | null = null;

    vi.mocked(subscribeGatewayEvent).mockImplementation((event: string, handler: any) => {
      expect(event).toBe(gatewayEventNames.requestSignal);
      eventHandler = handler;
      return {
        ready: Promise.resolve(),
        unsubscribe: vi.fn(),
      };
    });

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: true,
        requestLogsRefreshWindowMs: 1000,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    expect(eventHandler).not.toBeNull();

    act(() => {
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "start", ts: 1 });
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "complete", ts: 2 });
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "complete", ts: 2 });
    });

    expect(refreshRequestLogs).not.toHaveBeenCalled();

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("waits for foreground-active state before refreshing after foreground events", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);
    let foregroundArgs: { onForeground: () => void } | null = null;

    vi.mocked(useWindowForeground).mockImplementation((args: any) => {
      foregroundArgs = args;
    });

    const view = renderHook(
      (props: { overviewActive: boolean; foregroundActive: boolean }) =>
        useHomeFreshnessOwner({
          ...props,
          requestLogsRefreshWindowMs: 400,
          onRefreshRequestLogs: refreshRequestLogs,
        }),
      {
        initialProps: {
          overviewActive: true,
          foregroundActive: false,
        },
      }
    );

    act(() => {
      foregroundArgs?.onForeground();
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).not.toHaveBeenCalled();

    view.rerender({
      overviewActive: true,
      foregroundActive: true,
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);

    act(() => {
      foregroundArgs?.onForeground();
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });

  it("drops queued request log refresh when overview leaves foreground", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);
    let eventHandler:
      | ((payload: { trace_id: string; cli_key: string; phase: "complete"; ts: number }) => void)
      | null = null;

    vi.mocked(subscribeGatewayEvent).mockImplementation((event: string, handler: any) => {
      expect(event).toBe(gatewayEventNames.requestSignal);
      eventHandler = handler;
      return {
        ready: Promise.resolve(),
        unsubscribe: vi.fn(),
      };
    });

    const view = renderHook(
      (props: { overviewActive: boolean; foregroundActive: boolean }) =>
        useHomeFreshnessOwner({
          ...props,
          requestLogsRefreshWindowMs: 400,
          onRefreshRequestLogs: refreshRequestLogs,
        }),
      {
        initialProps: {
          overviewActive: true,
          foregroundActive: true,
        },
      }
    );

    act(() => {
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "complete", ts: 2 });
    });

    view.rerender({
      overviewActive: true,
      foregroundActive: false,
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("queues manual refresh while another refresh is in-flight", async () => {
    let resolveRefresh: ((value: unknown) => void) | null = null;
    const refreshRequestLogs = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveRefresh = resolve;
          })
      )
      .mockResolvedValueOnce(null);

    const { result } = renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: true,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    void result.current.refreshRequestLogsNow();
    await result.current.refreshRequestLogsNow();

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveRefresh?.(null);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(2);
  });

  it("logs and resolves failed automatic refreshes without leaking unhandled rejections", async () => {
    vi.useFakeTimers();
    const refreshError = new Error("refresh boom");
    const refreshRequestLogs = vi.fn().mockRejectedValue(refreshError);
    let eventHandler:
      | ((payload: { trace_id: string; cli_key: string; phase: "complete"; ts: number }) => void)
      | null = null;

    vi.mocked(subscribeGatewayEvent).mockImplementation((event: string, handler: any) => {
      expect(event).toBe(gatewayEventNames.requestSignal);
      eventHandler = handler;
      return {
        ready: Promise.resolve(),
        unsubscribe: vi.fn(),
      };
    });

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: true,
        requestLogsRefreshWindowMs: 400,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    act(() => {
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "complete", ts: 2 });
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      "首页请求记录刷新失败",
      expect.objectContaining({
        source: "request_signal.complete",
        error: String(refreshError),
      })
    );
    vi.useRealTimers();
  });

  it("returns null for manual refresh when inactive and reports subscribe failures", async () => {
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);

    const inactive = renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: false,
        foregroundActive: true,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    await expect(inactive.result.current.refreshRequestLogsNow()).resolves.toBeNull();
    expect(refreshRequestLogs).not.toHaveBeenCalled();
    inactive.unmount();

    const unsubscribe = vi.fn();
    vi.mocked(subscribeGatewayEvent).mockReturnValue({
      ready: Promise.reject(new Error("listen boom")),
      unsubscribe,
    });

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: true,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(unsubscribe).toHaveBeenCalled();
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      "首页请求记录实时监听初始化失败",
      expect.objectContaining({ stage: "useHomeFreshnessOwner" })
    );
  });
});
