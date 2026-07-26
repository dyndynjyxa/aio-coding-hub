import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
  HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
  HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY,
  HOME_USAGE_FULL_IDLE_GAP_MINUTES_OPTIONS,
  HOME_USAGE_SESSION_BREAK_GAP_MINUTES_OPTIONS,
  normalizeHomeUsageDevelopmentTimeThresholds,
  readHomeUsageDevelopmentTimeThresholdsFromStorage,
  subscribeHomeUsageDevelopmentTimeThresholds,
  writeHomeUsageFullIdleGapMinutesToStorage,
  writeHomeUsageSessionBreakGapMinutesToStorage,
} from "../homeUsageDevelopmentTime";

describe("services/home/homeUsageDevelopmentTime", () => {
  beforeEach(() => {
    window.localStorage.removeItem(HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY);
  });

  it("uses 15/30 defaults and exposes every fixed minute option", () => {
    expect(readHomeUsageDevelopmentTimeThresholdsFromStorage()).toEqual({
      fullIdleGapMinutes: HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
      sessionBreakGapMinutes: HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
    });
    expect(HOME_USAGE_FULL_IDLE_GAP_MINUTES_OPTIONS).toEqual(
      Array.from({ length: 30 }, (_, index) => index + 1)
    );
    expect(HOME_USAGE_SESSION_BREAK_GAP_MINUTES_OPTIONS).toEqual(
      Array.from({ length: 46 }, (_, index) => index + 15)
    );
  });

  it("falls back to defaults for malformed, out-of-range, or conflicting stored values", () => {
    expect(normalizeHomeUsageDevelopmentTimeThresholds(null)).toEqual({
      fullIdleGapMinutes: 15,
      sessionBreakGapMinutes: 30,
    });
    expect(
      normalizeHomeUsageDevelopmentTimeThresholds({
        fullIdleGapMinutes: 30,
        sessionBreakGapMinutes: 30,
      })
    ).toEqual({ fullIdleGapMinutes: 15, sessionBreakGapMinutes: 30 });

    window.localStorage.setItem(HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY, "invalid json");
    expect(readHomeUsageDevelopmentTimeThresholdsFromStorage()).toEqual({
      fullIdleGapMinutes: 15,
      sessionBreakGapMinutes: 30,
    });
  });

  it("automatically moves the other threshold by one minute when values conflict", () => {
    writeHomeUsageFullIdleGapMinutesToStorage(30);
    expect(readHomeUsageDevelopmentTimeThresholdsFromStorage()).toEqual({
      fullIdleGapMinutes: 30,
      sessionBreakGapMinutes: 31,
    });

    writeHomeUsageSessionBreakGapMinutesToStorage(15);
    expect(readHomeUsageDevelopmentTimeThresholdsFromStorage()).toEqual({
      fullIdleGapMinutes: 14,
      sessionBreakGapMinutes: 15,
    });
  });

  it("notifies local writes and matching storage events", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeHomeUsageDevelopmentTimeThresholds(listener);

    writeHomeUsageFullIdleGapMinutesToStorage(10);
    expect(listener).toHaveBeenCalledTimes(1);

    window.dispatchEvent(
      new StorageEvent("storage", { key: HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY })
    );
    expect(listener).toHaveBeenCalledTimes(2);

    unsubscribe();
  });
});
