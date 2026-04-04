// Usage:
// - Used by `src/pages/HomePage.tsx` to render the "概览" tab content.
// - This module is intentionally kept thin: it composes smaller, cohesive sub-components.

import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { useNowUnix } from "../../hooks/useNowUnix";
import type { OpenCircuitRow } from "../ProviderCircuitBadge";
import type { GatewayActiveSession } from "../../services/gateway";
import {
  HOME_OVERVIEW_TABS,
  readHomeOverviewTabOrderFromStorage,
  type HomeOverviewTabKey,
} from "../../services/homeOverviewTabOrder";
import { getOrderedClis } from "../../services/cliPriorityOrder";
import type { CliKey } from "../../services/providers";
import type { ProviderLimitUsageRow } from "../../services/providerLimitUsage";
import type { RequestLogSummary } from "../../services/requestLogs";
import type { SortModeSummary } from "../../services/sortModes";
import type { TraceSession } from "../../services/traceStore";
import type { UsageHourlyRow } from "../../services/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { TabList } from "../../ui/TabList";
import { formatCountdownSeconds } from "../../utils/formatters";
import { CliBrandIcon } from "./CliBrandIcon";
import { HomeRequestLogsPanel } from "./HomeRequestLogsPanel";
import { HomeUsageSection } from "./HomeUsageSection";
import { HomeWorkStatusCard } from "./HomeWorkStatusCard";
import type { HomeCliWorkspaceConfig } from "./homeWorkspaceConfigTypes";

const LazyHomeActiveSessionsCardContent = lazy(() =>
  import("./HomeActiveSessionsCard").then((m) => ({ default: m.HomeActiveSessionsCardContent }))
);

const LazyHomeProviderLimitPanelContent = lazy(() =>
  import("./HomeProviderLimitPanel").then((m) => ({ default: m.HomeProviderLimitPanelContent }))
);

const LazyHomeWorkspaceConfigPanel = lazy(() =>
  import("./HomeWorkspaceConfigPanel").then((m) => ({ default: m.HomeWorkspaceConfigPanel }))
);

const PREVIEW_CIRCUITS: OpenCircuitRow[] = [
  {
    cli_key: "claude",
    provider_id: 10001,
    provider_name: "Claude Main",
    open_until: Math.floor(Date.now() / 1000) + 12 * 60,
  },
  {
    cli_key: "codex",
    provider_id: 10002,
    provider_name: "Codex Fallback",
    open_until: Math.floor(Date.now() / 1000) + 5 * 60,
  },
  {
    cli_key: "gemini",
    provider_id: 10003,
    provider_name: "Gemini Mirror",
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

function didKeysChange(current: string[], previous: string[]) {
  return (
    current.length !== previous.length || current.some((key, index) => key !== previous[index])
  );
}

function OverviewPanelFallback() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-slate-600 dark:text-slate-400">
      <div className="flex items-center gap-3">
        <Spinner />
        <span>加载面板中…</span>
      </div>
    </div>
  );
}
export type HomeOverviewPanelProps = {
  showCustomTooltip: boolean;
  devPreviewEnabled?: boolean;
  showHomeHeatmap: boolean;
  showHomeUsage?: boolean;
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

  cliProxyLoading: boolean;
  cliProxyAvailable: boolean | null;
  cliProxyEnabled: Record<CliKey, boolean>;
  cliProxyAppliedToCurrentGateway: Record<CliKey, boolean | null>;
  cliProxyToggling: Record<CliKey, boolean>;
  onSetCliProxyEnabled: (cliKey: CliKey, enabled: boolean) => void;

  activeSessions: GatewayActiveSession[];
  activeSessionsLoading: boolean;
  activeSessionsAvailable: boolean | null;

  workspaceConfigs: HomeCliWorkspaceConfig[];

  providerLimitRows: ProviderLimitUsageRow[];
  providerLimitLoading: boolean;
  providerLimitAvailable: boolean | null;
  providerLimitRefreshing: boolean;
  onRefreshProviderLimit: () => void;

  openCircuits: OpenCircuitRow[];
  onResetCircuitProvider: (providerId: number) => void;
  resettingCircuitProviderIds: Set<number>;

  traces: TraceSession[];

  requestLogs: RequestLogSummary[];
  requestLogsLoading: boolean;
  requestLogsRefreshing: boolean;
  requestLogsAvailable: boolean | null;
  onRefreshRequestLogs: () => void;

  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

const PREVIEW_WORKSPACE_CONFIGS: HomeCliWorkspaceConfig[] = [
  {
    cliKey: "claude",
    cliLabel: "Claude Code",
    workspaceId: 1001,
    workspaceName: "工作区 Alpha",
    loading: false,
    items: [
      { id: "prompt:1001", type: "prompts", label: "Prompt", name: "PR Review" },
      { id: "mcp:1001", type: "mcp", label: "MCP", name: "filesystem" },
      { id: "mcp:1002", type: "mcp", label: "MCP", name: "browser-tools" },
      { id: "skill:1001", type: "skills", label: "Skill", name: "repo-auditor" },
      { id: "skill:1002", type: "skills", label: "Skill", name: "incident-helper" },
    ],
  },
  {
    cliKey: "codex",
    cliLabel: "Codex",
    workspaceId: 1002,
    workspaceName: "Default",
    loading: false,
    items: [
      { id: "prompt:1002", type: "prompts", label: "Prompt", name: "Fix First" },
      { id: "mcp:1003", type: "mcp", label: "MCP", name: "filesystem" },
      { id: "mcp:1004", type: "mcp", label: "MCP", name: "github" },
      { id: "skill:1002", type: "skills", label: "Skill", name: "code-review" },
      { id: "skill:1003", type: "skills", label: "Skill", name: "test-writer" },
    ],
  },
  {
    cliKey: "gemini",
    cliLabel: "Gemini",
    workspaceId: 1003,
    workspaceName: "工作区 Beta",
    loading: false,
    items: [
      { id: "mcp:1005", type: "mcp", label: "MCP", name: "browser-tools" },
      { id: "mcp:1006", type: "mcp", label: "MCP", name: "figma" },
      { id: "skill:1004", type: "skills", label: "Skill", name: "ux-auditor" },
      { id: "skill:1005", type: "skills", label: "Skill", name: "spec-writer" },
    ],
  },
];

export function HomeOverviewPanel({
  showCustomTooltip,
  devPreviewEnabled = false,
  showHomeHeatmap,
  showHomeUsage = true,
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
  cliProxyLoading,
  cliProxyAvailable,
  cliProxyEnabled,
  cliProxyAppliedToCurrentGateway,
  cliProxyToggling,
  onSetCliProxyEnabled,
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
  workspaceConfigs,
  providerLimitRows,
  providerLimitLoading,
  providerLimitAvailable,
  providerLimitRefreshing,
  onRefreshProviderLimit,
  openCircuits,
  onResetCircuitProvider,
  resettingCircuitProviderIds,
  traces,
  requestLogs,
  requestLogsLoading,
  requestLogsRefreshing,
  requestLogsAvailable,
  onRefreshRequestLogs,
  selectedLogId,
  onSelectLogId,
}: HomeOverviewPanelProps) {
  const [sessionsTabsOrder] = useState<HomeOverviewTabKey[]>(() =>
    readHomeOverviewTabOrderFromStorage()
  );
  const [sessionsTab, setSessionsTab] = useState<HomeOverviewTabKey>(
    () => sessionsTabsOrder[0] ?? "workspaceConfig"
  );
  const [selectedWorkspaceConfigCliKey, setSelectedWorkspaceConfigCliKey] = useState<CliKey | null>(
    null
  );
  const previousOpenCircuitKeysRef = useRef<string[] | null>(null);
  const circuitPreviewActive = openCircuits.length === 0 && devPreviewEnabled;
  const displayedCircuits = circuitPreviewActive ? PREVIEW_CIRCUITS : openCircuits;
  const displayedActiveSessions =
    devPreviewEnabled && activeSessions.length === 0 ? PREVIEW_ACTIVE_SESSIONS : activeSessions;
  const displayedProviderLimitRows =
    devPreviewEnabled && providerLimitRows.length === 0
      ? PREVIEW_PROVIDER_LIMIT_ROWS
      : providerLimitRows;
  const displayedWorkspaceConfigs = useMemo(() => {
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
  const circuitNowUnix = useNowUnix(sessionsTab === "circuit" && displayedCircuits.length > 0);
  const showUsageRow = showHomeHeatmap || showHomeUsage;
  const sessionsTabs = useMemo(() => {
    const labelByKey = new Map(HOME_OVERVIEW_TABS.map((item) => [item.key, item.label]));
    return sessionsTabsOrder.map((key) => ({ key, label: labelByKey.get(key) ?? key }));
  }, [sessionsTabsOrder]);

  const openCircuitKeys = useMemo(
    () =>
      openCircuits
        .map((row) => `${row.cli_key}:${row.provider_id}`)
        .sort((a, b) => a.localeCompare(b)),
    [openCircuits]
  );

  useEffect(() => {
    if (displayedWorkspaceConfigs.some((config) => config.cliKey === selectedWorkspaceConfigCliKey))
      return;
    const fallbackCliKey = displayedWorkspaceConfigs[0]?.cliKey;
    if (fallbackCliKey) setSelectedWorkspaceConfigCliKey(fallbackCliKey);
  }, [displayedWorkspaceConfigs, selectedWorkspaceConfigCliKey]);
  const effectiveSelectedWorkspaceConfigCliKey =
    selectedWorkspaceConfigCliKey ?? displayedWorkspaceConfigs[0]?.cliKey ?? "claude";

  useEffect(() => {
    const previousOpenCircuitKeys = previousOpenCircuitKeysRef.current;

    if (previousOpenCircuitKeys == null) {
      previousOpenCircuitKeysRef.current = openCircuitKeys;
      return;
    }

    const openCircuitChanged = didKeysChange(openCircuitKeys, previousOpenCircuitKeys);

    previousOpenCircuitKeysRef.current = openCircuitKeys;

    if (openCircuitChanged) {
      setSessionsTab("circuit");
    }
  }, [openCircuitKeys]);

  return (
    <div className="flex flex-col h-full gap-4">
      <div className="shrink-0">
        {showHomeHeatmap && showHomeUsage ? (
          <div className="space-y-4">
            <div className="flex">
              <HomeUsageSection
                devPreviewEnabled={devPreviewEnabled}
                showHeatmap={true}
                showUsageChart={true}
                usageWindowDays={usageWindowDays}
                usageHeatmapRows={usageHeatmapRows}
                usageHeatmapLoading={usageHeatmapLoading}
                onRefreshUsageHeatmap={onRefreshUsageHeatmap}
              />
            </div>

            <div className="flex">
              <HomeWorkStatusCard
                layout="horizontal"
                cliProxyLoading={cliProxyLoading}
                cliProxyAvailable={cliProxyAvailable}
                cliProxyEnabled={cliProxyEnabled}
                cliProxyAppliedToCurrentGateway={cliProxyAppliedToCurrentGateway}
                cliProxyToggling={cliProxyToggling}
                onSetCliProxyEnabled={onSetCliProxyEnabled}
              />
            </div>
          </div>
        ) : showUsageRow ? (
          <div className="grid gap-4 lg:grid-cols-12 lg:items-stretch">
            <div className="flex lg:col-span-4">
              <HomeWorkStatusCard
                layout="vertical"
                cliProxyLoading={cliProxyLoading}
                cliProxyAvailable={cliProxyAvailable}
                cliProxyEnabled={cliProxyEnabled}
                cliProxyAppliedToCurrentGateway={cliProxyAppliedToCurrentGateway}
                cliProxyToggling={cliProxyToggling}
                onSetCliProxyEnabled={onSetCliProxyEnabled}
              />
            </div>

            <div className="flex lg:col-span-8">
              <HomeUsageSection
                devPreviewEnabled={devPreviewEnabled}
                showHeatmap={showHomeHeatmap}
                showUsageChart={showHomeUsage}
                usageWindowDays={usageWindowDays}
                usageHeatmapRows={usageHeatmapRows}
                usageHeatmapLoading={usageHeatmapLoading}
                onRefreshUsageHeatmap={onRefreshUsageHeatmap}
              />
            </div>
          </div>
        ) : (
          <div className="flex">
            <HomeWorkStatusCard
              layout="horizontal"
              cliProxyLoading={cliProxyLoading}
              cliProxyAvailable={cliProxyAvailable}
              cliProxyEnabled={cliProxyEnabled}
              cliProxyAppliedToCurrentGateway={cliProxyAppliedToCurrentGateway}
              cliProxyToggling={cliProxyToggling}
              onSetCliProxyEnabled={onSetCliProxyEnabled}
            />
          </div>
        )}
      </div>

      <div className="grid gap-4 lg:grid-cols-12 flex-1 min-h-0">
        <div className="flex min-h-0 lg:col-span-5">
          <Card padding="sm" className="flex h-full min-h-0 flex-1 flex-col">
            <div className="flex items-center justify-between gap-2 shrink-0">
              <TabList
                ariaLabel="概览状态切换"
                items={sessionsTabs}
                value={sessionsTab}
                onChange={setSessionsTab}
                size="sm"
                className="w-full overflow-x-auto"
                buttonClassName="whitespace-nowrap flex-1"
              />
            </div>

            <div className="flex-1 min-h-0 mt-3">
              {sessionsTab === "sessions" ? (
                <Suspense fallback={<OverviewPanelFallback />}>
                  <LazyHomeActiveSessionsCardContent
                    activeSessions={displayedActiveSessions}
                    activeSessionsLoading={activeSessionsLoading}
                    activeSessionsAvailable={activeSessionsAvailable}
                  />
                </Suspense>
              ) : sessionsTab === "workspaceConfig" ? (
                <Suspense fallback={<OverviewPanelFallback />}>
                  <LazyHomeWorkspaceConfigPanel
                    configs={displayedWorkspaceConfigs}
                    selectedCliKey={effectiveSelectedWorkspaceConfigCliKey}
                    onSelectCliKey={setSelectedWorkspaceConfigCliKey}
                    sortModes={sortModes}
                    sortModesLoading={sortModesLoading}
                    sortModesAvailable={sortModesAvailable}
                    activeModeByCli={activeModeByCli}
                    activeModeToggling={activeModeToggling}
                    onSetCliActiveMode={onSetCliActiveMode}
                  />
                </Suspense>
              ) : sessionsTab === "providerLimit" ? (
                <Suspense fallback={<OverviewPanelFallback />}>
                  <LazyHomeProviderLimitPanelContent
                    rows={displayedProviderLimitRows}
                    loading={providerLimitLoading}
                    available={providerLimitAvailable}
                    onRefresh={onRefreshProviderLimit}
                    refreshing={providerLimitRefreshing}
                  />
                </Suspense>
              ) : displayedCircuits.length === 0 ? (
                <EmptyState title="当前没有熔断中的 Provider" />
              ) : (
                <div className="h-full overflow-y-auto pr-1">
                  <div className="space-y-3">
                    {displayedCircuits.map((row) => {
                      const remaining =
                        row.open_until != null && Number.isFinite(row.open_until)
                          ? formatCountdownSeconds(row.open_until - circuitNowUnix)
                          : "—";
                      const isResetting = resettingCircuitProviderIds.has(row.provider_id);

                      return (
                        <div
                          key={`${row.cli_key}:${row.provider_id}`}
                          className="flex items-center justify-between gap-3 rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-2 dark:border-slate-700 dark:bg-slate-800/50"
                        >
                          <div className="min-w-0 flex flex-1 items-center gap-2.5">
                            <CliBrandIcon
                              cliKey={row.cli_key as CliKey}
                              className="h-4 w-4 shrink-0 rounded-[4px] object-contain"
                            />
                            <div
                              className="truncate text-sm font-medium text-slate-700 dark:text-slate-300"
                              title={row.provider_name}
                            >
                              {row.provider_name || "未知"}
                            </div>
                          </div>
                          <div className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400">
                            {remaining}
                          </div>
                          <Button
                            variant="secondary"
                            size="sm"
                            disabled={isResetting || circuitPreviewActive}
                            onClick={() => {
                              if (circuitPreviewActive) return;
                              onResetCircuitProvider(row.provider_id);
                            }}
                          >
                            {isResetting ? "解除中..." : "解除熔断"}
                          </Button>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>

        <div className="lg:col-span-7 min-h-0">
          <HomeRequestLogsPanel
            showCustomTooltip={showCustomTooltip}
            devPreviewEnabled={devPreviewEnabled}
            showSummaryText={false}
            showOpenLogsPageButton={false}
            traces={traces}
            requestLogs={requestLogs}
            requestLogsLoading={requestLogsLoading}
            requestLogsRefreshing={requestLogsRefreshing}
            requestLogsAvailable={requestLogsAvailable}
            onRefreshRequestLogs={onRefreshRequestLogs}
            selectedLogId={selectedLogId}
            onSelectLogId={onSelectLogId}
          />
        </div>
      </div>
    </div>
  );
}
