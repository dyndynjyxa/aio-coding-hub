import { describe, expect, it } from "vitest";
import {
  CHART_COLORS,
  createAreaGradient,
  getAxisLineStroke,
  getAxisStyle,
  getCursorStroke,
  getGridLineStyle,
  getLegendStyle,
  getTooltipStyle,
  pickPaletteColor,
} from "../chartTheme";

describe("components/charts/chartTheme", () => {
  it("picks palette color by index and falls back to hsl", () => {
    expect(pickPaletteColor(0)).toBe(CHART_COLORS.primary);
    expect(pickPaletteColor(999)).toMatch(/^hsl\(.+\)$/);
  });

  it("returns dark/light axis and grid styles", () => {
    expect(getAxisStyle(false)).toEqual(
      expect.objectContaining({ fill: "#64748b", color: "#64748b" })
    );
    expect(getAxisStyle(true)).toEqual(
      expect.objectContaining({ fill: "#94a3b8", color: "#94a3b8" })
    );

    expect(getGridLineStyle(false)).toEqual(
      expect.objectContaining({ stroke: "rgba(15, 23, 42, 0.05)" })
    );
    expect(getGridLineStyle(true)).toEqual(
      expect.objectContaining({ stroke: "rgba(148, 163, 184, 0.08)" })
    );
  });

  it("returns dark/light tooltip and legend styles", () => {
    const lightTooltip = getTooltipStyle(false);
    const darkTooltip = getTooltipStyle(true);

    expect(lightTooltip).toEqual(
      expect.objectContaining({
        backgroundColor: "rgba(255, 255, 255, 0.98)",
        border: "1px solid rgba(148, 163, 184, 0.2)",
        color: undefined,
      })
    );
    expect(darkTooltip).toEqual(
      expect.objectContaining({
        backgroundColor: "rgba(30, 41, 59, 0.98)",
        border: "1px solid rgba(71, 85, 105, 0.3)",
        color: "#e2e8f0",
      })
    );

    expect(getLegendStyle(false)).toEqual(expect.objectContaining({ color: "#475569" }));
    expect(getLegendStyle(true)).toEqual(expect.objectContaining({ color: "#94a3b8" }));
  });

  it("returns dark/light axis line and cursor colors", () => {
    expect(getAxisLineStroke(false)).toBe("rgba(15, 23, 42, 0.12)");
    expect(getAxisLineStroke(true)).toBe("rgba(148, 163, 184, 0.2)");

    expect(getCursorStroke(false)).toBe("rgba(0, 82, 255, 0.15)");
    expect(getCursorStroke(true)).toBe("rgba(100, 150, 255, 0.25)");
  });

  it("creates area gradients with expected shape", () => {
    expect(createAreaGradient("#0052FF", "g1")).toEqual({
      id: "g1",
      x1: "0",
      y1: "0",
      x2: "0",
      y2: "1",
      gradientUnits: "userSpaceOnUse",
      stops: [
        { offset: "0%", stopColor: "#0052FF", stopOpacity: 0.25 },
        { offset: "100%", stopColor: "#0052FF", stopOpacity: 0.0 },
      ],
    });
  });
});
