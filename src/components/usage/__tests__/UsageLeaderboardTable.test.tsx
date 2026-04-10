import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { UsageLeaderboardRow, UsageSummary } from "../../../services/usage/usage";
import { UsageLeaderboardTable } from "../UsageLeaderboardTable";

const SUMMARY: UsageSummary = {
  requests_total: 0,
  requests_with_usage: 0,
  requests_success: 0,
  requests_failed: 0,
  avg_duration_ms: null,
  avg_ttfb_ms: null,
  avg_output_tokens_per_second: null,
  input_tokens: 0,
  output_tokens: 0,
  io_total_tokens: 0,
  total_tokens: 0,
  cache_read_input_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_creation_5m_input_tokens: 0,
};

const ROW_A: UsageLeaderboardRow = {
  key: "row-a",
  name: "Row A",
  requests_total: 0,
  requests_success: 0,
  requests_failed: 0,
  total_tokens: 0,
  io_total_tokens: 0,
  input_tokens: 0,
  output_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_read_input_tokens: 0,
  avg_duration_ms: null,
  avg_ttfb_ms: null,
  avg_output_tokens_per_second: null,
  cost_usd: null,
};

const ROW_B: UsageLeaderboardRow = {
  key: "row-b",
  name: "Row B",
  requests_total: 10,
  requests_success: 7,
  requests_failed: 3,
  total_tokens: 1200,
  io_total_tokens: 1000,
  input_tokens: 400,
  output_tokens: 600,
  cache_creation_input_tokens: 20,
  cache_read_input_tokens: 30,
  avg_duration_ms: 1234,
  avg_ttfb_ms: 120,
  avg_output_tokens_per_second: 45.6,
  cost_usd: 2.5,
};

describe("components/usage/UsageLeaderboardTable", () => {
  it("renders empty state without error", () => {
    render(<UsageLeaderboardTable rows={[]} summary={null} totalCostUsd={0} errorText={null} />);
    expect(screen.getByText("暂无用量数据。请先通过网关发起请求。")).toBeInTheDocument();
  });

  it("renders empty state with error", () => {
    render(<UsageLeaderboardTable rows={[]} summary={null} totalCostUsd={0} errorText="boom" />);
    expect(screen.getByText('加载失败：暂无可展示的数据。请点击上方"重试"。')).toBeInTheDocument();
  });

  it("renders empty leaderboard row when summary exists", () => {
    render(<UsageLeaderboardTable rows={[]} summary={SUMMARY} totalCostUsd={0} errorText={null} />);
    expect(screen.getByText("暂无 Leaderboard 数据。")).toBeInTheDocument();
    expect(screen.getByText("总计")).toBeInTheDocument();
  });

  it("renders rows and calculates $/1K token when possible", () => {
    render(
      <UsageLeaderboardTable
        rows={[ROW_A, ROW_B]}
        summary={null}
        totalCostUsd={2.5}
        errorText={null}
      />
    );

    expect(screen.getByText("Row A")).toBeInTheDocument();
    expect(screen.getByText("Row B")).toBeInTheDocument();
    expect(screen.getAllByText("$2.500000").length).toBeGreaterThan(0);
    expect(screen.getAllByRole("progressbar").length).toBe(2);
  });
});
