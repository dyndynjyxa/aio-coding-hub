import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

async function importFreshUseNowUnix() {
  vi.resetModules();
  return await import("../useNowUnix");
}

describe("hooks/useNowUnix", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("uses the shared 1s clock and returns seconds", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    const clearIntervalSpy = vi.spyOn(window, "clearInterval");
    const { useNowUnix } = await importFreshUseNowUnix();

    const first = renderHook(() => useNowUnix(true));
    const second = renderHook(() => useNowUnix(true));

    expect(setIntervalSpy).toHaveBeenCalledTimes(1);
    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 1000);
    expect(first.result.current).toBe(1_700_000_000);
    expect(second.result.current).toBe(1_700_000_000);

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(first.result.current).toBe(1_700_000_001);
    expect(second.result.current).toBe(1_700_000_001);

    first.unmount();
    expect(clearIntervalSpy).not.toHaveBeenCalled();
    second.unmount();
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);
  });
});
