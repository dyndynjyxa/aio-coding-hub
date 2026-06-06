import {
  parseAssociationAuditJson,
  type AssociationAuditResult,
  type AssociationAuditSignal,
  type RequestLogDetail,
} from "../../services/gateway/requestLogs";
import type { RequestLogErrorObservation } from "./requestLogErrorDetails";
import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatTokensPerSecond,
  formatUsd,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { RequestLogErrorObservationCard } from "./RequestLogErrorObservationCard";
import {
  buildRequestLogAuditMeta,
  computeStatusBadge,
  FastModeBadge,
  hasPriorityServiceTierSpecialSetting,
} from "./HomeLogShared";

export type RequestLogDetailSummaryTabProps = {
  selectedLog: RequestLogDetail;
  errorObservation: RequestLogErrorObservation | null;
  statusBadge: ReturnType<typeof computeStatusBadge> | null;
  hasTokens: boolean;
  displayDurationMs: number;
  isInProgress: boolean;
  attemptCount: number;
};

export function RequestLogDetailSummaryTab({
  selectedLog,
  errorObservation,
  statusBadge,
  hasTokens,
  displayDurationMs,
  isInProgress: _isInProgress,
  attemptCount: _attemptCount,
}: RequestLogDetailSummaryTabProps) {
  const auditMeta = buildRequestLogAuditMeta(selectedLog);
  const associationAudit = parseAssociationAuditJson(selectedLog.association_audit_json);
  const isPriorityServiceTier =
    selectedLog.cli_key === "codex" &&
    hasPriorityServiceTierSpecialSetting(selectedLog.special_settings_json);

  return (
    <div className="space-y-3">
      {/* Error observation card (request-level) */}
      <RequestLogErrorObservationCard observation={errorObservation} />

      {/* Audit meta */}
      {auditMeta && auditMeta.tags.length > 0 ? (
        <Card padding="sm">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">审计语义</div>
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

      {associationAudit.result || associationAudit.malformed ? (
        <AssociationAuditCard
          result={associationAudit.result}
          malformed={associationAudit.malformed}
        />
      ) : null}

      {/* Key metrics */}
      {hasTokens ? (
        <Card padding="sm">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">关键指标</div>
            <div className="flex flex-wrap items-center gap-2">
              {isPriorityServiceTier ? <FastModeBadge showCustomTooltip={false} /> : null}
              {statusBadge ? (
                <span
                  className={cn("rounded-full px-2.5 py-1 text-xs font-medium", statusBadge.tone)}
                  title={statusBadge.title}
                >
                  {statusBadge.text}
                </span>
              ) : null}
            </div>
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
            <MetricCard
              label="费用系数"
              value={formatCostMultiplier(selectedLog.cost_multiplier)}
            />
          </div>
        </Card>
      ) : null}
    </div>
  );
}

function AssociationAuditCard({
  result,
  malformed,
}: {
  result: AssociationAuditResult | null;
  malformed: boolean;
}) {
  const risk = result?.overall_risk ?? "unknown";
  const status = result?.status ?? "invalid_response";
  const signals = result?.signals ?? [];
  const scoreText =
    result?.association_score == null ? "—" : `${Math.round(result.association_score * 100)}%`;
  const providerText = [result?.provider_name, result?.model].filter(Boolean).join(" · ");

  return (
    <Card padding="sm">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            旁路关联审核
          </div>
          <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
            仅记录风险信号，不影响主请求结果。
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span
            className={cn(
              "rounded-full px-2.5 py-1 text-xs font-medium",
              associationAuditStatusTone(status, malformed)
            )}
          >
            {malformed ? "解析失败" : associationAuditStatusLabel(status)}
          </span>
          {!malformed ? (
            <span
              className={cn(
                "rounded-full px-2.5 py-1 text-xs font-medium",
                associationAuditRiskTone(risk)
              )}
            >
              {associationAuditRiskLabel(risk)}
            </span>
          ) : null}
        </div>
      </div>

      {malformed ? (
        <div className="mt-3 text-sm text-amber-700 dark:text-amber-300">
          审核结果不是可解析的 JSON 对象。
        </div>
      ) : null}

      {result ? (
        <div className="mt-3 space-y-3">
          <div className="grid gap-2 sm:grid-cols-3">
            <AuditMetric label="关联分" value={scoreText} />
            <AuditMetric label="信号数" value={signals.length} />
            <AuditMetric
              label="耗时"
              value={result.duration_ms == null ? "—" : formatDurationMs(result.duration_ms)}
            />
          </div>

          {providerText || result.reason || result.notes ? (
            <div className="space-y-1 text-sm text-slate-600 dark:text-slate-300">
              {providerText ? <div>{providerText}</div> : null}
              {result.reason ? <div>{associationAuditReasonLabel(result.reason)}</div> : null}
              {result.notes ? <div>{result.notes}</div> : null}
            </div>
          ) : null}

          {signals.length > 0 ? (
            <div className="space-y-2">
              {signals.map((signal, index) => (
                <AssociationAuditSignalRow key={`${signal.code}:${index}`} signal={signal} />
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </Card>
  );
}

function AuditMetric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-lg border border-slate-200/80 bg-slate-50/80 px-3 py-2 dark:border-slate-700 dark:bg-slate-800/70">
      <div className="text-xs text-slate-500 dark:text-slate-400">{label}</div>
      <div className="mt-1 text-sm font-semibold text-slate-900 dark:text-slate-100">{value}</div>
    </div>
  );
}

function AssociationAuditSignalRow({ signal }: { signal: AssociationAuditSignal }) {
  const confidenceText =
    signal.confidence == null ? null : `${Math.round(signal.confidence * 100)}%`;

  return (
    <div className="rounded-lg border border-slate-200/80 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-900/40">
      <div className="flex flex-wrap items-center gap-2">
        <span
          className={cn(
            "rounded-full px-2 py-0.5 text-[11px] font-medium",
            associationAuditSeverityTone(signal.severity)
          )}
        >
          {associationAuditSeverityLabel(signal.severity)}
        </span>
        <span className="font-mono text-xs text-slate-600 dark:text-slate-300">
          {signal.code}
        </span>
        {signal.event_kind ? (
          <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] text-slate-600 dark:bg-slate-700 dark:text-slate-300">
            {signal.event_kind}
          </span>
        ) : null}
        {confidenceText ? (
          <span className="text-xs text-slate-500 dark:text-slate-400">{confidenceText}</span>
        ) : null}
      </div>
      {signal.reason ? (
        <div className="mt-2 text-sm text-slate-700 dark:text-slate-200">{signal.reason}</div>
      ) : null}
      {signal.evidence ? (
        <div className="mt-2 rounded-md bg-slate-50 px-2 py-1 font-mono text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
          {signal.evidence}
        </div>
      ) : null}
    </div>
  );
}

function associationAuditStatusLabel(status: AssociationAuditResult["status"]) {
  switch (status) {
    case "pending":
      return "审核中";
    case "completed":
      return "已完成";
    case "skipped":
      return "已跳过";
    case "timeout":
      return "超时";
    case "invalid_response":
      return "无效响应";
    case "not_configured":
      return "未配置";
    case "unsupported":
      return "不支持";
    case "failed":
    default:
      return "失败";
  }
}

function associationAuditStatusTone(status: AssociationAuditResult["status"], malformed: boolean) {
  if (malformed || status === "invalid_response" || status === "failed" || status === "timeout") {
    return "bg-amber-50 text-amber-700 ring-1 ring-inset ring-amber-500/15 dark:bg-amber-500/15 dark:text-amber-300 dark:ring-amber-400/25";
  }
  if (status === "completed") {
    return "bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-500/15 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/25";
  }
  return "bg-slate-100 text-slate-600 ring-1 ring-inset ring-slate-500/10 dark:bg-slate-700 dark:text-slate-300 dark:ring-slate-500/20";
}

function associationAuditRiskLabel(risk: AssociationAuditResult["overall_risk"]) {
  switch (risk) {
    case "none":
      return "无风险";
    case "low":
      return "低风险";
    case "medium":
      return "中风险";
    case "high":
      return "高风险";
    case "critical":
      return "严重风险";
    case "unknown":
    default:
      return "风险未知";
  }
}

function associationAuditRiskTone(risk: AssociationAuditResult["overall_risk"]) {
  switch (risk) {
    case "critical":
    case "high":
      return "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-500/15 dark:bg-rose-500/15 dark:text-rose-300 dark:ring-rose-400/25";
    case "medium":
      return "bg-orange-50 text-orange-700 ring-1 ring-inset ring-orange-500/15 dark:bg-orange-500/15 dark:text-orange-300 dark:ring-orange-400/25";
    case "low":
      return "bg-amber-50 text-amber-700 ring-1 ring-inset ring-amber-500/15 dark:bg-amber-500/15 dark:text-amber-300 dark:ring-amber-400/25";
    case "none":
      return "bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-500/15 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/25";
    case "unknown":
    default:
      return "bg-slate-100 text-slate-600 ring-1 ring-inset ring-slate-500/10 dark:bg-slate-700 dark:text-slate-300 dark:ring-slate-500/20";
  }
}

function associationAuditSeverityLabel(severity: AssociationAuditSignal["severity"]) {
  switch (severity) {
    case "critical":
      return "严重";
    case "high":
      return "高";
    case "medium":
      return "中";
    case "low":
      return "低";
    case "info":
    default:
      return "信息";
  }
}

function associationAuditSeverityTone(severity: AssociationAuditSignal["severity"]) {
  switch (severity) {
    case "critical":
    case "high":
      return "bg-rose-50 text-rose-700 dark:bg-rose-500/15 dark:text-rose-300";
    case "medium":
      return "bg-orange-50 text-orange-700 dark:bg-orange-500/15 dark:text-orange-300";
    case "low":
      return "bg-amber-50 text-amber-700 dark:bg-amber-500/15 dark:text-amber-300";
    case "info":
    default:
      return "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300";
  }
}

function associationAuditReasonLabel(reason: string) {
  const known: Record<string, string> = {
    prefilter_no_signal: "预筛选未发现需要调用审核 LLM 的信号。",
    sampled_out: "采样未命中。",
    audit_queue_saturated: "旁路审核队列已满。",
    provider_not_configured: "未选择审核 Provider。",
    provider_missing: "审核 Provider 不存在。",
    provider_disabled: "审核 Provider 未启用。",
    model_not_configured: "未配置审核模型。",
    unsupported_provider: "当前 Provider 类型暂不支持旁路审核。",
    unsupported_auth_mode: "当前认证模式暂不支持旁路审核。",
    api_key_not_configured: "审核 Provider 缺少 API Key。",
    base_url_not_configured: "审核 Provider 缺少 Base URL。",
  };
  return known[reason] ?? reason;
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

function formatCostMultiplier(value: number | null | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) return "—";
  return value === 0 ? "免费" : `x${value.toFixed(2)}`;
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
