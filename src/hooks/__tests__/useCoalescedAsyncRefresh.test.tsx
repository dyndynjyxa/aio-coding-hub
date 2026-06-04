import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCoalescedAsyncRefresh } from "../useCoalescedAsyncRefresh";

describe("hooks/useCoalescedAsyncRefresh", () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it("coalesces duplicate scheduled refreshes into one task", async () => {
    vi.useFakeTimers();
    const task = vi.fn().mockResolvedValue("ok");

    const { result } = renderHook(() =>
      useCoalescedAsyncRefresh({
        enabled: true,
        delayMs: 400,
        task,
      })
    );

    act(() => {
      result.current.schedule("first");
      result.current.schedule("second");
      result.current.schedule("third");
    });

    expect(task).not.toHaveBeenCalled();

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(task).toHaveBeenCalledTimes(1);
    expect(task).toHaveBeenCalledWith("first");
    vi.useRealTimers();
  });

  it("runs one follow-up refresh with the latest queued source after in-flight work finishes", async () => {
    let resolveFirst: ((value: string) => void) | null = null;
    const task = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<string>((resolve) => {
            resolveFirst = resolve;
          })
      )
      .mockResolvedValueOnce("second-ok");

    const { result } = renderHook(() =>
      useCoalescedAsyncRefresh({
        enabled: true,
        delayMs: 400,
        task,
      })
    );

    void result.current.flush("manual");

    act(() => {
      result.current.schedule("queued-a");
      result.current.schedule("queued-b");
    });

    expect(task).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFirst?.("first-ok");
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(task).toHaveBeenCalledTimes(2);
    expect(task).toHaveBeenNthCalledWith(2, "queued-b");
  });

  it("clears queued work when disabled", async () => {
    vi.useFakeTimers();
    const task = vi.fn().mockResolvedValue("ok");

    const view = renderHook(
      (props: { enabled: boolean }) =>
        useCoalescedAsyncRefresh({
          enabled: props.enabled,
          delayMs: 400,
          task,
        }),
      { initialProps: { enabled: true } }
    );

    act(() => {
      view.result.current.schedule("queued");
    });

    view.rerender({ enabled: false });

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
      await Promise.resolve();
    });

    expect(task).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it("normalizes task errors through onError", async () => {
    const error = new Error("boom");
    const task = vi.fn().mockRejectedValue(error);
    const onError = vi.fn().mockReturnValue("handled");

    const { result } = renderHook(() =>
      useCoalescedAsyncRefresh({
        enabled: true,
        delayMs: 400,
        task,
        onError,
      })
    );

    await expect(result.current.flush("manual")).resolves.toBe("handled");
    expect(onError).toHaveBeenCalledWith(error, "manual");
  });
});
