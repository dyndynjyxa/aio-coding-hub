import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setDesktopWindowTheme } from "../../services/desktop/window";

vi.mock("../../services/desktop/window", () => ({
  setDesktopWindowTheme: vi.fn().mockResolvedValue(true),
}));

const mockTauriListen = vi.fn();
vi.mock("../../services/desktop/themeEvent", () => ({
  listenThemeChanged: mockTauriListen,
}));

describe("hooks/useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    mockTauriListen.mockReset();
    mockTauriListen.mockImplementation(() => Promise.resolve(() => {}));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  async function importFreshUseTheme() {
    vi.resetModules();
    return await import("../useTheme");
  }

  function mockMatchMediaWithChangeListener(initialMatches: boolean) {
    const original = window.matchMedia;
    let matches = initialMatches;
    let changeHandler: ((event?: MediaQueryListEvent) => void) | null = null;

    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation(() => ({
        get matches() {
          return matches;
        },
        media: "(prefers-color-scheme: dark)",
        onchange: null,
        addEventListener: (_event: string, handler: (event?: MediaQueryListEvent) => void) => {
          changeHandler = handler;
        },
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: () => false,
      })),
    });

    return {
      setMatches(next: boolean) {
        matches = next;
      },
      fireChange() {
        changeHandler?.();
      },
      restore() {
        Object.defineProperty(window, "matchMedia", { writable: true, value: original });
      },
    };
  }

  function mockTauriThemeListener() {
    let tauriHandler: ((theme: "light" | "dark") => void) | null = null;
    mockTauriListen.mockImplementation((handler: (theme: "light" | "dark") => void) => {
      tauriHandler = handler;
      return Promise.resolve(() => {});
    });

    return {
      fireTauriThemeChange(theme: "light" | "dark") {
        tauriHandler?.(theme);
      },
      get handler() {
        return tauriHandler;
      },
    };
  }

  it("defaults to system theme", async () => {
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    // matchMedia mock returns matches:false, so resolvedTheme = "light"
    expect(result.current.resolvedTheme).toBe("light");
  });

  it("setTheme(dark) updates theme and classList", async () => {
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("aio-theme")).toBe("dark");
    expect(setDesktopWindowTheme).toHaveBeenCalledWith("dark");
  });

  it("setTheme(light) removes dark class", async () => {
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    act(() => {
      result.current.setTheme("light");
    });
    expect(result.current.theme).toBe("light");
    expect(result.current.resolvedTheme).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("reads stored theme from localStorage", async () => {
    localStorage.setItem("aio-theme", "dark");
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
  });

  it("setTheme(system) follows matchMedia", async () => {
    localStorage.setItem("aio-theme", "dark");
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("system");
    });

    expect(result.current.theme).toBe("system");
    // matchMedia mock returns matches:false → light
    expect(result.current.resolvedTheme).toBe("light");
  });

  it("falls back safely when localStorage access throws during module init and updates", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });

    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");

    act(() => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("falls back safely when matchMedia is unavailable during module init", async () => {
    const original = window.matchMedia;
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: undefined,
    });

    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("still applies a stored dark theme when matchMedia is unavailable", async () => {
    const original = window.matchMedia;
    localStorage.setItem("aio-theme", "dark");
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: undefined,
    });

    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("uses addListener fallback when addEventListener is unavailable", async () => {
    const original = window.matchMedia;
    const addListener = vi.fn();
    const removeListener = vi.fn();

    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockReturnValue({
        matches: false,
        media: "(prefers-color-scheme: dark)",
        onchange: null,
        addListener,
        removeListener,
        dispatchEvent: () => false,
      }),
    });

    const { useTheme } = await importFreshUseTheme();
    const { unmount } = renderHook(() => useTheme());

    expect(addListener).toHaveBeenCalledTimes(1);
    unmount();
    expect(removeListener).toHaveBeenCalledTimes(1);

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("installs theme listeners only while hooks are subscribed", async () => {
    const original = window.matchMedia;
    const addMediaListener = vi.fn();
    const removeMediaListener = vi.fn();
    const nativeUnlisten = vi.fn();
    const addWindowListener = vi.spyOn(window, "addEventListener");
    const removeWindowListener = vi.spyOn(window, "removeEventListener");
    mockTauriListen.mockImplementation(() => Promise.resolve(nativeUnlisten));

    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockReturnValue({
        matches: false,
        media: "(prefers-color-scheme: dark)",
        onchange: null,
        addEventListener: addMediaListener,
        removeEventListener: removeMediaListener,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: () => false,
      }),
    });

    const { useTheme } = await importFreshUseTheme();
    const first = renderHook(() => useTheme());
    const second = renderHook(() => useTheme());
    await Promise.resolve();

    expect(addMediaListener).toHaveBeenCalledTimes(1);
    expect(addWindowListener).toHaveBeenCalledWith("storage", expect.any(Function));
    expect(mockTauriListen).toHaveBeenCalledTimes(1);

    first.unmount();
    expect(removeMediaListener).not.toHaveBeenCalled();
    expect(nativeUnlisten).not.toHaveBeenCalled();

    second.unmount();
    expect(removeMediaListener).toHaveBeenCalledWith("change", expect.any(Function));
    expect(removeWindowListener).toHaveBeenCalledWith("storage", expect.any(Function));
    expect(nativeUnlisten).toHaveBeenCalledTimes(1);

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("keeps theme subscribers isolated when one listener throws", async () => {
    const { subscribeTheme } = await importFreshUseTheme();
    const failingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeFailing = subscribeTheme(failingListener);
    const unsubscribeHealthy = subscribeTheme(healthyListener);

    expect(() => {
      window.localStorage.setItem("aio-theme", "dark");
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "aio-theme",
          storageArea: window.localStorage,
        })
      );
    }).not.toThrow();

    expect(failingListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledTimes(1);

    unsubscribeFailing();
    unsubscribeHealthy();
  });

  it("cleans up a Tauri theme listener that resolves after unmount", async () => {
    let resolveListen!: (unlisten: () => void) => void;
    const nativeUnlisten = vi.fn();
    mockTauriListen.mockImplementation(
      () =>
        new Promise<() => void>((resolve) => {
          resolveListen = resolve;
        })
    );

    const { useTheme } = await importFreshUseTheme();
    const view = renderHook(() => useTheme());
    view.unmount();

    resolveListen(nativeUnlisten);
    await Promise.resolve();

    expect(nativeUnlisten).toHaveBeenCalledTimes(1);
  });

  it("syncs stored theme changes from other tabs or WebViews", async () => {
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");

    act(() => {
      window.localStorage.setItem("aio-theme", "dark");
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "aio-theme",
          newValue: "dark",
          storageArea: window.localStorage,
        })
      );
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("ignores system theme change events after switching to an explicit theme", async () => {
    const media = mockMatchMediaWithChangeListener(false);
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");

    media.setMatches(true);
    act(() => {
      media.fireChange();
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(setDesktopWindowTheme).toHaveBeenCalledTimes(2);

    media.restore();
  });

  it("reacts to system theme changes while in system mode", async () => {
    const media = mockMatchMediaWithChangeListener(false);
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");

    media.setMatches(true);
    act(() => {
      media.fireChange();
    });

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(setDesktopWindowTheme).toHaveBeenLastCalledWith("system");

    media.restore();
  });

  it("keeps the same snapshot when the system theme event does not change the resolved theme", async () => {
    const media = mockMatchMediaWithChangeListener(false);
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");

    act(() => {
      media.fireChange();
    });

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");
    expect(setDesktopWindowTheme).toHaveBeenCalledTimes(2);

    media.restore();
  });

  it("keeps DOM theme state even when native window sync rejects", async () => {
    vi.mocked(setDesktopWindowTheme).mockRejectedValueOnce(new Error("native sync failed"));

    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });

    await Promise.resolve();

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("reacts to Tauri theme change events while in system mode", async () => {
    const media = mockMatchMediaWithChangeListener(false);
    const tauri = mockTauriThemeListener();
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");

    // Check that the listener was registered
    expect(mockTauriListen).toHaveBeenCalled();
    vi.mocked(setDesktopWindowTheme).mockClear();

    // Fire Tauri theme change event
    act(() => {
      tauri.handler?.("dark");
    });

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(setDesktopWindowTheme).toHaveBeenCalledTimes(1);
    expect(setDesktopWindowTheme).toHaveBeenLastCalledWith("system");

    media.restore();
  });

  it("ignores Tauri theme change events after switching to an explicit theme", async () => {
    mockTauriListen.mockClear();
    const media = mockMatchMediaWithChangeListener(false);
    const tauri = mockTauriThemeListener();
    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");

    act(() => {
      tauri.fireTauriThemeChange("light");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");

    media.restore();
  });

  it("handles Tauri listen failure gracefully", async () => {
    mockTauriListen.mockRejectedValueOnce(new Error("Tauri not available"));

    const { useTheme } = await importFreshUseTheme();
    const { result } = renderHook(() => useTheme());

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBe("light");
  });
});
