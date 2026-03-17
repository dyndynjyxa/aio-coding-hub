import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { dayKeyFromLocalDate } from "../../../utils/dateKeys";
import { HomeUsageSection } from "../HomeUsageSection";

vi.mock("../../UsageHeatmap15d", () => ({
  UsageHeatmap15d: () => <div>heatmap</div>,
}));

vi.mock("../../UsageTokensChart", () => ({
  UsageTokensChart: () => <div>tokens-chart</div>,
}));

describe("components/home/HomeUsageSection", () => {
  it("shows today's token total in the usage card header", () => {
    const today = dayKeyFromLocalDate(new Date());

    render(
      <HomeUsageSection
        showHeatmap={true}
        usageHeatmapRows={[
          {
            day: today,
            hour: 9,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 600_000,
          },
          {
            day: today,
            hour: 14,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 900_000,
          },
          {
            day: "2000-01-01",
            hour: 8,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 5_000_000,
          },
        ]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
      />
    );

    expect(screen.getByText("今日用量")).toBeInTheDocument();
    expect(screen.getByText("1.5M")).toBeInTheDocument();
  });

  it("keeps today's token total visible when heatmap is hidden", () => {
    const today = dayKeyFromLocalDate(new Date());

    render(
      <HomeUsageSection
        showHeatmap={false}
        usageHeatmapRows={[
          {
            day: today,
            hour: 10,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 2_400,
          },
        ]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
      />
    );

    expect(screen.queryByText("heatmap")).not.toBeInTheDocument();
    expect(screen.getByText("今日用量")).toBeInTheDocument();
    expect(screen.getByText("2.4K")).toBeInTheDocument();
  });
});
