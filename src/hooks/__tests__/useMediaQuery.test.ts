import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  BREAKPOINTS,
  useMediaQuery,
  useBreakpoint,
  useBreakpointBelow,
  useCurrentBreakpoint,
  useResponsive,
} from "../useMediaQuery";

describe("hooks/useMediaQuery", () => {
  it("BREAKPOINTS has correct values", () => {
    expect(BREAKPOINTS.xs).toBe(475);
    expect(BREAKPOINTS.sm).toBe(640);
    expect(BREAKPOINTS.md).toBe(768);
    expect(BREAKPOINTS.lg).toBe(1024);
    expect(BREAKPOINTS.xl).toBe(1280);
    expect(BREAKPOINTS["2xl"]).toBe(1536);
  });

  it("useMediaQuery returns false when matchMedia.matches is false", () => {
    const { result } = renderHook(() => useMediaQuery("(min-width: 768px)"));
    expect(result.current).toBe(false);
  });

  it("useMediaQuery returns true when matchMedia.matches is true", () => {
    const original = window.matchMedia;
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: (query: string) => ({
        matches: true,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }),
    });

    const { result } = renderHook(() => useMediaQuery("(min-width: 768px)"));
    expect(result.current).toBe(true);

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("useBreakpoint returns boolean", () => {
    const { result } = renderHook(() => useBreakpoint("md"));
    expect(typeof result.current).toBe("boolean");
  });

  it("useBreakpointBelow returns boolean", () => {
    const { result } = renderHook(() => useBreakpointBelow("md"));
    expect(typeof result.current).toBe("boolean");
  });

  it("useCurrentBreakpoint returns base when all false", () => {
    const { result } = renderHook(() => useCurrentBreakpoint());
    expect(result.current).toBe("base");
  });

  it("useResponsive returns correct shape", () => {
    const { result } = renderHook(() => useResponsive());
    expect(result.current).toEqual(
      expect.objectContaining({
        isMobile: expect.any(Boolean),
        isTablet: expect.any(Boolean),
        isDesktop: expect.any(Boolean),
        isLargeDesktop: expect.any(Boolean),
        shouldShowSidebar: expect.any(Boolean),
      })
    );
  });

  it("subscribe handles missing matchMedia gracefully", () => {
    const original = window.matchMedia;
    Object.defineProperty(window, "matchMedia", { writable: true, value: undefined });

    const { result } = renderHook(() => useMediaQuery("(min-width: 768px)"));
    expect(result.current).toBe(false);

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });

  it("uses addListener fallback when MediaQueryList event listeners are unavailable", () => {
    const original = window.matchMedia;
    const addListener = vi.fn();
    const removeListener = vi.fn();
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener,
        removeListener,
        dispatchEvent: () => false,
      }),
    });

    const { unmount } = renderHook(() => useMediaQuery("(min-width: 768px)"));
    expect(addListener).toHaveBeenCalledWith(expect.any(Function));

    unmount();
    expect(removeListener).toHaveBeenCalledWith(expect.any(Function));

    Object.defineProperty(window, "matchMedia", { writable: true, value: original });
  });
});
