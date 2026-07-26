// Usage:
// - Used by `HomeRequestLogsPanel` to show the selected request log detail.
// - Keeps the dialog UI isolated from the main overview panel to reduce file size and improve cohesion.

import { useCallback, useState } from "react";
import { useNowMs } from "../../hooks/useNowMs";
import { useRequestLogDetailSignalRefresh } from "../../hooks/useRequestLogDetailSignalRefresh";
import { isPersistedRequestLogIncomplete } from "../../services/gateway/requestLogState";
import { useTraceStore } from "../../services/gateway/traceStore";
import {
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
} from "../../query/requestLogs";
import { Dialog } from "../../ui/Dialog";
import { TabList } from "../../ui/TabList";
import { resolveProviderLabel } from "../../pages/providers/baseUrl";
import { resolveRequestLogErrorObservation } from "./requestLogErrorDetails";
import {
  buildRequestLogAuditMeta,
  computeStatusBadge,
  resolveLiveTraceDurationMs,
  resolveLiveTraceProvider,
} from "./requestLogPresentation";
import { RequestLogDetailSummaryTab } from "./RequestLogDetailSummaryTab";
import { RequestLogDetailChainTab } from "./RequestLogDetailChainTab";
import { RequestLogDetailRawTab } from "./RequestLogDetailRawTab";
import { useContributionsForSlot } from "../../plugins/contributions/useActiveContributions";
import { HostRenderedContribution } from "../../plugins/contributions/HostRenderedContribution";
import type { ContributionCommandHandler } from "../../plugins/contributions/types";
import { logToConsole } from "../../services/consoleLog";

export type RequestLogDetailDialogProps = {
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

type DetailTab = "summary" | "chain" | "raw";

const DETAIL_TABS: Array<{ key: DetailTab; label: string }> = [
  { key: "summary", label: "概览" },
  { key: "chain", label: "决策链" },
  { key: "raw", label: "原始数据" },
];

function pluginDetailTabKey(pluginId: string, contributionId: string) {
  return `plugin:${pluginId}:${contributionId}`;
}

export function RequestLogDetailDialog({
  selectedLogId,
  onSelectLogId,
}: RequestLogDetailDialogProps) {
  const [activeTab, setActiveTab] = useState<string>("summary");
  const { contributions: logDetailTabContributions } = useContributionsForSlot("logs.detail.tabs");
  const { traces } = useTraceStore();
  const selectedLogQuery = useRequestLogDetailQuery(selectedLogId);
  const queriedSelectedLog = selectedLogQuery.data ?? null;
  const selectedLog =
    queriedSelectedLog != null && queriedSelectedLog.id === selectedLogId
      ? queriedSelectedLog
      : null;
  const selectedLogLoading = selectedLogQuery.isFetching;

  const attemptLogsQuery = useRequestAttemptLogsByTraceIdQuery(selectedLog?.trace_id ?? null, 50);
  const attemptLogs = attemptLogsQuery.data ?? [];
  const attemptLogsLoading = attemptLogsQuery.isFetching;
  const refreshSelectedLogDetail = useCallback(async () => {
    await Promise.allSettled([selectedLogQuery.refetch(), attemptLogsQuery.refetch()]);
  }, [attemptLogsQuery, selectedLogQuery]);
  useRequestLogDetailSignalRefresh({
    traceId: selectedLog?.trace_id ?? null,
    enabled: selectedLogId != null,
    refresh: refreshSelectedLogDetail,
  });

  // Trace store is the authority on whether the request is still alive.
  const matchingTrace = selectedLog
    ? (traces.find((trace) => trace.trace_id === selectedLog.trace_id) ?? null)
    : null;
  const isInProgress =
    selectedLog != null && isPersistedRequestLogIncomplete(selectedLog) && matchingTrace != null;
  const liveTrace = isInProgress ? matchingTrace : null;
  const nowMs = useNowMs(isInProgress && liveTrace != null, 250);
  const liveProvider = resolveLiveTraceProvider(liveTrace);
  const providerName = isInProgress
    ? (liveProvider?.providerName ?? selectedLog?.final_provider_name)
    : selectedLog?.final_provider_name;
  const providerId = isInProgress
    ? (liveProvider?.providerId ?? selectedLog?.final_provider_id)
    : selectedLog?.final_provider_id;
  const auditMeta = selectedLog ? buildRequestLogAuditMeta(selectedLog) : null;
  const finalProviderText =
    auditMeta?.providerFallbackText ?? resolveProviderLabel(providerName, providerId);
  const displayDurationMs =
    selectedLog == null
      ? 0
      : isInProgress
        ? (resolveLiveTraceDurationMs(liveTrace, nowMs) ?? selectedLog.duration_ms ?? 0)
        : (selectedLog.duration_ms ?? 0);

  const errorObservation = selectedLog ? resolveRequestLogErrorObservation(selectedLog) : null;

  const statusBadge = selectedLog
    ? computeStatusBadge({
        status: selectedLog.status,
        errorCode: selectedLog.error_code,
        inProgress: isInProgress,
        hasFailover: attemptLogs.length > 1,
      })
    : null;

  const hasTokens =
    selectedLog != null &&
    (selectedLog.input_tokens != null ||
      selectedLog.output_tokens != null ||
      selectedLog.total_tokens != null ||
      selectedLog.cache_read_input_tokens != null ||
      selectedLog.cache_creation_input_tokens != null ||
      selectedLog.cache_creation_5m_input_tokens != null ||
      selectedLog.cache_creation_1h_input_tokens != null ||
      selectedLog.cost_usd != null ||
      selectedLog.duration_ms != null ||
      selectedLog.ttfb_ms != null ||
      (isInProgress && liveTrace != null));

  const detailTabs = [
    ...DETAIL_TABS,
    ...logDetailTabContributions.map((contribution) => ({
      key: pluginDetailTabKey(contribution.pluginId, contribution.contributionId),
      label: contribution.title ?? contribution.contributionId,
    })),
  ];
  const activeLogDetailContribution =
    typeof activeTab === "string"
      ? (logDetailTabContributions.find(
          (contribution) =>
            pluginDetailTabKey(contribution.pluginId, contribution.contributionId) === activeTab
        ) ?? null)
      : null;
  const handleContributionCommand = useCallback<ContributionCommandHandler>(
    (command, context) => {
      if (!selectedLog) return;
      logToConsole("info", "插件日志详情命令", {
        command,
        traceId: selectedLog.trace_id,
        logId: selectedLog.id,
        pluginId: context.pluginId,
        contributionId: context.contributionId,
        cliKey: selectedLog.cli_key,
      });
    },
    [selectedLog]
  );

  return (
    <Dialog
      open={selectedLogId != null}
      onOpenChange={(open) => {
        if (!open) {
          onSelectLogId(null);
          setActiveTab("summary");
        }
      }}
      title="代理记录详情"
      className="max-w-3xl lg:max-w-5xl"
    >
      {selectedLogLoading ? (
        <div className="text-sm text-muted-foreground">加载中…</div>
      ) : !selectedLog ? (
        <div className="text-sm text-muted-foreground">
          未找到记录详情（可能已过期被留存策略清理）。
        </div>
      ) : (
        <div className="space-y-3">
          <TabList<string>
            ariaLabel="日志详情"
            items={detailTabs}
            value={activeTab}
            onChange={(next) => setActiveTab(next as DetailTab)}
          />

          {activeTab === "summary" && (
            <RequestLogDetailSummaryTab
              selectedLog={selectedLog}
              errorObservation={errorObservation}
              statusBadge={statusBadge}
              hasTokens={hasTokens}
              displayDurationMs={displayDurationMs}
              isInProgress={isInProgress}
              attemptCount={attemptLogs.length}
            />
          )}

          {activeTab === "chain" && (
            <RequestLogDetailChainTab
              selectedLog={selectedLog}
              attemptLogs={attemptLogs}
              attemptLogsLoading={attemptLogsLoading}
              isInProgress={isInProgress}
              finalProviderText={finalProviderText}
            />
          )}

          {activeTab === "raw" && <RequestLogDetailRawTab selectedLog={selectedLog} />}

          {activeLogDetailContribution ? (
            <HostRenderedContribution
              contribution={activeLogDetailContribution}
              onCommand={handleContributionCommand}
            />
          ) : null}
        </div>
      )}
    </Dialog>
  );
}
