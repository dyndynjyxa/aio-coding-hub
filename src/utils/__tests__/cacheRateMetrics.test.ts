import { describe, expect, it } from "vitest";
import { computeCacheHitRate, computeCacheHitRateDenomTokens } from "../cacheRateMetrics";

describe("utils/cacheRateMetrics", () => {
  it("computes denom and hit rate (includes cache creation)", () => {
    expect(computeCacheHitRateDenomTokens(70, 10, 30)).toBe(110);
    expect(computeCacheHitRate(70, 10, 30)).toBeCloseTo(30 / 110);
  });

  it("handles non-finite and negative inputs", () => {
    expect(computeCacheHitRateDenomTokens(-1, -2, -3)).toBe(0);
    expect(computeCacheHitRateDenomTokens(undefined, 1, 2)).toBe(3);
    expect(Number.isNaN(computeCacheHitRate(0, 0, 0))).toBe(true);
    expect(computeCacheHitRate(0, 0, 10)).toBe(1);
  });
});
