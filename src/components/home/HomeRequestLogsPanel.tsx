// Usage:
// - Render as the right side column in `HomeOverviewPanel` to show realtime traces + request logs list.
// - Selection state is controlled by parent; the detail dialog is rendered outside the grid layout.

import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import type { RequestLogSummary } from "../../services/requestLogs";
import type { TraceSession } from "../../services/traceStore";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Tooltip } from "../../ui/Tooltip";
import { cn } from "../../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatRelativeTimeFromUnixSeconds,
  formatTokensPerSecond,
  formatUsd,
  sanitizeTtfbMs,
} from "../../utils/formatters";
import {
  computeEffectiveInputTokens,
  computeStatusBadge,
  getErrorCodeLabel,
  SessionReuseBadge,
} from "./HomeLogShared";
import {
  Clock,
  CheckCircle2,
  XCircle,
  Server,
  Terminal,
  Cpu,
  RefreshCw,
  ArrowUpRight,
} from "lucide-react";
import { RealtimeTraceCards } from "./RealtimeTraceCards";

export type HomeRequestLogsPanelProps = {
  showCustomTooltip: boolean;
  title?: string;
  showOpenLogsPageButton?: boolean;

  traces: TraceSession[];

  requestLogs: RequestLogSummary[];
  requestLogsLoading: boolean;
  requestLogsRefreshing: boolean;
  requestLogsAvailable: boolean | null;
  onRefreshRequestLogs: () => void;

  selectedLogId: number | null;
  onSelectLogId: (id: number | null) => void;
};

export function HomeRequestLogsPanel({
  showCustomTooltip,
  title,
  showOpenLogsPageButton = true,
  traces,
  requestLogs,
  requestLogsLoading,
  requestLogsRefreshing,
  requestLogsAvailable,
  onRefreshRequestLogs,
  selectedLogId,
  onSelectLogId,
}: HomeRequestLogsPanelProps) {
  const navigate = useNavigate();
  const realtimeTraceCandidates = useMemo(() => {
    const nowMs = Date.now();
    return traces
      .filter((t) => nowMs - t.first_seen_ms < 15 * 60 * 1000)
      .sort((a, b) => b.first_seen_ms - a.first_seen_ms)
      .slice(0, 20);
  }, [traces]);

  function formatUnixSeconds(ts: number) {
    return formatRelativeTimeFromUnixSeconds(ts);
  }

  return (
    <Card padding="sm" className="flex flex-col gap-3 lg:col-span-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="text-sm font-semibold">{title ?? "使用记录（最近 50 条）"}</div>
        </div>

        <div className="flex items-center gap-2">
          <div className="text-xs text-slate-500">
            {requestLogsAvailable === false
              ? "仅在 Tauri Desktop 环境可用"
              : requestLogs.length === 0 && requestLogsLoading
                ? "加载中…"
                : requestLogsLoading || requestLogsRefreshing
                  ? `更新中… · 共 ${requestLogs.length} 条`
                  : `共 ${requestLogs.length} 条`}
          </div>
          {showOpenLogsPageButton && (
            <Button
              onClick={() => navigate("/logs")}
              variant="ghost"
              size="sm"
              className="h-8 px-2 text-slate-500 hover:text-indigo-600"
              disabled={requestLogsAvailable === false}
              title="打开日志页"
            >
              <ArrowUpRight className="h-4 w-4 mr-1.5" />
              日志
            </Button>
          )}
          <Button
            onClick={onRefreshRequestLogs}
            variant="ghost"
            size="sm"
            className="h-8 px-2 text-slate-500 hover:text-indigo-600"
            disabled={requestLogsAvailable === false || requestLogsLoading || requestLogsRefreshing}
          >
            <RefreshCw
              className={cn(
                "h-4 w-4 mr-1.5",
                (requestLogsLoading || requestLogsRefreshing) && "animate-spin"
              )}
            />
            刷新
          </Button>
        </div>
      </div>

      <div className="border rounded-lg border-slate-200 bg-white shadow-sm overflow-hidden">
        <div className="scrollbar-overlay max-h-[50vh] lg:max-h-[calc(100vh-320px)] pr-1">
          <RealtimeTraceCards
            traces={realtimeTraceCandidates}
            formatUnixSeconds={formatUnixSeconds}
            showCustomTooltip={showCustomTooltip}
          />

          {requestLogsAvailable === false ? (
            <div className="text-sm text-slate-600">仅在 Tauri Desktop 环境可用</div>
          ) : requestLogs.length === 0 ? (
            requestLogsLoading ? (
              <div className="text-sm text-slate-600">加载中…</div>
            ) : null
          ) : (
            requestLogs.map((log) => {
              const statusBadge = computeStatusBadge({
                status: log.status,
                errorCode: log.error_code,
              });

              const providerText =
                log.final_provider_id === 0 ||
                !log.final_provider_name ||
                log.final_provider_name.trim().length === 0 ||
                log.final_provider_name === "Unknown"
                  ? "未知"
                  : log.final_provider_name;

              const providerChainText = (() => {
                const hops = log.route ?? [];
                if (hops.length === 0) return null;
                const parts = hops.map((hop, idx) => {
                  const raw = hop.provider_name?.trim();
                  const name = !raw || raw === "Unknown" ? "未知" : raw;
                  const status =
                    hop.status ?? (idx === hops.length - 1 ? log.status : null) ?? null;
                  const statusText = status == null ? "—" : String(status);
                  if (hop.ok) return `${name}(${statusText})`;
                  const code = hop.error_code ?? null;
                  const label = code ? getErrorCodeLabel(code) : "失败";
                  return `${name}(${statusText} ${label})`;
                });
                return parts.join("→");
              })();

              const providerTitle = providerText;

              const modelText =
                log.requested_model && log.requested_model.trim()
                  ? log.requested_model.trim()
                  : "未知";

              const cliLabel = cliShortLabel(log.cli_key);
              const cliTone = cliBadgeTone(log.cli_key);

              const ttfbMs = sanitizeTtfbMs(log.ttfb_ms, log.duration_ms);
              const outputTokensPerSecond = computeOutputTokensPerSecond(
                log.output_tokens,
                log.duration_ms,
                ttfbMs
              );

              const costMultiplier = log.cost_multiplier;
              const showCostMultiplier =
                Number.isFinite(costMultiplier) &&
                costMultiplier > 0 &&
                Math.abs(costMultiplier - 1) > 0.0001;

              const cacheWrite = (() => {
                if (log.cache_creation_5m_input_tokens != null) {
                  return {
                    tokens: log.cache_creation_5m_input_tokens,
                    ttl: "5m" as const,
                  };
                }
                if (log.cache_creation_input_tokens != null) {
                  return {
                    tokens: log.cache_creation_input_tokens,
                    ttl: null,
                  };
                }
                return { tokens: null, ttl: null };
              })();

              const effectiveInputTokens = computeEffectiveInputTokens(
                log.cli_key,
                log.input_tokens,
                log.cache_read_input_tokens
              );

              return (
                <button
                  key={log.id}
                  type="button"
                  onClick={() => onSelectLogId(log.id)}
                  className="w-full text-left group"
                >
                  <div
                    className={cn(
                      "relative transition-all duration-150 group/item",
                      selectedLogId === log.id ? "bg-slate-100/80" : "bg-white hover:bg-slate-50/80"
                    )}
                  >
                    {/* Selection indicator */}
                    <div
                      className={cn(
                        "absolute left-0 top-1 bottom-1 w-0.5 rounded-full transition-all duration-150",
                        selectedLogId === log.id
                          ? "bg-indigo-500 opacity-100"
                          : "bg-slate-300 opacity-0 group-hover/item:opacity-50"
                      )}
                    />

                    <div className="flex flex-col gap-1 px-3 py-2">
                      {/* Row 1: Status + CLI + Model + Time + Badges */}
                      <div className="flex items-center gap-2 min-w-0">
                        <span
                          className={cn(
                            "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                            statusBadge.tone
                          )}
                          title={statusBadge.title}
                        >
                          {statusBadge.isError ? (
                            <XCircle className="h-3 w-3" />
                          ) : (
                            <CheckCircle2 className="h-3 w-3" />
                          )}
                          {statusBadge.text}
                        </span>

                        <span
                          className={cn(
                            "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium shrink-0",
                            cliTone
                          )}
                        >
                          {log.cli_key === "claude" ? (
                            <Terminal className="h-3 w-3" />
                          ) : (
                            <Cpu className="h-3 w-3" />
                          )}
                          {cliLabel}
                        </span>

                        <span
                          className="text-xs font-medium text-slate-800 truncate"
                          title={modelText}
                        >
                          {modelText}
                        </span>

                        {log.session_reuse && (
                          <SessionReuseBadge showCustomTooltip={showCustomTooltip} />
                        )}

                        {log.error_code && (
                          <span className="rounded bg-amber-50 px-1 py-0.5 text-[10px] font-medium text-amber-700 shrink-0">
                            {getErrorCodeLabel(log.error_code)}
                          </span>
                        )}

                        <span className="flex items-center gap-1 text-[11px] text-slate-400 ml-auto shrink-0">
                          <Clock className="h-3 w-3" />
                          {formatUnixSeconds(log.created_at)}
                        </span>
                      </div>

                      {/* Row 2: Provider + Stats Grid (2 rows x 4 cols for alignment) */}
                      <div className="flex items-start gap-3 text-[11px]">
                        {/* Provider - left side (2 rows: name + multiplier) */}
                        <div
                          className="flex flex-col gap-y-0.5 w-[90px] shrink-0"
                          title={providerTitle}
                        >
                          <div className="flex items-center gap-1 h-4">
                            <Server className="h-3 w-3 text-slate-400 shrink-0" />
                            <span className="truncate font-medium text-slate-600">
                              {providerText}
                            </span>
                          </div>
                          <div className="flex items-center h-4">
                            <div className="flex items-center gap-1 min-w-0 w-full">
                              {providerChainText ? (
                                showCustomTooltip ? (
                                  <Tooltip
                                    content={providerChainText}
                                    contentClassName="max-w-[520px] break-words font-mono"
                                    placement="top"
                                  >
                                    <span className="text-[10px] text-slate-400 hover:text-indigo-600 cursor-help">
                                      链路
                                    </span>
                                  </Tooltip>
                                ) : (
                                  <span
                                    className="text-[10px] text-slate-400 cursor-help"
                                    title={providerChainText}
                                  >
                                    链路
                                  </span>
                                )
                              ) : null}

                              {showCostMultiplier ? (
                                <span className="inline-flex items-center rounded bg-indigo-50 px-1 text-[10px] font-medium text-indigo-600 shrink-0">
                                  x{costMultiplier.toFixed(2)}
                                </span>
                              ) : null}
                            </div>
                          </div>
                        </div>

                        {/* Stats Grid: 2 rows x 4 cols */}
                        <div className="grid grid-cols-4 gap-x-4 gap-y-0.5 flex-1 text-slate-500">
                          {/* Row 1: 输入 | 缓存创建 | 首字 | 花费 */}
                          <div className="flex items-center gap-1 h-4" title="Input Tokens">
                            <span className="text-slate-400 shrink-0">输入</span>
                            <span className="font-mono tabular-nums text-slate-600">
                              {formatInteger(effectiveInputTokens)}
                            </span>
                          </div>
                          <div className="flex items-center gap-1 h-4" title="Cache Write">
                            <span className="text-slate-400 shrink-0">缓存创建</span>
                            {cacheWrite.tokens ? (
                              <>
                                <span className="font-mono tabular-nums text-slate-600">
                                  {formatInteger(cacheWrite.tokens)}
                                </span>
                                {cacheWrite.ttl && (
                                  <span className="text-slate-400 text-[10px]">
                                    ({cacheWrite.ttl})
                                  </span>
                                )}
                              </>
                            ) : (
                              <span className="text-slate-300">—</span>
                            )}
                          </div>
                          <div className="flex items-center gap-1 h-4" title="TTFB">
                            <span className="text-slate-400 shrink-0">首字</span>
                            <span className="font-mono tabular-nums text-slate-600">
                              {ttfbMs != null ? formatDurationMs(ttfbMs) : "—"}
                            </span>
                          </div>
                          <div className="flex items-center gap-1 h-4" title="Cost">
                            <span className="text-slate-400 shrink-0">花费</span>
                            <span className="font-mono tabular-nums text-slate-600">
                              {formatUsd(log.cost_usd)}
                            </span>
                          </div>

                          {/* Row 2: 输出 | 缓存读取 | 耗时 | 速率 */}
                          <div className="flex items-center gap-1 h-4" title="Output Tokens">
                            <span className="text-slate-400 shrink-0">输出</span>
                            <span className="font-mono tabular-nums text-slate-600">
                              {formatInteger(log.output_tokens)}
                            </span>
                          </div>
                          <div className="flex items-center gap-1 h-4" title="Cache Read">
                            <span className="text-slate-400 shrink-0">缓存读取</span>
                            {log.cache_read_input_tokens ? (
                              <span className="font-mono tabular-nums text-slate-600">
                                {formatInteger(log.cache_read_input_tokens)}
                              </span>
                            ) : (
                              <span className="text-slate-300">—</span>
                            )}
                          </div>
                          <div className="flex items-center gap-1 h-4" title="Duration">
                            <span className="text-slate-400 shrink-0">耗时</span>
                            <span className="font-mono tabular-nums text-slate-600">
                              {formatDurationMs(log.duration_ms)}
                            </span>
                          </div>
                          <div className="flex items-center gap-1 h-4" title="Tokens/s">
                            <span className="text-slate-400 shrink-0">速率</span>
                            {outputTokensPerSecond ? (
                              <span className="font-mono tabular-nums text-slate-600">
                                {formatTokensPerSecond(outputTokensPerSecond)}
                              </span>
                            ) : (
                              <span className="text-slate-300">—</span>
                            )}
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Subtle bottom divider */}
                    <div className="absolute inset-x-3 bottom-0 h-px bg-slate-100" />
                  </div>
                </button>
              );
            })
          )}
        </div>
      </div>
    </Card>
  );
}
