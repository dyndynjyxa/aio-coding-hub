import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderMetricsTrendRowV1 } from "../../../services/usage/usage";

vi.mock("../../../components/UsageProviderMetricsTrendChart", () => ({
  UsageProviderMetricsTrendChart: ({
    metric,
    rows,
    period,
  }: {
    metric: string;
    rows: unknown[];
    period: string;
  }) => (
    <div
      data-testid="metrics-chart"
      data-metric={metric}
      data-rows={rows.length}
      data-period={period}
    />
  ),
}));

import { MetricsTrendBody } from "../UsageDataPanelBodies";

const sampleRow: UsageProviderMetricsTrendRowV1 = {
  day: "2026-02-20",
  hour: null,
  key: "codex:1",
  name: "codex/OpenAI",
  avg_duration_ms: 1200,
  avg_ttfb_ms: 300,
  avg_output_tokens_per_second: 42.5,
  requests_success: 10,
};

function renderBody(overrides: Partial<Parameters<typeof MetricsTrendBody>[0]> = {}) {
  return render(
    <MetricsTrendBody
      metricsTrendLoading={false}
      metricsTrendRows={[]}
      errorText={null}
      customPending={false}
      period="weekly"
      customApplied={null}
      {...overrides}
    />
  );
}

describe("pages/usage/MetricsTrendBody", () => {
  it("shows skeleton while loading with no data yet", () => {
    const { container } = renderBody({ metricsTrendLoading: true });
    expect(container.querySelector(".animate-pulse")).not.toBeNull();
    expect(screen.queryByTestId("metrics-chart")).toBeNull();
    // metric toggle stays visible in every state
    expect(screen.getByRole("tab", { name: "耗时" })).toBeTruthy();
  });

  it("shows retry hint when empty due to error", () => {
    renderBody({ errorText: "boom" });
    expect(screen.getByText(/加载失败/)).toBeTruthy();
    expect(screen.queryByTestId("metrics-chart")).toBeNull();
  });

  it("shows apply hint when custom range is pending", () => {
    renderBody({ customPending: true });
    expect(screen.getByText(/自定义范围/)).toBeTruthy();
  });

  it("shows empty placeholder when no data", () => {
    renderBody();
    expect(screen.getByText("暂无可展示的指标数据。")).toBeTruthy();
  });

  it("keeps chart visible while refetching with existing rows", () => {
    const { container } = renderBody({
      metricsTrendLoading: true,
      metricsTrendRows: [sampleRow],
    });
    expect(screen.getByTestId("metrics-chart")).toBeTruthy();
    expect(container.querySelector(".animate-pulse")).toBeNull();
  });

  it("switches metric and hint via tabs", () => {
    renderBody({ metricsTrendRows: [sampleRow] });

    const chart = screen.getByTestId("metrics-chart");
    expect(chart.getAttribute("data-metric")).toBe("duration");
    expect(chart.getAttribute("data-rows")).toBe("1");
    expect(screen.getByText(/^平均耗时=/)).toBeTruthy();

    fireEvent.click(screen.getByRole("tab", { name: "首字" }));
    expect(screen.getByTestId("metrics-chart").getAttribute("data-metric")).toBe("ttfb");
    expect(screen.getByText(/^平均首字=/)).toBeTruthy();

    fireEvent.click(screen.getByRole("tab", { name: "速率" }));
    expect(screen.getByTestId("metrics-chart").getAttribute("data-metric")).toBe("rate");
    expect(screen.getByText(/^平均速率=/)).toBeTruthy();
  });
});
