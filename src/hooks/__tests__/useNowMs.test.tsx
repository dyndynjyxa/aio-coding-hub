import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

async function importFreshUseNowMs() {
  vi.resetModules();
  return await import("../useNowMs");
}

describe("hooks/useNowMs", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("shares one interval for subscribers with the same cadence", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    const clearIntervalSpy = vi.spyOn(window, "clearInterval");
    const { useNowMs } = await importFreshUseNowMs();

    const first = renderHook(() => useNowMs(true, 250));
    const second = renderHook(() => useNowMs(true, 250));

    expect(setIntervalSpy).toHaveBeenCalledTimes(1);
    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 250);
    expect(first.result.current).toBe(1_000);
    expect(second.result.current).toBe(1_000);

    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(first.result.current).toBe(1_250);
    expect(second.result.current).toBe(1_250);

    first.unmount();
    expect(clearIntervalSpy).not.toHaveBeenCalled();

    second.unmount();
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);
  });

  it("keeps distinct cadences isolated", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(2_000);
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    const clearIntervalSpy = vi.spyOn(window, "clearInterval");
    const { useNowMs } = await importFreshUseNowMs();

    const fast = renderHook(() => useNowMs(true, 250));
    const slow = renderHook(() => useNowMs(true, 1000));

    expect(setIntervalSpy).toHaveBeenCalledTimes(2);
    const delays = setIntervalSpy.mock.calls
      .map(([, delay]) => delay)
      .filter((delay): delay is number => typeof delay === "number")
      .sort((a, b) => a - b);
    expect(delays).toEqual([250, 1000]);

    fast.unmount();
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);
    slow.unmount();
    expect(clearIntervalSpy).toHaveBeenCalledTimes(2);
  });

  it("keeps clock subscribers isolated when one listener throws", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(3_000);
    const { subscribeNowMs } = await importFreshUseNowMs();
    const failingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeFailing = subscribeNowMs(250, failingListener);
    const unsubscribeHealthy = subscribeNowMs(250, healthyListener);

    expect(() => {
      act(() => {
        vi.advanceTimersByTime(250);
      });
    }).not.toThrow();

    expect(failingListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledWith(3_250);

    unsubscribeFailing();
    unsubscribeHealthy();
  });
});
