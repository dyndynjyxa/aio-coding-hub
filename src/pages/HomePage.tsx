// Usage: Dashboard / overview page. Backend commands: `request_logs_*`, `request_attempt_logs_*`, `usage_*`, `gateway_*`, `providers_*`, `sort_modes_*`, `provider_limit_usage_*`.

import { lazy, Suspense, useMemo, useReducer, useSyncExternalStore, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CLIS } from "../constants/clis";
import {
  HomeOverviewPanel,
  type HomeOverviewPanelProps,
  type HomeOverviewUsageView,
} from "../components/home/HomeOverviewPanel";
import type { HomeWorkspaceConfigItem } from "../components/home/homeWorkspaceConfigTypes";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { useGatewaySessionsListQuery } from "../query/gateway";
import { mcpKeys, promptsKeys, skillsKeys, workspacesKeys } from "../query/keys";
import { useSettingsQuery } from "../query/settings";
import { useWorkspaceApplyMutation } from "../query/workspaces";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { PageHeader } from "../ui/PageHeader";
import { Spinner } from "../ui/Spinner";
import { TabList } from "../ui/TabList";
import { normalizeCliPriorityOrder } from "../services/cli/cliPriorityOrder";
import { useTraceStore } from "../services/gateway/traceStore";
import {
  readHomeOverviewLogsPrimaryLayoutFromStorage,
  subscribeHomeOverviewLogsPrimaryLayout,
} from "../services/home/homeOverviewLayout";
import {
  readHomeWorkspaceConfigShowAllFromStorage,
  subscribeHomeWorkspaceConfigShowAll,
} from "../services/home/homeWorkspaceConfigDisplay";
import { promptSetEnabled } from "../services/workspace/prompts";
import { mcpServerSetEnabled } from "../services/workspace/mcp";
import { skillSetEnabled } from "../services/workspace/skills";
import { logToConsole } from "../services/consoleLog";
import type { CliKey } from "../services/providers/providers";
import { DEFAULT_HOME_USAGE_PERIOD } from "../utils/homeUsagePeriod";
import { resolveHomeUsageWindowDays } from "../utils/homeUsagePeriod";
import { useHomeCircuitState } from "./home/hooks/useHomeCircuitState";
import { useHomeSortMode } from "./home/hooks/useHomeSortMode";
import { useHomeOverviewFeed } from "./home/hooks/useHomeOverviewFeed";
import { useHomeOAuthQuota } from "./home/hooks/useHomeOAuthQuota";
import { useHomeWorkspaceConfigs } from "./home/hooks/useHomeWorkspaceConfigs";

type HomeTabKey = "overview" | "tokenCost";

type HomeTabItem = {
  key: HomeTabKey;
  label: string;
};

const HOME_TABS: HomeTabItem[] = [
  { key: "overview", label: "概览" },
  { key: "tokenCost", label: "用量" },
];

type HomeUiState = {
  tab: HomeTabKey;
  selectedLogId: number | null;
  togglingWorkspaceConfigItemId: string | null;
  switchingWorkspaceKey: string | null;
  personalizedUsageView: HomeOverviewUsageView;
};

type HomeUiAction =
  | { type: "setTab"; tab: HomeTabKey }
  | { type: "setSelectedLogId"; selectedLogId: number | null }
  | { type: "setTogglingWorkspaceConfigItemId"; itemId: string | null }
  | { type: "setSwitchingWorkspaceKey"; key: string | null }
  | { type: "togglePersonalizedUsageView" };

const initialHomeUiState: HomeUiState = {
  tab: "overview",
  selectedLogId: null,
  togglingWorkspaceConfigItemId: null,
  switchingWorkspaceKey: null,
  personalizedUsageView: "summary",
};

function homeUiReducer(state: HomeUiState, action: HomeUiAction): HomeUiState {
  switch (action.type) {
    case "setTab":
      return { ...state, tab: action.tab };
    case "setSelectedLogId":
      return { ...state, selectedLogId: action.selectedLogId };
    case "setTogglingWorkspaceConfigItemId":
      return { ...state, togglingWorkspaceConfigItemId: action.itemId };
    case "setSwitchingWorkspaceKey":
      return { ...state, switchingWorkspaceKey: action.key };
    case "togglePersonalizedUsageView":
      return {
        ...state,
        personalizedUsageView: state.personalizedUsageView === "summary" ? "usageChart" : "summary",
      };
  }
}

const LazyHomeTokenCostPanel = lazy(() =>
  import("../components/home/HomeTokenCostPanel").then((m) => ({
    default: m.HomeTokenCostPanel,
  }))
);

const LazyRequestLogDetailDialog = lazy(() =>
  import("../components/home/RequestLogDetailDialog").then((m) => ({
    default: m.RequestLogDetailDialog,
  }))
);

type PendingSortModeSwitch = ReturnType<typeof useHomeSortMode>["pendingSortModeSwitch"];

function HomePageHeaderActions({
  devPreviewEnabled,
  isDevMode,
  personalizedLayoutEnabled,
  personalizedUsageView,
  tab,
  tabs,
  onTabChange,
  onToggleDevPreview,
  onTogglePersonalizedUsageView,
}: {
  devPreviewEnabled: boolean;
  isDevMode: boolean;
  personalizedLayoutEnabled: boolean;
  personalizedUsageView: HomeOverviewUsageView;
  tab: HomeTabKey;
  tabs: HomeTabItem[];
  onTabChange: (tab: HomeTabKey) => void;
  onToggleDevPreview: () => void;
  onTogglePersonalizedUsageView: () => void;
}) {
  return (
    <>
      {isDevMode ? (
        <Button
          variant={devPreviewEnabled ? "primary" : "secondary"}
          size="md"
          onClick={onToggleDevPreview}
        >
          {devPreviewEnabled ? "Dev关闭预览数据" : "Dev开启预览数据"}
        </Button>
      ) : null}
      {personalizedLayoutEnabled && tab === "overview" ? (
        <Button variant="secondary" size="md" onClick={onTogglePersonalizedUsageView}>
          {personalizedUsageView === "summary" ? "今日总览" : "用量趋势"}
        </Button>
      ) : null}
      <TabList ariaLabel="首页视图切换" items={tabs} value={tab} onChange={onTabChange} />
    </>
  );
}

function HomeTabLoadingFallback({ label }: { label: string }) {
  return (
    <Card padding="md" className="flex h-full items-center justify-center">
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <Spinner />
        <span>{label}</span>
      </div>
    </Card>
  );
}

function HomePageTabContent({
  devPreviewEnabled,
  overviewPanel,
  tab,
}: {
  devPreviewEnabled: boolean;
  overviewPanel: ReactNode;
  tab: HomeTabKey;
}) {
  if (tab === "overview") return overviewPanel;

  return (
    <Suspense fallback={<HomeTabLoadingFallback label="加载用量面板中…" />}>
      <LazyHomeTokenCostPanel devPreviewEnabled={devPreviewEnabled} />
    </Suspense>
  );
}

function SortModeConfirmDialog({
  pendingSortModeSwitch,
  onCancel,
  onConfirm,
}: {
  pendingSortModeSwitch: PendingSortModeSwitch;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <Dialog
      open={pendingSortModeSwitch != null}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      title={
        pendingSortModeSwitch
          ? `确认切换 ${CLIS.find((cli) => cli.key === pendingSortModeSwitch.cliKey)?.name ?? pendingSortModeSwitch.cliKey} 模板？`
          : "确认切换模板？"
      }
      description={
        pendingSortModeSwitch
          ? `目前还有 ${pendingSortModeSwitch.activeSessionCount} 个活跃 Session，切换模板可能导致会话中断，是否确认？`
          : undefined
      }
      className="max-w-lg"
    >
      <div className="flex items-center justify-end gap-2">
        <Button variant="secondary" size="md" onClick={onCancel}>
          取消
        </Button>
        <Button variant="primary" size="md" onClick={onConfirm}>
          确认切换
        </Button>
      </div>
    </Dialog>
  );
}

function RequestLogDetailDialogSlot({
  selectedLogId,
  onSelectLogId,
}: {
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
}) {
  if (selectedLogId == null) return null;

  return (
    <Suspense
      fallback={
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open) onSelectLogId(null);
          }}
          title="代理记录详情"
          description="先看关键指标，再看为什么会重试、跳过或切换供应商。"
          className="max-w-3xl"
        >
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <Spinner />
            <span>加载代理记录详情中…</span>
          </div>
        </Dialog>
      }
    >
      <LazyRequestLogDetailDialog selectedLogId={selectedLogId} onSelectLogId={onSelectLogId} />
    </Suspense>
  );
}

function HomePageFrame({
  headerActions,
  tabContent,
  pendingSortModeSwitch,
  selectedLogId,
  onCancelSortModeSwitch,
  onConfirmSortModeSwitch,
  onSelectLogId,
}: {
  headerActions: ReactNode;
  tabContent: ReactNode;
  pendingSortModeSwitch: PendingSortModeSwitch;
  selectedLogId: number | null;
  onCancelSortModeSwitch: () => void;
  onConfirmSortModeSwitch: () => void;
  onSelectLogId: (id: number | null) => void;
}) {
  return (
    <div className="flex flex-col h-full gap-6 overflow-hidden">
      <div className="shrink-0">
        <PageHeader title="首页" actions={headerActions} />
      </div>

      <div className="flex-1 min-h-0">{tabContent}</div>

      <SortModeConfirmDialog
        pendingSortModeSwitch={pendingSortModeSwitch}
        onCancel={onCancelSortModeSwitch}
        onConfirm={onConfirmSortModeSwitch}
      />

      <RequestLogDetailDialogSlot selectedLogId={selectedLogId} onSelectLogId={onSelectLogId} />
    </div>
  );
}

export function HomePage() {
  const queryClient = useQueryClient();
  const { traces } = useTraceStore();
  const showCustomTooltip = true;
  const foregroundActive = useDocumentVisibility();
  const settingsQuery = useSettingsQuery();
  const showHomeHeatmap = settingsQuery.data?.show_home_heatmap ?? true;
  const showHomeUsage = settingsQuery.data?.show_home_usage ?? true;
  const showOverviewUsageSection = showHomeHeatmap || showHomeUsage;
  const homeUsagePeriod = settingsQuery.data?.home_usage_period ?? DEFAULT_HOME_USAGE_PERIOD;
  const homeUsageWindowDays = resolveHomeUsageWindowDays(homeUsagePeriod);
  const cliPriorityOrder = normalizeCliPriorityOrder(settingsQuery.data?.cli_priority_order);
  const isDevMode = import.meta.env.DEV;
  const devPreview = useDevPreviewData();
  const personalizedLayoutEnabled = useSyncExternalStore(
    subscribeHomeOverviewLogsPrimaryLayout,
    readHomeOverviewLogsPrimaryLayoutFromStorage,
    () => false
  );
  const showAllWorkspaceConfigItems = useSyncExternalStore(
    subscribeHomeWorkspaceConfigShowAll,
    readHomeWorkspaceConfigShowAllFromStorage,
    () => false
  );
  const [homeUiState, dispatchHomeUi] = useReducer(homeUiReducer, initialHomeUiState);
  const tab = homeUiState.tab;
  const {
    selectedLogId,
    togglingWorkspaceConfigItemId,
    switchingWorkspaceKey,
    personalizedUsageView,
  } = homeUiState;
  const setSelectedLogId = (selectedLogId: number | null) =>
    dispatchHomeUi({ type: "setSelectedLogId", selectedLogId });
  const personalizedUsageChartVisible =
    personalizedLayoutEnabled && personalizedUsageView === "usageChart";
  const overviewUsageSeriesEnabled =
    tab === "overview" &&
    (personalizedUsageChartVisible || (!personalizedLayoutEnabled && showOverviewUsageSection));
  const shouldRefetchOverviewUsageSeries =
    personalizedUsageChartVisible || (!personalizedLayoutEnabled && showOverviewUsageSection);

  // --- Delegated state hooks ---
  const circuit = useHomeCircuitState();

  const overviewForegroundPollingEnabled = tab === "overview" && foregroundActive;

  const sessionsQuery = useGatewaySessionsListQuery(50, {
    enabled: overviewForegroundPollingEnabled,
    refetchIntervalMs: overviewForegroundPollingEnabled ? 5000 : false,
  });
  const activeSessions = sessionsQuery.data ?? [];
  const activeSessionsLoading = sessionsQuery.isLoading;
  const activeSessionsAvailable: boolean | null = sessionsQuery.isLoading
    ? null
    : sessionsQuery.data != null;

  const {
    usageHeatmapRows,
    usageHeatmapLoading,
    providerLimitRows,
    providerLimitLoading,
    providerLimitRefreshing,
    providerLimitAvailable,
    requestLogs,
    activeRequests,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsAvailable,
    refreshUsageHeatmap,
    refreshProviderLimit,
    refreshRequestLogs,
  } = useHomeOverviewFeed({
    overviewActive: tab === "overview",
    foregroundActive,
    overviewUsageSeriesEnabled,
    shouldRefetchOverviewUsageSeries,
    homeUsageWindowDays,
    providerLimitEnabled: !personalizedLayoutEnabled,
  });
  const sortMode = useHomeSortMode(activeSessions);
  const workspaceConfigs = useHomeWorkspaceConfigs({
    enabled: tab === "overview",
    showAllItems: showAllWorkspaceConfigItems,
  });
  const workspaceApplyMutation = useWorkspaceApplyMutation();
  const workspaceConfigToggleMutation = useMutation({
    mutationFn: async (input: {
      workspaceId: number;
      item: HomeWorkspaceConfigItem;
      enabled: boolean;
    }) => {
      if (input.item.type === "prompts") {
        return promptSetEnabled(input.item.resourceId, input.enabled);
      }
      if (input.item.type === "mcp") {
        return mcpServerSetEnabled({
          workspaceId: input.workspaceId,
          serverId: input.item.resourceId,
          enabled: input.enabled,
        });
      }
      return skillSetEnabled({
        workspaceId: input.workspaceId,
        skillId: input.item.resourceId,
        enabled: input.enabled,
      });
    },
    onSettled: async (_result, _error, input) => {
      if (!input) return;
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: promptsKeys.summary(input.workspaceId) }),
        queryClient.invalidateQueries({ queryKey: promptsKeys.list(input.workspaceId) }),
        queryClient.invalidateQueries({ queryKey: mcpKeys.serversList(input.workspaceId) }),
        queryClient.invalidateQueries({ queryKey: skillsKeys.installedList(input.workspaceId) }),
      ]);
      dispatchHomeUi({ type: "setTogglingWorkspaceConfigItemId", itemId: null });
    },
  });
  const oauthQuota = useHomeOAuthQuota({
    cliPriorityOrder,
    requestLogs,
    enabled: tab === "overview",
  });
  const { pendingSortModeSwitch } = sortMode;
  const togglingWorkspaceConfigItemIds = useMemo(
    () => new Set(togglingWorkspaceConfigItemId ? [togglingWorkspaceConfigItemId] : []),
    [togglingWorkspaceConfigItemId]
  );

  function toggleWorkspaceConfigItem(
    workspaceId: number,
    item: HomeWorkspaceConfigItem,
    enabled: boolean
  ) {
    if (workspaceConfigToggleMutation.isPending) return;
    dispatchHomeUi({ type: "setTogglingWorkspaceConfigItemId", itemId: item.id });
    workspaceConfigToggleMutation.mutate({ workspaceId, item, enabled });
  }

  async function switchWorkspace(cliKey: CliKey, workspaceId: number) {
    if (switchingWorkspaceKey != null || workspaceApplyMutation.isPending) return;

    const config = workspaceConfigs.find((row) => row.cliKey === cliKey);
    if (config?.workspaceId === workspaceId) return;

    const nextSwitchingWorkspaceKey = `${cliKey}:${workspaceId}`;
    dispatchHomeUi({ type: "setSwitchingWorkspaceKey", key: nextSwitchingWorkspaceKey });

    try {
      const report = await workspaceApplyMutation.mutateAsync({ cliKey, workspaceId });
      if (report) {
        toast("已切换为当前工作区");
      }
    } catch (error) {
      logToConsole("error", "首页切换工作区失败", {
        cliKey,
        workspaceId,
        error: String(error),
      });
      toast(`切换失败：${String(error)}`);
    } finally {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: workspacesKeys.list(cliKey) }),
        queryClient.invalidateQueries({ queryKey: promptsKeys.summary(workspaceId) }),
        queryClient.invalidateQueries({ queryKey: promptsKeys.list(workspaceId) }),
        queryClient.invalidateQueries({ queryKey: mcpKeys.serversList(workspaceId) }),
        queryClient.invalidateQueries({ queryKey: skillsKeys.installedList(workspaceId) }),
      ]);
      dispatchHomeUi({ type: "setSwitchingWorkspaceKey", key: null });
    }
  }

  const overviewPanelProps: HomeOverviewPanelProps = {
    displayOptions: {
      customTooltip: showCustomTooltip,
      heatmap: showHomeHeatmap,
      usage: showHomeUsage,
      workspaceConfigQuickToggle: showAllWorkspaceConfigItems,
    },
    devPreviewEnabled: devPreview.enabled,
    cliPriorityOrder,
    usageWindowDays: homeUsageWindowDays,
    usageHeatmapRows,
    usageHeatmapLoading,
    onRefreshUsageHeatmap: refreshUsageHeatmap,
    sortModes: sortMode.sortModes,
    sortModesLoading: sortMode.sortModesLoading,
    sortModesAvailable: sortMode.sortModesAvailable,
    activeModeByCli: sortMode.activeModeByCli,
    activeModeToggling: sortMode.activeModeToggling,
    onSetCliActiveMode: sortMode.requestCliActiveModeSwitch,
    activeSessions,
    activeSessionsLoading,
    activeSessionsAvailable,
    workspaceConfigs,
    togglingWorkspaceConfigItemIds,
    switchingWorkspaceKey,
    onSwitchWorkspace: (cliKey, workspaceId) => {
      void switchWorkspace(cliKey, workspaceId);
    },
    onToggleWorkspaceConfigItemEnabled: toggleWorkspaceConfigItem,
    providerLimitRows,
    providerLimitLoading,
    providerLimitAvailable,
    providerLimitRefreshing,
    onRefreshProviderLimit: refreshProviderLimit,
    oauthQuotaRows: oauthQuota.oauthQuotaRows,
    oauthQuotaVisible: oauthQuota.oauthQuotaVisible,
    oauthQuotaRefreshing: oauthQuota.oauthQuotaRefreshing,
    oauthQuotaHasRefreshed: oauthQuota.oauthQuotaHasRefreshed,
    onRefreshOAuthQuota: oauthQuota.refreshOAuthQuota,
    onRefreshOAuthQuotaRow: oauthQuota.refreshOAuthQuotaRow,
    onResetOAuthQuotaRow: oauthQuota.resetOAuthQuotaRow,
    openCircuits: circuit.openCircuits,
    onResetCircuitProvider: circuit.handleResetProvider,
    resettingCircuitProviderIds: circuit.resettingProviderIds,
    traces,
    requestLogs,
    activeRequests,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsAvailable,
    onRefreshRequestLogs: refreshRequestLogs,
    selectedLogId,
    onSelectLogId: setSelectedLogId,
    personalizedUsageView,
  };
  const headerActions = (
    <HomePageHeaderActions
      devPreviewEnabled={devPreview.enabled}
      isDevMode={isDevMode}
      personalizedLayoutEnabled={personalizedLayoutEnabled}
      personalizedUsageView={personalizedUsageView}
      tab={tab}
      tabs={HOME_TABS}
      onTabChange={(tab) => dispatchHomeUi({ type: "setTab", tab })}
      onToggleDevPreview={() => devPreview.toggle()}
      onTogglePersonalizedUsageView={() => dispatchHomeUi({ type: "togglePersonalizedUsageView" })}
    />
  );
  const tabContent = (
    <HomePageTabContent
      devPreviewEnabled={devPreview.enabled}
      overviewPanel={<HomeOverviewPanel {...overviewPanelProps} />}
      tab={tab}
    />
  );

  return (
    <HomePageFrame
      headerActions={headerActions}
      tabContent={tabContent}
      pendingSortModeSwitch={pendingSortModeSwitch}
      selectedLogId={selectedLogId}
      onCancelSortModeSwitch={() => sortMode.setPendingSortModeSwitch(null)}
      onConfirmSortModeSwitch={sortMode.confirmPendingSortModeSwitch}
      onSelectLogId={setSelectedLogId}
    />
  );
}
