// Usage: Dashboard / overview page. Backend commands: `request_logs_*`, `request_attempt_logs_*`, `usage_*`, `gateway_*`, `providers_*`, `sort_modes_*`, `provider_limit_usage_*`.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { CLIS } from "../constants/clis";
import { HomeCostPanel } from "../components/home/HomeCostPanel";
import { HomeOverviewPanel } from "../components/home/HomeOverviewPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { useWindowForeground } from "../hooks/useWindowForeground";
import { useGatewaySessionsListQuery } from "../query/gateway";
import { useProviderLimitUsageV1Query } from "../query/providerLimitUsage";
import {
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
  useRequestLogsIncrementalPollQuery,
  useRequestLogsListAllQuery,
} from "../query/requestLogs";
import { useSettingsQuery } from "../query/settings";
import { useUsageHourlySeriesQuery } from "../query/usage";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Dialog } from "../ui/Dialog";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { useTraceStore } from "../services/traceStore";
import { useHomeCircuitState } from "./home/hooks/useHomeCircuitState";
import { useHomeSortMode } from "./home/hooks/useHomeSortMode";
import { useHomeCliProxy } from "./home/hooks/useHomeCliProxy";

type HomeTabKey = "overview" | "cost" | "more";

const HOME_TABS: Array<{ key: HomeTabKey; label: string }> = [
  { key: "overview", label: "概览" },
  { key: "cost", label: "花费" },
  { key: "more", label: "更多" },
];

export function HomePage() {
  const { traces } = useTraceStore();
  const showCustomTooltip = true;
  const foregroundActive = useDocumentVisibility();
  const settingsQuery = useSettingsQuery();
  const showHomeHeatmap = settingsQuery.data?.show_home_heatmap ?? true;

  const [tab, setTab] = useState<HomeTabKey>("overview");
  const tabRef = useRef(tab);
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);

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

  const sortMode = useHomeSortMode(activeSessions);
  const cliProxyState = useHomeCliProxy();

  // --- Overview data queries ---
  const usageHeatmapQuery = useUsageHourlySeriesQuery(15, { enabled: tab === "overview" });
  const usageHeatmapRows = usageHeatmapQuery.data ?? [];
  const usageHeatmapLoading = usageHeatmapQuery.isFetching;

  const providerLimitQuery = useProviderLimitUsageV1Query(null, {
    enabled: overviewForegroundPollingEnabled,
    refetchIntervalMs: overviewForegroundPollingEnabled ? 30000 : false,
  });
  const providerLimitRows = providerLimitQuery.data ?? [];
  const providerLimitLoading = providerLimitQuery.isLoading;
  const providerLimitRefreshing = providerLimitQuery.isFetching && !providerLimitQuery.isLoading;
  const providerLimitAvailable: boolean | null = providerLimitQuery.isLoading
    ? null
    : providerLimitQuery.data != null;

  const requestLogsQuery = useRequestLogsListAllQuery(50, { enabled: tab === "overview" });
  useRequestLogsIncrementalPollQuery(50, {
    enabled: overviewForegroundPollingEnabled,
    refetchIntervalMs: overviewForegroundPollingEnabled ? 1000 : false,
  });
  const requestLogsRaw = requestLogsQuery.data;
  const requestLogs = useMemo(() => requestLogsRaw ?? [], [requestLogsRaw]);
  const requestLogsLoading = requestLogsQuery.isLoading;
  const requestLogsRefreshing = requestLogsQuery.isFetching && !requestLogsQuery.isLoading;
  const requestLogsAvailable: boolean | null = requestLogsQuery.isLoading
    ? null
    : requestLogsQuery.data != null;

  // --- Refresh callbacks ---
  const refreshUsageHeatmap = useCallback(() => {
    void usageHeatmapQuery.refetch().then((res) => {
      if (res.error) toast("刷新用量失败：请查看控制台日志");
    });
  }, [usageHeatmapQuery]);

  const refreshRequestLogs = useCallback(() => {
    void requestLogsQuery.refetch().then((res) => {
      if (res.error) toast("读取使用记录失败：请查看控制台日志");
    });
  }, [requestLogsQuery]);

  const refreshProviderLimit = useCallback(() => {
    void providerLimitQuery.refetch().then((res) => {
      if (res.error) toast("读取供应商限额失败：请查看控制台日志");
    });
  }, [providerLimitQuery]);

  // Refetch overview data when switching back to overview tab
  useEffect(() => {
    const prev = tabRef.current;
    tabRef.current = tab;
    if (prev !== "overview" && tab === "overview") {
      void usageHeatmapQuery.refetch();
      void requestLogsQuery.refetch();
      void providerLimitQuery.refetch();
    }
  }, [providerLimitQuery, requestLogsQuery, tab, usageHeatmapQuery]);

  useWindowForeground({
    enabled: tab === "overview",
    throttleMs: 1000,
    onForeground: () => {
      void usageHeatmapQuery.refetch();
      void requestLogsQuery.refetch();
      void providerLimitQuery.refetch();
    },
  });

  // --- Selected log detail ---
  const selectedLogQuery = useRequestLogDetailQuery(selectedLogId);
  const selectedLog = selectedLogQuery.data ?? null;
  const selectedLogLoading = selectedLogQuery.isFetching;

  const attemptLogsQuery = useRequestAttemptLogsByTraceIdQuery(selectedLog?.trace_id ?? null, 50);
  const attemptLogs = attemptLogsQuery.data ?? [];
  const attemptLogsLoading = attemptLogsQuery.isFetching;

  const { pendingSortModeSwitch } = sortMode;
  const { pendingCliProxyEnablePrompt } = cliProxyState;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="shrink-0 mb-5">
        <PageHeader
          title="首页"
          actions={
            <TabList ariaLabel="首页视图切换" items={HOME_TABS} value={tab} onChange={setTab} />
          }
        />
      </div>

      <div className="flex-1 min-h-0">
        {tab === "overview" ? (
          <HomeOverviewPanel
            showCustomTooltip={showCustomTooltip}
            showHomeHeatmap={showHomeHeatmap}
            usageHeatmapRows={usageHeatmapRows}
            usageHeatmapLoading={usageHeatmapLoading}
            onRefreshUsageHeatmap={refreshUsageHeatmap}
            sortModes={sortMode.sortModes}
            sortModesLoading={sortMode.sortModesLoading}
            sortModesAvailable={sortMode.sortModesAvailable}
            activeModeByCli={sortMode.activeModeByCli}
            activeModeToggling={sortMode.activeModeToggling}
            onSetCliActiveMode={sortMode.requestCliActiveModeSwitch}
            cliProxyEnabled={cliProxyState.cliProxyEnabled}
            cliProxyToggling={cliProxyState.cliProxyToggling}
            onSetCliProxyEnabled={cliProxyState.requestCliProxyEnabledSwitch}
            activeSessions={activeSessions}
            activeSessionsLoading={activeSessionsLoading}
            activeSessionsAvailable={activeSessionsAvailable}
            providerLimitRows={providerLimitRows}
            providerLimitLoading={providerLimitLoading}
            providerLimitAvailable={providerLimitAvailable}
            providerLimitRefreshing={providerLimitRefreshing}
            onRefreshProviderLimit={refreshProviderLimit}
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
          />
        ) : tab === "cost" ? (
          <HomeCostPanel />
        ) : (
          <Card padding="md">
            <div className="text-sm text-slate-600 dark:text-slate-400">更多功能开发中…</div>
          </Card>
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

      <Dialog
        open={pendingCliProxyEnablePrompt != null}
        onOpenChange={(open) => {
          if (!open) cliProxyState.setPendingCliProxyEnablePrompt(null);
        }}
        title={
          pendingCliProxyEnablePrompt
            ? `检测到 ${CLIS.find((cli) => cli.key === pendingCliProxyEnablePrompt.cliKey)?.name ?? pendingCliProxyEnablePrompt.cliKey} 代理相关环境变量冲突`
            : "检测到环境变量冲突"
        }
        description="继续启用可能会被这些环境变量覆盖（不会显示变量值）。是否继续？"
      >
        {pendingCliProxyEnablePrompt ? (
          <div className="space-y-4">
            <ul className="space-y-2">
              {pendingCliProxyEnablePrompt.conflicts.map((row) => (
                <li
                  key={`${row.var_name}:${row.source_type}:${row.source_path}`}
                  className="rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2"
                >
                  <div className="font-mono text-xs text-slate-800 dark:text-slate-200">
                    {row.var_name}
                  </div>
                  <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                    {row.source_path}
                  </div>
                </li>
              ))}
            </ul>

            <div className="flex items-center justify-end gap-2">
              <Button
                variant="secondary"
                size="md"
                onClick={() => cliProxyState.setPendingCliProxyEnablePrompt(null)}
              >
                取消
              </Button>
              <Button
                variant="primary"
                size="md"
                onClick={cliProxyState.confirmPendingCliProxyEnable}
              >
                继续启用
              </Button>
            </div>
          </div>
        ) : null}
      </Dialog>

      <RequestLogDetailDialog
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
        selectedLog={selectedLog}
        selectedLogLoading={selectedLogLoading}
        attemptLogs={attemptLogs}
        attemptLogsLoading={attemptLogsLoading}
      />
    </div>
  );
}
