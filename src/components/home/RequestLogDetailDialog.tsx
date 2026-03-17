// Usage:
// - Used by `HomeRequestLogsPanel` to show the selected request log detail.
// - Keeps the dialog UI isolated from the main overview panel to reduce file size and improve cohesion.

import { toast } from "sonner";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { copyText } from "../../services/clipboard";
import { logToConsole } from "../../services/consoleLog";
import type { RequestAttemptLog, RequestLogDetail } from "../../services/requestLogs";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Dialog } from "../../ui/Dialog";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatRelativeTimeFromUnixSeconds,
  formatTokensPerSecond,
  formatUsdRaw,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import { ProviderChainView } from "../ProviderChainView";
import { resolveProviderLabel } from "../../pages/providers/baseUrl";
import { computeStatusBadge } from "./HomeLogShared";

export type RequestLogDetailDialogProps = {
  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;

  selectedLog: RequestLogDetail | null;
  selectedLogLoading: boolean;

  attemptLogs: RequestAttemptLog[];
  attemptLogsLoading: boolean;
};

export function RequestLogDetailDialog({
  selectedLogId,
  onSelectLogId,
  selectedLog,
  selectedLogLoading,
  attemptLogs,
  attemptLogsLoading,
}: RequestLogDetailDialogProps) {
  function formatUnixSeconds(ts: number) {
    return formatRelativeTimeFromUnixSeconds(ts);
  }

  const finalProviderText = resolveProviderLabel(
    selectedLog?.final_provider_name,
    selectedLog?.final_provider_id
  );

  const sourceProviderText = resolveProviderLabel(
    selectedLog?.final_provider_source_name,
    selectedLog?.final_provider_source_id
  );

  return (
    <Dialog
      open={selectedLogId != null}
      onOpenChange={(open) => {
        if (!open) onSelectLogId(null);
      }}
      title="使用记录"
      description="点击列表项查看详情（trace_id / failover attempts / error_code）。"
      className="max-w-3xl"
    >
      {selectedLogLoading ? (
        <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
      ) : !selectedLog ? (
        <div className="text-sm text-slate-600 dark:text-slate-400">
          未找到记录详情（可能已过期被留存策略清理）。
        </div>
      ) : (
        <div className="space-y-3">
          <Card padding="sm">
            <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  <span className="mr-2 font-mono text-xs text-slate-500 dark:text-slate-400">
                    {selectedLog.method.toUpperCase()}
                  </span>
                  <span className="truncate">{selectedLog.path}</span>
                </div>
                <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
                  <span
                    className={cn(
                      "rounded-full px-2 py-0.5 font-medium",
                      cliBadgeTone(selectedLog.cli_key)
                    )}
                  >
                    {cliShortLabel(selectedLog.cli_key)}
                  </span>
                  {(() => {
                    const badge = computeStatusBadge({
                      status: selectedLog.status,
                      errorCode: selectedLog.error_code,
                    });
                    return (
                      <span
                        className={cn("rounded-full px-2 py-0.5 font-medium", badge.tone)}
                        title={badge.title}
                      >
                        {badge.text}
                      </span>
                    );
                  })()}
                  {selectedLog.error_code ? (
                    <span className="rounded-full bg-amber-50 dark:bg-amber-900/30 px-2 py-0.5 font-medium text-amber-700 dark:text-amber-400">
                      {selectedLog.error_code}
                    </span>
                  ) : null}
                  {finalProviderText ? (
                    <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                      Provider {finalProviderText}
                    </span>
                  ) : null}
                  {sourceProviderText ? (
                    <span className="rounded-full bg-violet-50 dark:bg-violet-900/30 px-2 py-0.5 font-mono text-violet-700 dark:text-violet-300">
                      source: {sourceProviderText}
                    </span>
                  ) : null}
                  <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                    耗时 {formatDurationMs(selectedLog.duration_ms)}
                  </span>
                  <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                    成本 {formatUsdRaw(selectedLog.cost_usd)}
                    {(() => {
                      const m = selectedLog.cost_multiplier;
                      const show = Number.isFinite(m) && m >= 0 && Math.abs(m - 1) > 0.0001;
                      return show ? (
                        <span className="ml-1 text-slate-500 dark:text-slate-400">
                          (x{m.toFixed(2)})
                        </span>
                      ) : null;
                    })()}
                  </span>
                  {(() => {
                    const ttfbMs = sanitizeTtfbMs(selectedLog.ttfb_ms, selectedLog.duration_ms);
                    return ttfbMs != null ? (
                      <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                        首字 {formatDurationMs(ttfbMs)}
                      </span>
                    ) : null;
                  })()}
                  {(() => {
                    const rate = computeOutputTokensPerSecond(
                      selectedLog.output_tokens,
                      selectedLog.duration_ms,
                      sanitizeTtfbMs(selectedLog.ttfb_ms, selectedLog.duration_ms)
                    );
                    if (rate == null) return null;
                    return (
                      <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                        速率 {formatTokensPerSecond(rate)}
                      </span>
                    );
                  })()}
                  <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
                    {formatUnixSeconds(selectedLog.created_at)}
                  </span>
                </div>
                {selectedLog.query ? (
                  <div className="mt-2 break-words text-xs text-slate-500 dark:text-slate-400">
                    查询：<span className="font-mono">{selectedLog.query}</span>
                  </div>
                ) : null}
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <Button
                  onClick={async () => {
                    try {
                      await copyText(selectedLog.trace_id);
                      toast("已复制 trace_id");
                    } catch (err) {
                      logToConsole("error", "复制 trace_id 失败", {
                        error: String(err),
                      });
                      toast("复制失败：当前环境不支持剪贴板");
                    }
                  }}
                  variant="secondary"
                >
                  复制 trace_id
                </Button>
              </div>
            </div>
          </Card>

          <Card padding="sm">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              故障切换尝试
            </div>
            <ProviderChainView
              attemptLogs={attemptLogs}
              attemptLogsLoading={attemptLogsLoading}
              attemptsJson={selectedLog.attempts_json}
            />
          </Card>

          {(() => {
            const hasTokens =
              selectedLog.input_tokens != null ||
              selectedLog.output_tokens != null ||
              selectedLog.total_tokens != null ||
              selectedLog.cache_read_input_tokens != null ||
              selectedLog.cache_creation_input_tokens != null ||
              selectedLog.cache_creation_5m_input_tokens != null;

            if (!hasTokens && !selectedLog.usage_json) return null;

            return (
              <Card padding="sm">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                    Token 用量
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    {selectedLog.usage_json ? (
                      <Button
                        onClick={async () => {
                          try {
                            await copyText(selectedLog.usage_json ?? "");
                            toast("已复制 usage_json");
                          } catch (err) {
                            logToConsole("error", "复制 usage_json 失败", {
                              error: String(err),
                            });
                            toast("复制失败：当前环境不支持剪贴板");
                          }
                        }}
                        variant="secondary"
                      >
                        复制 usage_json
                      </Button>
                    ) : null}
                  </div>
                </div>

                <div className="mt-3 grid gap-2 text-sm text-slate-700 dark:text-slate-300 sm:grid-cols-2">
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">input_tokens</span>
                    <span className="font-mono">{selectedLog.input_tokens ?? "—"}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">output_tokens</span>
                    <span className="font-mono">{selectedLog.output_tokens ?? "—"}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">total_tokens</span>
                    <span className="font-mono">{selectedLog.total_tokens ?? "—"}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">
                      cache_read_input_tokens
                    </span>
                    <span className="font-mono">{selectedLog.cache_read_input_tokens ?? "—"}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">
                      cache_creation_input_tokens
                    </span>
                    <span className="font-mono">
                      {selectedLog.cache_creation_input_tokens ?? "—"}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">
                      cache_creation_5m_input_tokens
                    </span>
                    <span className="font-mono">
                      {selectedLog.cache_creation_5m_input_tokens ?? "—"}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-slate-500 dark:text-slate-400">
                      cache_creation_1h_input_tokens
                    </span>
                    <span className="font-mono">
                      {selectedLog.cache_creation_1h_input_tokens ?? "—"}
                    </span>
                  </div>
                </div>

                {selectedLog.usage_json ? (
                  <pre className="mt-3 max-h-[240px] overflow-auto rounded-lg bg-slate-950 p-3 text-xs text-slate-100">
                    {(() => {
                      try {
                        const parsed: unknown = JSON.parse(selectedLog.usage_json ?? "");
                        return JSON.stringify(parsed, null, 2);
                      } catch {
                        return selectedLog.usage_json;
                      }
                    })()}
                  </pre>
                ) : null}
              </Card>
            );
          })()}
        </div>
      )}
    </Dialog>
  );
}
