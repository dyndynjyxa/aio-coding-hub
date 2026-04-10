import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ProviderLimitUsageRow } from "../../../services/providers/providerLimitUsage";
import { HomeProviderLimitPanel, HomeProviderLimitPanelContent } from "../HomeProviderLimitPanel";

function makeRow(partial: Partial<ProviderLimitUsageRow>): ProviderLimitUsageRow {
  return {
    cli_key: "claude",
    provider_id: 1,
    provider_name: "P",
    enabled: true,
    limit_5h_usd: null,
    limit_daily_usd: null,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00:00",
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    usage_5h_usd: 0,
    usage_daily_usd: 0,
    usage_weekly_usd: 0,
    usage_monthly_usd: 0,
    usage_total_usd: 0,
    window_5h_start_ts: 1_700_000_000,
    window_daily_start_ts: 1_700_000_000,
    window_weekly_start_ts: 1_700_000_000,
    window_monthly_start_ts: 1_700_000_000,
    ...partial,
  };
}

describe("components/home/HomeProviderLimitPanel", () => {
  it("renders loading / unavailable / empty states (content)", () => {
    const { rerender } = render(
      <HomeProviderLimitPanelContent rows={[]} loading={true} available={true} />
    );
    expect(screen.getByText("加载中...")).toBeInTheDocument();

    rerender(<HomeProviderLimitPanelContent rows={[]} loading={false} available={false} />);
    expect(screen.getByText("数据不可用")).toBeInTheDocument();

    rerender(<HomeProviderLimitPanelContent rows={[]} loading={false} available={true} />);
    expect(screen.getByText("暂无配置限额的供应商")).toBeInTheDocument();
  });

  it("renders rows, sorts them, and shows labels + warnings", () => {
    const rows: ProviderLimitUsageRow[] = [
      makeRow({
        cli_key: "claude",
        provider_id: 1,
        provider_name: "Beta",
        enabled: true,
        limit_5h_usd: 10,
        usage_5h_usd: 8,
        limit_daily_usd: 100,
        usage_daily_usd: 10,
        daily_reset_mode: "fixed",
        limit_weekly_usd: 100,
        usage_weekly_usd: 0,
        limit_monthly_usd: 100,
        usage_monthly_usd: 80, // warning: percent >= 0.8
        limit_total_usd: 0, // covers limit==0 -> percent 0 branch
        usage_total_usd: 999,
      }),
      makeRow({
        cli_key: "claude",
        provider_id: 2,
        provider_name: "Alpha",
        enabled: false,
        limit_daily_usd: 10,
        usage_daily_usd: 1,
        daily_reset_mode: "rolling",
      }),
      makeRow({
        cli_key: "codex",
        provider_id: 3,
        provider_name: "Charlie",
        enabled: true,
        limit_daily_usd: 10,
        usage_daily_usd: 9,
        daily_reset_mode: "rolling",
      }),
    ];

    render(<HomeProviderLimitPanelContent rows={rows} loading={false} available={true} />);

    expect(screen.getByText("5h")).toBeInTheDocument();
    expect(screen.getByText("Daily")).toBeInTheDocument();
    expect(screen.getAllByText("24h").length).toBeGreaterThan(0);
    expect(screen.getByText("Weekly")).toBeInTheDocument();
    expect(screen.getByText("Monthly")).toBeInTheDocument();
    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getByText("$8 / $10")).toBeInTheDocument();

    expect(screen.getByText("已禁用")).toBeInTheDocument();
    expect(screen.getAllByText("接近限额").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/→/).length).toBeGreaterThan(0);

    const alpha = screen.getByText("Alpha");
    const beta = screen.getByText("Beta");
    expect(alpha.compareDocumentPosition(beta) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("supports refresh button and disabled state (wrapper)", () => {
    const onRefresh = vi.fn();
    const rows = [makeRow({ provider_id: 1 }), makeRow({ provider_id: 2, cli_key: "codex" })];

    const { rerender } = render(
      <HomeProviderLimitPanel
        rows={rows}
        loading={false}
        available={true}
        onRefresh={onRefresh}
        refreshing={false}
      />
    );

    expect(screen.getByText("供应商限额")).toBeInTheDocument();
    expect(screen.getByText("2 个供应商")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(onRefresh).toHaveBeenCalledTimes(1);

    rerender(
      <HomeProviderLimitPanel
        rows={rows}
        loading={false}
        available={true}
        onRefresh={onRefresh}
        refreshing={true}
      />
    );

    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
  });
});
