import type { RequestLogDetail } from "../../services/gateway/requestLogs";
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
  const pluginAuditLogs = selectedLog.plugin_audit_logs ?? [];
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
            <div className="text-sm font-semibold text-foreground">审计语义</div>
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
            <div className="mt-3 text-sm text-muted-foreground dark:text-secondary-foreground">
              {auditMeta.summary}
            </div>
          ) : null}
        </Card>
      ) : null}

      {pluginAuditLogs.length > 0 ? <PluginAuditLogsCard logs={pluginAuditLogs} /> : null}

      {/* Key metrics */}
      {hasTokens ? (
        <Card padding="sm">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="text-sm font-semibold text-foreground">关键指标</div>
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

function PluginAuditLogsCard({ logs }: { logs: RequestLogDetail["plugin_audit_logs"] }) {
  const visibleLogs = logs.slice(0, 5);

  return (
    <Card padding="sm">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-sm font-semibold text-foreground">插件审计</div>
          <div className="mt-1 text-xs text-muted-foreground">
            与当前 trace 关联的通用插件审计记录
          </div>
        </div>
        <span className="rounded-full bg-secondary px-2.5 py-1 text-xs font-medium text-muted-foreground ring-1 ring-inset ring-border">
          {logs.length} 条
        </span>
      </div>

      <div className="mt-3 space-y-2">
        {visibleLogs.map((log) => {
          const detailsPreview = formatPluginAuditDetailsPreview(log.details);
          return (
            <div
              key={log.id}
              className="rounded-xl border border-border/80 bg-secondary/60 px-3 py-3 dark:border-border dark:bg-secondary/50"
            >
              <div className="flex flex-wrap items-center gap-2 text-xs">
                <span className="font-medium text-foreground">{log.plugin_id ?? "unknown"}</span>
                <span
                  className={cn(
                    "rounded-full px-2 py-0.5 font-medium",
                    riskLevelClassName(log.risk_level)
                  )}
                >
                  {log.risk_level || "low"}
                </span>
                <span className="text-muted-foreground">{log.event_type}</span>
              </div>
              <div className="mt-2 text-sm text-foreground">{log.message || log.event_type}</div>
              {detailsPreview ? (
                <pre className="mt-2 max-h-24 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-background/80 p-2 text-xs font-mono text-muted-foreground">
                  {detailsPreview}
                </pre>
              ) : null}
            </div>
          );
        })}
      </div>

      {logs.length > visibleLogs.length ? (
        <div className="mt-2 text-xs text-muted-foreground">
          仅显示前 {visibleLogs.length} 条，其余记录可在原始数据页查看。
        </div>
      ) : null}
    </Card>
  );
}

function riskLevelClassName(riskLevel: string | null | undefined) {
  switch ((riskLevel ?? "").toLowerCase()) {
    case "critical":
    case "high":
      return "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-500/15 dark:bg-rose-500/15 dark:text-rose-200 dark:ring-rose-400/20";
    case "medium":
      return "bg-amber-50 text-amber-700 ring-1 ring-inset ring-amber-500/15 dark:bg-amber-500/15 dark:text-amber-200 dark:ring-amber-400/20";
    default:
      return "bg-sky-50 text-sky-700 ring-1 ring-inset ring-sky-500/15 dark:bg-sky-500/15 dark:text-sky-200 dark:ring-sky-400/20";
  }
}

function formatPluginAuditDetailsPreview(details: unknown): string | null {
  if (details == null) return null;
  try {
    return JSON.stringify(details, null, 2);
  } catch {
    return String(details);
  }
}

function MetricCard({
  label,
  value,
}: {
  label: string;
  value: string | number | null | undefined;
}) {
  return (
    <div className="rounded-xl border border-border/80 bg-secondary/80 px-3 py-3 dark:border-border dark:bg-secondary/70">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-lg font-semibold text-foreground">
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
