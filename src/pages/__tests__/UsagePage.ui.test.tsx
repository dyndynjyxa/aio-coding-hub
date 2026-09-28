import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { UsagePage } from "../UsagePage";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { useCustomDateRange } from "../../hooks/useCustomDateRange";
import {
  useUsageLeaderboardV2Query,
  useUsageProviderCacheRateTrendV1Query,
  useUsageSummaryV2Query,
} from "../../query/usage";
import { useProvidersListQuery } from "../../query/providers";

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../hooks/useCustomDateRange", async () => {
  const actual = await vi.importActual<typeof import("../../hooks/useCustomDateRange")>(
    "../../hooks/useCustomDateRange"
  );
  return { ...actual, useCustomDateRange: vi.fn() };
});

vi.mock("../../query/usage", async () => {
  const actual = await vi.importActual<typeof import("../../query/usage")>("../../query/usage");
  return {
    ...actual,
    useUsageSummaryV2Query: vi.fn(),
    useUsageLeaderboardV2Query: vi.fn(),
    useUsageProviderCacheRateTrendV1Query: vi.fn(),
  };
});

vi.mock("../../query/providers", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/providers")>("../../query/providers");
  return { ...actual, useProvidersListQuery: vi.fn() };
});

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

function mockProvidersListQuery() {
  vi.mocked(useProvidersListQuery).mockImplementation((cliKey) => {
    if (cliKey === "claude") {
      return {
        data: [{ id: 11, cli_key: "claude", name: "Anthropic A" }],
        isFetching: false,
      } as any;
    }
    if (cliKey === "codex") {
      return {
        data: [{ id: 21, cli_key: "codex", name: "OpenAI A" }],
        isFetching: false,
      } as any;
    }
    return { data: [], isFetching: false } as any;
  });
}

describe("pages/UsagePage (ui)", () => {
  it("renders usage tab table when data is available", () => {
    setTauriRuntime();
    mockProvidersListQuery();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: 10, endTs: 20 },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 10,
        requests_with_usage: 8,
        requests_success: 9,
        io_total_tokens: 1000,
        input_tokens: 400,
        output_tokens: 600,
        total_tokens: 1100,
        cache_creation_input_tokens: 10,
        cache_read_input_tokens: 20,
        cache_creation_5m_input_tokens: 5,
        avg_duration_ms: 100,
        avg_ttfb_ms: 20,
        avg_output_tokens_per_second: 12.3,
        cost_usd: 0.0,
      },
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "p1",
          name: "Provider-1",
          requests_total: 4,
          requests_success: 3,
          io_total_tokens: 700,
          input_tokens: 300,
          output_tokens: 400,
          total_tokens: 800,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          cache_creation_5m_input_tokens: 0,
          avg_duration_ms: 120,
          avg_ttfb_ms: 30,
          avg_output_tokens_per_second: 9.5,
          cost_usd: 1.23,
        },
      ],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageProviderCacheRateTrendV1Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<UsagePage />);

    expect(screen.getByRole("tab", { name: "用量" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "缓存走势图" })).toBeInTheDocument();
    expect(screen.getByText("Provider-1")).toBeInTheDocument();
    expect(screen.getAllByText("$1.230000").length).toBeGreaterThan(0);
    expect(screen.getByText("总计")).toBeInTheDocument();
    expect(screen.getByText("缓存 / 命中率")).toBeInTheDocument();
    const summaryCalls = vi.mocked(useUsageSummaryV2Query).mock.calls;
    const leaderboardCalls = vi.mocked(useUsageLeaderboardV2Query).mock.calls;
    expect(summaryCalls[summaryCalls.length - 1]?.[1]).not.toHaveProperty("dayStartHour");
    expect(leaderboardCalls[leaderboardCalls.length - 1]?.[2]).not.toHaveProperty("dayStartHour");
  });

  it("switches to cache trend tab, locks provider scope, and restores previous scope", () => {
    setTauriRuntime();
    mockProvidersListQuery();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: 10, endTs: 20 },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageProviderCacheRateTrendV1Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<UsagePage />);

    fireEvent.click(screen.getByRole("button", { name: "CLI" }));
    expect(screen.getByLabelText("供应商筛选")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "缓存走势图" }));
    expect(screen.queryByRole("button", { name: "CLI" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "用量" }));
    expect(screen.getByLabelText("供应商筛选")).toBeInTheDocument();
  });

  it("shows custom range form and wires apply/clear handlers", async () => {
    setTauriRuntime();
    mockProvidersListQuery();

    const applyCustomRange = vi.fn();
    const clearCustomRange = vi.fn();
    const setCustomStartDate = vi.fn();
    const setCustomEndDate = vi.fn();

    vi.mocked(useCustomDateRange).mockImplementation((period: any) => {
      const custom = period === "custom";
      return {
        customStartDate: "2026-01-01",
        setCustomStartDate,
        customEndDate: "2026-01-02",
        setCustomEndDate,
        customApplied: custom
          ? { startDate: "2026-01-01", endDate: "2026-01-02", startTs: 1, endTs: 2 }
          : null,
        bounds: { startTs: 1, endTs: 2 },
        showCustomForm: custom,
        applyCustomRange,
        clearCustomRange,
      } as any;
    });

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageProviderCacheRateTrendV1Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<UsagePage />);

    fireEvent.click(screen.getByRole("button", { name: "自定义" }));
    expect(await screen.findByLabelText("开始日期")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("开始日期"), { target: { value: "2026-01-03" } });
    expect(setCustomStartDate).toHaveBeenCalledWith("2026-01-03");

    fireEvent.change(screen.getByLabelText("结束日期"), { target: { value: "2026-01-04" } });
    expect(setCustomEndDate).toHaveBeenCalledWith("2026-01-04");

    fireEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(applyCustomRange).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    expect(clearCustomRange).toHaveBeenCalled();
  });

  it("applies provider filter and clears it after switching to an incompatible CLI", async () => {
    setTauriRuntime();
    mockProvidersListQuery();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: 1, endTs: 2 },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageProviderCacheRateTrendV1Query).mockReturnValue({
      data: [],
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    renderWithProviders(<UsagePage />);

    fireEvent.change(screen.getByLabelText("供应商筛选"), { target: { value: "11" } });

    expect(useUsageSummaryV2Query).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ cliKey: null, providerId: 11 }),
      expect.objectContaining({ enabled: true })
    );

    fireEvent.click(screen.getByRole("button", { name: "Codex" }));

    await waitFor(() => {
      expect(screen.getByLabelText("供应商筛选")).toHaveValue("all");
    });
    expect(useUsageSummaryV2Query).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ cliKey: "codex", providerId: null }),
      expect.objectContaining({ enabled: true })
    );
  });
});
