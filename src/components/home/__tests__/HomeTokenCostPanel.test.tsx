import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HomeTokenCostPanel } from "../HomeTokenCostPanel";
import { useUsageLeaderboardV2Query, useUsageSummaryV2Query } from "../../../query/usage";

vi.mock("../../../query/usage", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/usage")>("../../../query/usage");
  return {
    ...actual,
    useUsageSummaryV2Query: vi.fn(),
    useUsageLeaderboardV2Query: vi.fn(),
  };
});

describe("components/home/HomeTokenCostPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("prefers real data over dev preview fallback and can switch to model view", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 24,
        requests_with_usage: 24,
        requests_success: 21,
        requests_failed: 3,
        avg_duration_ms: 1200,
        avg_ttfb_ms: 320,
        avg_output_tokens_per_second: 88.4,
        input_tokens: 12000,
        output_tokens: 6000,
        io_total_tokens: 18000,
        total_tokens: 22500,
        cache_read_input_tokens: 3000,
        cache_creation_input_tokens: 1500,
        cache_creation_5m_input_tokens: 800,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "provider"
              ? [
                  {
                    key: "provider-1",
                    name: "OpenAI 主供应商",
                    requests_total: 10,
                    requests_success: 10,
                    requests_failed: 0,
                    total_tokens: 12000,
                    io_total_tokens: 10000,
                    input_tokens: 7000,
                    output_tokens: 3000,
                    cache_creation_input_tokens: 500,
                    cache_read_input_tokens: 1500,
                    avg_duration_ms: 1000,
                    avg_ttfb_ms: 260,
                    avg_output_tokens_per_second: 96.2,
                    cost_usd: 1.2,
                  },
                ]
              : [
                  {
                    key: "model-1",
                    name: "gpt-5.4",
                    requests_total: 8,
                    requests_success: 8,
                    requests_failed: 0,
                    total_tokens: 9000,
                    io_total_tokens: 7600,
                    input_tokens: 4800,
                    output_tokens: 2800,
                    cache_creation_input_tokens: 300,
                    cache_read_input_tokens: 1100,
                    avg_duration_ms: 920,
                    avg_ttfb_ms: 240,
                    avg_output_tokens_per_second: 101.5,
                    cost_usd: 0.9,
                  },
                ],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );

    render(<HomeTokenCostPanel devPreviewEnabled={true} />);

    const cachedTotalCard = screen.getByText("含缓存总 Token");
    const totalCard = screen.getByText("总 Token");
    const totalCostCard = screen.getAllByText("总花费")[0];
    const successCard = screen.getByText("成功请求");
    const cacheHitRateCard = screen.getByText("缓存命中率");
    const providerCountCard = screen.getByText("供应商数");

    expect(screen.getAllByText("总花费")).toHaveLength(2);
    expect(screen.getByText("OpenAI 主供应商")).toBeInTheDocument();
    expect(screen.getByText("18.0K")).toBeInTheDocument();
    expect(screen.getAllByText("$1.20")).toHaveLength(2);
    expect(screen.getByText("12K / 2K / 16.7%")).toBeInTheDocument();
    expect(screen.getByText("18.2%")).toBeInTheDocument();
    expect(screen.getByText("含缓存总量 / 缓存量 / 缓存占比")).toBeInTheDocument();
    expect(screen.queryByText("平均耗时")).not.toBeInTheDocument();
    expect(
      cachedTotalCard.compareDocumentPosition(totalCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      totalCard.compareDocumentPosition(totalCostCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      totalCostCard.compareDocumentPosition(successCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      successCard.compareDocumentPosition(cacheHitRateCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      cacheHitRateCard.compareDocumentPosition(providerCountCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(screen.queryByText("OpenAI Primary")).not.toBeInTheDocument();
    expect(screen.queryByText("总请求 24 / 失败 3")).not.toBeInTheDocument();
    expect(screen.queryByText("总 Token、缓存占比、总花费。")).not.toBeInTheDocument();
    expect(screen.queryByText("今天 · 1 个供应商")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "模型" }));

    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.getAllByText("$0.90")).toHaveLength(2);
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "model",
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        limit: null,
      })
    );
  });

  it("maps range filters to the expected usage query periods and bounds", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 3, 16, 10, 0, 0));

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: "最近3天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 14, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
      })
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 14, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        limit: null,
      })
    );

    fireEvent.click(screen.getByRole("button", { name: "最近7天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith("weekly", {
      startTs: null,
      endTs: null,
      cliKey: null,
      providerId: null,
    });

    fireEvent.click(screen.getByRole("button", { name: "当月" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith("monthly", {
      startTs: null,
      endTs: null,
      cliKey: null,
      providerId: null,
    });
  });

  it("renders preview rows when dev preview is enabled and queries are empty", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel devPreviewEnabled={true} />);

    expect(screen.getByText("OpenAI Primary")).toBeInTheDocument();
    expect(screen.getByText("Gemini Mirror")).toBeInTheDocument();
    expect(screen.getByText("99.0K")).toBeInTheDocument();
    expect(screen.getByText("$3.36")).toBeInTheDocument();
    expect(screen.getByText("49.2K / 7.2K / 14.6%")).toBeInTheDocument();
    expect(screen.getByText("17.0%")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "模型" }));

    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.getByText("claude-3.7-sonnet")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "最近30天" }));

    expect(screen.getByText("3.0M")).toBeInTheDocument();
    expect(screen.getByText("$100.80")).toBeInTheDocument();
  });

  it("retries summary and leaderboard queries from the error card", () => {
    const refetchSummary = vi.fn();
    const refetchLeaderboard = vi.fn();

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: new Error("summary failed"),
      refetch: refetchSummary,
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: new Error("leaderboard failed"),
      refetch: refetchLeaderboard,
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: "重试" }));

    expect(refetchSummary).toHaveBeenCalled();
    expect(refetchLeaderboard).toHaveBeenCalled();
  });
});
