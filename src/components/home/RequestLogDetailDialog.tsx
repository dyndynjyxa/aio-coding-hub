// Usage:
// - Used by `HomeRequestLogsPanel` to show the selected request log detail.
// - Keeps the dialog UI isolated from the main overview panel to reduce file size and improve cohesion.

import { useState } from "react";
import { useNowMs } from "../../hooks/useNowMs";
import { isPersistedRequestLogInProgress } from "../../services/gateway/requestLogState";
import { useTraceStore } from "../../services/gateway/traceStore";
import {
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
} from "../../query/requestLogs";
import { Dialog } from "../../ui/Dialog";
import { TabList } from "../../ui/TabList";
import { resolveProviderLabel } from "../../pages/providers/baseUrl";
import { resolveRequestLogErrorObservation } from "./requestLogErrorDetails";
import { computeStatusBadge, resolveLiveTraceDurationMs, resolveLiveTraceProvider } from "./HomeLogShared";
import { RequestLogDetailSummaryTab } from "./RequestLogDetailSummaryTab";
import { RequestLogDetailChainTab } from "./RequestLogDetailChainTab";
import { RequestLogDetailRawTab } from "./RequestLogDetailRawTab";

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

export function RequestLogDetailDialog({
  selectedLogId,
  onSelectLogId,
}: RequestLogDetailDialogProps) {
  const [activeTab, setActiveTab] = useState<DetailTab>("summary");
  const { traces } = useTraceStore();
  const selectedLogQuery = useRequestLogDetailQuery(selectedLogId);
  const selectedLog = selectedLogQuery.data ?? null;
  const selectedLogLoading = selectedLogQuery.isFetching;

  const attemptLogsQuery = useRequestAttemptLogsByTraceIdQuery(selectedLog?.trace_id ?? null, 50);
  const attemptLogs = attemptLogsQuery.data ?? [];
  const attemptLogsLoading = attemptLogsQuery.isFetching;

  // Trace store is the authority on whether the request is still alive.
  const matchingTrace = selectedLog
    ? (traces.find((trace) => trace.trace_id === selectedLog.trace_id) ?? null)
    : null;
  const isInProgress =
    selectedLog != null && isPersistedRequestLogInProgress(selectedLog) && matchingTrace != null;
  const liveTrace = isInProgress ? matchingTrace : null;
  const nowMs = useNowMs(isInProgress && liveTrace != null, 250);
  const liveProvider = resolveLiveTraceProvider(liveTrace);
  const providerName = isInProgress
    ? (liveProvider?.providerName ?? selectedLog?.final_provider_name)
    : selectedLog?.final_provider_name;
  const providerId = isInProgress
    ? (liveProvider?.providerId ?? selectedLog?.final_provider_id)
    : selectedLog?.final_provider_id;
  const finalProviderText = resolveProviderLabel(providerName, providerId);
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
        <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
      ) : !selectedLog ? (
        <div className="text-sm text-slate-600 dark:text-slate-400">
          未找到记录详情（可能已过期被留存策略清理）。
        </div>
      ) : (
        <div className="space-y-3">
          <TabList<DetailTab>
            ariaLabel="日志详情"
            items={DETAIL_TABS}
            value={activeTab}
            onChange={setActiveTab}
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

          {activeTab === "raw" && (
            <RequestLogDetailRawTab selectedLog={selectedLog} />
          )}
        </div>
      )}
    </Dialog>
  );
}
