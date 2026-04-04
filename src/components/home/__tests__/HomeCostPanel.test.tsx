import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { clearTauriRuntime, setTauriRuntime } from "../../../test/utils/tauriRuntime";
import { useCustomDateRange } from "../../../hooks/useCustomDateRange";
import { useCostAnalyticsV1Query } from "../../../query/cost";
import { HomeCostPanel } from "../HomeCostPanel";

vi.mock("sonner", () => ({ toast: vi.fn() }));

// Mock recharts ResponsiveContainer to avoid resize observer issues in tests
vi.mock("recharts", async () => {
  const actual = await vi.importActual<typeof import("recharts")>("recharts");
  return {
    ...actual,
    ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
      <div data-testid="recharts-responsive-container" style={{ width: 400, height: 300 }}>
        {children}
      </div>
    ),
  };
});

vi.mock("../../../hooks/useCustomDateRange", async () => {
  const actual = await vi.importActual<typeof import("../../../hooks/useCustomDateRange")>(
    "../../../hooks/useCustomDateRange"
  );
  return { ...actual, useCustomDateRange: vi.fn() };
});

vi.mock("../../../query/cost", async () => {
  const actual = await vi.importActual<typeof import("../../../query/cost")>("../../../query/cost");
  return {
    ...actual,
    useCostAnalyticsV1Query: vi.fn(),
  };
});

describe("components/home/HomeCostPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders with data and shows summary + charts", () => {
    setTauriRuntime();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: null, endTs: null },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: {
        summary: {
          requests_total: 100,
          requests_success: 90,
          requests_failed: 10,
          cost_covered_success: 80,
          total_cost_usd: 12.34,
          avg_cost_usd_per_covered_success: 0.12,
        },
        trend: [
          {
            day: "2026-01-01",
            hour: null,
            cost_usd: 1.2,
            requests_success: 3,
            cost_covered_success: 2,
          },
        ],
        providers: [
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            requests_success: 10,
            cost_covered_success: 8,
            cost_usd: 3.21,
          },
        ],
        models: [
          {
            model: "claude-3-opus",
            requests_success: 10,
            cost_covered_success: 8,
            cost_usd: 3.21,
          },
        ],
        scatter: [
          {
            cli_key: "claude",
            provider_name: "P1",
            model: "claude-3-opus",
            requests_success: 10,
            total_cost_usd: 3.21,
            total_duration_ms: 1234,
          },
        ],
        topRequests: [
          {
            log_id: 1,
            trace_id: "t1",
            cli_key: "claude",
            method: "POST",
            path: "/v1/messages",
            requested_model: "claude-3-opus",
            provider_id: 1,
            provider_name: "P1",
            duration_ms: 1234,
            ttfb_ms: 120,
            cost_usd: 1.23,
            cost_multiplier: 1,
            created_at: Math.floor(Date.now() / 1000),
          },
          {
            log_id: 2,
            trace_id: "t2",
            cli_key: "claude",
            method: "POST",
            path: "/v1/messages",
            requested_model: " ",
            provider_id: 2,
            provider_name: "P2",
            duration_ms: 2222,
            ttfb_ms: 220,
            cost_usd: 2.34,
            cost_multiplier: 1.5,
            created_at: Math.floor(Date.now() / 1000),
          },
        ],
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeCostPanel />);

    expect(screen.getByText("总花费（已计算）")).toBeInTheDocument();
    expect(screen.getByText("$12.340000")).toBeInTheDocument();
    expect(screen.getByText("成本覆盖率")).toBeInTheDocument();
    expect(screen.getByText("花费占比")).toBeInTheDocument();
    // Check that recharts containers are rendered (we mock ResponsiveContainer)
    expect(screen.getAllByTestId("recharts-responsive-container").length).toBeGreaterThanOrEqual(3);
  });

  it("drives filter controls and triggers refetch", () => {
    setTauriRuntime();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: null, endTs: null },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    const refetch = vi.fn();
    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: {
        summary: {
          requests_total: 10,
          requests_success: 10,
          requests_failed: 0,
          cost_covered_success: 10,
          total_cost_usd: 1.23,
          avg_cost_usd_per_covered_success: 0.12,
        },
        trend: [
          {
            day: "2026-01-01",
            hour: 1,
            cost_usd: 1.2,
            requests_success: 3,
            cost_covered_success: 2,
          },
        ],
        providers: [
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            requests_success: 10,
            cost_covered_success: 8,
            cost_usd: 3.21,
          },
        ],
        models: [
          {
            model: "claude-3-opus",
            requests_success: 10,
            cost_covered_success: 8,
            cost_usd: 3.21,
          },
        ],
        scatter: [],
        topRequests: [],
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch,
    } as any);

    render(<HomeCostPanel />);

    // Top refresh button.
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(refetch).toHaveBeenCalled();

    // Filter rows.
    const filterCard = screen.getByText("筛选条件").closest("div")?.parentElement
      ?.parentElement?.parentElement;
    expect(filterCard).toBeTruthy();
    fireEvent.click(within(filterCard as HTMLElement).getByRole("button", { name: "Codex" }));

    fireEvent.click(within(filterCard as HTMLElement).getByRole("button", { name: "近 7 天" }));

    const selects = within(filterCard as HTMLElement).getAllByRole("combobox");
    const providerSelect = selects[0];
    fireEvent.change(providerSelect, { target: { value: "1" } });
    fireEvent.change(providerSelect, { target: { value: "0" } });
    fireEvent.change(providerSelect, { target: { value: "all" } });

    const modelSelect = selects[1];
    fireEvent.change(modelSelect, { target: { value: "claude-3-opus" } });
    fireEvent.change(modelSelect, { target: { value: "all" } });

    // Chart filter buttons are separate handlers.
    const trendHeader = screen.getByText("总花费趋势").parentElement?.parentElement;
    expect(trendHeader).toBeTruthy();
    fireEvent.click(
      within(trendHeader as HTMLElement).getByRole("button", { name: "Claude Code" })
    );

    const scatterHeader = screen.getByText("总成本 × 总耗时").parentElement?.parentElement;
    expect(scatterHeader).toBeTruthy();
    fireEvent.click(
      within(scatterHeader as HTMLElement).getByRole("button", { name: "Claude Code" })
    );

    // Query hook should have been called with multiple periods/filters across rerenders.
    const calls = vi.mocked(useCostAnalyticsV1Query).mock.calls;
    expect(calls.some((call) => call[0] === "weekly")).toBe(true);
    expect(calls.some((call) => call[1]?.cliKey === "codex")).toBe(true);
  });

  it("shows tauri hint when runtime is unavailable", () => {
    clearTauriRuntime();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: null, endTs: null },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeCostPanel />);
    expect(screen.getByText(/未检测到 Tauri Runtime/)).toBeInTheDocument();
  });

  it("renders custom range controls and triggers apply/clear handlers", () => {
    setTauriRuntime();

    const applyCustomRange = vi.fn();
    const clearCustomRange = vi.fn();
    const setCustomStartDate = vi.fn();
    const setCustomEndDate = vi.fn();

    vi.mocked(useCustomDateRange).mockImplementation((period: any) => {
      const custom = period === "custom";
      return {
        customStartDate: "2026-01-01",
        setCustomStartDate,
        customEndDate: "2026-01-03",
        setCustomEndDate,
        customApplied: null,
        bounds: { startTs: null, endTs: null },
        showCustomForm: custom,
        applyCustomRange,
        clearCustomRange,
      } as any;
    });

    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: "自定义" }));
    expect(screen.getByText("开始日期")).toBeInTheDocument();
    expect(screen.getByDisplayValue("2026-01-01")).toBeInTheDocument();

    fireEvent.change(screen.getByDisplayValue("2026-01-01"), { target: { value: "2026-01-02" } });
    expect(setCustomStartDate).toHaveBeenCalledWith("2026-01-02");
    fireEvent.change(screen.getByDisplayValue("2026-01-03"), { target: { value: "2026-01-04" } });
    expect(setCustomEndDate).toHaveBeenCalledWith("2026-01-04");

    fireEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(applyCustomRange).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    expect(clearCustomRange).toHaveBeenCalled();

    expect(screen.getAllByText("自定义范围：请选择日期后点击「应用」。").length).toBeGreaterThan(0);
  });

  it("toasts when cost query errors", async () => {
    setTauriRuntime();

    vi.mocked(useCustomDateRange).mockReturnValue({
      customStartDate: "",
      setCustomStartDate: vi.fn(),
      customEndDate: "",
      setCustomEndDate: vi.fn(),
      customApplied: null,
      bounds: { startTs: null, endTs: null },
      showCustomForm: false,
      applyCustomRange: vi.fn(),
      clearCustomRange: vi.fn(),
    } as any);

    const refetch = vi.fn();
    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: new Error("boom"),
      refetch,
    } as any);

    render(<HomeCostPanel />);
    await waitFor(() => {
      expect(toast).toHaveBeenCalledWith("加载花费失败：请重试（详情见页面错误信息）");
    });
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(refetch).toHaveBeenCalled();
  });

  it("renders loading skeleton cards and triggers onInvalid callback", async () => {
    setTauriRuntime();

    vi.mocked(useCustomDateRange).mockImplementation((period: any, options: any) => {
      void period;
      if (typeof options?.onInvalid === "function") {
        options.onInvalid("bad-range");
      }
      return {
        customStartDate: "",
        setCustomStartDate: vi.fn(),
        customEndDate: "",
        setCustomEndDate: vi.fn(),
        customApplied: null,
        bounds: { startTs: null, endTs: null },
        showCustomForm: false,
        applyCustomRange: vi.fn(),
        clearCustomRange: vi.fn(),
      } as any;
    });

    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: null,
      isLoading: true,
      isFetching: true,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeCostPanel />);

    expect(toast).toHaveBeenCalledWith("bad-range");
    expect(document.querySelectorAll(".animate-pulse").length).toBe(2);
  });

  it("renders charts with various data scenarios", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-03T00:00:00Z"));

    vi.mocked(useCustomDateRange).mockImplementation((period: any) => {
      const customApplied =
        period === "custom"
          ? {
              startTs: Math.floor(new Date("2026-01-01T00:00:00Z").getTime() / 1000),
              endTs: Math.floor(new Date("2026-01-03T00:00:00Z").getTime() / 1000) + 1,
              startDate: "2026-01-01",
              endDate: "2026-01-03",
            }
          : null;
      return {
        customStartDate: "2026-01-01",
        setCustomStartDate: vi.fn(),
        customEndDate: "2026-01-03",
        setCustomEndDate: vi.fn(),
        customApplied,
        bounds: { startTs: customApplied?.startTs ?? null, endTs: customApplied?.endTs ?? null },
        showCustomForm: period === "custom",
        applyCustomRange: vi.fn(),
        clearCustomRange: vi.fn(),
      } as any;
    });

    vi.mocked(useCostAnalyticsV1Query).mockReturnValue({
      data: {
        summary: {
          requests_total: 10,
          requests_success: 10,
          requests_failed: 0,
          cost_covered_success: 10,
          total_cost_usd: 12.34,
          avg_cost_usd_per_covered_success: 0.12,
        },
        trend: [
          {
            day: "2026-01-01",
            hour: null,
            cost_usd: 1,
            requests_success: 1,
            cost_covered_success: 1,
          },
          {
            day: "2026-01-02",
            hour: null,
            cost_usd: 2,
            requests_success: 1,
            cost_covered_success: 1,
          },
          {
            day: "2026-01-03",
            hour: null,
            cost_usd: 3,
            requests_success: 1,
            cost_covered_success: 1,
          },
        ],
        providers: Array.from({ length: 10 }).map((_, idx) => ({
          cli_key: "claude",
          provider_id: idx + 1,
          provider_name: `P${idx + 1}`,
          requests_success: 10,
          cost_covered_success: 8,
          cost_usd: 10 - idx,
        })),
        models: Array.from({ length: 10 }).map((_, idx) => ({
          model: `M${idx + 1}`,
          requests_success: 10,
          cost_covered_success: 8,
          cost_usd: 10 - idx,
        })),
        scatter: [
          {
            cli_key: "claude",
            provider_name: "  ",
            model: "",
            requests_success: 0,
            total_cost_usd: 3.21,
            total_duration_ms: 1234,
          },
          {
            cli_key: "claude",
            provider_name: "P1",
            model: "M1",
            requests_success: 2,
            total_cost_usd: 5,
            total_duration_ms: 1000,
          },
          {
            cli_key: "codex",
            provider_name: "P2",
            model: "M2",
            requests_success: 1,
            total_cost_usd: 6,
            total_duration_ms: 2000,
          },
        ],
        topRequests: [],
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeCostPanel />);

    // Switch to monthly and custom to hit day-key builders.
    fireEvent.click(screen.getByRole("button", { name: "本月" }));
    fireEvent.click(screen.getByRole("button", { name: "自定义" }));

    // Verify charts are rendered
    expect(screen.getByTestId("home-cost-trend-chart")).toBeInTheDocument();
    expect(screen.getByTestId("home-cost-donut-charts")).toBeInTheDocument();
    expect(screen.getByTestId("home-cost-scatter-chart")).toBeInTheDocument();

    vi.useRealTimers();
  });
});
