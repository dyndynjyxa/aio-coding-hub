import { afterEach, describe, expect, it, vi } from "vitest";
import {
  HOME_USAGE_DAY_START_HOUR_STORAGE_KEY,
  HOME_USAGE_DEFAULT_DAY_START_HOUR,
  formatUsageDayHourLabel,
  formatUsageDayHourMinuteFromMs,
  formatUsageDayHourTickLabel,
  normalizeHomeUsageDayStartHour,
  orderedUsageDayHours,
  readHomeUsageDayStartHourFromStorage,
  subscribeHomeUsageDayStartHour,
  writeHomeUsageDayStartHourToStorage,
} from "../homeUsageDayBoundary";

describe("services/home/homeUsageDayBoundary", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("normalizes, reads, and writes the shared day start hour preference", () => {
    expect(HOME_USAGE_DEFAULT_DAY_START_HOUR).toBe(0);
    expect(normalizeHomeUsageDayStartHour(null)).toBe(HOME_USAGE_DEFAULT_DAY_START_HOUR);
    expect(normalizeHomeUsageDayStartHour(7)).toBe(7);
    expect(normalizeHomeUsageDayStartHour(10)).toBe(HOME_USAGE_DEFAULT_DAY_START_HOUR);

    expect(readHomeUsageDayStartHourFromStorage()).toBe(HOME_USAGE_DEFAULT_DAY_START_HOUR);

    writeHomeUsageDayStartHourToStorage(7);
    expect(window.localStorage.getItem(HOME_USAGE_DAY_START_HOUR_STORAGE_KEY)).toBe("7");
    expect(readHomeUsageDayStartHourFromStorage()).toBe(7);

    writeHomeUsageDayStartHourToStorage(12);
    expect(window.localStorage.getItem(HOME_USAGE_DAY_START_HOUR_STORAGE_KEY)).toBe("0");
    expect(readHomeUsageDayStartHourFromStorage()).toBe(HOME_USAGE_DEFAULT_DAY_START_HOUR);
  });

  it("notifies subscribers on writes and matching storage events", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeHomeUsageDayStartHour(listener);

    writeHomeUsageDayStartHourToStorage(6);
    expect(listener).toHaveBeenCalledTimes(1);

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: "unrelated",
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(1);

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: HOME_USAGE_DAY_START_HOUR_STORAGE_KEY,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(2);

    unsubscribe();
    window.dispatchEvent(
      new StorageEvent("storage", {
        key: HOME_USAGE_DAY_START_HOUR_STORAGE_KEY,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("orders usage day hours from the configured boundary", () => {
    expect(orderedUsageDayHours(5)).toEqual([
      5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 0, 1, 2, 3, 4,
    ]);
    expect(formatUsageDayHourLabel(2, 5)).toBe("次日02:00");
    expect(formatUsageDayHourTickLabel(4, 5)).toBe("次日04");
    expect(formatUsageDayHourLabel(9, 5)).toBe("09:00");
  });

  it("formats timestamps against the configured usage day window", () => {
    const first = new Date(2026, 3, 16, 9, 0).getTime();
    const nextDay = new Date(2026, 3, 17, 2, 0).getTime();

    expect(formatUsageDayHourMinuteFromMs(first, "2026-04-16", 5)).toBe("09:00");
    expect(formatUsageDayHourMinuteFromMs(nextDay, "2026-04-16", 5)).toBe("次日02:00");
    expect(formatUsageDayHourMinuteFromMs(nextDay, "2026-04-16", 0)).toBe("02:00");
  });
});
