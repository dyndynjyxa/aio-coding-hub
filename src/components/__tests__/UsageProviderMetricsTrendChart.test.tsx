import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderMetricsTrendRowV1 } from "../../services/usage/usage";
import type { UsageTrendMetric } from "../UsageProviderMetricsTrendChart";

vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => ({ theme: "light", resolvedTheme: "light", setTheme: vi.fn() }),
}));

vi.mock("../charts/lazyRecharts", () => {
  const renderTooltipContent = (content: any, props: any) => {
    if (!content || typeof content.type !== "function") return null;
    const TooltipContent = content.type;
    return <TooltipContent {...content.props} {...props} />;
  };

  const payload = [
    {
      dataKey: "openai",
      name: "codex/OpenAI",
      color: "#22c55e",
      value: 1200,
      payload: { openai_meta: { requestsSuccess: 20 } },
    },
    // filtered out by the tooltip: empty key, missing meta, non-finite value
    { dataKey: "", value: 1, payload: {} },
    { dataKey: "missing-meta", value: 1, payload: {} },
    { dataKey: "nan", value: Number.NaN, payload: { nan_meta: {} } },
  ];

  return {
    CartesianGrid: () => <g data-testid="grid" />,
    Legend: () => <div data-testid="legend" />,
    Line: ({ dataKey }: any) => <path data-testid={`line-${dataKey}`} />,
    LineChart: ({ children, data }: any) => (
      <div data-testid="line-chart" data-points={data?.length ?? 0}>
        {children}
      </div>
    ),
    ResponsiveContainer: ({ children }: any) => <div data-testid="responsive">{children}</div>,
    Tooltip: ({ content }: any) => (
      <div data-testid="tooltip">
        {renderTooltipContent(content, { active: false, payload: null, label: "empty" })}
        {renderTooltipContent(content, { active: true, payload: [], label: "empty-list" })}
        {renderTooltipContent(content, { active: true, label: "point", payload })}
      </div>
    ),
    XAxis: ({ ticks }: any) => <div data-testid="x-axis" data-ticks={ticks?.join(",") ?? ""} />,
    YAxis: ({ tickFormatter }: any) => (
      <div data-testid="y-axis" data-formatted={tickFormatter ? tickFormatter(1200) : ""} />
    ),
  };
});

import { UsageProviderMetricsTrendChart } from "../UsageProviderMetricsTrendChart";

const sampleRow: UsageProviderMetricsTrendRowV1 = {
  day: "2026-02-20",
  hour: null,
  key: "openai",
  name: "codex/OpenAI",
  avg_duration_ms: 1200,
  avg_ttfb_ms: 300,
  avg_output_tokens_per_second: 42.5,
  requests_success: 10,
};

describe("components/UsageProviderMetricsTrendChart", () => {
  const metrics: UsageTrendMetric[] = ["duration", "ttfb", "rate"];

  it("renders 7 empty day buckets and no lines without data", () => {
    const { container, getByTestId } = render(
      <UsageProviderMetricsTrendChart
        rows={[]}
        period="weekly"
        metric="duration"
        customApplied={null}
      />
    );
    expect(getByTestId("line-chart").getAttribute("data-points")).toBe("7");
    expect(container.querySelectorAll('path[data-testid^="line-"]')).toHaveLength(0);
  });

  it("renders tooltip items sorted with meta and filters invalid payload entries", () => {
    const { getByText, queryByText } = render(
      <UsageProviderMetricsTrendChart
        rows={[sampleRow]}
        period="weekly"
        metric="duration"
        customApplied={null}
      />
    );
    // only the valid payload entry survives: 1 provider, name + ok count shown
    expect(getByText("供应商: 1")).toBeTruthy();
    expect(getByText("codex/OpenAI")).toBeTruthy();
    expect(getByText("ok 20")).toBeTruthy();
    expect(queryByText(/missing-meta|nan/)).toBeNull();
  });

  for (const metric of metrics) {
    it(`renders weekly data for metric=${metric}`, () => {
      const rows: UsageProviderMetricsTrendRowV1[] = [
        sampleRow,
        { ...sampleRow, day: "2026-02-21", avg_duration_ms: 900 },
        { ...sampleRow, key: "anthropic", name: "claude/Anthropic" },
      ];
      const { getByTestId } = render(
        <UsageProviderMetricsTrendChart
          rows={rows}
          period="weekly"
          metric={metric}
          customApplied={null}
        />
      );
      // Two provider lines present.
      expect(getByTestId("line-openai")).toBeTruthy();
      expect(getByTestId("line-anthropic")).toBeTruthy();
    });
  }

  it("formats rate axis as tokens/second", () => {
    const { getByTestId } = render(
      <UsageProviderMetricsTrendChart
        rows={[sampleRow]}
        period="weekly"
        metric="rate"
        customApplied={null}
      />
    );
    // rate formatter must not render a duration/ms string
    expect(getByTestId("y-axis").getAttribute("data-formatted")).not.toContain("ms");
  });

  it("renders 24 hour buckets with every-2nd tick for daily period", () => {
    const rows: UsageProviderMetricsTrendRowV1[] = [
      { ...sampleRow, hour: 10 },
      { ...sampleRow, hour: 14 },
    ];
    const { getByTestId } = render(
      <UsageProviderMetricsTrendChart
        rows={rows}
        period="daily"
        metric="duration"
        customApplied={null}
      />
    );
    expect(getByTestId("line-chart").getAttribute("data-points")).toBe("24");
    expect(getByTestId("x-axis").getAttribute("data-ticks")).toBe(
      "00,02,04,06,08,10,12,14,16,18,20,22"
    );
    expect(getByTestId("line-openai")).toBeTruthy();
  });

  it("renders month buckets derived from data for allTime period", () => {
    const { getByTestId } = render(
      <UsageProviderMetricsTrendChart
        rows={[{ ...sampleRow, day: "2026-02" }]}
        period="allTime"
        metric="ttfb"
        customApplied={null}
      />
    );
    expect(getByTestId("line-chart").getAttribute("data-points")).toBe("1");
    expect(getByTestId("x-axis").getAttribute("data-ticks")).toBe("2026-02");
    expect(getByTestId("line-openai")).toBeTruthy();
  });

  it("renders inclusive day buckets with MM/DD ticks for custom date range", () => {
    const { getByTestId } = render(
      <UsageProviderMetricsTrendChart
        rows={[sampleRow]}
        period="custom"
        metric="duration"
        customApplied={{
          startDate: "2026-02-15",
          endDate: "2026-02-25",
          startTs: 1739577600,
          endTs: 1740441600,
        }}
      />
    );
    // 2026-02-15..25 inclusive = 11 buckets, every 3rd label ticked
    expect(getByTestId("line-chart").getAttribute("data-points")).toBe("11");
    expect(getByTestId("x-axis").getAttribute("data-ticks")).toBe("02/15,02/18,02/21,02/24");
  });
});
