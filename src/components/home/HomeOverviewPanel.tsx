// Usage:
// - Used by `src/pages/HomePage.tsx` to render the "概览" tab content.
// - This module is intentionally kept thin: it composes smaller, cohesive sub-components.

import {
  lazy,
  Suspense,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from "react";
import { useNowUnix } from "../../hooks/useNowUnix";
import { CIRCUIT_ROW_STATUS, type OpenCircuitRow } from "../ProviderCircuitBadge";
import type { GatewayActiveSession } from "../../services/gateway/gateway";
import { readHomeOverviewLogsPrimaryLayoutFromStorage } from "../../services/home/homeOverviewLayout";
import {
  HOME_OVERVIEW_TABS,
  readHomeOverviewTabOrderFromStorage,
  type HomeOverviewTabKey,
} from "../../services/home/homeOverviewTabOrder";
import { getOrderedClis } from "../../services/cli/cliPriorityOrder";
import type { CliKey } from "../../services/providers/providers";
import type { ProviderLimitUsageRow } from "../../services/providers/providerLimitUsage";
import type { RequestLogSummary } from "../../services/gateway/requestLogs";
import type { ActiveRequestSnapshotItem } from "../../services/gateway/requestActivityProjection";
import type { SortModeSummary } from "../../services/providers/sortModes";
import type { TraceSession } from "../../services/gateway/traceStore";
import type { UsageHourlyRow } from "../../services/usage/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { TabList } from "../../ui/TabList";
import { formatCountdownSeconds } from "../../utils/formatters";
import { CliBrandIcon } from "./CliBrandIcon";
import { HomeCliRouteStrategyControl } from "./HomeCliRouteStrategyControl";
import type { HomeOAuthQuotaRow } from "./homeOAuthQuotaTypes";
import { HomeRequestLogsPanel } from "./HomeRequestLogsPanel";
import { HomeTodayProviderUsageOverview } from "./HomeTodayProviderUsageOverview";
import { HomeUsageSection } from "./HomeUsageSection";
import type { HomeCliWorkspaceConfig, HomeWorkspaceConfigItem } from "./homeWorkspaceConfigTypes";

export type HomeOverviewUsageView = "summary" | "usageChart";

const LazyHomeActiveSessionsCardContent = lazy(() =>
  import("./HomeActiveSessionsCard").then((m) => ({ default: m.HomeActiveSessionsCardContent }))
);

const LazyHomeProviderLimitPanelContent = lazy(() =>
  import("./HomeProviderLimitPanel").then((m) => ({ default: m.HomeProviderLimitPanelContent }))
);

const LazyHomeOAuthQuotaPanelContent = lazy(() =>
  import("./HomeOAuthQuotaPanel").then((m) => ({ default: m.HomeOAuthQuotaPanelContent }))
);

const LazyHomeWorkspaceConfigPanel = lazy(() =>
  import("./HomeWorkspaceConfigPanel").then((m) => ({ default: m.HomeWorkspaceConfigPanel }))
);

const PREVIEW_CIRCUITS: OpenCircuitRow[] = [
  {
    cli_key: "claude",
    provider_id: 10001,
    provider_name: "Claude Main",
    displayState: "open",
    open_until: Math.floor(Date.now() / 1000) + 12 * 60,
  },
  {
    cli_key: "codex",
    provider_id: 10002,
    provider_name: "Codex Fallback",
    displayState: "cooldown",
    open_until: Math.floor(Date.now() / 1000) + 5 * 60,
  },
  {
    cli_key: "gemini",
    provider_id: 10003,
    provider_name: "Gemini Mirror",
    displayState: "half_open",
    open_until: null,
  },
];

const PREVIEW_ACTIVE_SESSIONS: GatewayActiveSession[] = [
  {
    cli_key: "claude",
    session_id: "preview-claude-1",
    session_suffix: "a1f4",
    provider_id: 101,
    provider_name: "Claude Main",
    expires_at: Math.floor(Date.now() / 1000) + 18 * 60,
    request_count: 12,
    total_input_tokens: 18240,
    total_output_tokens: 9132,
    total_cost_usd: 1.284,
    total_duration_ms: 48200,
  },
  {
    cli_key: "codex",
    session_id: "preview-codex-1",
    session_suffix: "c9d2",
    provider_id: 102,
    provider_name: "OpenAI Primary",
    expires_at: Math.floor(Date.now() / 1000) + 11 * 60,
    request_count: 7,
    total_input_tokens: 9640,
    total_output_tokens: 4408,
    total_cost_usd: 0.632,
    total_duration_ms: 27500,
  },
  {
    cli_key: "gemini",
    session_id: "preview-gemini-1",
    session_suffix: "g7b8",
    provider_id: 103,
    provider_name: "Gemini Mirror",
    expires_at: Math.floor(Date.now() / 1000) + 25 * 60,
    request_count: 15,
    total_input_tokens: 20512,
    total_output_tokens: 11032,
    total_cost_usd: 0.948,
    total_duration_ms: 53400,
  },
];

const PREVIEW_PROVIDER_LIMIT_ROWS: ProviderLimitUsageRow[] = [
  {
    cli_key: "claude",
    provider_id: 201,
    provider_name: "Claude Main",
    enabled: true,
    limit_5h_usd: 12,
    limit_daily_usd: 40,
    daily_reset_mode: "rolling",
    daily_reset_time: null,
    limit_weekly_usd: 180,
    limit_monthly_usd: null,
    limit_total_usd: null,
    usage_5h_usd: 8.6,
    usage_daily_usd: 19.4,
    usage_weekly_usd: 84.2,
    usage_monthly_usd: 0,
    usage_total_usd: 0,
    window_5h_start_ts: 1_710_000_000,
    window_daily_start_ts: 1_710_018_000,
    window_weekly_start_ts: 1_709_481_600,
    window_monthly_start_ts: 1_709_395_200,
  },
  {
    cli_key: "codex",
    provider_id: 202,
    provider_name: "OpenAI Primary",
    enabled: true,
    limit_5h_usd: null,
    limit_daily_usd: 25,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00:00",
    limit_weekly_usd: null,
    limit_monthly_usd: 300,
    limit_total_usd: 900,
    usage_5h_usd: 0,
    usage_daily_usd: 21.8,
    usage_weekly_usd: 0,
    usage_monthly_usd: 126.4,
    usage_total_usd: 402.6,
    window_5h_start_ts: 1_710_000_000,
    window_daily_start_ts: 1_710_028_800,
    window_weekly_start_ts: 1_709_481_600,
    window_monthly_start_ts: 1_709_395_200,
  },
  {
    cli_key: "gemini",
    provider_id: 203,
    provider_name: "Gemini Mirror",
    enabled: false,
    limit_5h_usd: null,
    limit_daily_usd: 18,
    daily_reset_mode: "rolling",
    daily_reset_time: null,
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    usage_5h_usd: 0,
    usage_daily_usd: 4.1,
    usage_weekly_usd: 0,
    usage_monthly_usd: 0,
    usage_total_usd: 0,
    window_5h_start_ts: 1_710_000_000,
    window_daily_start_ts: 1_710_018_000,
    window_weekly_start_ts: 1_709_481_600,
    window_monthly_start_ts: 1_709_395_200,
  },
];

const PREVIEW_OAUTH_QUOTA_ROWS: HomeOAuthQuotaRow[] = [
  {
    providerId: 301,
    cliKey: "claude",
    providerName: "Claude OAuth 主账号",
    enabled: true,
    state: "success",
    limits: {
      limit_short_label: "5h",
      limit_5h_text: "29%",
      limit_weekly_text: "83%",
      limit_5h_reset_at: Math.floor(Date.now() / 1000) + 2 * 3600 + 34 * 60,
      limit_weekly_reset_at: Math.floor(Date.now() / 1000) + 3 * 86400 + 2 * 3600 + 29 * 60,
      reset_credit_available_count: null,
    },
    error: null,
  },
  {
    providerId: 302,
    cliKey: "codex",
    providerName: "Codex OAuth 空数据",
    enabled: true,
    state: "success",
    limits: {
      limit_short_label: "5h",
      limit_5h_text: null,
      limit_weekly_text: null,
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
      reset_credit_available_count: 2,
    },
    error: null,
  },
  {
    providerId: 303,
    cliKey: "gemini",
    providerName: "Gemini OAuth 未刷新",
    enabled: true,
    state: "idle",
    limits: null,
    error: null,
  },
  {
    providerId: 304,
    cliKey: "codex",
    providerName: "Codex OAuth 刷新失败",
    enabled: true,
    state: "error",
    limits: null,
    error: "preview error",
  },
  {
    providerId: 305,
    cliKey: "gemini",
    providerName: "Gemini OAuth 已禁用",
    enabled: false,
    state: "loading",
    limits: null,
    error: null,
  },
];

function didKeysChange(current: string[], previous: string[]) {
  return (
    current.length !== previous.length || current.some((key, index) => key !== previous[index])
  );
}

type SessionsTabState = {
  tab: HomeOverviewTabKey;
  openCircuitKeys: string[] | null;
};

type HomeOverviewTabItem = {
  key: HomeOverviewTabKey;
  label: string;
};

function OverviewPanelFallback() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      <div className="flex items-center gap-3">
        <Spinner />
        <span>加载面板中…</span>
      </div>
    </div>
  );
}

function CircuitProvidersPanel({
  rows,
  nowUnix,
  previewActive,
  resettingProviderIds,
  onResetProvider,
}: {
  rows: OpenCircuitRow[];
  nowUnix: number;
  previewActive: boolean;
  resettingProviderIds: Set<number>;
  onResetProvider: (providerId: number) => void;
}) {
  if (rows.length === 0) {
    return <EmptyState title="当前没有熔断中的 Provider" />;
  }

  return (
    <div className="h-full overflow-y-auto pr-1 scrollbar-overlay">
      <div className="space-y-3">
        {rows.map((row) => {
          const status = CIRCUIT_ROW_STATUS[row.displayState];
          // half_open 无 until 语义，不渲染倒计时。
          const remaining =
            row.displayState !== "half_open" &&
            row.open_until != null &&
            Number.isFinite(row.open_until)
              ? formatCountdownSeconds(row.open_until - nowUnix)
              : null;
          const isResetting = resettingProviderIds.has(row.provider_id);

          return (
            <div
              key={`${row.cli_key}:${row.provider_id}`}
              className="flex items-center justify-between gap-3 rounded-lg border border-border bg-secondary/70 px-3 py-2 dark:border-border dark:bg-secondary/50"
            >
              <div className="min-w-0 flex flex-1 items-center gap-2.5">
                <CliBrandIcon
                  cliKey={row.cli_key as CliKey}
                  className="h-4 w-4 shrink-0 rounded-[4px] object-contain"
                />
                <div
                  className="truncate text-sm font-medium text-foreground"
                  title={row.provider_name}
                >
                  {row.provider_name || "未知"}
                </div>
              </div>
              <div className="shrink-0 text-xs">
                <span className={`font-medium ${status.className}`}>{status.label}</span>
                {remaining != null ? (
                  <span className="ml-1 font-mono text-muted-foreground">{remaining}</span>
                ) : null}
              </div>
              <Button
                variant="secondary"
                size="sm"
                disabled={isResetting || previewActive}
                onClick={() => {
                  if (previewActive) return;
                  onResetProvider(row.provider_id);
                }}
              >
                {isResetting ? "解除中..." : "解除熔断"}
              </Button>
            </div>
          );
        })}
      </div>
    </div>
  );
}

type ActiveSessionsPanelState = {
  sessions: GatewayActiveSession[];
  loading: boolean;
  available: boolean | null;
};

type WorkspaceConfigPanelState = {
  configs: HomeCliWorkspaceConfig[];
  selectedCliKey: CliKey | null;
  onSelectCliKey: (cliKey: CliKey) => void;
  headerAddon: ReactNode;
  showQuickToggle: boolean;
  togglingItemIds: Set<string>;
  switchingWorkspaceKey: string | null;
  onSwitchWorkspace?: (cliKey: CliKey, workspaceId: number) => void;
  onToggleItemEnabled?: (
    workspaceId: number,
    item: HomeWorkspaceConfigItem,
    enabled: boolean
  ) => void;
};

type ProviderLimitPanelState = {
  rows: ProviderLimitUsageRow[];
  loading: boolean;
  available: boolean | null;
  refreshing: boolean;
  onRefresh: () => void;
};

type CircuitPanelState = {
  rows: OpenCircuitRow[];
  nowUnix: number;
  previewActive: boolean;
  resettingProviderIds: Set<number>;
  onResetProvider: (providerId: number) => void;
};

function WorkspaceConfigPanelSlot({ workspace }: { workspace: WorkspaceConfigPanelState }) {
  return (
    <Suspense fallback={<OverviewPanelFallback />}>
      <LazyHomeWorkspaceConfigPanel
        configs={workspace.configs}
        selectedCliKey={workspace.selectedCliKey}
        onSelectCliKey={workspace.onSelectCliKey}
        headerAddon={workspace.headerAddon}
        showQuickToggle={workspace.showQuickToggle}
        togglingItemIds={workspace.togglingItemIds}
        switchingWorkspaceKey={workspace.switchingWorkspaceKey}
        onSwitchWorkspace={workspace.onSwitchWorkspace}
        onToggleItemEnabled={workspace.onToggleItemEnabled}
      />
    </Suspense>
  );
}

function OverviewInfoPanel({
  tabs,
  tab,
  onTabChange,
  activeSessions,
  workspace,
  providerLimit,
  oauthQuotaPanelContent,
  circuit,
}: {
  tabs: HomeOverviewTabItem[];
  tab: HomeOverviewTabKey;
  onTabChange: (tab: HomeOverviewTabKey) => void;
  activeSessions: ActiveSessionsPanelState;
  workspace: WorkspaceConfigPanelState;
  providerLimit: ProviderLimitPanelState;
  oauthQuotaPanelContent: ReactNode;
  circuit: CircuitPanelState;
}) {
  const tabValue = tabs.some((item) => item.key === tab) ? tab : "workspaceConfig";

  return (
    <Card padding="sm" className="flex h-full min-h-0 flex-1 flex-col">
      <TabList
        ariaLabel="概览状态切换"
        items={tabs}
        value={tabValue}
        onChange={onTabChange}
        variant="compact"
        className="shrink-0 -mt-1"
      />

      <div className="flex-1 min-h-0 mt-2">
        {tab === "sessions" ? (
          <Suspense fallback={<OverviewPanelFallback />}>
            <LazyHomeActiveSessionsCardContent
              activeSessions={activeSessions.sessions}
              activeSessionsLoading={activeSessions.loading}
              activeSessionsAvailable={activeSessions.available}
            />
          </Suspense>
        ) : tab === "workspaceConfig" ? (
          <WorkspaceConfigPanelSlot workspace={workspace} />
        ) : tab === "providerLimit" ? (
          <Suspense fallback={<OverviewPanelFallback />}>
            <LazyHomeProviderLimitPanelContent
              rows={providerLimit.rows}
              loading={providerLimit.loading}
              available={providerLimit.available}
              onRefresh={providerLimit.onRefresh}
              refreshing={providerLimit.refreshing}
            />
          </Suspense>
        ) : tab === "oauthQuota" ? (
          oauthQuotaPanelContent
        ) : (
          <CircuitProvidersPanel
            rows={circuit.rows}
            nowUnix={circuit.nowUnix}
            previewActive={circuit.previewActive}
            resettingProviderIds={circuit.resettingProviderIds}
            onResetProvider={circuit.onResetProvider}
          />
        )}
      </div>
    </Card>
  );
}

function LogsPrimaryInfoPanel({
  tabs,
  tab,
  onTabChange,
  workspace,
  oauthQuotaPanelContent,
  circuit,
}: {
  tabs: HomeOverviewTabItem[];
  tab: HomeOverviewTabKey;
  onTabChange: (tab: HomeOverviewTabKey) => void;
  workspace: WorkspaceConfigPanelState;
  oauthQuotaPanelContent: ReactNode;
  circuit: CircuitPanelState;
}) {
  const tabValue = tabs.some((item) => item.key === tab) ? tab : "workspaceConfig";

  return (
    <Card padding="sm" className="flex h-full min-h-0 flex-1 flex-col">
      <TabList
        ariaLabel="新布局信息切换"
        items={tabs}
        value={tabValue}
        onChange={onTabChange}
        variant="compact"
        className="shrink-0 -mt-1"
      />

      <div className="mt-2 min-h-0 flex-1">
        {tab === "circuit" ? (
          <CircuitProvidersPanel
            rows={circuit.rows}
            nowUnix={circuit.nowUnix}
            previewActive={circuit.previewActive}
            resettingProviderIds={circuit.resettingProviderIds}
            onResetProvider={circuit.onResetProvider}
          />
        ) : tab === "oauthQuota" ? (
          oauthQuotaPanelContent
        ) : (
          <WorkspaceConfigPanelSlot workspace={workspace} />
        )}
      </div>
    </Card>
  );
}

function useDisplayedWorkspaceConfigs({
  cliPriorityOrder,
  devPreviewEnabled,
  workspaceConfigs,
}: {
  cliPriorityOrder?: CliKey[];
  devPreviewEnabled: boolean;
  workspaceConfigs: HomeCliWorkspaceConfig[];
}) {
  return useMemo(() => {
    let nextConfigs: HomeCliWorkspaceConfig[];
    if (workspaceConfigs.length === 0) {
      nextConfigs = devPreviewEnabled ? PREVIEW_WORKSPACE_CONFIGS : [];
    } else if (!devPreviewEnabled) {
      nextConfigs = workspaceConfigs;
    } else {
      const previewConfigByCli = new Map(
        PREVIEW_WORKSPACE_CONFIGS.map((config) => [config.cliKey, config])
      );

      nextConfigs = workspaceConfigs.map((config) => {
        if (config.loading || config.items.length > 0) return config;

        const previewConfig = previewConfigByCli.get(config.cliKey);
        if (!previewConfig) return config;

        return {
          ...config,
          workspaceId: config.workspaceId ?? previewConfig.workspaceId,
          workspaceName: config.workspaceName?.trim()
            ? config.workspaceName
            : previewConfig.workspaceName,
          items: previewConfig.items,
        };
      });
    }

    const orderedCliKeys = getOrderedClis(
      cliPriorityOrder,
      nextConfigs.map((config) => config.cliKey)
    ).map((cli) => cli.key);
    const configByCli = new Map(nextConfigs.map((config) => [config.cliKey, config]));

    return orderedCliKeys
      .map((cliKey) => configByCli.get(cliKey))
      .filter((config): config is HomeCliWorkspaceConfig => config != null);
  }, [cliPriorityOrder, devPreviewEnabled, workspaceConfigs]);
}

function useWorkspaceConfigPanelState({
  activeModeByCli,
  activeModeToggling,
  displayedWorkspaceConfigs,
  selectedWorkspaceConfigCliKey,
  setSelectedWorkspaceConfigCliKey,
  showQuickToggle,
  sortModes,
  sortModesAvailable,
  sortModesLoading,
  togglingItemIds,
  switchingWorkspaceKey,
  onSetCliActiveMode,
  onSwitchWorkspace,
  onToggleItemEnabled,
}: {
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  displayedWorkspaceConfigs: HomeCliWorkspaceConfig[];
  selectedWorkspaceConfigCliKey: CliKey | null;
  setSelectedWorkspaceConfigCliKey: Dispatch<SetStateAction<CliKey | null>>;
  showQuickToggle: boolean;
  sortModes: SortModeSummary[];
  sortModesAvailable: boolean | null;
  sortModesLoading: boolean;
  togglingItemIds: Set<string>;
  switchingWorkspaceKey: string | null;
  onSetCliActiveMode: (cliKey: CliKey, modeId: number | null) => void;
  onSwitchWorkspace?: (cliKey: CliKey, workspaceId: number) => void;
  onToggleItemEnabled?: (
    workspaceId: number,
    item: HomeWorkspaceConfigItem,
    enabled: boolean
  ) => void;
}): WorkspaceConfigPanelState {
  let effectiveSelectedCliKey =
    selectedWorkspaceConfigCliKey ?? displayedWorkspaceConfigs[0]?.cliKey ?? null;
  if (
    effectiveSelectedCliKey != null &&
    !displayedWorkspaceConfigs.some((config) => config.cliKey === effectiveSelectedCliKey)
  ) {
    effectiveSelectedCliKey = displayedWorkspaceConfigs[0]?.cliKey ?? null;
    setSelectedWorkspaceConfigCliKey(effectiveSelectedCliKey);
  }

  const effectiveConfig = useMemo(
    () =>
      displayedWorkspaceConfigs.find((config) => config.cliKey === effectiveSelectedCliKey) ??
      displayedWorkspaceConfigs[0] ??
      null,
    [displayedWorkspaceConfigs, effectiveSelectedCliKey]
  );
  const headerAddon = useMemo(
    () =>
      effectiveConfig ? (
        <div className="flex min-w-0 items-center gap-1.5 rounded-lg border border-border bg-secondary/70 px-2.5 py-1 text-sm dark:border-border dark:bg-secondary/50">
          <span className="shrink-0 font-medium text-muted-foreground">路由策略：</span>
          <HomeCliRouteStrategyControl
            cliKey={effectiveConfig.cliKey}
            cliLabel={effectiveConfig.cliLabel}
            sortModes={sortModes}
            sortModesLoading={sortModesLoading}
            sortModesAvailable={sortModesAvailable}
            activeModeByCli={activeModeByCli}
            activeModeToggling={activeModeToggling}
            onSetCliActiveMode={onSetCliActiveMode}
            orientation="horizontal"
            className="min-w-0 flex-1"
            selectClassName="h-7 min-w-0 flex-1 border-0 bg-transparent px-0 py-0 text-sm font-semibold text-secondary-foreground shadow-none outline-none focus:border-transparent focus:bg-transparent focus:ring-0 focus:ring-offset-0 disabled:bg-transparent dark:bg-transparent dark:text-foreground dark:disabled:bg-transparent"
          />
        </div>
      ) : null,
    [
      activeModeByCli,
      activeModeToggling,
      effectiveConfig,
      onSetCliActiveMode,
      sortModes,
      sortModesAvailable,
      sortModesLoading,
    ]
  );

  return {
    configs: displayedWorkspaceConfigs,
    selectedCliKey: effectiveSelectedCliKey,
    onSelectCliKey: setSelectedWorkspaceConfigCliKey,
    headerAddon,
    showQuickToggle,
    togglingItemIds,
    switchingWorkspaceKey,
    onSwitchWorkspace,
    onToggleItemEnabled,
  };
}

function useHomeOverviewTabs({
  displayedOAuthQuotaVisible,
  logsPrimaryLayout,
  openCircuits,
  sessionsTabState,
  sessionsTabsOrder,
  setSessionsTabState,
}: {
  displayedOAuthQuotaVisible: boolean;
  logsPrimaryLayout: boolean;
  openCircuits: OpenCircuitRow[];
  sessionsTabState: SessionsTabState;
  sessionsTabsOrder: HomeOverviewTabKey[];
  setSessionsTabState: Dispatch<SetStateAction<SessionsTabState>>;
}) {
  const legacySessionsTabs = useMemo(() => {
    const labelByKey = new Map(HOME_OVERVIEW_TABS.map((item) => [item.key, item.label]));
    return sessionsTabsOrder.map((key) => ({ key, label: labelByKey.get(key) ?? key }));
  }, [sessionsTabsOrder]);
  const logsPrimaryTabs = useMemo(
    () =>
      legacySessionsTabs.filter(
        (item) =>
          item.key === "workspaceConfig" ||
          item.key === "circuit" ||
          (item.key === "oauthQuota" && displayedOAuthQuotaVisible)
      ),
    [displayedOAuthQuotaVisible, legacySessionsTabs]
  );
  const openCircuitKeys = useMemo(
    () =>
      openCircuits
        .map((row) => `${row.cli_key}:${row.provider_id}`)
        .sort((a, b) => a.localeCompare(b)),
    [openCircuits]
  );

  const visibleTabs = logsPrimaryLayout ? logsPrimaryTabs : legacySessionsTabs;
  let effectiveSessionsTabState = sessionsTabState;
  const openCircuitChanged =
    sessionsTabState.openCircuitKeys != null &&
    didKeysChange(openCircuitKeys, sessionsTabState.openCircuitKeys);

  if (openCircuitChanged) {
    let nextTab = sessionsTabState.tab;
    if (openCircuitKeys.length > 0) {
      nextTab = "circuit";
    } else if (sessionsTabState.tab === "circuit") {
      nextTab = "workspaceConfig";
    }
    effectiveSessionsTabState = {
      tab: nextTab,
      openCircuitKeys,
    };
    setSessionsTabState(effectiveSessionsTabState);
  } else if (!visibleTabs.some((tab) => tab.key === sessionsTabState.tab)) {
    effectiveSessionsTabState = {
      tab: "workspaceConfig",
      openCircuitKeys,
    };
    setSessionsTabState(effectiveSessionsTabState);
  } else if (sessionsTabState.openCircuitKeys !== openCircuitKeys) {
    effectiveSessionsTabState = {
      ...sessionsTabState,
      openCircuitKeys,
    };
    setSessionsTabState(effectiveSessionsTabState);
  }

  const sessionsTab = effectiveSessionsTabState.tab;
  return {
    legacySessionsTabs,
    logsPrimaryTabs,
    sessionsTab,
    setSessionsTab: (tab: HomeOverviewTabKey) => {
      setSessionsTabState((current) => ({ ...current, tab }));
    },
  };
}

function HomeOverviewUsageStrip({
  devPreviewEnabled,
  showHeatmap,
  showUsageChart,
  usageWindowDays,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
}: {
  devPreviewEnabled: boolean;
  showHeatmap: boolean;
  showUsageChart: boolean;
  usageWindowDays: number;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;
}) {
  if (!showHeatmap && !showUsageChart) return null;

  const usageSection = (
    <HomeUsageSection
      devPreviewEnabled={devPreviewEnabled}
      showHeatmap={showHeatmap}
      showUsageChart={showUsageChart}
      usageWindowDays={usageWindowDays}
      usageHeatmapRows={usageHeatmapRows}
      usageHeatmapLoading={usageHeatmapLoading}
      onRefreshUsageHeatmap={onRefreshUsageHeatmap}
    />
  );

  if (showHeatmap && showUsageChart) {
    return (
      <div className="shrink-0">
        <div className="space-y-4">
          <div className="flex">{usageSection}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="shrink-0">
      <div className="flex">{usageSection}</div>
    </div>
  );
}

function HomeOverviewContentLayout({
  activeSessions,
  devPreviewEnabled,
  logsPrimaryInfoPanel,
  logsPrimaryLayout,
  overviewInfoPanel,
  personalizedUsageView,
  requestLogs,
  activeRequests,
  requestLogsPanel,
  traces,
  usageWindowDays,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
}: {
  activeSessions: GatewayActiveSession[];
  devPreviewEnabled: boolean;
  logsPrimaryInfoPanel: ReactNode;
  logsPrimaryLayout: boolean;
  overviewInfoPanel: ReactNode;
  personalizedUsageView: HomeOverviewUsageView;
  requestLogs: RequestLogSummary[];
  activeRequests: ActiveRequestSnapshotItem[];
  requestLogsPanel: ReactNode;
  traces: TraceSession[];
  usageWindowDays: number;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;
}) {
  if (logsPrimaryLayout) {
    return (
      <div className="grid flex-1 min-h-0 gap-4 lg:grid-cols-12">
        <div className="flex min-h-0 lg:col-span-4">{logsPrimaryInfoPanel}</div>
        <div className="flex min-h-0 flex-col gap-4 lg:col-span-8">
          <div className="shrink-0">
            {personalizedUsageView === "summary" ? (
              <div className="space-y-4">
                <HomeTodayProviderUsageOverview
                  devPreviewEnabled={devPreviewEnabled}
                  activeSessions={activeSessions}
                  requestLogs={requestLogs}
                  activeRequests={activeRequests}
                  traces={traces}
                />
              </div>
            ) : (
              <HomeUsageSection
                devPreviewEnabled={devPreviewEnabled}
                showHeatmap={false}
                showUsageChart={true}
                usageWindowDays={usageWindowDays}
                usageHeatmapRows={usageHeatmapRows}
                usageHeatmapLoading={usageHeatmapLoading}
                onRefreshUsageHeatmap={onRefreshUsageHeatmap}
              />
            )}
          </div>
          <div className="min-h-0 flex-1">{requestLogsPanel}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-12 flex-1 min-h-0">
      <div className="flex min-h-0 lg:col-span-5">{overviewInfoPanel}</div>
      <div className="lg:col-span-7 min-h-0">{requestLogsPanel}</div>
    </div>
  );
}

export type HomeOverviewDisplayOptions = {
  customTooltip: boolean;
  heatmap: boolean;
  usage: boolean;
  workspaceConfigQuickToggle: boolean;
};

export type HomeOverviewPanelProps = {
  displayOptions: HomeOverviewDisplayOptions;
  devPreviewEnabled?: boolean;
  cliPriorityOrder?: CliKey[];

  usageWindowDays: number;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;

  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  onSetCliActiveMode: (cliKey: CliKey, modeId: number | null) => void;

  activeSessions: GatewayActiveSession[];
  activeSessionsLoading: boolean;
  activeSessionsAvailable: boolean | null;

  workspaceConfigs: HomeCliWorkspaceConfig[];
  togglingWorkspaceConfigItemIds?: Set<string>;
  switchingWorkspaceKey?: string | null;
  onSwitchWorkspace?: (cliKey: CliKey, workspaceId: number) => void;
  onToggleWorkspaceConfigItemEnabled?: (
    workspaceId: number,
    item: HomeWorkspaceConfigItem,
    enabled: boolean
  ) => void;

  providerLimitRows: ProviderLimitUsageRow[];
  providerLimitLoading: boolean;
  providerLimitAvailable: boolean | null;
  providerLimitRefreshing: boolean;
  onRefreshProviderLimit: () => void;
  oauthQuotaRows: HomeOAuthQuotaRow[];
  oauthQuotaVisible: boolean;
  oauthQuotaRefreshing: boolean;
  oauthQuotaHasRefreshed: boolean;
  onRefreshOAuthQuota: () => Promise<void>;
  onRefreshOAuthQuotaRow: (providerId: number) => Promise<void>;
  onResetOAuthQuotaRow?: (providerId: number) => Promise<void>;

  openCircuits: OpenCircuitRow[];
  onResetCircuitProvider: (providerId: number) => void;
  resettingCircuitProviderIds: Set<number>;

  traces: TraceSession[];

  requestLogs: RequestLogSummary[];
  activeRequests?: ActiveRequestSnapshotItem[];
  requestLogsLoading: boolean;
  requestLogsRefreshing: boolean;
  requestLogsAvailable: boolean | null;
  onRefreshRequestLogs: () => void;

  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
  personalizedUsageView: HomeOverviewUsageView;
};

const PREVIEW_WORKSPACE_CONFIGS: HomeCliWorkspaceConfig[] = [
  {
    cliKey: "claude",
    cliLabel: "Claude",
    workspaceId: 1001,
    workspaceName: "工作区 Alpha",
    workspaces: [{ id: 1001, name: "工作区 Alpha", isActive: true }],
    loading: false,
    items: [
      {
        id: "prompt:1001",
        resourceId: 1001,
        type: "prompts",
        label: "Prompt",
        name: "PR Review",
        enabled: true,
      },
      {
        id: "mcp:1001",
        resourceId: 1001,
        type: "mcp",
        label: "MCP",
        name: "filesystem",
        enabled: true,
      },
      {
        id: "mcp:1002",
        resourceId: 1002,
        type: "mcp",
        label: "MCP",
        name: "browser-tools",
        enabled: false,
      },
      {
        id: "skill:1001",
        resourceId: 1001,
        type: "skills",
        label: "Skill",
        name: "repo-auditor",
        enabled: true,
      },
      {
        id: "skill:1002",
        resourceId: 1002,
        type: "skills",
        label: "Skill",
        name: "incident-helper",
        enabled: false,
      },
    ],
  },
  {
    cliKey: "codex",
    cliLabel: "Codex",
    workspaceId: 1002,
    workspaceName: "Default",
    workspaces: [{ id: 1002, name: "Default", isActive: true }],
    loading: false,
    items: [
      {
        id: "prompt:1002",
        resourceId: 1002,
        type: "prompts",
        label: "Prompt",
        name: "Fix First",
        enabled: true,
      },
      {
        id: "mcp:1003",
        resourceId: 1003,
        type: "mcp",
        label: "MCP",
        name: "filesystem",
        enabled: true,
      },
      {
        id: "mcp:1004",
        resourceId: 1004,
        type: "mcp",
        label: "MCP",
        name: "github",
        enabled: true,
      },
      {
        id: "skill:1002",
        resourceId: 1002,
        type: "skills",
        label: "Skill",
        name: "code-review",
        enabled: true,
      },
      {
        id: "skill:1003",
        resourceId: 1003,
        type: "skills",
        label: "Skill",
        name: "test-writer",
        enabled: true,
      },
    ],
  },
  {
    cliKey: "gemini",
    cliLabel: "Gemini",
    workspaceId: 1003,
    workspaceName: "工作区 Beta",
    workspaces: [{ id: 1003, name: "工作区 Beta", isActive: true }],
    loading: false,
    items: [
      {
        id: "mcp:1005",
        resourceId: 1005,
        type: "mcp",
        label: "MCP",
        name: "browser-tools",
        enabled: true,
      },
      {
        id: "mcp:1006",
        resourceId: 1006,
        type: "mcp",
        label: "MCP",
        name: "figma",
        enabled: false,
      },
      {
        id: "skill:1004",
        resourceId: 1004,
        type: "skills",
        label: "Skill",
        name: "ux-auditor",
        enabled: true,
      },
      {
        id: "skill:1005",
        resourceId: 1005,
        type: "skills",
        label: "Skill",
        name: "spec-writer",
        enabled: false,
      },
    ],
  },
];

export function HomeOverviewPanel({
  displayOptions,
  devPreviewEnabled = false,
  cliPriorityOrder,
  usageWindowDays,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
  sortModes,
  sortModesLoading,
  sortModesAvailable,
  activeModeByCli,
  activeModeToggling,
  onSetCliActiveMode,
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
  workspaceConfigs,
  togglingWorkspaceConfigItemIds = new Set<string>(),
  switchingWorkspaceKey = null,
  onSwitchWorkspace,
  onToggleWorkspaceConfigItemEnabled,
  providerLimitRows,
  providerLimitLoading,
  providerLimitAvailable,
  providerLimitRefreshing,
  onRefreshProviderLimit,
  oauthQuotaRows,
  oauthQuotaVisible,
  oauthQuotaRefreshing,
  oauthQuotaHasRefreshed,
  onRefreshOAuthQuota,
  onRefreshOAuthQuotaRow,
  onResetOAuthQuotaRow,
  openCircuits,
  onResetCircuitProvider,
  resettingCircuitProviderIds,
  traces,
  requestLogs,
  activeRequests = [],
  requestLogsLoading,
  requestLogsRefreshing,
  requestLogsAvailable,
  onRefreshRequestLogs,
  selectedLogId,
  onSelectLogId,
  personalizedUsageView,
}: HomeOverviewPanelProps) {
  const showCustomTooltip = displayOptions.customTooltip;
  const showHomeHeatmap = displayOptions.heatmap;
  const showHomeUsage = displayOptions.usage;
  const showWorkspaceConfigQuickToggle = displayOptions.workspaceConfigQuickToggle;
  const [sessionsTabsOrder] = useState<HomeOverviewTabKey[]>(() =>
    readHomeOverviewTabOrderFromStorage()
  );
  const [logsPrimaryLayout] = useState(() => readHomeOverviewLogsPrimaryLayoutFromStorage());
  const [sessionsTabState, setSessionsTabState] = useState<SessionsTabState>(() => ({
    tab: sessionsTabsOrder[0] ?? "workspaceConfig",
    openCircuitKeys: null,
  }));
  const [selectedWorkspaceConfigCliKey, setSelectedWorkspaceConfigCliKey] = useState<CliKey | null>(
    null
  );
  const circuitPreviewActive = openCircuits.length === 0 && devPreviewEnabled;
  const displayedCircuits = circuitPreviewActive ? PREVIEW_CIRCUITS : openCircuits;
  const displayedActiveSessions =
    devPreviewEnabled && activeSessions.length === 0 ? PREVIEW_ACTIVE_SESSIONS : activeSessions;
  const displayedProviderLimitRows =
    devPreviewEnabled && providerLimitRows.length === 0
      ? PREVIEW_PROVIDER_LIMIT_ROWS
      : providerLimitRows;
  const oauthQuotaPreviewActive = devPreviewEnabled;
  const displayedOAuthQuotaRows = oauthQuotaPreviewActive
    ? PREVIEW_OAUTH_QUOTA_ROWS
    : oauthQuotaRows;
  const displayedOAuthQuotaVisible = oauthQuotaPreviewActive || oauthQuotaVisible;
  const displayedOAuthQuotaHasRefreshed = oauthQuotaPreviewActive ? true : oauthQuotaHasRefreshed;
  const displayedOAuthQuotaRefreshing = oauthQuotaPreviewActive ? false : oauthQuotaRefreshing;
  const displayedWorkspaceConfigs = useDisplayedWorkspaceConfigs({
    cliPriorityOrder,
    devPreviewEnabled,
    workspaceConfigs,
  });
  const { legacySessionsTabs, logsPrimaryTabs, sessionsTab, setSessionsTab } = useHomeOverviewTabs({
    displayedOAuthQuotaVisible,
    logsPrimaryLayout,
    openCircuits,
    sessionsTabState,
    sessionsTabsOrder,
    setSessionsTabState,
  });
  const workspacePanelState = useWorkspaceConfigPanelState({
    activeModeByCli,
    activeModeToggling,
    displayedWorkspaceConfigs,
    selectedWorkspaceConfigCliKey,
    setSelectedWorkspaceConfigCliKey,
    showQuickToggle: showWorkspaceConfigQuickToggle,
    sortModes,
    sortModesAvailable,
    sortModesLoading,
    togglingItemIds: togglingWorkspaceConfigItemIds,
    switchingWorkspaceKey,
    onSetCliActiveMode,
    onSwitchWorkspace,
    onToggleItemEnabled: onToggleWorkspaceConfigItemEnabled,
  });
  const circuitNowUnix = useNowUnix(sessionsTab === "circuit" && displayedCircuits.length > 0);

  const requestLogsPanel = (
    <HomeRequestLogsPanel
      displayOptions={{
        customTooltip: showCustomTooltip,
        summaryText: false,
        openLogsPageButton: false,
        refreshButton: !logsPrimaryLayout,
        compactModeToggle: !logsPrimaryLayout,
      }}
      devPreviewEnabled={devPreviewEnabled}
      compactModeOverride={logsPrimaryLayout ? true : undefined}
      traces={traces}
      requestLogs={requestLogs}
      activeRequests={activeRequests}
      requestLogsLoading={requestLogsLoading}
      requestLogsRefreshing={requestLogsRefreshing}
      requestLogsAvailable={requestLogsAvailable}
      onRefreshRequestLogs={onRefreshRequestLogs}
      selectedLogId={selectedLogId}
      onSelectLogId={onSelectLogId}
    />
  );

  const oauthQuotaPanelContent = (
    <Suspense fallback={<OverviewPanelFallback />}>
      <LazyHomeOAuthQuotaPanelContent
        rows={displayedOAuthQuotaRows}
        hasProviders={displayedOAuthQuotaVisible}
        hasRefreshed={displayedOAuthQuotaHasRefreshed}
        refreshing={displayedOAuthQuotaRefreshing}
        onRefresh={() => {
          if (oauthQuotaPreviewActive) return;
          void onRefreshOAuthQuota();
        }}
        onRefreshRow={(providerId) => {
          if (oauthQuotaPreviewActive) return;
          void onRefreshOAuthQuotaRow(providerId);
        }}
        onResetRow={(providerId) => {
          if (oauthQuotaPreviewActive) return;
          if (!onResetOAuthQuotaRow) return;
          return onResetOAuthQuotaRow(providerId);
        }}
      />
    </Suspense>
  );

  const activeSessionsPanelState: ActiveSessionsPanelState = {
    sessions: displayedActiveSessions,
    loading: activeSessionsLoading,
    available: activeSessionsAvailable,
  };
  const providerLimitPanelState: ProviderLimitPanelState = {
    rows: displayedProviderLimitRows,
    loading: providerLimitLoading,
    available: providerLimitAvailable,
    refreshing: providerLimitRefreshing,
    onRefresh: onRefreshProviderLimit,
  };
  const circuitPanelState: CircuitPanelState = {
    rows: displayedCircuits,
    nowUnix: circuitNowUnix,
    previewActive: circuitPreviewActive,
    resettingProviderIds: resettingCircuitProviderIds,
    onResetProvider: onResetCircuitProvider,
  };

  const overviewInfoPanel = (
    <OverviewInfoPanel
      tabs={legacySessionsTabs}
      tab={sessionsTab}
      onTabChange={setSessionsTab}
      activeSessions={activeSessionsPanelState}
      workspace={workspacePanelState}
      providerLimit={providerLimitPanelState}
      oauthQuotaPanelContent={oauthQuotaPanelContent}
      circuit={circuitPanelState}
    />
  );

  const logsPrimaryInfoPanel = (
    <LogsPrimaryInfoPanel
      tabs={logsPrimaryTabs}
      tab={sessionsTab}
      onTabChange={setSessionsTab}
      workspace={workspacePanelState}
      oauthQuotaPanelContent={oauthQuotaPanelContent}
      circuit={circuitPanelState}
    />
  );

  return (
    <div className="flex flex-col h-full gap-4">
      {!logsPrimaryLayout ? (
        <HomeOverviewUsageStrip
          devPreviewEnabled={devPreviewEnabled}
          showHeatmap={showHomeHeatmap}
          showUsageChart={showHomeUsage}
          usageWindowDays={usageWindowDays}
          usageHeatmapRows={usageHeatmapRows}
          usageHeatmapLoading={usageHeatmapLoading}
          onRefreshUsageHeatmap={onRefreshUsageHeatmap}
        />
      ) : null}

      <HomeOverviewContentLayout
        activeSessions={displayedActiveSessions}
        devPreviewEnabled={devPreviewEnabled}
        logsPrimaryInfoPanel={logsPrimaryInfoPanel}
        logsPrimaryLayout={logsPrimaryLayout}
        overviewInfoPanel={overviewInfoPanel}
        personalizedUsageView={personalizedUsageView}
        requestLogs={requestLogs}
        activeRequests={activeRequests}
        requestLogsPanel={requestLogsPanel}
        traces={traces}
        usageWindowDays={usageWindowDays}
        usageHeatmapRows={usageHeatmapRows}
        usageHeatmapLoading={usageHeatmapLoading}
        onRefreshUsageHeatmap={onRefreshUsageHeatmap}
      />
    </div>
  );
}
