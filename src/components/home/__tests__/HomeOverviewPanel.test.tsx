import { fireEvent, render, screen } from "@testing-library/react";
import { useState, type ComponentProps, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HomeOverviewPanel } from "../HomeOverviewPanel";

const { homeRequestLogsPanelMock } = vi.hoisted(() => ({
  homeRequestLogsPanelMock: vi.fn(() => <div>request-logs</div>),
}));

vi.mock("../HomeUsageSection", () => ({
  HomeUsageSection: ({
    showHeatmap,
    showUsageChart = true,
  }: {
    showHeatmap: boolean;
    showUsageChart?: boolean;
  }) => <div>{`usage-section:${String(showHeatmap)}:${String(showUsageChart)}`}</div>,
}));

vi.mock("../HomeTodayProviderUsageOverview", () => ({
  HomeTodayProviderUsageOverview: ({ devPreviewEnabled }: { devPreviewEnabled?: boolean }) => (
    <div>{`today-provider-usage:${String(Boolean(devPreviewEnabled))}`}</div>
  ),
}));

vi.mock("../HomeActiveSessionsCard", () => ({
  HomeActiveSessionsCardContent: ({
    activeSessions,
  }: {
    activeSessions: Array<{ session_id: string }>;
  }) => <div>active-sessions:{activeSessions.length}</div>,
}));

vi.mock("../HomeProviderLimitPanel", () => ({
  HomeProviderLimitPanelContent: ({ rows }: { rows: Array<{ provider_id: number }> }) => (
    <div>provider-limit:{rows.length}</div>
  ),
}));

vi.mock("../HomeOAuthQuotaPanel", () => ({
  HomeOAuthQuotaPanelContent: ({
    rows,
    hasProviders,
    hasRefreshed,
    refreshing,
    onRefresh,
    onRefreshRow,
  }: {
    rows: Array<{ providerId: number }>;
    hasProviders: boolean;
    hasRefreshed: boolean;
    refreshing: boolean;
    onRefresh?: () => void;
    onRefreshRow?: (providerId: number) => void;
  }) => (
    <div>
      <div>{`oauth-quota:${rows.length}:${String(hasProviders)}:${String(hasRefreshed)}:${String(refreshing)}`}</div>
      <button type="button" onClick={() => onRefresh?.()}>
        refresh-oauth-quota
      </button>
      <button type="button" onClick={() => onRefreshRow?.(rows[0]?.providerId ?? 0)}>
        refresh-oauth-quota-row
      </button>
    </div>
  ),
}));

vi.mock("../HomeWorkspaceConfigPanel", () => ({
  HomeWorkspaceConfigPanel: ({
    configs,
    selectedCliKey,
    onSelectCliKey,
    headerAddon,
  }: {
    configs: Array<{
      cliKey: "claude" | "codex" | "gemini";
      cliLabel: string;
      workspaceName: string | null;
      items: Array<{ id: string; name: string }>;
    }>;
    selectedCliKey: "claude" | "codex" | "gemini" | null;
    onSelectCliKey: (cliKey: "claude" | "codex" | "gemini") => void;
    headerAddon?: ReactNode;
  }) => {
    const selectedConfig =
      configs.find((config) => config.cliKey === selectedCliKey) ?? configs[0] ?? null;

    if (!selectedConfig) {
      return <div>workspace-config:empty</div>;
    }

    return (
      <div>
        <div>
          {configs.map((config) => (
            <button key={config.cliKey} type="button" onClick={() => onSelectCliKey(config.cliKey)}>
              {config.cliLabel}
            </button>
          ))}
        </div>
        <div>
          <span>工作区：</span>
          <span>{selectedConfig.workspaceName?.trim() || "默认"}</span>
        </div>
        {headerAddon}
        <div>
          {selectedConfig.items.map((item) => (
            <div key={item.id}>{item.name}</div>
          ))}
        </div>
      </div>
    );
  },
}));

vi.mock("../HomeRequestLogsPanel", () => ({
  HomeRequestLogsPanel: homeRequestLogsPanelMock,
}));

function renderPanel(overrides: Partial<ComponentProps<typeof HomeOverviewPanel>> = {}) {
  const onResetCircuitProvider = vi.fn();
  const onSetCliActiveMode = vi.fn();
  const view = render(
    <HomeOverviewPanel
      showCustomTooltip={false}
      showHomeHeatmap={true}
      cliPriorityOrder={["claude", "codex", "gemini"]}
      usageWindowDays={15}
      usageHeatmapRows={[]}
      usageHeatmapLoading={false}
      onRefreshUsageHeatmap={vi.fn()}
      sortModes={[]}
      sortModesLoading={false}
      sortModesAvailable={true}
      activeModeByCli={{ claude: null, codex: null, gemini: null }}
      activeModeToggling={{ claude: false, codex: false, gemini: false }}
      onSetCliActiveMode={onSetCliActiveMode}
      activeSessions={[]}
      activeSessionsLoading={false}
      activeSessionsAvailable={true}
      workspaceConfigs={[
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "默认",
          loading: false,
          items: [],
        },
        {
          cliKey: "codex",
          cliLabel: "Codex",
          workspaceId: 2,
          workspaceName: "Default",
          loading: false,
          items: [],
        },
        {
          cliKey: "gemini",
          cliLabel: "Gemini",
          workspaceId: 3,
          workspaceName: "工作区 2",
          loading: false,
          items: [],
        },
      ]}
      providerLimitRows={[]}
      providerLimitLoading={false}
      providerLimitAvailable={true}
      providerLimitRefreshing={false}
      onRefreshProviderLimit={vi.fn()}
      oauthQuotaRows={[]}
      oauthQuotaVisible={false}
      oauthQuotaRefreshing={false}
      oauthQuotaHasRefreshed={false}
      onRefreshOAuthQuota={vi.fn()}
      onRefreshOAuthQuotaRow={vi.fn()}
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
      personalizedUsageView="summary"
      {...overrides}
    />
  );

  return { ...view, onResetCircuitProvider, onSetCliActiveMode };
}

describe("components/home/HomeOverviewPanel", () => {
  beforeEach(() => {
    window.localStorage.clear();
    homeRequestLogsPanelMock.mockClear();
  });

  it("renders preview circuit rows when dev preview is enabled and there are no real open circuits", () => {
    const { onResetCircuitProvider } = renderPanel({ devPreviewEnabled: true });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("Claude Main")).toBeInTheDocument();
    expect(screen.getByText("Codex Fallback")).toBeInTheDocument();
    expect(screen.getByText("Gemini Mirror")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "预览熔断样式" })).not.toBeInTheDocument();

    expect(screen.getAllByRole("button", { name: "解除熔断" })[0]).toBeDisabled();
    fireEvent.click(screen.getAllByRole("button", { name: "解除熔断" })[0]);
    expect(screen.getByText("Claude Main")).toBeInTheDocument();
    expect(onResetCircuitProvider).not.toHaveBeenCalled();
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

  it("does not render preview circuit rows when dev preview is disabled", () => {
    renderPanel({ devPreviewEnabled: false });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("当前没有熔断中的 Provider")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "预览熔断样式" })).not.toBeInTheDocument();
  });

  it("shows workspace config pills and allows switching sort mode for the selected cli", async () => {
    const { onSetCliActiveMode } = renderPanel({
      sortModes: [{ id: 1, name: "工作策略", created_at: 1, updated_at: 1 }],
      activeModeByCli: { claude: 1, codex: null, gemini: null },
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [
            { id: "prompt:1", type: "prompts", label: "Prompt", name: "默认提示词" },
            { id: "mcp:1", type: "mcp", label: "MCP", name: "filesystem" },
          ],
        },
        {
          cliKey: "codex",
          cliLabel: "Codex",
          workspaceId: 2,
          workspaceName: "Default",
          loading: false,
          items: [{ id: "skill:1", type: "skills", label: "Skill", name: "code-review" }],
        },
        {
          cliKey: "gemini",
          cliLabel: "Gemini",
          workspaceId: 3,
          workspaceName: "工作区 B",
          loading: false,
          items: [],
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "配置信息" }));
    expect(await screen.findByRole("button", { name: "Claude Code" })).toBeInTheDocument();
    expect(screen.getByText("工作区 A")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Claude Code 路由策略" })).toHaveValue("1");
    expect(screen.getByRole("option", { name: "工作策略" })).toBeInTheDocument();
    expect(screen.getByText("默认提示词")).toBeInTheDocument();
    expect(screen.getByText("filesystem")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Codex" }));
    expect(screen.getByRole("combobox", { name: "Codex 路由策略" })).toHaveValue("");
    expect(screen.getByText("code-review")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox", { name: "Codex 路由策略" }), {
      target: { value: "1" },
    });
    expect(onSetCliActiveMode).toHaveBeenCalledWith("codex", 1);
  });

  it("keeps route strategy out of the workspace header in logs-primary layout", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    renderPanel({
      sortModes: [{ id: 1, name: "工作策略", created_at: 1, updated_at: 1 }],
      activeModeByCli: { claude: 1, codex: null, gemini: null },
    });

    fireEvent.click(screen.getByRole("tab", { name: "配置信息" }));
    expect(await screen.findByText("工作区：")).toBeInTheDocument();
    expect(screen.queryByText("路由策略：")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", { name: "Claude Code 路由策略" })
    ).not.toBeInTheDocument();
  });

  it("uses CLI priority order for workspace config button order and default selection", async () => {
    renderPanel({
      cliPriorityOrder: ["gemini", "codex", "claude"],
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
        },
        {
          cliKey: "codex",
          cliLabel: "Codex",
          workspaceId: 2,
          workspaceName: "工作区 B",
          loading: false,
          items: [{ id: "prompt:2", type: "prompts", label: "Prompt", name: "Codex Prompt" }],
        },
        {
          cliKey: "gemini",
          cliLabel: "Gemini",
          workspaceId: 3,
          workspaceName: "工作区 C",
          loading: false,
          items: [{ id: "prompt:3", type: "prompts", label: "Prompt", name: "Gemini Prompt" }],
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "配置信息" }));
    expect(
      screen
        .getAllByRole("button", { name: /Claude Code|Codex|Gemini/ })
        .map((button) => button.textContent)
    ).toEqual(["Gemini", "Codex", "Claude Code"]);
    expect(await screen.findByText("工作区 C")).toBeInTheDocument();
    expect(screen.getByText("Gemini Prompt")).toBeInTheDocument();
  });

  it("renders preview workspace config rows when dev preview is enabled and there is no real config data", async () => {
    renderPanel({ workspaceConfigs: [], devPreviewEnabled: true });

    fireEvent.click(screen.getByRole("tab", { name: "配置信息" }));
    expect(await screen.findByRole("button", { name: "Claude Code" })).toBeInTheDocument();
    expect(screen.getByText("工作区 Alpha")).toBeInTheDocument();
    expect(screen.getByText("PR Review")).toBeInTheDocument();
    expect(screen.getByText("filesystem")).toBeInTheDocument();
  });

  it("fills preview workspace items for the selected empty cli when dev preview is enabled", async () => {
    renderPanel({
      devPreviewEnabled: true,
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "默认提示词" }],
        },
        {
          cliKey: "codex",
          cliLabel: "Codex",
          workspaceId: 2,
          workspaceName: "Default",
          loading: false,
          items: [],
        },
        {
          cliKey: "gemini",
          cliLabel: "Gemini",
          workspaceId: 3,
          workspaceName: "工作区 B",
          loading: false,
          items: [],
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "配置信息" }));
    fireEvent.click(await screen.findByRole("button", { name: "Codex" }));

    expect(screen.getAllByText("Default")).toHaveLength(2);
    expect(screen.getByText("Fix First")).toBeInTheDocument();
    expect(screen.getByText("code-review")).toBeInTheDocument();
  });

  it("omits the legacy top metrics row when both heatmap and usage are hidden", () => {
    renderPanel({ showHomeHeatmap: false, showHomeUsage: false });

    expect(screen.queryByText(/usage-section:/)).not.toBeInTheDocument();
  });

  it("renders usage statistics when heatmap is hidden", () => {
    renderPanel({ showHomeHeatmap: false, showHomeUsage: true });

    expect(screen.getByText("usage-section:false:true")).toBeInTheDocument();
  });

  it("renders the heatmap when usage statistics are hidden", () => {
    renderPanel({ showHomeHeatmap: true, showHomeUsage: false });

    expect(screen.getByText("usage-section:true:false")).toBeInTheDocument();
  });

  it("uses the legacy overview layout by default", () => {
    renderPanel();

    const requestLogs = screen.getByText("request-logs");
    const overviewTab = screen.getByRole("tab", { name: "配置信息" });

    expect(
      overviewTab.compareDocumentPosition(requestLogs) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
  });

  it("uses the logs-primary layout when the local preference is enabled", () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    renderPanel();

    const requestLogs = screen.getByText("request-logs");
    const usageSection = screen.getByText("today-provider-usage:false");

    expect(
      usageSection.compareDocumentPosition(requestLogs) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(screen.getByRole("tab", { name: "配置信息" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "熔断信息" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "供应商限额" })).not.toBeInTheDocument();
    expect(homeRequestLogsPanelMock).toHaveBeenCalled();
    const latestCall = (homeRequestLogsPanelMock as any).mock.calls[
      (homeRequestLogsPanelMock as any).mock.calls.length - 1
    ];
    const latestProps = latestCall?.[0];
    expect(latestProps?.compactModeOverride).toBe(true);
    expect(latestProps?.showCompactModeToggle).toBe(false);
    expect(latestProps?.showRefreshButton).toBe(false);
  });

  it("does not render proxy controls in logs-primary layout", () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    renderPanel({ showHomeHeatmap: true, showHomeUsage: false });

    const usageSummary = screen.getByText("today-provider-usage:false");
    expect(usageSummary).toBeInTheDocument();
    expect(screen.queryByText("代理状态")).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "配置信息" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "熔断信息" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "供应商限额" })).not.toBeInTheDocument();
  });

  it("does not render the provider limit tab in logs-primary layout", () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    renderPanel({
      providerLimitRows: [{ provider_id: 1 } as any],
      providerLimitAvailable: true,
      providerLimitLoading: false,
    });

    expect(screen.queryByRole("tab", { name: "供应商限额" })).not.toBeInTheDocument();
  });

  it("renders the OAuth quota tab in logs-primary layout and forwards refresh actions", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");
    const onRefreshOAuthQuota = vi.fn().mockResolvedValue(undefined);
    const onRefreshOAuthQuotaRow = vi.fn().mockResolvedValue(undefined);

    renderPanel({
      oauthQuotaVisible: true,
      oauthQuotaRows: [{ providerId: 9 } as any],
      oauthQuotaHasRefreshed: true,
      onRefreshOAuthQuota,
      onRefreshOAuthQuotaRow,
    });

    fireEvent.click(screen.getByRole("tab", { name: "OAuth 配额" }));
    expect(await screen.findByText("oauth-quota:1:true:true:false")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota" }));
    expect(onRefreshOAuthQuota).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota-row" }));
    expect(onRefreshOAuthQuotaRow).toHaveBeenCalledWith(9);
  });

  it("renders preview OAuth quota rows when dev preview is enabled", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");
    const onRefreshOAuthQuota = vi.fn().mockResolvedValue(undefined);
    const onRefreshOAuthQuotaRow = vi.fn().mockResolvedValue(undefined);

    renderPanel({
      devPreviewEnabled: true,
      oauthQuotaVisible: true,
      oauthQuotaRows: [{ providerId: 9 } as any],
      oauthQuotaHasRefreshed: true,
      onRefreshOAuthQuota,
      onRefreshOAuthQuotaRow,
    });

    fireEvent.click(screen.getByRole("tab", { name: "OAuth 配额" }));
    expect(await screen.findByText("oauth-quota:5:true:true:false")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota" }));
    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota-row" }));
    expect(onRefreshOAuthQuota).not.toHaveBeenCalled();
    expect(onRefreshOAuthQuotaRow).not.toHaveBeenCalled();
  });

  it("renders the OAuth quota tab in the legacy layout and forwards refresh actions", async () => {
    const onRefreshOAuthQuota = vi.fn().mockResolvedValue(undefined);
    const onRefreshOAuthQuotaRow = vi.fn().mockResolvedValue(undefined);

    renderPanel({
      oauthQuotaVisible: true,
      oauthQuotaRows: [{ providerId: 9 } as any],
      oauthQuotaHasRefreshed: true,
      onRefreshOAuthQuota,
      onRefreshOAuthQuotaRow,
    });

    fireEvent.click(screen.getByRole("tab", { name: "OAuth 配额" }));
    expect(await screen.findByText("oauth-quota:1:true:true:false")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota" }));
    expect(onRefreshOAuthQuota).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota-row" }));
    expect(onRefreshOAuthQuotaRow).toHaveBeenCalledWith(9);
  });

  it("switches back to 配置信息 when OAuth providers disappear in logs-primary layout", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    const { rerender } = renderPanel({
      oauthQuotaVisible: true,
      oauthQuotaRows: [{ providerId: 9 } as any],
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "OAuth 配额" }));
    expect(screen.getByText("oauth-quota:1:true:false:false")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
        cliPriorityOrder={["claude", "codex", "gemini"]}
        usageWindowDays={15}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
        sortModes={[]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: null, gemini: null }}
        activeModeToggling={{ claude: false, codex: false, gemini: false }}
        onSetCliActiveMode={vi.fn()}
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
        workspaceConfigs={[
          {
            cliKey: "claude",
            cliLabel: "Claude Code",
            workspaceId: 1,
            workspaceName: "工作区 A",
            loading: false,
            items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
          },
        ]}
        providerLimitRows={[]}
        providerLimitLoading={false}
        providerLimitAvailable={true}
        providerLimitRefreshing={false}
        onRefreshProviderLimit={vi.fn()}
        oauthQuotaRows={[]}
        oauthQuotaVisible={false}
        oauthQuotaRefreshing={false}
        oauthQuotaHasRefreshed={false}
        onRefreshOAuthQuota={vi.fn()}
        onRefreshOAuthQuotaRow={vi.fn()}
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
        personalizedUsageView="summary"
      />
    );

    expect(await screen.findByText("工作区：")).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "OAuth 配额" })).not.toBeInTheDocument();
  });

  it("falls back to 配置信息 when stored tab order starts with provider limit in logs-primary layout", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");
    window.localStorage.setItem(
      "aio-home-overview-tab-order",
      JSON.stringify(["providerLimit", "circuit", "workspaceConfig", "sessions", "oauthQuota"])
    );

    renderPanel({
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
        },
      ],
    });

    expect(await screen.findByText("工作区：")).toBeInTheDocument();
    expect(screen.getByText("工作区 A")).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "供应商限额" })).not.toBeInTheDocument();
  });

  it("renders the usage chart branch when the personalized usage view switches", () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    function Wrapper() {
      const [view, setView] = useState<"summary" | "usageChart">("summary");

      return (
        <>
          <button type="button" onClick={() => setView("usageChart")}>
            switch-to-usage-chart
          </button>
          <HomeOverviewPanel
            showCustomTooltip={false}
            showHomeHeatmap={true}
            cliPriorityOrder={["claude", "codex", "gemini"]}
            usageWindowDays={15}
            usageHeatmapRows={[]}
            usageHeatmapLoading={false}
            onRefreshUsageHeatmap={vi.fn()}
            sortModes={[]}
            sortModesLoading={false}
            sortModesAvailable={true}
            activeModeByCli={{ claude: null, codex: null, gemini: null }}
            activeModeToggling={{ claude: false, codex: false, gemini: false }}
            onSetCliActiveMode={vi.fn()}
            activeSessions={[]}
            activeSessionsLoading={false}
            activeSessionsAvailable={true}
            workspaceConfigs={[
              {
                cliKey: "claude",
                cliLabel: "Claude Code",
                workspaceId: 1,
                workspaceName: "默认",
                loading: false,
                items: [],
              },
              {
                cliKey: "codex",
                cliLabel: "Codex",
                workspaceId: 2,
                workspaceName: "Default",
                loading: false,
                items: [],
              },
              {
                cliKey: "gemini",
                cliLabel: "Gemini",
                workspaceId: 3,
                workspaceName: "工作区 2",
                loading: false,
                items: [],
              },
            ]}
            providerLimitRows={[]}
            providerLimitLoading={false}
            providerLimitAvailable={true}
            providerLimitRefreshing={false}
            onRefreshProviderLimit={vi.fn()}
            oauthQuotaRows={[]}
            oauthQuotaVisible={false}
            oauthQuotaRefreshing={false}
            oauthQuotaHasRefreshed={false}
            onRefreshOAuthQuota={vi.fn()}
            onRefreshOAuthQuotaRow={vi.fn()}
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
            personalizedUsageView={view}
          />
        </>
      );
    }

    render(<Wrapper />);

    expect(screen.getByText("today-provider-usage:false")).toBeInTheDocument();
    expect(screen.queryByText("usage-section:false:true")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "switch-to-usage-chart" }));

    expect(screen.getByText("usage-section:false:true")).toBeInTheDocument();
    expect(screen.queryByText("today-provider-usage:false")).not.toBeInTheDocument();
  });

  it("renders preview active sessions when dev preview is enabled and there are no real sessions", async () => {
    renderPanel({ devPreviewEnabled: true, activeSessions: [] });

    fireEvent.click(screen.getByRole("tab", { name: "活跃 Session" }));
    expect(await screen.findByText("active-sessions:3")).toBeInTheDocument();
  });

  it("renders preview provider limits when dev preview is enabled and there are no real rows", async () => {
    renderPanel({ devPreviewEnabled: true, providerLimitRows: [] });

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(await screen.findByText("provider-limit:3")).toBeInTheDocument();
  });

  it("restores a persisted tab order from localStorage", () => {
    window.localStorage.setItem(
      "aio-home-overview-tab-order",
      JSON.stringify(["providerLimit", "sessions", "circuit", "workspaceConfig"])
    );

    renderPanel();

    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "供应商限额",
      "活跃 Session",
      "熔断信息",
      "配置信息",
      "OAuth 配额",
    ]);
  });

  it("uses the first sorted tab as the default selection", async () => {
    window.localStorage.setItem(
      "aio-home-overview-tab-order",
      JSON.stringify(["providerLimit", "sessions", "circuit", "workspaceConfig"])
    );

    renderPanel({ devPreviewEnabled: true, providerLimitRows: [] });

    expect(await screen.findByText("provider-limit:3")).toBeInTheDocument();
  });

  it("auto-switches to 熔断信息 when new open circuits arrive", () => {
    const { rerender } = renderPanel();

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(screen.getByText("provider-limit:0")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
        cliPriorityOrder={["claude", "codex", "gemini"]}
        usageWindowDays={15}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
        sortModes={[]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: null, gemini: null }}
        activeModeToggling={{ claude: false, codex: false, gemini: false }}
        onSetCliActiveMode={vi.fn()}
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
        workspaceConfigs={[]}
        providerLimitRows={[]}
        providerLimitLoading={false}
        providerLimitAvailable={true}
        providerLimitRefreshing={false}
        onRefreshProviderLimit={vi.fn()}
        oauthQuotaRows={[]}
        oauthQuotaVisible={false}
        oauthQuotaRefreshing={false}
        oauthQuotaHasRefreshed={false}
        onRefreshOAuthQuota={vi.fn()}
        onRefreshOAuthQuotaRow={vi.fn()}
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
        personalizedUsageView="summary"
      />
    );

    expect(screen.getByText("Claude New Circuit")).toBeInTheDocument();
  });

  it("auto-switches to 配置信息 when open circuits are removed", () => {
    const { rerender } = renderPanel({
      openCircuits: [
        {
          cli_key: "claude",
          provider_id: 9,
          provider_name: "Claude New Circuit",
          open_until: Math.floor(Date.now() / 1000) + 60,
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(screen.getByText("provider-limit:0")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
        cliPriorityOrder={["claude", "codex", "gemini"]}
        usageWindowDays={15}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
        sortModes={[]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: null, gemini: null }}
        activeModeToggling={{ claude: false, codex: false, gemini: false }}
        onSetCliActiveMode={vi.fn()}
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
        workspaceConfigs={[]}
        providerLimitRows={[]}
        providerLimitLoading={false}
        providerLimitAvailable={true}
        providerLimitRefreshing={false}
        onRefreshProviderLimit={vi.fn()}
        oauthQuotaRows={[]}
        oauthQuotaVisible={false}
        oauthQuotaRefreshing={false}
        oauthQuotaHasRefreshed={false}
        onRefreshOAuthQuota={vi.fn()}
        onRefreshOAuthQuotaRow={vi.fn()}
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
        personalizedUsageView="summary"
      />
    );

    expect(screen.getByText("workspace-config:empty")).toBeInTheDocument();
  });

  it("switches back to 配置信息 when circuits become empty in logs-primary layout", async () => {
    window.localStorage.setItem("aio-home-overview-logs-primary-layout", "true");

    const { rerender } = renderPanel({
      openCircuits: [
        {
          cli_key: "claude",
          provider_id: 9,
          provider_name: "Claude New Circuit",
          open_until: Math.floor(Date.now() / 1000) + 60,
        },
      ],
      workspaceConfigs: [
        {
          cliKey: "claude",
          cliLabel: "Claude Code",
          workspaceId: 1,
          workspaceName: "工作区 A",
          loading: false,
          items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
        },
      ],
    });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("Claude New Circuit")).toBeInTheDocument();

    rerender(
      <HomeOverviewPanel
        showCustomTooltip={false}
        showHomeHeatmap={true}
        cliPriorityOrder={["claude", "codex", "gemini"]}
        usageWindowDays={15}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
        onRefreshUsageHeatmap={vi.fn()}
        sortModes={[]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: null, gemini: null }}
        activeModeToggling={{ claude: false, codex: false, gemini: false }}
        onSetCliActiveMode={vi.fn()}
        activeSessions={[]}
        activeSessionsLoading={false}
        activeSessionsAvailable={true}
        workspaceConfigs={[
          {
            cliKey: "claude",
            cliLabel: "Claude Code",
            workspaceId: 1,
            workspaceName: "工作区 A",
            loading: false,
            items: [{ id: "prompt:1", type: "prompts", label: "Prompt", name: "Claude Prompt" }],
          },
        ]}
        providerLimitRows={[]}
        providerLimitLoading={false}
        providerLimitAvailable={true}
        providerLimitRefreshing={false}
        onRefreshProviderLimit={vi.fn()}
        oauthQuotaRows={[]}
        oauthQuotaVisible={false}
        oauthQuotaRefreshing={false}
        oauthQuotaHasRefreshed={false}
        onRefreshOAuthQuota={vi.fn()}
        onRefreshOAuthQuotaRow={vi.fn()}
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
        personalizedUsageView="summary"
      />
    );

    expect(await screen.findByText("工作区：")).toBeInTheDocument();
    expect(screen.getByText("工作区 A")).toBeInTheDocument();
  });
});
