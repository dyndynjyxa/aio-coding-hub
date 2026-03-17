import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { HomeOverviewPanel } from "../HomeOverviewPanel";

vi.mock("../HomeUsageSection", () => ({
  HomeUsageSection: () => <div>usage-section</div>,
}));

vi.mock("../HomeWorkStatusCard", () => ({
  HomeWorkStatusCard: () => <div>work-status-card</div>,
}));

vi.mock("../HomeActiveSessionsCard", () => ({
  HomeActiveSessionsCardContent: () => <div>active-sessions</div>,
}));

vi.mock("../HomeProviderLimitPanel", () => ({
  HomeProviderLimitPanelContent: () => <div>provider-limit</div>,
}));

vi.mock("../HomeRequestLogsPanel", () => ({
  HomeRequestLogsPanel: () => <div>request-logs</div>,
}));

function renderPanel(overrides: Partial<ComponentProps<typeof HomeOverviewPanel>> = {}) {
  const onResetCircuitProvider = vi.fn();

  render(
    <HomeOverviewPanel
      showCustomTooltip={false}
      usageHeatmapRows={[]}
      usageHeatmapLoading={false}
      onRefreshUsageHeatmap={vi.fn()}
      sortModes={[]}
      sortModesLoading={false}
      sortModesAvailable={true}
      activeModeByCli={{ claude: null, codex: null, gemini: null }}
      activeModeToggling={{ claude: false, codex: false, gemini: false }}
      onSetCliActiveMode={vi.fn()}
      cliProxyEnabled={{ claude: false, codex: false, gemini: false }}
      cliProxyToggling={{ claude: false, codex: false, gemini: false }}
      onSetCliProxyEnabled={vi.fn()}
      activeSessions={[]}
      activeSessionsLoading={false}
      activeSessionsAvailable={true}
      providerLimitRows={[]}
      providerLimitLoading={false}
      providerLimitAvailable={true}
      providerLimitRefreshing={false}
      onRefreshProviderLimit={vi.fn()}
      openCircuits={[]}
      onResetCircuitProvider={onResetCircuitProvider}
      resettingCircuitProviderIds={new Set()}
      traces={[]}
      requestLogs={[]}
      requestLogsLoading={false}
      requestLogsRefreshing={false}
      requestLogsAvailable={true}
      onRefreshRequestLogs={vi.fn()}
      selectedLogId={null}
      onSelectLogId={vi.fn()}
      {...overrides}
    />
  );

  return { onResetCircuitProvider };
}

describe("components/home/HomeOverviewPanel", () => {
  it("supports previewing circuit rows locally when there are no real open circuits", () => {
    const { onResetCircuitProvider } = renderPanel();

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("当前没有熔断中的 Provider")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "预览熔断样式" }));
    expect(screen.getByText("Claude Main")).toBeInTheDocument();
    expect(screen.getByText("Codex Fallback")).toBeInTheDocument();
    expect(screen.getByText("Gemini Mirror")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭预览" })).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole("button", { name: "解除熔断" })[0]);
    expect(screen.queryByText("Claude Main")).not.toBeInTheDocument();
    expect(onResetCircuitProvider).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "关闭预览" }));
    expect(screen.getByText("当前没有熔断中的 Provider")).toBeInTheDocument();
  });

  it("uses real circuit rows when provided and forwards reset actions", () => {
    const { onResetCircuitProvider } = renderPanel({
      openCircuits: [
        {
          cli_key: "claude",
          provider_id: 7,
          provider_name: "Real Claude Provider",
          open_until: Math.floor(Date.now() / 1000) + 60,
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("Real Claude Provider")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "预览熔断样式" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "解除熔断" }));
    expect(onResetCircuitProvider).toHaveBeenCalledWith(7);
  });

  it("hides preview action when circuit preview is disabled", () => {
    renderPanel({ circuitPreviewEnabled: false });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("当前没有熔断中的 Provider")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "预览熔断样式" })).not.toBeInTheDocument();
  });
});
