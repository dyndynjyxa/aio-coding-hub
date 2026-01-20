// Usage: Diagnostics console and request detail viewer. Backend commands: `request_logs_*` (incl. `request_log_get_by_trace_id`), `request_attempt_logs_by_trace_id`.

import { useEffect, useMemo, useRef, useState } from "react";
import { clearConsoleLogs, useConsoleLogs } from "../services/consoleLog";
import { requestLogGetByTraceId, type RequestLogDetail } from "../services/requestLogs";
import { setSearchTraceId, setSelectedTraceId, useTraceStore } from "../services/traceStore";
import { toast } from "sonner";
import { ProviderChainView } from "../components/ProviderChainView";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { cn } from "../utils/cn";
import {
  computeOutputTokensPerSecond,
  formatDurationMs,
  formatInteger,
  formatTokensPerSecond,
  formatUsd,
  sanitizeTtfbMs,
} from "../utils/formatters";

function formatTs(ts: number) {
  return new Date(ts).toLocaleString();
}

function levelClass(level: string) {
  switch (level) {
    case "error":
      return "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-200";
    case "warn":
      return "bg-amber-50 text-amber-700 ring-1 ring-inset ring-amber-200";
    default:
      return "bg-slate-100 text-slate-700 ring-1 ring-inset ring-slate-200";
  }
}

type ConsoleTab = "timeline" | "logs";
type LogLevelFilter = "all" | "info" | "warn" | "error";

function traceResultLabel(trace: {
  summary?: { status: number | null; error_code: string | null };
  attempts: Array<{ outcome: string; status: number | null }>;
}) {
  if (trace.summary) {
    const failed =
      Boolean(trace.summary.error_code) ||
      (trace.summary.status != null && (trace.summary.status < 200 || trace.summary.status >= 300));
    return failed ? "失败" : "成功";
  }
  const lastAttempt = trace.attempts.length > 0 ? trace.attempts[trace.attempts.length - 1] : null;
  if (!lastAttempt) return "进行中";
  return lastAttempt.outcome === "success" ? "成功" : "失败";
}

function traceResultClass(label: string) {
  switch (label) {
    case "成功":
      return "bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-200";
    case "失败":
      return "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-200";
    default:
      return "bg-slate-100 text-slate-700 ring-1 ring-inset ring-slate-200";
  }
}

export function ConsolePage() {
  const [tab, setTab] = useState<ConsoleTab>("logs");
  const logs = useConsoleLogs();
  const { traces, selectedTraceId, searchTraceId, maxTraces } = useTraceStore();
  const [logLevel, setLogLevel] = useState<LogLevelFilter>("all");
  const [logTraceId, setLogTraceId] = useState<string>("");
  const [logCli, setLogCli] = useState<string>("all");
  const [logProvider, setLogProvider] = useState<string>("all");

  const filteredTraces = useMemo(() => {
    const q = searchTraceId.trim();
    if (!q) return traces;
    return traces.filter((t) => t.trace_id.includes(q));
  }, [traces, searchTraceId]);

  const effectiveSelectedTraceId = selectedTraceId ?? filteredTraces[0]?.trace_id ?? null;

  const selectedTrace = useMemo(() => {
    if (!effectiveSelectedTraceId) return null;
    return traces.find((t) => t.trace_id === effectiveSelectedTraceId) ?? null;
  }, [effectiveSelectedTraceId, traces]);

  const [requestLogsByTraceId, setRequestLogsByTraceId] = useState<
    Record<string, RequestLogDetail | null>
  >({});
  const requestLogInflightTraceIdsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (tab !== "timeline") return;
    if (filteredTraces.length === 0) return;

    const traceIds = filteredTraces.map((t) => t.trace_id);
    const toFetch = traceIds.filter((traceId) => {
      if (requestLogInflightTraceIdsRef.current.has(traceId)) return false;
      return !(traceId in requestLogsByTraceId);
    });
    if (toFetch.length === 0) return;

    let cancelled = false;
    for (const traceId of toFetch) requestLogInflightTraceIdsRef.current.add(traceId);

    Promise.allSettled(
      toFetch.map(async (traceId) => {
        try {
          const detail = await requestLogGetByTraceId(traceId);
          return { traceId, detail };
        } catch {
          return { traceId, detail: null };
        }
      })
    )
      .then((results) => {
        if (cancelled) return;
        setRequestLogsByTraceId((prev) => {
          const next = { ...prev };
          for (const result of results) {
            if (result.status !== "fulfilled") continue;
            next[result.value.traceId] = result.value.detail;
          }
          return next;
        });
      })
      .finally(() => {
        for (const traceId of toFetch) requestLogInflightTraceIdsRef.current.delete(traceId);
      });

    return () => {
      cancelled = true;
    };
  }, [filteredTraces, requestLogsByTraceId, tab]);

  const selectedTraceLog =
    tab === "timeline" && effectiveSelectedTraceId
      ? (requestLogsByTraceId[effectiveSelectedTraceId] ?? null)
      : null;

  const logCliOptions = useMemo(() => {
    const set = new Set<string>();
    for (const entry of logs) {
      const cliKey = entry.meta?.cli_key;
      if (cliKey) set.add(cliKey);
    }
    return Array.from(set).sort();
  }, [logs]);

  const logProviderOptions = useMemo(() => {
    const set = new Set<string>();
    for (const entry of logs) {
      for (const provider of entry.meta?.providers ?? []) {
        if (provider) set.add(provider);
      }
    }
    return Array.from(set).sort();
  }, [logs]);

  const filteredLogs = useMemo(() => {
    const traceQ = logTraceId.trim();
    const cliQ = logCli === "all" ? null : logCli;
    const providerQ = logProvider === "all" ? null : logProvider;
    const levelQ = logLevel === "all" ? null : logLevel;

    return logs.filter((l) => {
      if (levelQ && l.level !== levelQ) return false;
      if (cliQ && (l.meta?.cli_key ?? null) !== cliQ) return false;
      if (providerQ) {
        const providers = l.meta?.providers ?? [];
        if (!providers.includes(providerQ)) return false;
      }
      if (traceQ) {
        const traceId = l.meta?.trace_id;
        if (!traceId || !traceId.includes(traceQ)) return false;
      }
      return true;
    });
  }, [logs, logCli, logLevel, logProvider, logTraceId]);

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">控制台</h1>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            onClick={() => setTab("timeline")}
            variant={tab === "timeline" ? "primary" : "secondary"}
          >
            Trace Timeline
          </Button>
          <Button onClick={() => setTab("logs")} variant={tab === "logs" ? "primary" : "secondary"}>
            Logs
          </Button>

          {tab === "logs" ? (
            <Button
              onClick={() => {
                clearConsoleLogs();
                toast("已清空控制台日志");
              }}
              variant="secondary"
            >
              清空
            </Button>
          ) : null}
        </div>
      </div>

      {tab === "timeline" ? (
        <Card padding="none">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-200 px-4 py-3 text-sm font-medium">
            <div>Trace Timeline（最近 {maxTraces} 条）</div>
            <div className="text-xs font-normal text-slate-500">
              实时事件：gateway:attempt / gateway:request
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-[360px_1fr]">
            <div className="border-b border-slate-200 p-4 lg:border-b-0 lg:border-r">
              <Input
                type="text"
                value={searchTraceId}
                onChange={(e) => setSearchTraceId(e.currentTarget.value)}
                placeholder="按 trace_id 搜索（支持包含匹配）"
                className="w-full"
              />

              <div className="mt-3 space-y-2">
                {filteredTraces.length === 0 ? (
                  <div className="rounded-lg border border-dashed border-slate-200 px-3 py-8 text-center text-sm text-slate-500">
                    暂无追踪记录。可发起一次请求触发故障切换事件。
                  </div>
                ) : (
                  filteredTraces.map((t) => {
                    const isActive = t.trace_id === effectiveSelectedTraceId;
                    const result = traceResultLabel(t);
                    const requestLog = requestLogsByTraceId[t.trace_id] ?? null;

                    const statusCode = requestLog?.status ?? t.summary?.status ?? null;
                    const errorCode = requestLog?.error_code ?? t.summary?.error_code ?? null;
                    const statusText = statusCode == null ? result : String(statusCode);
                    const statusTone =
                      statusCode == null
                        ? traceResultClass(result)
                        : statusCode >= 200 && statusCode < 300 && !errorCode
                          ? "bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-200"
                          : errorCode || statusCode >= 400
                            ? "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-200"
                            : "bg-slate-100 text-slate-700 ring-1 ring-inset ring-slate-200";

                    const routeAttempts =
                      (t.summary?.attempts?.length ?? 0) > 0 ? t.summary!.attempts : t.attempts;
                    const startProvider = routeAttempts[0] ?? null;
                    const finalProvider =
                      routeAttempts.length > 0 ? routeAttempts[routeAttempts.length - 1] : null;
                    const startProviderName =
                      startProvider?.provider_name && startProvider.provider_name !== "Unknown"
                        ? startProvider.provider_name
                        : "未知";
                    const finalProviderName =
                      finalProvider?.provider_name && finalProvider.provider_name !== "Unknown"
                        ? finalProvider.provider_name
                        : "未知";
                    const routeSummary =
                      routeAttempts.length === 0
                        ? "—"
                        : startProviderName === finalProviderName
                          ? startProviderName
                          : `${startProviderName} → ${finalProviderName}${
                              routeAttempts.length > 2 ? ` +${routeAttempts.length - 2}` : ""
                            }`;

                    const modelText =
                      requestLog?.requested_model && requestLog.requested_model.trim()
                        ? requestLog.requested_model.trim()
                        : "未知";

                    const inputTokens = requestLog?.input_tokens ?? t.summary?.input_tokens ?? null;
                    const outputTokens =
                      requestLog?.output_tokens ?? t.summary?.output_tokens ?? null;
                    const cacheReadInputTokens =
                      requestLog?.cache_read_input_tokens ??
                      t.summary?.cache_read_input_tokens ??
                      null;
                    const cacheCreation5mInputTokens =
                      requestLog?.cache_creation_5m_input_tokens ??
                      t.summary?.cache_creation_5m_input_tokens ??
                      null;
                    const cacheCreationInputTokens =
                      requestLog?.cache_creation_input_tokens ??
                      t.summary?.cache_creation_input_tokens ??
                      null;
                    const cacheWrite = (() => {
                      if (cacheCreation5mInputTokens != null) {
                        return { tokens: cacheCreation5mInputTokens, ttl: "5m" as const };
                      }
                      if (cacheCreationInputTokens != null) {
                        return { tokens: cacheCreationInputTokens, ttl: null };
                      }
                      return { tokens: null, ttl: null };
                    })();

                    const durationMs = requestLog?.duration_ms ?? t.summary?.duration_ms ?? null;
                    const ttfbMs = sanitizeTtfbMs(
                      requestLog?.ttfb_ms ?? t.summary?.ttfb_ms ?? null,
                      durationMs
                    );
                    const outputTokensPerSecond = computeOutputTokensPerSecond(
                      outputTokens,
                      durationMs,
                      ttfbMs
                    );

                    const costMultiplier = requestLog?.cost_multiplier ?? 1.0;
                    const showCostMultiplier =
                      Number.isFinite(costMultiplier) &&
                      costMultiplier > 0 &&
                      Math.abs(costMultiplier - 1) > 0.0001;

                    return (
                      <button
                        key={t.trace_id}
                        type="button"
                        onClick={() => setSelectedTraceId(t.trace_id)}
                        className={cn(
                          "w-full rounded-xl border px-3 py-2 text-left transition",
                          "hover:bg-slate-50",
                          isActive
                            ? "border-[#0052FF]/40 bg-[#0052FF]/5"
                            : "border-slate-200 bg-white"
                        )}
                      >
                        <div className="flex items-center justify-between gap-2">
                          <div className="min-w-0 truncate font-mono text-xs text-slate-700">
                            {t.trace_id}
                          </div>
                          <div className="flex items-center gap-2">
                            {errorCode ? (
                              <span className="rounded-full bg-amber-50 px-2 py-0.5 text-xs font-medium text-amber-700">
                                {errorCode}
                              </span>
                            ) : null}
                            <span
                              className={cn(
                                "shrink-0 rounded-full px-2 py-0.5 text-xs font-medium",
                                statusTone
                              )}
                            >
                              {statusText}
                            </span>
                          </div>
                        </div>
                        <div className="mt-2 grid grid-cols-2 gap-2">
                          <div className="min-w-0">
                            <div className="text-[11px] text-slate-500">供应商</div>
                            <div className="truncate text-xs font-medium text-slate-900">
                              {finalProviderName}
                              {showCostMultiplier ? (
                                <span className="ml-2 rounded-full bg-slate-100 px-2 py-0.5 font-mono text-[10px] text-slate-600">
                                  x{costMultiplier.toFixed(2)}
                                </span>
                              ) : null}
                            </div>
                            <div className="truncate text-[11px] text-slate-500">
                              {routeSummary}
                            </div>
                          </div>

                          <div className="min-w-0">
                            <div className="text-[11px] text-slate-500">模型</div>
                            <div
                              className="truncate text-xs font-medium text-slate-900"
                              title={modelText}
                            >
                              {modelText}
                            </div>
                          </div>

                          <div>
                            <div className="text-[11px] text-slate-500">Token</div>
                            <div className="font-mono text-[11px] text-slate-700">
                              输入 {formatInteger(inputTokens)}
                            </div>
                            <div className="font-mono text-[11px] text-slate-700">
                              输出 {formatInteger(outputTokens)}
                            </div>
                          </div>

                          <div>
                            <div className="text-[11px] text-slate-500">缓存</div>
                            <div className="flex items-center gap-2 font-mono text-[11px] text-slate-700">
                              <span>写入 {formatInteger(cacheWrite.tokens)}</span>
                              {cacheWrite.ttl ? (
                                <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] text-slate-600">
                                  {cacheWrite.ttl}
                                </span>
                              ) : null}
                            </div>
                            <div className="font-mono text-[11px] text-slate-700">
                              读取 {formatInteger(cacheReadInputTokens)}
                            </div>
                          </div>

                          <div>
                            <div className="text-[11px] text-slate-500">成本</div>
                            <div className="font-mono text-[11px] text-slate-700">
                              {formatUsd(requestLog?.cost_usd ?? null)}
                            </div>
                          </div>

                          <div>
                            <div className="text-[11px] text-slate-500">性能</div>
                            <div className="font-mono text-[11px] text-slate-700">
                              耗时 {formatDurationMs(durationMs)}
                            </div>
                            <div className="font-mono text-[11px] text-slate-700">
                              首字 {formatDurationMs(ttfbMs)}
                            </div>
                            <div className="font-mono text-[11px] text-slate-700">
                              速率 {formatTokensPerSecond(outputTokensPerSecond)}
                            </div>
                          </div>
                        </div>
                        <div className="mt-1 text-xs text-slate-500">
                          最近更新：{formatTs(t.last_seen_ms)}
                        </div>
                      </button>
                    );
                  })
                )}
              </div>
            </div>

            <div className="p-4">
              {selectedTrace ? (
                <div className="space-y-3">
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <div className="text-sm font-semibold">Trace</div>
                      <div className="mt-1 font-mono text-xs text-slate-600">
                        {selectedTrace.trace_id}
                      </div>
                      <div className="mt-1 text-sm text-slate-700">
                        {selectedTrace.cli_key} · {selectedTrace.method} {selectedTrace.path}
                      </div>
                      <div className="mt-1 text-xs text-slate-500">
                        首次出现：{formatTs(selectedTrace.first_seen_ms)} · 最近更新：
                        {formatTs(selectedTrace.last_seen_ms)}
                      </div>
                    </div>

                    <span
                      className={cn(
                        "rounded-full px-2 py-1 text-xs font-medium",
                        traceResultClass(traceResultLabel(selectedTrace))
                      )}
                    >
                      {traceResultLabel(selectedTrace)}
                    </span>
                  </div>

                  <Card padding="sm" className="bg-slate-50">
                    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                      <div>
                        <div className="text-xs text-slate-500">状态码</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {selectedTrace.summary?.status ?? "—"}
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-slate-500">总耗时</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {formatDurationMs(selectedTrace.summary?.duration_ms ?? null)}
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-slate-500">首字时间</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {formatDurationMs(
                            sanitizeTtfbMs(
                              selectedTrace.summary?.ttfb_ms ?? null,
                              selectedTrace.summary?.duration_ms ?? null
                            )
                          )}
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-slate-500">输入 Token</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {selectedTrace.summary?.input_tokens ?? "—"}
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-slate-500">输出 Token</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {selectedTrace.summary?.output_tokens ?? "—"}
                        </div>
                      </div>
                      <div>
                        <div className="text-xs text-slate-500">错误码</div>
                        <div className="mt-1 font-mono text-sm text-slate-900">
                          {selectedTrace.summary?.error_code ?? "—"}
                        </div>
                      </div>
                    </div>
                  </Card>

                  <Card padding="sm">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="text-sm font-semibold text-slate-900">Provider Chain</div>
                    </div>

                    <ProviderChainView
                      attemptLogs={selectedTrace.attempts}
                      attemptsJson={selectedTraceLog?.attempts_json ?? null}
                    />
                  </Card>
                </div>
              ) : (
                <div className="rounded-lg border border-dashed border-slate-200 px-4 py-10 text-sm text-slate-500">
                  请选择左侧 Trace 查看详情。
                </div>
              )}
            </div>
          </div>
        </Card>
      ) : (
        <Card padding="none">
          <div className="border-b border-slate-200 px-4 py-3">
            <div className="flex flex-wrap items-end justify-between gap-2">
              <div className="text-sm font-medium">
                Logs（{filteredLogs.length}/{logs.length}）
              </div>
              <div className="text-xs text-slate-500">
                结构化过滤：trace_id / level / cli / provider
              </div>
            </div>

            <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
              <Input
                type="text"
                value={logTraceId}
                onChange={(e) => setLogTraceId(e.currentTarget.value)}
                placeholder="按 trace_id 过滤（包含匹配）"
                className="w-full"
              />

              <Select
                value={logLevel}
                onChange={(e) => {
                  const next = e.currentTarget.value;
                  if (next === "all" || next === "info" || next === "warn" || next === "error") {
                    setLogLevel(next);
                  }
                }}
                className="w-full"
              >
                <option value="all">全部 level</option>
                <option value="info">info</option>
                <option value="warn">warn</option>
                <option value="error">error</option>
              </Select>

              <Select
                value={logCli}
                onChange={(e) => setLogCli(e.currentTarget.value)}
                className="w-full"
              >
                <option value="all">全部 CLI</option>
                {logCliOptions.map((cliKey) => (
                  <option key={cliKey} value={cliKey}>
                    {cliKey}
                  </option>
                ))}
              </Select>

              <Select
                value={logProvider}
                onChange={(e) => setLogProvider(e.currentTarget.value)}
                className="w-full"
              >
                <option value="all">全部 Provider</option>
                {logProviderOptions.map((provider) => (
                  <option key={provider} value={provider}>
                    {provider}
                  </option>
                ))}
              </Select>
            </div>
          </div>

          <div className="max-h-[65vh] overflow-auto">
            {filteredLogs.length === 0 ? (
              <div className="px-4 py-10 text-sm text-slate-500">
                {logs.length === 0
                  ? "暂无日志。可在设置页启动网关后发起一次请求（curl/CLI）来验证事件日志。"
                  : "无匹配日志：请调整 trace_id / level / cli / provider 过滤条件。"}
              </div>
            ) : (
              <div className="divide-y divide-slate-200">
                {filteredLogs.map((l) => (
                  <div key={l.id} className="px-4 py-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mono text-xs text-slate-500">{formatTs(l.ts)}</span>
                      <span
                        className={`rounded-full px-2 py-0.5 text-xs font-medium ${levelClass(l.level)}`}
                      >
                        {l.level}
                      </span>
                      <span className="text-sm font-medium">{l.title}</span>
                    </div>

                    {l.meta ? (
                      <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                        {l.meta.trace_id ? (
                          <span className="font-mono">trace {l.meta.trace_id}</span>
                        ) : null}
                        {l.meta.cli_key ? (
                          <span className="font-mono">cli {l.meta.cli_key}</span>
                        ) : null}
                        {l.meta.providers && l.meta.providers.length > 0 ? (
                          <span className="font-mono">
                            provider {l.meta.providers[0]}
                            {l.meta.providers.length > 1 ? ` +${l.meta.providers.length - 1}` : ""}
                          </span>
                        ) : null}
                        {l.meta.error_code ? (
                          <span className="font-mono">code {l.meta.error_code}</span>
                        ) : null}
                      </div>
                    ) : null}

                    {l.details ? (
                      <pre className="mt-2 overflow-auto rounded-lg bg-slate-950 p-3 text-xs text-slate-100">
                        {l.details}
                      </pre>
                    ) : null}
                  </div>
                ))}
              </div>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
