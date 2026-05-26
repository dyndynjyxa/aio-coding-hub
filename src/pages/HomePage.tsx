// Usage: Dashboard / overview page. Backend commands: `request_logs_*`, `request_attempt_logs_*`, `usage_*`, `gateway_*`, `providers_*`, `sort_modes_*`, `provider_limit_usage_*`.

import { lazy, Suspense, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { CLIS } from "../constants/clis";
import {
  HomeOverviewPanel,
  type HomeOverviewUsageView,
} from "../components/home/HomeOverviewPanel";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { useGatewaySessionsListQuery } from "../query/gateway";
import { useSettingsQuery } from "../query/settings";
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
import { DEFAULT_HOME_USAGE_PERIOD } from "../utils/homeUsagePeriod";
import { resolveHomeUsageWindowDays } from "../utils/homeUsagePeriod";
import { useHomeCircuitState } from "./home/hooks/useHomeCircuitState";
import { useHomeSortMode } from "./home/hooks/useHomeSortMode";
import { useHomeOverviewFeed } from "./home/hooks/useHomeOverviewFeed";
import { useHomeOAuthQuota } from "./home/hooks/useHomeOAuthQuota";
import { useHomeWorkspaceConfigs } from "./home/hooks/useHomeWorkspaceConfigs";

type HomeTabKey = "overview" | "cost" | "tokenCost";

function buildHomeTabs(
  personalizedLayoutEnabled: boolean
): Array<{ key: HomeTabKey; label: string }> {
  return personalizedLayoutEnabled
    ? [
        { key: "overview", label: "概览" },
        { key: "tokenCost", label: "用量" },
      ]
    : [
        { key: "overview", label: "概览" },
        { key: "cost", label: "花费" },
        { key: "tokenCost", label: "用量" },
      ];
}

const LazyHomeCostPanel = lazy(() =>
  import("../components/home/HomeCostPanel").then((m) => ({ default: m.HomeCostPanel }))
);

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

export function HomePage() {
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
  const homeTabs = useMemo(
    () => buildHomeTabs(personalizedLayoutEnabled),
    [personalizedLayoutEnabled]
  );

  const [tab, setTab] = useState<HomeTabKey>("overview");
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const [personalizedUsageView, setPersonalizedUsageView] =
    useState<HomeOverviewUsageView>("summary");
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
  const workspaceConfigs = useHomeWorkspaceConfigs({ enabled: tab === "overview" });
  const oauthQuota = useHomeOAuthQuota({
    cliPriorityOrder,
    requestLogs,
    enabled: tab === "overview",
  });
  const { pendingSortModeSwitch } = sortMode;

  useEffect(() => {
    if (personalizedLayoutEnabled && tab === "cost") setTab("tokenCost");
  }, [personalizedLayoutEnabled, tab]);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="shrink-0 mb-5">
        <PageHeader
          title="首页"
          actions={
            <>
              {isDevMode ? (
                <Button
                  variant={devPreview.enabled ? "primary" : "secondary"}
                  size="md"
                  onClick={() => devPreview.toggle()}
                >
                  {devPreview.enabled ? "Dev关闭预览数据" : "Dev开启预览数据"}
                </Button>
              ) : null}
              {personalizedLayoutEnabled && tab === "overview" ? (
                <Button
                  variant="secondary"
                  size="md"
                  onClick={() =>
                    setPersonalizedUsageView((current) =>
                      current === "summary" ? "usageChart" : "summary"
                    )
                  }
                >
                  {personalizedUsageView === "summary" ? "查看曲线" : "查看总览"}
                </Button>
              ) : null}
              <TabList ariaLabel="首页视图切换" items={homeTabs} value={tab} onChange={setTab} />
            </>
          }
        />
      </div>

      <div className="flex-1 min-h-0">
        {tab === "overview" ? (
          <HomeOverviewPanel
            showCustomTooltip={showCustomTooltip}
            devPreviewEnabled={devPreview.enabled}
            showHomeHeatmap={showHomeHeatmap}
            showHomeUsage={showHomeUsage}
            cliPriorityOrder={cliPriorityOrder}
            usageWindowDays={homeUsageWindowDays}
            usageHeatmapRows={usageHeatmapRows}
            usageHeatmapLoading={usageHeatmapLoading}
            onRefreshUsageHeatmap={refreshUsageHeatmap}
            sortModes={sortMode.sortModes}
            sortModesLoading={sortMode.sortModesLoading}
            sortModesAvailable={sortMode.sortModesAvailable}
            activeModeByCli={sortMode.activeModeByCli}
            activeModeToggling={sortMode.activeModeToggling}
            onSetCliActiveMode={sortMode.requestCliActiveModeSwitch}
            activeSessions={activeSessions}
            activeSessionsLoading={activeSessionsLoading}
            activeSessionsAvailable={activeSessionsAvailable}
            workspaceConfigs={workspaceConfigs}
            providerLimitRows={providerLimitRows}
            providerLimitLoading={providerLimitLoading}
            providerLimitAvailable={providerLimitAvailable}
            providerLimitRefreshing={providerLimitRefreshing}
            onRefreshProviderLimit={refreshProviderLimit}
            oauthQuotaRows={oauthQuota.oauthQuotaRows}
            oauthQuotaVisible={oauthQuota.oauthQuotaVisible}
            oauthQuotaRefreshing={oauthQuota.oauthQuotaRefreshing}
            oauthQuotaHasRefreshed={oauthQuota.oauthQuotaHasRefreshed}
            onRefreshOAuthQuota={oauthQuota.refreshOAuthQuota}
            onRefreshOAuthQuotaRow={oauthQuota.refreshOAuthQuotaRow}
            openCircuits={circuit.openCircuits}
            onResetCircuitProvider={circuit.handleResetProvider}
            resettingCircuitProviderIds={circuit.resettingProviderIds}
            traces={traces}
            requestLogs={requestLogs}
            requestLogsLoading={requestLogsLoading}
            requestLogsRefreshing={requestLogsRefreshing}
            requestLogsAvailable={requestLogsAvailable}
            onRefreshRequestLogs={refreshRequestLogs}
            selectedLogId={selectedLogId}
            onSelectLogId={setSelectedLogId}
            personalizedUsageView={personalizedUsageView}
          />
        ) : tab === "cost" ? (
          <Suspense
            fallback={
              <Card padding="md" className="flex h-full items-center justify-center">
                <div className="flex items-center gap-3 text-sm text-muted-foreground">
                  <Spinner />
                  <span>加载花费面板中…</span>
                </div>
              </Card>
            }
          >
            <LazyHomeCostPanel devPreviewEnabled={devPreview.enabled} />
          </Suspense>
        ) : tab === "tokenCost" ? (
          <Suspense
            fallback={
              <Card padding="md" className="flex h-full items-center justify-center">
                <div className="flex items-center gap-3 text-sm text-muted-foreground">
                  <Spinner />
                  <span>加载用量面板中…</span>
                </div>
              </Card>
            }
          >
            <LazyHomeTokenCostPanel devPreviewEnabled={devPreview.enabled} />
          </Suspense>
        ) : (
          <div />
        )}
      </div>

      <Dialog
        open={pendingSortModeSwitch != null}
        onOpenChange={(open) => {
          if (!open) sortMode.setPendingSortModeSwitch(null);
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
      >
        <div className="flex items-center justify-end gap-2">
          <Button
            variant="secondary"
            size="md"
            onClick={() => sortMode.setPendingSortModeSwitch(null)}
          >
            取消
          </Button>
          <Button variant="primary" size="md" onClick={sortMode.confirmPendingSortModeSwitch}>
            确认切换
          </Button>
        </div>
      </Dialog>

      {selectedLogId != null ? (
        <Suspense
          fallback={
            <Dialog
              open
              onOpenChange={(open) => {
                if (!open) setSelectedLogId(null);
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
          <LazyRequestLogDetailDialog
            selectedLogId={selectedLogId}
            onSelectLogId={setSelectedLogId}
          />
        </Suspense>
      ) : null}
    </div>
  );
}
