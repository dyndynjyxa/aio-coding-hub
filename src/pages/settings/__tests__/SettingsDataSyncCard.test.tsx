import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ModelPricesSyncReport } from "../../../services/usage/modelPrices";
import { SettingsDataSyncCard } from "../SettingsDataSyncCard";

function report(
  status: ModelPricesSyncReport["status"],
  overrides: Partial<ModelPricesSyncReport> = {}
): ModelPricesSyncReport {
  return {
    status,
    inserted: 0,
    updated: 0,
    unchanged: 0,
    total: 0,
    error: status === "failed" ? "fetch failed" : null,
    ...overrides,
  };
}

function renderCard(overrides: Record<string, unknown> = {}) {
  const props = {
    about: { run_mode: "installed" } as any,
    lastModelPricesSyncError: null,
    lastModelPricesSyncReport: null,
    lastModelPricesSyncTime: null,
    openModelPriceAliasesDialog: vi.fn(),
    todayRequestsAvailable: "available" as const,
    todayRequestsTotal: 9,
    syncingModelPrices: false,
    syncModelPrices: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };

  return {
    ...render(<SettingsDataSyncCard {...props} />),
    props,
  };
}

describe("pages/settings/SettingsDataSyncCard", () => {
  it("uses one normal sync action and keeps alias config gated by app availability", () => {
    const { props } = renderCard({ about: null, todayRequestsAvailable: "checking" });

    expect(screen.getByText("未同步")).toBeInTheDocument();
    expect(screen.getByText("加载中…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "配置" })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "强制" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "同步" }));
    expect(props.syncModelPrices).toHaveBeenCalledWith();
  });

  it("shows failed attempts with relative attempt time", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-24T12:00:00Z"));

    renderCard({
      lastModelPricesSyncError: "boom",
      lastModelPricesSyncTime: Date.now() - 5 * 60_000,
      todayRequestsAvailable: "unavailable",
      todayRequestsTotal: null,
    });

    expect(screen.getByText("同步失败")).toBeInTheDocument();
    expect(screen.getByText("5 分钟前 · 尝试")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    vi.useRealTimers();
  });

  it("distinguishes no-change, updated counts, and failed syncs", () => {
    const { rerender } = renderCard({
      lastModelPricesSyncReport: report("not_modified"),
      lastModelPricesSyncTime: Date.now(),
      todayRequestsTotal: null,
    });

    expect(screen.getByText("无变更")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();

    rerender(
      <SettingsDataSyncCard
        about={{ run_mode: "installed" } as any}
        lastModelPricesSyncError={null}
        lastModelPricesSyncReport={report("updated", { inserted: 3, updated: 2, total: 48 })}
        lastModelPricesSyncTime={Date.now()}
        openModelPriceAliasesDialog={vi.fn()}
        todayRequestsAvailable="available"
        todayRequestsTotal={18}
        syncingModelPrices
        syncModelPrices={vi.fn().mockResolvedValue(undefined)}
      />
    );

    expect(screen.getByText("+3 / ~2 · 共 48 条")).toBeInTheDocument();
    expect(screen.getByText(/更新$/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "同步中" })).toBeDisabled();
    expect(screen.getByText("18")).toBeInTheDocument();

    rerender(
      <SettingsDataSyncCard
        about={{ run_mode: "installed" } as any}
        lastModelPricesSyncError={null}
        lastModelPricesSyncReport={report("failed")}
        lastModelPricesSyncTime={Date.now()}
        openModelPriceAliasesDialog={vi.fn()}
        todayRequestsAvailable="available"
        todayRequestsTotal={18}
        syncingModelPrices={false}
        syncModelPrices={vi.fn().mockResolvedValue(undefined)}
      />
    );
    expect(screen.getByText("同步失败")).toBeInTheDocument();
    expect(screen.getByText(/尝试$/)).toBeInTheDocument();
  });
});
