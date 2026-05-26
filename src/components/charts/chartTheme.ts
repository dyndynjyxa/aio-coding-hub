// Shared chart theme and configuration for Recharts
// Provides consistent colors, gradients, and styling across all charts

import { BRAND, STATUS, CHART_PALETTE } from "../../constants/colors";

/**
 * Primary color palette for charts
 * Derived from shared brand/status constants in constants/colors.ts
 */
export const CHART_COLORS = {
  primary: BRAND.accent,
  secondary: BRAND.accentSecondary,
  success: STATUS.success,
  warning: STATUS.warning,
  danger: STATUS.danger,
  info: STATUS.info,
  purple: "#9333EA",
  emerald: "#059669",
  orange: "#EA580C",
  red: "#B91C1C",
} as const;

/**
 * Color palette for multi-series charts
 */
export const MULTI_SERIES_PALETTE: readonly string[] = CHART_PALETTE;

/**
 * Pick a color from palette by index, with HSL fallback for large series
 */
export function pickPaletteColor(index: number): string {
  if (index < MULTI_SERIES_PALETTE.length) {
    return MULTI_SERIES_PALETTE[index] ?? CHART_COLORS.primary;
  }

  // HSL fallback for series beyond palette
  const hue = (index * 137.508) % 360;
  return `hsl(${hue} 70% 45%)`;
}

/**
 * Default grid padding for charts
 */
export const CHART_GRID = {
  left: 0,
  right: 16,
  top: 8,
  bottom: 24,
} as const;

/**
 * Axis styling (dark-mode aware)
 */
export function getAxisStyle(isDark: boolean) {
  return {
    fontSize: 10,
    fontWeight: 500,
    fill: isDark ? "#94a3b8" : "#64748b",
    color: isDark ? "#94a3b8" : "#64748b",
  };
}

/** @deprecated Use getAxisStyle(isDark) for dark mode support */
export const AXIS_STYLE = getAxisStyle(false);

/**
 * Grid line styling (dark-mode aware)
 */
export function getGridLineStyle(isDark: boolean) {
  return {
    stroke: isDark ? "rgba(100, 150, 255, 0.15)" : "rgba(0, 82, 255, 0.10)",
    strokeDasharray: "3 3",
  };
}

/** @deprecated Use getGridLineStyle(isDark) for dark mode support */
export const GRID_LINE_STYLE = getGridLineStyle(false);

/**
 * Tooltip styling (dark-mode aware)
 */
export function getTooltipStyle(isDark: boolean) {
  return {
    backgroundColor: isDark ? "rgba(30, 41, 59, 0.98)" : "rgba(255, 255, 255, 0.98)",
    border: isDark ? "1px solid rgba(71, 85, 105, 0.3)" : "1px solid rgba(148, 163, 184, 0.2)",
    borderRadius: "8px",
    boxShadow: isDark ? "0 4px 12px rgba(0, 0, 0, 0.4)" : "0 4px 12px rgba(15, 23, 42, 0.12)",
    padding: "12px",
    color: isDark ? "#e2e8f0" : undefined,
  };
}

/** @deprecated Use getTooltipStyle(isDark) for dark mode support */
export const TOOLTIP_STYLE = getTooltipStyle(false);

/**
 * Legend styling (dark-mode aware)
 */
export function getLegendStyle(isDark: boolean) {
  return {
    fontSize: 11,
    fontWeight: 500,
    color: isDark ? "#94a3b8" : "#475569",
  };
}

/** @deprecated Use getLegendStyle(isDark) for dark mode support */
export const LEGEND_STYLE = getLegendStyle(false);

/**
 * Axis line stroke color (dark-mode aware)
 */
export function getAxisLineStroke(isDark: boolean) {
  return isDark ? "rgba(148, 163, 184, 0.2)" : "rgba(15, 23, 42, 0.12)";
}

/**
 * Cursor stroke color for tooltips (dark-mode aware)
 */
export function getCursorStroke(isDark: boolean) {
  return isDark ? "rgba(100, 150, 255, 0.25)" : "rgba(0, 82, 255, 0.15)";
}

/**
 * Gradient configuration for area charts
 */
export const createAreaGradient = (color: string, id: string) => ({
  id,
  x1: "0",
  y1: "0",
  x2: "0",
  y2: "1",
  gradientUnits: "userSpaceOnUse" as const,
  stops: [
    { offset: "0%", stopColor: color, stopOpacity: 0.25 },
    { offset: "100%", stopColor: color, stopOpacity: 0.0 },
  ],
});

/**
 * Animation configuration
 */
export const CHART_ANIMATION = {
  animationDuration: 300,
  animationEasing: "ease-in-out" as const,
} as const;

/**
 * Threshold zone colors
 */
export const THRESHOLD_COLORS = {
  warning: "rgba(220, 38, 38, 0.06)",
  warningLine: "rgba(220, 38, 38, 0.70)",
} as const;
