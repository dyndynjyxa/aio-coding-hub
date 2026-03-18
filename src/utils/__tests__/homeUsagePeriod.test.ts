import { describe, expect, it } from "vitest";
import { DEFAULT_HOME_USAGE_PERIOD, resolveHomeUsageWindowDays } from "../homeUsagePeriod";

describe("utils/homeUsagePeriod", () => {
  it("defaults to last15", () => {
    expect(DEFAULT_HOME_USAGE_PERIOD).toBe("last15");
    expect(resolveHomeUsageWindowDays(DEFAULT_HOME_USAGE_PERIOD)).toBe(15);
  });

  it("maps rolling windows directly", () => {
    expect(resolveHomeUsageWindowDays("last7")).toBe(7);
    expect(resolveHomeUsageWindowDays("last15")).toBe(15);
    expect(resolveHomeUsageWindowDays("last30")).toBe(30);
  });

  it("maps month to the current local day-of-month", () => {
    expect(resolveHomeUsageWindowDays("month", new Date("2026-03-01T09:00:00"))).toBe(1);
    expect(resolveHomeUsageWindowDays("month", new Date("2026-03-18T09:00:00"))).toBe(18);
  });
});
