import { useSyncExternalStore } from "react";

// Tailwind breakpoint values (must match tailwind.config.ts)
export const BREAKPOINTS = {
  xs: 475,
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
} as const;

export type Breakpoint = keyof typeof BREAKPOINTS;

/**
 * Hook to check if a media query matches
 * @param query - CSS media query string (e.g., "(min-width: 768px)")
 * @returns boolean indicating if the query matches
 */
export function useMediaQuery(query: string): boolean {
  const subscribe = (callback: () => void) => {
    // Defensive check for test environments
    if (!window.matchMedia) {
      return () => {};
    }
    const mql = window.matchMedia(query);
    if (typeof mql.addEventListener === "function") {
      mql.addEventListener("change", callback);
      return () => mql.removeEventListener("change", callback);
    }
    mql.addListener(callback);
    return () => mql.removeListener(callback);
  };

  const getSnapshot = () => {
    // Defensive check for test environments where matchMedia may not be fully mocked
    const mql = window.matchMedia?.(query);
    return mql?.matches ?? false;
  };

  // SSR fallback - assume desktop
  const getServerSnapshot = () => true;

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

/**
 * Hook to check if viewport is at or above a breakpoint
 * @param breakpoint - Tailwind breakpoint name
 * @returns boolean indicating if viewport >= breakpoint
 */
export function useBreakpoint(breakpoint: Breakpoint): boolean {
  const minWidth = BREAKPOINTS[breakpoint];
  return useMediaQuery(`(min-width: ${minWidth}px)`);
}

/**
 * Hook to check if viewport is below a breakpoint
 * @param breakpoint - Tailwind breakpoint name
 * @returns boolean indicating if viewport < breakpoint
 */
export function useBreakpointBelow(breakpoint: Breakpoint): boolean {
  const minWidth = BREAKPOINTS[breakpoint];
  return useMediaQuery(`(max-width: ${minWidth - 1}px)`);
}

/**
 * Hook to get the current active breakpoint
 * @returns The current breakpoint name
 */
export function useCurrentBreakpoint(): Breakpoint | "base" {
  const isXs = useBreakpoint("xs");
  const isSm = useBreakpoint("sm");
  const isMd = useBreakpoint("md");
  const isLg = useBreakpoint("lg");
  const isXl = useBreakpoint("xl");
  const is2xl = useBreakpoint("2xl");

  if (is2xl) return "2xl";
  if (isXl) return "xl";
  if (isLg) return "lg";
  if (isMd) return "md";
  if (isSm) return "sm";
  if (isXs) return "xs";
  return "base";
}

/**
 * Hook for common responsive layout checks
 * @returns Object with boolean flags for common breakpoint checks
 */
export function useResponsive() {
  const isMobile = useBreakpointBelow("md"); // < 768px
  const isTablet = useMediaQuery(
    `(min-width: ${BREAKPOINTS.md}px) and (max-width: ${BREAKPOINTS.lg - 1}px)`
  );
  const isDesktop = useBreakpoint("lg"); // >= 1024px
  const isLargeDesktop = useBreakpoint("xl"); // >= 1280px

  return {
    isMobile,
    isTablet,
    isDesktop,
    isLargeDesktop,
    // Sidebar visibility helpers
    shouldShowSidebar: isDesktop,
  };
}
