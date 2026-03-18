import { describe, expect, it } from "vitest";
import { buildUsageTokensXAxisTicks } from "../UsageTokensChart";

describe("components/UsageTokensChart", () => {
  it("shows all labels for a 7-day window", () => {
    const labels = ["03/12", "03/13", "03/14", "03/15", "03/16", "03/17", "03/18"];
    expect(buildUsageTokensXAxisTicks(labels)).toEqual(labels);
  });

  it("downsamples a 15-day window while keeping the last label", () => {
    const labels = Array.from({ length: 15 }, (_, i) => `03/${String(i + 1).padStart(2, "0")}`);

    expect(buildUsageTokensXAxisTicks(labels)).toEqual([
      "03/01",
      "03/04",
      "03/07",
      "03/10",
      "03/13",
      "03/15",
    ]);
  });

  it("keeps roughly seven ticks for a 30-day window", () => {
    const labels = Array.from({ length: 30 }, (_, i) => `03/${String(i + 1).padStart(2, "0")}`);

    expect(buildUsageTokensXAxisTicks(labels)).toEqual([
      "03/01",
      "03/06",
      "03/11",
      "03/16",
      "03/21",
      "03/26",
      "03/30",
    ]);
  });
});
