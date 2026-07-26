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
    const refreshActiveRequests = vi.fn().mockResolvedValue(null);
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
        onRefreshActiveRequests: refreshActiveRequests,
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
    expect(refreshActiveRequests).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("queues a fresh active snapshot when complete arrives during a start refresh", async () => {
    vi.useFakeTimers();
    let resolveFirstRefresh: (() => void) | null = null;
    const refreshActiveRequests = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirstRefresh = () => resolve(null);
          })
      )
      .mockResolvedValueOnce(null);
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);
    let eventHandler:
      | ((payload: {
          trace_id: string;
          cli_key: string;
          phase: "start" | "complete";
          ts: number;
        }) => void)
      | null = null;

    vi.mocked(subscribeGatewayEvent).mockImplementation((_event: string, handler: any) => {
      eventHandler = handler;
      return { ready: Promise.resolve(), unsubscribe: vi.fn() };
    });

    const view = renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: true,
        requestLogsRefreshWindowMs: 1_000,
        onRefreshActiveRequests: refreshActiveRequests,
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    act(() => {
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "start", ts: 1 });
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(refreshActiveRequests).toHaveBeenCalledTimes(1);

    act(() => {
      eventHandler?.({ trace_id: "t-1", cli_key: "claude", phase: "complete", ts: 2 });
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(refreshActiveRequests).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFirstRefresh?.();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(refreshActiveRequests).toHaveBeenCalledTimes(2);
    expect(refreshRequestLogs).not.toHaveBeenCalled();

    view.unmount();
    vi.useRealTimers();
  });

  it("refreshes from foreground events while overview is active even if the foreground prop lags", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);
    let foregroundArgs: { onForeground: () => void } | null = null;

    vi.mocked(useWindowForeground).mockImplementation((args: any) => {
      foregroundArgs = args;
    });

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: false,
        requestLogsRefreshWindowMs: 400,
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    act(() => {
      foregroundArgs?.onForeground();
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("polls while request activity is pending so stale active snapshots self-correct", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);

    const view = renderHook(
      (props: {
        overviewActive: boolean;
        foregroundActive: boolean;
        requestActivityPending: boolean;
      }) =>
        useHomeFreshnessOwner({
          ...props,
          requestLogsRefreshWindowMs: 200,
          requestActivityWatchdogIntervalMs: 5000,
          onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
          onRefreshRequestLogs: refreshRequestLogs,
        }),
      {
        initialProps: {
          overviewActive: true,
          foregroundActive: true,
          requestActivityPending: true,
        },
      }
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(4999);
      await Promise.resolve();
    });
    expect(refreshRequestLogs).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
      await Promise.resolve();
    });
    expect(refreshRequestLogs).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
      await Promise.resolve();
    });
    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);

    view.rerender({
      overviewActive: true,
      foregroundActive: true,
      requestActivityPending: false,
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5200);
      await Promise.resolve();
    });

    expect(refreshRequestLogs).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("keeps signal-driven refresh while the window is backgrounded", async () => {
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

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: false,
        requestLogsRefreshWindowMs: 400,
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
    vi.useRealTimers();
  });

  it("drops queued request log refresh when overview deactivates", async () => {
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
          onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
      overviewActive: false,
      foregroundActive: true,
    });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(refreshRequestLogs).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("does not run the activity watchdog while the window is backgrounded", async () => {
    vi.useFakeTimers();
    const refreshRequestLogs = vi.fn().mockResolvedValue(null);

    renderHook(() =>
      useHomeFreshnessOwner({
        overviewActive: true,
        foregroundActive: false,
        requestActivityPending: true,
        requestLogsRefreshWindowMs: 200,
        requestActivityWatchdogIntervalMs: 5000,
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
        onRefreshRequestLogs: refreshRequestLogs,
      })
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(11_000);
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
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
        onRefreshActiveRequests: vi.fn().mockResolvedValue(null),
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
