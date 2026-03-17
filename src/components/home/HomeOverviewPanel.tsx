// Usage:
// - Used by `src/pages/HomePage.tsx` to render the "概览" tab content.
// - This module is intentionally kept thin: it composes smaller, cohesive sub-components.

import { useEffect, useMemo, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { useNowUnix } from "../../hooks/useNowUnix";
import type { OpenCircuitRow } from "../ProviderCircuitBadge";
import type { GatewayActiveSession } from "../../services/gateway";
import type { CliKey } from "../../services/providers";
import type { ProviderLimitUsageRow } from "../../services/providerLimitUsage";
import type { RequestLogSummary } from "../../services/requestLogs";
import type { SortModeSummary } from "../../services/sortModes";
import type { TraceSession } from "../../services/traceStore";
import type { UsageHourlyRow } from "../../services/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { EmptyState } from "../../ui/EmptyState";
import { TabList } from "../../ui/TabList";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds } from "../../utils/formatters";
import { HomeActiveSessionsCardContent } from "./HomeActiveSessionsCard";
import { CliBrandIcon } from "./CliBrandIcon";
import { HomeProviderLimitPanelContent } from "./HomeProviderLimitPanel";
import { HomeRequestLogsPanel } from "./HomeRequestLogsPanel";
import { HomeUsageSection } from "./HomeUsageSection";
import { HomeWorkStatusCard } from "./HomeWorkStatusCard";

type SessionsTabKey = "sessions" | "providerLimit" | "circuit";

const SESSIONS_TABS: Array<{ key: SessionsTabKey; label: string }> = [
  { key: "sessions", label: "活跃 Session" },
  { key: "circuit", label: "熔断信息" },
  { key: "providerLimit", label: "供应商限额" },
];

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

export type HomeOverviewPanelProps = {
  showCustomTooltip: boolean;
  circuitPreviewEnabled?: boolean;
  showHomeHeatmap: boolean;

  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
  onRefreshUsageHeatmap: () => void;

  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  onSetCliActiveMode: (cliKey: CliKey, modeId: number | null) => void;

  cliProxyEnabled: Record<CliKey, boolean>;
  cliProxyToggling: Record<CliKey, boolean>;
  onSetCliProxyEnabled: (cliKey: CliKey, enabled: boolean) => void;

  activeSessions: GatewayActiveSession[];
  activeSessionsLoading: boolean;
  activeSessionsAvailable: boolean | null;

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

export function HomeOverviewPanel({
  showCustomTooltip,
  circuitPreviewEnabled = import.meta.env.DEV,
  showHomeHeatmap,
  usageHeatmapRows,
  usageHeatmapLoading,
  onRefreshUsageHeatmap,
  sortModes,
  sortModesLoading,
  sortModesAvailable,
  activeModeByCli,
  activeModeToggling,
  cliProxyEnabled,
  cliProxyToggling,
  onSetCliProxyEnabled,
  activeSessions,
  activeSessionsLoading,
  activeSessionsAvailable,
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
  const [sessionsTab, setSessionsTab] = useState<SessionsTabKey>("sessions");
  const [previewCircuits, setPreviewCircuits] = useState<OpenCircuitRow[]>([]);
  const previousActiveSessionKeysRef = useRef<string[] | null>(null);
  const previousOpenCircuitKeysRef = useRef<string[] | null>(null);
  const displayedCircuits = openCircuits.length > 0 ? openCircuits : previewCircuits;
  const circuitPreviewActive = openCircuits.length === 0 && previewCircuits.length > 0;
  const circuitNowUnix = useNowUnix(sessionsTab === "circuit" && displayedCircuits.length > 0);

  const activeSessionKeys = useMemo(
    () =>
      activeSessions
        .map((row) => `${row.cli_key}:${row.session_id}`)
        .sort((a, b) => a.localeCompare(b)),
    [activeSessions]
  );

  const openCircuitKeys = useMemo(
    () =>
      openCircuits
        .map((row) => `${row.cli_key}:${row.provider_id}`)
        .sort((a, b) => a.localeCompare(b)),
    [openCircuits]
  );

  useEffect(() => {
    const previousActiveSessionKeys = previousActiveSessionKeysRef.current;
    const previousOpenCircuitKeys = previousOpenCircuitKeysRef.current;

    if (previousActiveSessionKeys == null || previousOpenCircuitKeys == null) {
      previousActiveSessionKeysRef.current = activeSessionKeys;
      previousOpenCircuitKeysRef.current = openCircuitKeys;
      return;
    }

    const hasNewOpenCircuit = openCircuitKeys.some((key) => !previousOpenCircuitKeys.includes(key));
    const hasNewActiveSession = activeSessionKeys.some(
      (key) => !previousActiveSessionKeys.includes(key)
    );

    previousActiveSessionKeysRef.current = activeSessionKeys;
    previousOpenCircuitKeysRef.current = openCircuitKeys;

    if (hasNewOpenCircuit) {
      setSessionsTab("circuit");
      return;
    }
    if (hasNewActiveSession) {
      setSessionsTab("sessions");
    }
  }, [activeSessionKeys, openCircuitKeys]);

  return (
    <div className="flex flex-col h-full gap-4">
      <div className="shrink-0">
        {showHomeHeatmap ? (
          <div className="space-y-4">
            <div className="flex">
              <HomeUsageSection
                showHeatmap={true}
                usageHeatmapRows={usageHeatmapRows}
                usageHeatmapLoading={usageHeatmapLoading}
                onRefreshUsageHeatmap={onRefreshUsageHeatmap}
              />
            </div>

            <div className="flex">
              <HomeWorkStatusCard
                layout="horizontal"
                sortModes={sortModes}
                sortModesLoading={sortModesLoading}
                sortModesAvailable={sortModesAvailable}
                activeModeByCli={activeModeByCli}
                activeModeToggling={activeModeToggling}
                cliProxyEnabled={cliProxyEnabled}
                cliProxyToggling={cliProxyToggling}
                onSetCliProxyEnabled={onSetCliProxyEnabled}
              />
            </div>
          </div>
        ) : (
          <div className="grid gap-4 lg:grid-cols-12 lg:items-stretch">
            <div className="flex lg:col-span-4">
              <HomeWorkStatusCard
                layout="vertical"
                sortModes={sortModes}
                sortModesLoading={sortModesLoading}
                sortModesAvailable={sortModesAvailable}
                activeModeByCli={activeModeByCli}
                activeModeToggling={activeModeToggling}
                cliProxyEnabled={cliProxyEnabled}
                cliProxyToggling={cliProxyToggling}
                onSetCliProxyEnabled={onSetCliProxyEnabled}
              />
            </div>

            <div className="flex lg:col-span-8">
              <HomeUsageSection
                showHeatmap={false}
                usageHeatmapRows={usageHeatmapRows}
                usageHeatmapLoading={usageHeatmapLoading}
                onRefreshUsageHeatmap={onRefreshUsageHeatmap}
              />
            </div>
          </div>
        )}
      </div>

      <div className="grid gap-4 lg:grid-cols-12 flex-1 min-h-0">
        <div className="flex min-h-0 lg:col-span-5">
          <Card padding="sm" className="flex h-full min-h-0 flex-1 flex-col">
            <div className="flex items-center justify-between gap-2 shrink-0">
              <TabList
                ariaLabel="概览状态切换"
                items={SESSIONS_TABS}
                value={sessionsTab}
                onChange={setSessionsTab}
                size="sm"
              />
              <div className="flex items-center gap-2 text-xs text-slate-400">
                {sessionsTab === "providerLimit" && (
                  <button
                    type="button"
                    onClick={onRefreshProviderLimit}
                    disabled={providerLimitRefreshing}
                    className={cn(
                      "flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-all",
                      providerLimitRefreshing
                        ? "bg-slate-100 dark:bg-slate-700 text-slate-400 cursor-not-allowed"
                        : "bg-indigo-50 text-indigo-600 hover:bg-indigo-100"
                    )}
                  >
                    <RefreshCw
                      className={cn("h-3 w-3", providerLimitRefreshing && "animate-spin")}
                    />
                    刷新
                  </button>
                )}
                {sessionsTab === "circuit" && circuitPreviewActive && (
                  <button
                    type="button"
                    onClick={() => setPreviewCircuits([])}
                    className="flex items-center gap-1 rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-600 transition hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300 dark:hover:bg-slate-600"
                  >
                    关闭预览
                  </button>
                )}
              </div>
            </div>

            <div className="flex-1 min-h-0 mt-3">
              {sessionsTab === "sessions" ? (
                <HomeActiveSessionsCardContent
                  activeSessions={activeSessions}
                  activeSessionsLoading={activeSessionsLoading}
                  activeSessionsAvailable={activeSessionsAvailable}
                />
              ) : sessionsTab === "providerLimit" ? (
                <HomeProviderLimitPanelContent
                  rows={providerLimitRows}
                  loading={providerLimitLoading}
                  available={providerLimitAvailable}
                />
              ) : displayedCircuits.length === 0 ? (
                <EmptyState
                  title="当前没有熔断中的 Provider"
                  action={
                    circuitPreviewEnabled ? (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => setPreviewCircuits(PREVIEW_CIRCUITS)}
                      >
                        预览熔断样式
                      </Button>
                    ) : undefined
                  }
                />
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
                            disabled={isResetting}
                            onClick={() => {
                              if (circuitPreviewActive) {
                                setPreviewCircuits((prev) =>
                                  prev.filter((item) => item.provider_id !== row.provider_id)
                                );
                                return;
                              }
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
