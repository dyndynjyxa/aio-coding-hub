import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { dayKeyFromLocalDate } from "../../../utils/dateKeys";
import { HomeUsageSection } from "../HomeUsageSection";

const heatmapSpy = vi.fn();
const tokensChartSpy = vi.fn();

vi.mock("../../UsageHeatmap15d", () => ({
  UsageHeatmap15d: (props: any) => {
    heatmapSpy(props);
    return <div>heatmap</div>;
  },
}));

vi.mock("../../UsageTokensChart", () => ({
  UsageTokensChart: (props: any) => {
    tokensChartSpy(props);
    return <div>tokens-chart</div>;
  },
}));

describe("components/home/HomeUsageSection", () => {
  beforeEach(() => {
    heatmapSpy.mockClear();
    tokensChartSpy.mockClear();
  });

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

  it("supports rendering only the heatmap card", () => {
    render(
      <HomeUsageSection
        showHeatmap={true}
        showUsageChart={false}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
      />
    );

    expect(screen.getByText("heatmap")).toBeInTheDocument();
    expect(screen.queryByText("tokens-chart")).not.toBeInTheDocument();
    expect(screen.queryByText("今日用量")).not.toBeInTheDocument();
  });

  it("renders preview usage data when dev preview is enabled and rows are empty", () => {
    render(
      <HomeUsageSection
        devPreviewEnabled={true}
        showHeatmap={true}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
      />
    );

    expect(screen.getByText("heatmap")).toBeInTheDocument();
    expect(screen.getByText("tokens-chart")).toBeInTheDocument();
    expect(screen.getByText("今日用量")).toBeInTheDocument();
    expect(screen.getByText(/\d+(\.\d)?M/)).toBeInTheDocument();
  });

  it("passes the configured usage window days to both charts", () => {
    render(
      <HomeUsageSection
        showHeatmap={true}
        usageWindowDays={30}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
      />
    );

    const lastHeatmapCall = heatmapSpy.mock.calls[heatmapSpy.mock.calls.length - 1];
    const lastTokensChartCall = tokensChartSpy.mock.calls[tokensChartSpy.mock.calls.length - 1];

    expect(lastHeatmapCall?.[0]).toEqual(expect.objectContaining({ days: 30 }));
    expect(lastTokensChartCall?.[0]).toEqual(expect.objectContaining({ days: 30 }));
  });
});
