import type { RequestLogDetail } from "../../services/gateway/requestLogs";
import type { ProviderChainAttemptLog } from "../ProviderChainView";
import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { ProviderChainView } from "../ProviderChainView";
import { resolveCodexResponsesTransportRecords } from "../../services/gateway/requestLogSpecialSettings";
import { parseAttemptsJson } from "../../services/gateway/attemptsJson";
import { resolveProviderLabel } from "../../pages/providers/baseUrl";

const TRANSPORT_ACTION_LABELS = new Map([
  ["selected", "选择传输"],
  ["http_fallback", "同供应商降级 HTTP"],
  ["provider_switch", "切换供应商"],
  ["full_input_retry", "恢复上下文并重发完整请求"],
  ["ws_cooldown_skip", "WS 冷却中，使用 HTTP"],
  ["ws_budget_exhausted", "WS 尝试预算已用尽"],
]);

const FAILURE_CLASS_LABELS = new Map([
  ["transport", "传输失败"],
  ["provider", "供应商失败"],
  ["client_input", "请求输入错误"],
  ["context", "上下文缺失"],
  ["local", "本地错误"],
  ["cancelled", "请求取消"],
]);

function transportLabel(value: "http" | "responses_ws" | null) {
  return value === "responses_ws" ? "WS" : value === "http" ? "HTTP" : "未记录";
}

export type RequestLogDetailChainTabProps = {
  selectedLog: RequestLogDetail;
  attemptLogs: ProviderChainAttemptLog[];
  attemptLogsLoading: boolean;
  isInProgress: boolean;
  finalProviderText: string | null;
};

export function RequestLogDetailChainTab({
  selectedLog,
  attemptLogs,
  attemptLogsLoading,
  isInProgress,
  finalProviderText,
}: RequestLogDetailChainTabProps) {
  const transportRecords = resolveCodexResponsesTransportRecords(selectedLog.special_settings_json);
  const attempts =
    attemptLogs.length > 0 ? attemptLogs : (parseAttemptsJson(selectedLog.attempts_json) ?? []);

  return (
    <div className="space-y-3">
      {transportRecords.length > 0 ? (
        <Card padding="sm">
          <div className="text-sm font-semibold text-foreground">Responses 传输记录</div>
          <ol className="mt-2 divide-y divide-border">
            {transportRecords.map((record, index) => {
              const provider = attempts.find(
                (attempt) => attempt.provider_id === record.providerId
              );
              const providerLabel = resolveProviderLabel(
                provider?.provider_name ??
                  (record.providerId === selectedLog.final_provider_id
                    ? selectedLog.final_provider_name
                    : null),
                record.providerId
              );
              const actionLabel = record.transportAction
                ? (TRANSPORT_ACTION_LABELS.get(record.transportAction) ?? "未知传输动作")
                : "传输记录";
              return (
                <li key={index} className="space-y-1 py-2 text-xs text-muted-foreground">
                  <div className="flex flex-wrap gap-x-3 gap-y-1">
                    <span className="font-medium text-foreground">{actionLabel}</span>
                    {providerLabel ? <span>供应商：{providerLabel}</span> : null}
                    <span>
                      客户端 {transportLabel(record.clientTransport)} · 上游{" "}
                      {transportLabel(record.upstreamTransport)}
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-x-3 gap-y-1">
                    {record.failureClass ? (
                      <span>{FAILURE_CLASS_LABELS.get(record.failureClass) ?? "未知失败分类"}</span>
                    ) : null}
                    {record.terminal ? (
                      <span>
                        终态：
                        {record.terminal === "incomplete"
                          ? "不完整结束"
                          : record.terminal === "completed"
                            ? "完整结束"
                            : "失败结束"}
                      </span>
                    ) : null}
                    {record.handshakeStatus ? <span>WS 握手：{record.handshakeStatus}</span> : null}
                    {record.eventStatus ? <span>响应事件状态：{record.eventStatus}</span> : null}
                    {record.reasonCode ? (
                      <span className="break-all font-mono">原因：{record.reasonCode}</span>
                    ) : null}
                    {record.outputCommitted != null ? (
                      <span>{record.outputCommitted ? "已开始输出" : "尚未开始输出"}</span>
                    ) : null}
                  </div>
                  {record.outputCommitted &&
                  record.failureClass &&
                  record.failureClass !== "cancelled" ? (
                    <p>响应中断，已输出内容保留，不能拼接新供应商响应。</p>
                  ) : null}
                  {record.recoveryFromTraceId ? (
                    <p className="break-all">
                      恢复自请求：<span className="font-mono">{record.recoveryFromTraceId}</span>
                    </p>
                  ) : null}
                </li>
              );
            })}
          </ol>
        </Card>
      ) : null}
      <Card padding="sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="text-sm font-semibold text-foreground">决策链</div>
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span
              className={cn(
                "rounded-full px-2 py-0.5 font-medium",
                cliBadgeTone(selectedLog.cli_key)
              )}
            >
              {cliShortLabel(selectedLog.cli_key)}
            </span>
            <span className="rounded-full bg-secondary px-2 py-0.5">
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
  );
}
