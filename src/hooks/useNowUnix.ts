// Usage:
// - Provides a reactive Unix timestamp (seconds) that ticks every second.
// - Automatically starts/stops the interval based on the `enabled` flag.
// - Eliminates duplication of the nowUnix + setInterval pattern across components.

import { useNowMs } from "./useNowMs";

/**
 * Returns the current Unix time in seconds, updating every 1s while `enabled` is true.
 * When `enabled` is false the timer is paused and the last value is retained.
 */
export function useNowUnix(enabled: boolean): number {
  return Math.floor(useNowMs(enabled, 1000) / 1000);
}
