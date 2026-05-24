import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useWindowForeground } from "../useWindowForeground";

describe("hooks/useWindowForeground", () => {
  it("does nothing when disabled", () => {
    const onForeground = vi.fn();
    renderHook(() => useWindowForeground({ enabled: false, onForeground, throttleMs: 1000 }));
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(onForeground).not.toHaveBeenCalled();
  });

  it("fires on focus/visibility with throttle", () => {
    vi.useFakeTimers();
    const base = 1_700_000_000_000;
    vi.setSystemTime(base);

    const onForeground = vi.fn();
    const { unmount } = renderHook(() =>
      useWindowForeground({ enabled: true, onForeground, throttleMs: 1000 })
    );

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(onForeground).toHaveBeenCalledTimes(1);

    vi.setSystemTime(base + 500);
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(onForeground).toHaveBeenCalledTimes(1);

    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    vi.setSystemTime(base + 1100);
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(onForeground).toHaveBeenCalledTimes(2);

    unmount();
    vi.useRealTimers();
  });

  it("shares one foreground listener pair across enabled subscribers", () => {
    const addWindowListener = vi.spyOn(window, "addEventListener");
    const removeWindowListener = vi.spyOn(window, "removeEventListener");
    const addDocumentListener = vi.spyOn(document, "addEventListener");
    const removeDocumentListener = vi.spyOn(document, "removeEventListener");
    const firstForeground = vi.fn();
    const secondForeground = vi.fn();

    const first = renderHook(() =>
      useWindowForeground({ enabled: true, onForeground: firstForeground, throttleMs: 0 })
    );
    const second = renderHook(() =>
      useWindowForeground({ enabled: true, onForeground: secondForeground, throttleMs: 0 })
    );

    expect(
      addWindowListener.mock.calls.filter(([eventName]) => eventName === "focus")
    ).toHaveLength(1);
    expect(
      addDocumentListener.mock.calls.filter(([eventName]) => eventName === "visibilitychange")
    ).toHaveLength(1);

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(firstForeground).toHaveBeenCalledTimes(1);
    expect(secondForeground).toHaveBeenCalledTimes(1);

    first.unmount();
    expect(
      removeWindowListener.mock.calls.filter(([eventName]) => eventName === "focus")
    ).toHaveLength(0);
    expect(
      removeDocumentListener.mock.calls.filter(([eventName]) => eventName === "visibilitychange")
    ).toHaveLength(0);

    second.unmount();
    expect(
      removeWindowListener.mock.calls.filter(([eventName]) => eventName === "focus")
    ).toHaveLength(1);
    expect(
      removeDocumentListener.mock.calls.filter(([eventName]) => eventName === "visibilitychange")
    ).toHaveLength(1);

    addWindowListener.mockRestore();
    removeWindowListener.mockRestore();
    addDocumentListener.mockRestore();
    removeDocumentListener.mockRestore();
  });
});
