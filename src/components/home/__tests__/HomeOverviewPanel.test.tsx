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
  const view = render(
    <HomeOverviewPanel
      showCustomTooltip={false}
      showHomeHeatmap={true}
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

  return { ...view, onResetCircuitProvider };
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

  it("auto-switches to 活跃 Session when new sessions arrive", () => {
    const { rerender } = renderPanel();

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(screen.getByText("provider-limit")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
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
        activeSessions={[
          {
            cli_key: "claude",
            session_id: "sess-1",
            session_suffix: "s1",
            provider_id: 1,
            provider_name: "P1",
            expires_at: Math.floor(Date.now() / 1000) + 60,
            request_count: 1,
            total_input_tokens: 10,
            total_output_tokens: 20,
            total_cost_usd: 0.01,
            total_duration_ms: 1000,
          },
        ]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
        providerLimitRows={[]}
        providerLimitLoading={false}
        providerLimitAvailable={true}
        providerLimitRefreshing={false}
        onRefreshProviderLimit={vi.fn()}
        openCircuits={[]}
        onResetCircuitProvider={vi.fn()}
        resettingCircuitProviderIds={new Set()}
        traces={[]}
        requestLogs={[]}
        requestLogsLoading={false}
        requestLogsRefreshing={false}
        requestLogsAvailable={true}
        onRefreshRequestLogs={vi.fn()}
        selectedLogId={null}
        onSelectLogId={vi.fn()}
      />
    );

    expect(screen.getByText("active-sessions")).toBeInTheDocument();
  });

  it("auto-switches to 熔断信息 when new open circuits arrive", () => {
    const { rerender } = renderPanel();

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(screen.getByText("provider-limit")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
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
        openCircuits={[
          {
            cli_key: "claude",
            provider_id: 9,
            provider_name: "Claude New Circuit",
            open_until: Math.floor(Date.now() / 1000) + 60,
          },
        ]}
        onResetCircuitProvider={vi.fn()}
        resettingCircuitProviderIds={new Set()}
        traces={[]}
        requestLogs={[]}
        requestLogsLoading={false}
        requestLogsRefreshing={false}
        requestLogsAvailable={true}
        onRefreshRequestLogs={vi.fn()}
        selectedLogId={null}
        onSelectLogId={vi.fn()}
      />
    );

    expect(screen.getByText("Claude New Circuit")).toBeInTheDocument();
  });
});
