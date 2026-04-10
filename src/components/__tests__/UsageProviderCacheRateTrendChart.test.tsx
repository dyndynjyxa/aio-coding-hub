import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderCacheRateTrendRowV1 } from "../../services/usage/usage";

vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => ({ theme: "light", resolvedTheme: "light", setTheme: vi.fn() }),
}));

import { UsageProviderCacheRateTrendChart } from "../UsageProviderCacheRateTrendChart";

const sampleRow: UsageProviderCacheRateTrendRowV1 = {
  day: "2026-02-20",
  hour: null,
  key: "openai",
  name: "OpenAI",
  denom_tokens: 200,
  cache_read_input_tokens: 100,
  requests_success: 10,
};

describe("components/UsageProviderCacheRateTrendChart", () => {
  it("renders without data", () => {
    const { container } = render(
      <UsageProviderCacheRateTrendChart rows={[]} period="weekly" customApplied={null} />
    );
    expect(container).toBeTruthy();
  });

  it("renders with weekly data", () => {
    const rows: UsageProviderCacheRateTrendRowV1[] = [
      sampleRow,
      { ...sampleRow, day: "2026-02-21", cache_read_input_tokens: 200 },
      { ...sampleRow, key: "anthropic", name: "Anthropic", day: "2026-02-20" },
    ];
    const { container } = render(
      <UsageProviderCacheRateTrendChart rows={rows} period="weekly" customApplied={null} />
    );
    expect(container).toBeTruthy();
  });

  it("renders with daily (hourly) period", () => {
    const rows: UsageProviderCacheRateTrendRowV1[] = [
      { ...sampleRow, hour: 10 },
      { ...sampleRow, hour: 14 },
    ];
    const { container } = render(
      <UsageProviderCacheRateTrendChart rows={rows} period="daily" customApplied={null} />
    );
    expect(container).toBeTruthy();
  });

  it("renders with monthly period", () => {
    const { container } = render(
      <UsageProviderCacheRateTrendChart rows={[sampleRow]} period="monthly" customApplied={null} />
    );
    expect(container).toBeTruthy();
  });

  it("renders with allTime period", () => {
    const { container } = render(
      <UsageProviderCacheRateTrendChart rows={[sampleRow]} period="allTime" customApplied={null} />
    );
    expect(container).toBeTruthy();
  });

  it("renders with custom date range", () => {
    const { container } = render(
      <UsageProviderCacheRateTrendChart
        rows={[sampleRow]}
        period="custom"
        customApplied={{
          startDate: "2026-02-15",
          endDate: "2026-02-25",
          startTs: 1739577600,
          endTs: 1740441600,
        }}
      />
    );
    expect(container).toBeTruthy();
  });
});
