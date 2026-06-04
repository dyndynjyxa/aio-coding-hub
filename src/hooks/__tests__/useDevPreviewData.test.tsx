import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

async function importFreshDevPreview() {
  vi.resetModules();
  return await import("../useDevPreviewData");
}

describe("hooks/useDevPreviewData", () => {
  beforeEach(() => {
    window.localStorage.removeItem("devPreview.enabled");
    vi.restoreAllMocks();
  });

  it("syncs same-tab writes through the external store", async () => {
    const { useDevPreviewData } = await importFreshDevPreview();
    const { result } = renderHook(() => useDevPreviewData());

    expect(result.current.enabled).toBe(false);

    act(() => {
      result.current.setEnabled(true);
    });

    expect(result.current.enabled).toBe(true);
    expect(window.localStorage.getItem("devPreview.enabled")).toBe("1");
  });

  it("keeps subscribers isolated when one listener throws", async () => {
    const { setDevPreviewEnabled, subscribeDevPreview } = await importFreshDevPreview();
    const failingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeFailing = subscribeDevPreview(failingListener);
    const unsubscribeHealthy = subscribeDevPreview(healthyListener);

    expect(() => setDevPreviewEnabled(true)).not.toThrow();
    expect(failingListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledTimes(1);

    unsubscribeFailing();
    unsubscribeHealthy();
  });

  it("syncs storage events from another tab or WebView", async () => {
    const { useDevPreviewData } = await importFreshDevPreview();
    const { result } = renderHook(() => useDevPreviewData());

    expect(result.current.enabled).toBe(false);

    act(() => {
      window.localStorage.setItem("devPreview.enabled", "1");
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "devPreview.enabled",
          newValue: "1",
          storageArea: window.localStorage,
        })
      );
    });

    expect(result.current.enabled).toBe(true);
  });

  it("installs the storage listener only while subscribed", async () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    const removeSpy = vi.spyOn(window, "removeEventListener");
    const { useDevPreviewData } = await importFreshDevPreview();

    const first = renderHook(() => useDevPreviewData());
    const second = renderHook(() => useDevPreviewData());

    expect(addSpy).toHaveBeenCalledTimes(1);
    expect(addSpy).toHaveBeenCalledWith("storage", expect.any(Function));

    first.unmount();
    expect(removeSpy).not.toHaveBeenCalledWith("storage", expect.any(Function));

    second.unmount();
    expect(removeSpy).toHaveBeenCalledTimes(1);
    expect(removeSpy).toHaveBeenCalledWith("storage", expect.any(Function));
  });
});
