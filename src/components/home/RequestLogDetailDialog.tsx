// Usage:
// - Used by `HomeRequestLogsPanel` to show the selected request log detail.
// - Keeps the dialog UI isolated from the main overview panel to reduce file size and improve cohesion.

import type { ReactNode } from "react";
import { useNowMs } from "../../hooks/useNowMs";
import { useTraceStore } from "../../services/traceStore";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import {
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
} from "../../query/requestLogs";
import type { RequestLogDetail } from "../../services/requestLogs";
import { Card } from "../../ui/Card";
import { Dialog } from "../../ui/Dialog";
import { cn } from "../../utils/cn";
import { getGatewayErrorShortLabel } from "../../constants/gatewayErrorCodes";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatTokensPerSecond,
  formatUsd,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { ProviderChainView } from "../ProviderChainView";
import { resolveProviderLabel } from "../../pages/providers/baseUrl";
import { resolveRequestLogErrorObservation } from "./requestLogErrorDetails";
import {
  buildRequestLogAuditMeta,
  computeStatusBadge,
  isPersistedRequestLogInProgress,
  resolveLiveTraceDurationMs,
  resolveLiveTraceProvider,
} from "./HomeLogShared";

export type RequestLogDetailDialogProps = {
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

export function RequestLogDetailDialog({
  selectedLogId,
  onSelectLogId,
}: RequestLogDetailDialogProps) {
  const { traces } = useTraceStore();
  const selectedLogQuery = useRequestLogDetailQuery(selectedLogId);
  const selectedLog = selectedLogQuery.data ?? null;
  const selectedLogLoading = selectedLogQuery.isFetching;

  const attemptLogsQuery = useRequestAttemptLogsByTraceIdQuery(selectedLog?.trace_id ?? null, 50);
  const attemptLogs = attemptLogsQuery.data ?? [];
  const attemptLogsLoading = attemptLogsQuery.isFetching;

  const isInProgress = selectedLog ? isPersistedRequestLogInProgress(selectedLog) : false;
  const liveTrace =
    selectedLog && isInProgress
      ? (traces.find((trace) => trace.trace_id === selectedLog.trace_id) ?? null)
      : null;
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
  const auditMeta = selectedLog ? buildRequestLogAuditMeta(selectedLog) : null;

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

  const errorObservation = selectedLog ? resolveRequestLogErrorObservation(selectedLog) : null;
  const showGatewayErrorCodeRow =
    errorObservation?.gatewayErrorCode != null &&
    errorObservation.displayErrorCode != null &&
    errorObservation.gatewayErrorCode !== errorObservation.displayErrorCode;
  const observationProviderLabel = errorObservation
    ? resolveProviderLabel(errorObservation.providerName, errorObservation.providerId)
    : null;
  const attemptCursor = errorObservation
    ? formatAttemptCursor(errorObservation.providerIndex, errorObservation.retryIndex)
    : null;
  const circuitLabel = errorObservation
    ? formatCircuitLabel(errorObservation.circuitStateBefore, errorObservation.circuitStateAfter)
    : null;
  const circuitCounter = errorObservation
    ? formatCircuitCounter(
        errorObservation.circuitFailureCount,
        errorObservation.circuitFailureThreshold
      )
    : null;

  return (
    <Dialog
      open={selectedLogId != null}
      onOpenChange={(open) => {
        if (!open) onSelectLogId(null);
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
          {auditMeta && auditMeta.tags.length > 0 ? (
            <Card padding="sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  审计语义
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  {auditMeta.tags.map((tag) => (
                    <span
                      key={tag.label}
                      className={cn("rounded-full px-2.5 py-1 text-xs font-medium", tag.className)}
                      title={tag.title}
                    >
                      {tag.label}
                    </span>
                  ))}
                </div>
              </div>
              {auditMeta.summary ? (
                <div className="mt-3 text-sm text-slate-600 dark:text-slate-300">
                  {auditMeta.summary}
                </div>
              ) : null}
            </Card>
          ) : null}

          {hasTokens ? (
            <Card padding="sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  关键指标
                </div>
                {statusBadge ? (
                  <span
                    className={cn("rounded-full px-2.5 py-1 text-xs font-medium", statusBadge.tone)}
                    title={statusBadge.title}
                  >
                    {statusBadge.text}
                  </span>
                ) : null}
              </div>

              <div className="mt-3 grid gap-2 grid-cols-2 sm:grid-cols-3 lg:grid-cols-4">
                <MetricCard label="输入 Token" value={selectedLog.input_tokens} />
                <MetricCard label="输出 Token" value={selectedLog.output_tokens} />
                <MetricCard label="缓存创建" value={resolveCacheWriteValue(selectedLog)} />
                <MetricCard label="缓存读取" value={selectedLog.cache_read_input_tokens} />
                <MetricCard label="总耗时" value={formatDurationMs(displayDurationMs)} />
                <MetricCard
                  label="TTFB"
                  value={(() => {
                    const ttfbMs = sanitizeTtfbMs(selectedLog.ttfb_ms, displayDurationMs);
                    return ttfbMs != null ? formatDurationMs(ttfbMs) : "—";
                  })()}
                />
                <MetricCard
                  label="速率"
                  value={(() => {
                    const rate = computeOutputTokensPerSecond(
                      selectedLog.output_tokens,
                      displayDurationMs,
                      sanitizeTtfbMs(selectedLog.ttfb_ms, displayDurationMs)
                    );
                    return rate != null ? formatTokensPerSecond(rate) : "—";
                  })()}
                />
                <MetricCard label="花费" value={formatUsd(selectedLog.cost_usd)} />
              </div>
            </Card>
          ) : null}

          {errorObservation && selectedLog.status != null && selectedLog.status >= 400 ? (
            <Card padding="sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  错误详情
                </div>
                {errorObservation.upstreamStatus != null ? (
                  <span className="rounded-full bg-rose-50 px-2.5 py-1 text-xs font-medium text-rose-600 ring-1 ring-inset ring-rose-500/10 dark:bg-rose-500/15 dark:text-rose-400 dark:ring-rose-400/20">
                    上游 {errorObservation.upstreamStatus}
                  </span>
                ) : null}
              </div>

              <div className="mt-3 rounded-lg border border-rose-200/60 bg-rose-50/50 p-3 dark:border-rose-500/20 dark:bg-rose-950/20">
                <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
                  {errorObservation.displayErrorCode ? (
                    <ObservationField
                      label={showGatewayErrorCodeRow ? "尝试错误码" : "错误码"}
                      value={
                        <div className="flex flex-wrap items-center gap-2">
                          <code className="rounded bg-rose-100 px-1.5 py-0.5 font-mono text-rose-700 dark:bg-rose-900/40 dark:text-rose-300">
                            {errorObservation.displayErrorCode}
                          </code>
                          <span className="text-slate-500 dark:text-slate-400">
                            {getGatewayErrorShortLabel(errorObservation.displayErrorCode)}
                          </span>
                        </div>
                      }
                    />
                  ) : null}
                  {showGatewayErrorCodeRow ? (
                    <ObservationField
                      label="网关错误码"
                      value={
                        <div className="flex flex-wrap items-center gap-2">
                          <code className="rounded bg-rose-100 px-1.5 py-0.5 font-mono text-rose-700 dark:bg-rose-900/40 dark:text-rose-300">
                            {errorObservation.gatewayErrorCode}
                          </code>
                          <span className="text-slate-500 dark:text-slate-400">
                            {getGatewayErrorShortLabel(errorObservation.gatewayErrorCode ?? "")}
                          </span>
                        </div>
                      }
                    />
                  ) : null}
                  {errorObservation.errorCategory ? (
                    <ObservationField label="错误分类" value={errorObservation.errorCategory} />
                  ) : null}
                  {errorObservation.outcome ? (
                    <ObservationField label="尝试结果" value={errorObservation.outcome} />
                  ) : null}
                  {observationProviderLabel ? (
                    <ObservationField label="命中供应商" value={observationProviderLabel} />
                  ) : null}
                  {errorObservation.decision ? (
                    <ObservationField
                      label="调度决策"
                      value={formatDecisionLabel(errorObservation.decision)}
                    />
                  ) : null}
                  {errorObservation.selectionMethod ? (
                    <ObservationField
                      label="选择方式"
                      value={formatSelectionMethodLabel(errorObservation.selectionMethod)}
                    />
                  ) : null}
                  {attemptCursor ? (
                    <ObservationField label="尝试位置" value={attemptCursor} />
                  ) : null}
                  {errorObservation.reasonCode ? (
                    <ObservationField label="原因标签" value={errorObservation.reasonCode} />
                  ) : null}
                  {errorObservation.matchedRule ? (
                    <ObservationField label="匹配规则" value={errorObservation.matchedRule} />
                  ) : null}
                  {errorObservation.attemptDurationMs != null ? (
                    <ObservationField
                      label="本次耗时"
                      value={formatDurationMs(errorObservation.attemptDurationMs)}
                    />
                  ) : null}
                  {circuitLabel ? <ObservationField label="熔断状态" value={circuitLabel} /> : null}
                  {circuitCounter ? (
                    <ObservationField label="失败计数" value={circuitCounter} />
                  ) : null}
                </div>

                {errorObservation.reason ? (
                  <ObservationBlock label="原因" className="mt-3">
                    {errorObservation.reason}
                  </ObservationBlock>
                ) : null}

                {errorObservation.upstreamBodyPreview ? (
                  <ObservationBlock label="上游返回" className="mt-3">
                    {errorObservation.upstreamBodyPreview}
                  </ObservationBlock>
                ) : null}

                {errorObservation.rawDetailsText ? (
                  <ObservationBlock label="原始详情" className="mt-3">
                    {errorObservation.rawDetailsText}
                  </ObservationBlock>
                ) : null}

                {errorObservation.gwDescription ? (
                  <div className="mt-3 space-y-1 border-t border-rose-200/40 pt-2 dark:border-rose-500/10">
                    <p className="text-xs text-slate-600 dark:text-slate-400">
                      {errorObservation.gwDescription.desc}
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      建议：{errorObservation.gwDescription.suggestion}
                    </p>
                  </div>
                ) : null}
              </div>
            </Card>
          ) : null}

          <Card padding="sm">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">决策链</div>
              <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                <span
                  className={cn(
                    "rounded-full px-2 py-0.5 font-medium",
                    cliBadgeTone(selectedLog.cli_key)
                  )}
                >
                  {cliShortLabel(selectedLog.cli_key)}
                </span>
                <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                  {isInProgress ? "当前供应商" : "最终供应商"}：{finalProviderText || "未知"}
                </span>
              </div>
            </div>
            <ProviderChainView
              attemptLogs={attemptLogs}
              attemptLogsLoading={attemptLogsLoading}
              attemptsJson={selectedLog.attempts_json}
            />
          </Card>
        </div>
      )}
    </Dialog>
  );
}

function ObservationField({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="rounded-lg border border-rose-200/60 bg-white/70 px-3 py-2 dark:border-rose-500/10 dark:bg-slate-900/20">
      <div className="text-[11px] font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
        {label}
      </div>
      <div className="mt-1 text-sm text-slate-800 dark:text-slate-100 break-all">{value}</div>
    </div>
  );
}

function ObservationBlock({
  label,
  className,
  children,
}: {
  label: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={className}>
      <span className="text-xs font-medium text-slate-600 dark:text-slate-400">{label}</span>
      <pre className="mt-1 whitespace-pre-wrap break-all rounded-lg border border-rose-200/60 bg-white/70 px-3 py-2 font-mono text-xs leading-relaxed text-rose-800 dark:border-rose-500/10 dark:bg-slate-900/20 dark:text-rose-200">
        {children}
      </pre>
    </div>
  );
}

function MetricCard({
  label,
  value,
}: {
  label: string;
  value: string | number | null | undefined;
}) {
  return (
    <div className="rounded-xl border border-slate-200/80 bg-slate-50/80 px-3 py-3 dark:border-slate-700 dark:bg-slate-800/70">
      <div className="text-xs text-slate-500 dark:text-slate-400">{label}</div>
      <div className="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">
        {value == null || value === "" ? "—" : value}
      </div>
    </div>
  );
}

function formatDecisionLabel(decision: string) {
  switch (decision) {
    case "retry":
      return "同供应商重试";
    case "switch":
      return "切换供应商";
    case "abort":
      return "直接终止";
    default:
      return decision;
  }
}

function formatSelectionMethodLabel(selectionMethod: string) {
  switch (selectionMethod) {
    case "session_reuse":
      return "会话复用";
    case "ordered":
      return "顺序选择";
    case "filtered":
      return "过滤后选择";
    default:
      return selectionMethod;
  }
}

function formatAttemptCursor(providerIndex: number | null, retryIndex: number | null) {
  const parts = [
    providerIndex != null ? `供应商 ${providerIndex}` : null,
    retryIndex != null ? `重试 ${retryIndex}` : null,
  ].filter((value): value is string => value != null);
  return parts.length > 0 ? parts.join(" / ") : null;
}

function formatCircuitLabel(before: string | null, after: string | null) {
  if (before && after && before !== after) {
    return `${before} → ${after}`;
  }
  return after ?? before ?? null;
}

function formatCircuitCounter(current: number | null, threshold: number | null) {
  if (current == null && threshold == null) return null;
  if (current != null && threshold != null) return `${current} / ${threshold}`;
  return current != null ? `${current}` : `${threshold}`;
}

function resolveCacheWriteValue(selectedLog: RequestLogDetail) {
  if (
    selectedLog.cache_creation_5m_input_tokens != null &&
    selectedLog.cache_creation_5m_input_tokens > 0
  ) {
    return `${selectedLog.cache_creation_5m_input_tokens} (5m)`;
  }
  if (
    selectedLog.cache_creation_1h_input_tokens != null &&
    selectedLog.cache_creation_1h_input_tokens > 0
  ) {
    return `${selectedLog.cache_creation_1h_input_tokens} (1h)`;
  }
  if (selectedLog.cache_creation_input_tokens != null) {
    return selectedLog.cache_creation_input_tokens;
  }
  if (selectedLog.cache_creation_5m_input_tokens != null) {
    return `${selectedLog.cache_creation_5m_input_tokens} (5m)`;
  }
  if (selectedLog.cache_creation_1h_input_tokens != null) {
    return `${selectedLog.cache_creation_1h_input_tokens} (1h)`;
  }
  return "—";
}
