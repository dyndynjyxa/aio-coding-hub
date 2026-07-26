import { describe, expect, it } from "vitest";
import {
  buildDayKeysInRangeInclusive,
  buildMonthKeysFromRows,
  buildMonthToTodayDayKeys,
  buildRecentDayKeys,
  dayKeyFromLocalDate,
  toMmDd,
} from "../dateKeys";

describe("utils/dateKeys", () => {
  it("formats day keys as MM/DD", () => {
    expect(toMmDd("2026-02-20")).toBe("02/20");
  });

  it("builds recent day keys ending today", () => {
    const keys = buildRecentDayKeys(7);
    expect(keys).toHaveLength(7);
    expect(keys[6]).toBe(dayKeyFromLocalDate(new Date()));
  });

  it("builds inclusive day keys across a month boundary", () => {
    expect(buildDayKeysInRangeInclusive("2026-02-27", "2026-03-02")).toEqual([
      "2026-02-27",
      "2026-02-28",
      "2026-03-01",
      "2026-03-02",
    ]);
  });

  it("returns empty for invalid or reversed ranges", () => {
    expect(buildDayKeysInRangeInclusive("not-a-date", "2026-03-02")).toEqual([]);
    expect(buildDayKeysInRangeInclusive("2026-03-02", "not-a-date")).toEqual([]);
    expect(buildDayKeysInRangeInclusive("2026-03-02", "2026-03-01")).toEqual([]);
  });

  it("builds day keys from the 1st of the current month through today", () => {
    const keys = buildMonthToTodayDayKeys();
    const now = new Date();
    expect(keys).toHaveLength(now.getDate());
    expect(keys[0]).toBe(dayKeyFromLocalDate(new Date(now.getFullYear(), now.getMonth(), 1)));
    expect(keys[keys.length - 1]).toBe(dayKeyFromLocalDate(now));
  });

  it("dedupes, sorts, and filters month keys from rows", () => {
    const rows = [
      { day: "2026-02" },
      { day: "2026-01" },
      { day: "2026-02" },
      { day: "2026-02-20" },
      { day: "" },
    ];
    expect(buildMonthKeysFromRows(rows)).toEqual(["2026-01", "2026-02"]);
  });
});
