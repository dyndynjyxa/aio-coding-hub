import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { subscribeDocumentVisibility, useDocumentVisibility } from "../useDocumentVisibility";

describe("hooks/useDocumentVisibility", () => {
  it("tracks document visibility changes", () => {
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    const { result, unmount } = renderHook(() => useDocumentVisibility());
    expect(result.current).toBe(true);

    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(result.current).toBe(false);

    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(result.current).toBe(true);

    unmount();
  });

  it("shares one document listener across mounted subscribers", () => {
    const addListener = vi.spyOn(document, "addEventListener");
    const removeListener = vi.spyOn(document, "removeEventListener");

    const first = renderHook(() => useDocumentVisibility());
    const second = renderHook(() => useDocumentVisibility());

    const addedVisibilityListeners = addListener.mock.calls.filter(
      ([eventName]) => eventName === "visibilitychange"
    );
    expect(addedVisibilityListeners).toHaveLength(1);

    first.unmount();
    expect(
      removeListener.mock.calls.filter(([eventName]) => eventName === "visibilitychange")
    ).toHaveLength(0);

    second.unmount();
    expect(
      removeListener.mock.calls.filter(([eventName]) => eventName === "visibilitychange")
    ).toHaveLength(1);

    addListener.mockRestore();
    removeListener.mockRestore();
  });

  it("keeps visibility subscribers isolated when one listener throws", () => {
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    const failingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeFailing = subscribeDocumentVisibility(failingListener);
    const unsubscribeHealthy = subscribeDocumentVisibility(healthyListener);

    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    expect(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    }).not.toThrow();

    expect(failingListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledTimes(1);

    unsubscribeFailing();
    unsubscribeHealthy();
  });
});
