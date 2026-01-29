// Usage:
// - Logs page aligned with claude-code-hub `/dashboard/logs` (status codes like 499/524).
// - Entry: Home "日志" button -> `/#/logs`.
// - Backend commands: `request_logs_list_all`, `request_logs_list_after_id_all`, `request_log_get`, `request_attempt_logs_by_trace_id`.

import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { HomeRequestLogsPanel } from "../components/home/HomeRequestLogsPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { CLI_FILTER_ITEMS, type CliFilterKey } from "../constants/clis";
import { logToConsole } from "../services/consoleLog";
import {
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAfterIdAll,
  requestLogsListAll,
  type RequestAttemptLog,
  type RequestLogDetail,
  type RequestLogSummary,
} from "../services/requestLogs";
import { hasTauriRuntime } from "../services/tauriInvoke";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";

const LOGS_PAGE_LIMIT = 200;
const AUTO_REFRESH_INTERVAL_MS = 2000;

type StatusPredicate = (status: number | null) => boolean;

function buildStatusPredicate(query: string): StatusPredicate | null {
  const raw = query.trim();
  if (!raw) return null;

  const exact = raw.match(/^(\d{3})$/);
  if (exact) {
    const target = Number(exact[1]);
    return (status) => status === target;
  }

  const not = raw.match(/^!\s*(\d{3})$/);
  if (not) {
    const target = Number(not[1]);
    return (status) => status == null || status !== target;
  }

  const gte = raw.match(/^>=\s*(\d{3})$/);
  if (gte) {
    const target = Number(gte[1]);
    return (status) => status != null && status >= target;
  }

  const lte = raw.match(/^<=\s*(\d{3})$/);
  if (lte) {
    const target = Number(lte[1]);
    return (status) => status != null && status <= target;
  }

  return null;
}

function requestLogCreatedAtMs(log: Pick<RequestLogSummary, "created_at" | "created_at_ms">) {
  const ms = log.created_at_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return log.created_at * 1000;
}

function sortRequestLogsDesc(a: RequestLogSummary, b: RequestLogSummary) {
  const aTsMs = requestLogCreatedAtMs(a);
  const bTsMs = requestLogCreatedAtMs(b);
  if (aTsMs !== bTsMs) return bTsMs - aTsMs;
  return b.id - a.id;
}

function computeRequestLogsCursorId(rows: RequestLogSummary[]) {
  let maxId = 0;
  for (const row of rows) {
    if (Number.isFinite(row.id) && row.id > maxId) maxId = row.id;
  }
  return maxId;
}

function mergeRequestLogs(prev: RequestLogSummary[], incoming: RequestLogSummary[], limit: number) {
  const byId = new Map<number, RequestLogSummary>();
  for (const row of incoming) byId.set(row.id, row);
  for (const row of prev) {
    if (!byId.has(row.id)) byId.set(row.id, row);
  }
  const merged = Array.from(byId.values());
  merged.sort(sortRequestLogsDesc);
  return merged.slice(0, limit);
}

export function LogsPage() {
  const showCustomTooltip = hasTauriRuntime();

  const [cliKey, setCliKey] = useState<CliFilterKey>("all");
  const [statusFilter, setStatusFilter] = useState("");
  const [errorCodeFilter, setErrorCodeFilter] = useState("");
  const [pathFilter, setPathFilter] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);

  const [requestLogs, setRequestLogs] = useState<RequestLogSummary[]>([]);
  const [requestLogsLoading, setRequestLogsLoading] = useState(false);
  const [requestLogsRefreshing, setRequestLogsRefreshing] = useState(false);
  const [requestLogsAvailable, setRequestLogsAvailable] = useState<boolean | null>(null);

  const requestLogsRef = useRef<RequestLogSummary[]>([]);
  const requestLogsInFlightRef = useRef(false);

  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const [selectedLog, setSelectedLog] = useState<RequestLogDetail | null>(null);
  const [selectedLogLoading, setSelectedLogLoading] = useState(false);
  const [attemptLogs, setAttemptLogs] = useState<RequestAttemptLog[]>([]);
  const [attemptLogsLoading, setAttemptLogsLoading] = useState(false);

  useEffect(() => {
    requestLogsRef.current = requestLogs;
  }, [requestLogs]);

  const statusPredicate = useMemo(() => buildStatusPredicate(statusFilter), [statusFilter]);
  const statusFilterValid = statusFilter.trim().length === 0 || statusPredicate != null;

  const filteredLogs = useMemo(() => {
    const errorNeedle = errorCodeFilter.trim().toLowerCase();
    const pathNeedle = pathFilter.trim().toLowerCase();

    return requestLogs.filter((log) => {
      if (cliKey !== "all" && log.cli_key !== cliKey) return false;
      if (statusPredicate && !statusPredicate(log.status)) return false;
      if (errorNeedle) {
        const raw = (log.error_code ?? "").toLowerCase();
        if (!raw.includes(errorNeedle)) return false;
      }
      if (pathNeedle) {
        const haystack = `${log.method} ${log.path}`.toLowerCase();
        if (!haystack.includes(pathNeedle)) return false;
      }
      return true;
    });
  }, [cliKey, errorCodeFilter, pathFilter, requestLogs, statusPredicate]);

  async function refreshRequestLogs(mode: "blocking" | "background" = "blocking") {
    if (!hasTauriRuntime()) {
      setRequestLogsAvailable(false);
      setRequestLogs([]);
      return;
    }

    if (requestLogsInFlightRef.current) return;
    requestLogsInFlightRef.current = true;
    if (mode === "blocking") setRequestLogsLoading(true);
    if (mode === "background") setRequestLogsRefreshing(true);

    try {
      const items = await requestLogsListAll(LOGS_PAGE_LIMIT);
      if (!items) {
        setRequestLogsAvailable(false);
        setRequestLogs([]);
        return;
      }
      setRequestLogsAvailable(true);
      const next = (items ?? []).slice().sort(sortRequestLogsDesc);
      setRequestLogs(next);
    } catch (err) {
      setRequestLogsAvailable(true);
      logToConsole("error", "读取日志失败", { error: String(err) });
      toast("读取日志失败：请查看控制台日志");
    } finally {
      requestLogsInFlightRef.current = false;
      if (mode === "blocking") setRequestLogsLoading(false);
      if (mode === "background") setRequestLogsRefreshing(false);
    }
  }

  async function refreshRequestLogsIncremental() {
    if (!hasTauriRuntime()) return;

    const prev = requestLogsRef.current;
    if (prev.length === 0) {
      await refreshRequestLogs("background");
      return;
    }

    if (requestLogsInFlightRef.current) return;
    requestLogsInFlightRef.current = true;
    setRequestLogsRefreshing(true);

    try {
      const afterId = computeRequestLogsCursorId(prev);
      const items = await requestLogsListAfterIdAll(afterId, LOGS_PAGE_LIMIT);
      if (!items) {
        setRequestLogsAvailable(false);
        setRequestLogs([]);
        return;
      }
      setRequestLogsAvailable(true);
      const incoming = items ?? [];
      if (incoming.length === 0) return;
      setRequestLogs((cur) => mergeRequestLogs(cur, incoming, LOGS_PAGE_LIMIT));
    } catch (err) {
      logToConsole("warn", "增量刷新日志失败", { error: String(err) });
    } finally {
      requestLogsInFlightRef.current = false;
      setRequestLogsRefreshing(false);
    }
  }

  useEffect(() => {
    setSelectedLogId(null);
    setSelectedLog(null);
    requestLogsInFlightRef.current = false;
    setRequestLogsLoading(false);
    setRequestLogsRefreshing(false);
    setRequestLogs([]);
    void refreshRequestLogs("blocking");
  }, []);

  useEffect(() => {
    if (!autoRefresh) return;
    if (!hasTauriRuntime()) return;
    const timer = window.setInterval(
      () => void refreshRequestLogsIncremental(),
      AUTO_REFRESH_INTERVAL_MS
    );
    return () => window.clearInterval(timer);
  }, [autoRefresh]);

  useEffect(() => {
    if (selectedLogId == null) {
      setSelectedLog(null);
      setSelectedLogLoading(false);
      setAttemptLogs([]);
      setAttemptLogsLoading(false);
      return;
    }

    let cancelled = false;
    setSelectedLogLoading(true);
    requestLogGet(selectedLogId)
      .then((detail) => {
        if (cancelled) return;
        if (!detail) {
          setSelectedLog(null);
          return;
        }
        setSelectedLog(detail);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取日志详情失败", { log_id: selectedLogId, error: String(err) });
        toast(`读取详情失败：${String(err)}`);
      })
      .finally(() => {
        if (cancelled) return;
        setSelectedLogLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedLogId]);

  useEffect(() => {
    if (!selectedLog) {
      setAttemptLogs([]);
      setAttemptLogsLoading(false);
      return;
    }

    let cancelled = false;
    setAttemptLogsLoading(true);

    requestAttemptLogsByTraceId(selectedLog.trace_id, 50)
      .then((items) => {
        if (cancelled) return;
        setAttemptLogs(items ?? []);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取 attempt logs 失败", {
          trace_id: selectedLog.trace_id,
          error: String(err),
        });
        setAttemptLogs([]);
      })
      .finally(() => {
        if (cancelled) return;
        setAttemptLogsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedLog]);

  return (
    <div className="flex flex-col gap-6 pb-10">
      <PageHeader
        title="日志"
        actions={
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-slate-600">
              <span>自动刷新</span>
              <Switch
                checked={autoRefresh}
                onCheckedChange={setAutoRefresh}
                size="sm"
                disabled={requestLogsAvailable === false}
              />
            </div>
          </div>
        }
      />

      <Card padding="md" className="flex flex-col gap-4">
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="text-sm font-semibold">筛选</div>
            <div className="text-xs text-slate-500">
              {filteredLogs.length} / {requestLogs.length}
            </div>
          </div>

          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-3">
              <div className="text-xs font-medium text-slate-600 w-16">CLI</div>
              <TabList
                ariaLabel="CLI 过滤"
                items={CLI_FILTER_ITEMS}
                value={cliKey}
                onChange={setCliKey}
                size="sm"
                buttonClassName="px-3 py-1.5"
              />
            </div>

            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-slate-600">Status</div>
                <Input
                  value={statusFilter}
                  onChange={(e) => setStatusFilter(e.target.value)}
                  placeholder="例：499 / 524 / !200 / >=400"
                  mono
                  disabled={requestLogsAvailable === false}
                />
                {!statusFilterValid && (
                  <div className="text-[11px] leading-4 text-rose-600">
                    表达式不合法：支持 499 / !200 / &gt;=400 / &lt;=399
                  </div>
                )}
              </div>
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-slate-600">error_code</div>
                <Input
                  value={errorCodeFilter}
                  onChange={(e) => setErrorCodeFilter(e.target.value)}
                  placeholder="例：GW_UPSTREAM_TIMEOUT"
                  mono
                  disabled={requestLogsAvailable === false}
                />
              </div>
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-slate-600">Path</div>
                <Input
                  value={pathFilter}
                  onChange={(e) => setPathFilter(e.target.value)}
                  placeholder="例：/v1/messages"
                  mono
                  disabled={requestLogsAvailable === false}
                />
              </div>
            </div>
          </div>
        </div>
      </Card>

      <HomeRequestLogsPanel
        showCustomTooltip={showCustomTooltip}
        title="日志列表"
        showOpenLogsPageButton={false}
        traces={[]}
        requestLogs={filteredLogs}
        requestLogsLoading={requestLogsLoading}
        requestLogsRefreshing={requestLogsRefreshing}
        requestLogsAvailable={requestLogsAvailable}
        onRefreshRequestLogs={() => void refreshRequestLogs("blocking")}
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
      />

      <RequestLogDetailDialog
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
        selectedLog={selectedLog}
        selectedLogLoading={selectedLogLoading}
        attemptLogs={attemptLogs}
        attemptLogsLoading={attemptLogsLoading}
      />
    </div>
  );
}
