// Canonical brand and status color hex values.
// These MUST stay in sync with the Tailwind `accent` / status tokens
// defined in tailwind.config.ts.

/** Brand colors */
export const BRAND = {
  accent: "#2563EB",
  accentSecondary: "#0EA5E9",
} as const;

/** Semantic status colors */
export const STATUS = {
  success: "#34D399",
  warning: "#FB923C",
  danger: "#F87171",
  info: "#0EA5E9",
} as const;

/** Extended palette for multi-series charts */
export const CHART_PALETTE = [
  BRAND.accent,
  BRAND.accentSecondary,
  STATUS.success,
  STATUS.warning,
  STATUS.danger,
  STATUS.info,
  "#9333EA",
  "#059669",
  "#EA580C",
  "#B91C1C",
] as const;
