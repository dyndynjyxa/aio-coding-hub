// Usage:
// - Entry: Home "代理记录" button -> `/#/logs`.
// - Backend commands: `request_logs_list_all`, `request_logs_list_after_id_all`, `request_log_get`, `request_attempt_logs_by_trace_id`.

import { useMemo, useState } from "react";
import { HomeRequestLogsPanel } from "../components/home/HomeRequestLogsPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { CLI_FILTER_ITEMS, type CliFilterKey } from "../constants/clis";
import { GatewayErrorCodes } from "../constants/gatewayErrorCodes";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import {
  useRequestLogsIncrementalPollQuery,
  useRequestLogsListAllQuery,
} from "../query/requestLogs";
import { useWindowForeground } from "../hooks/useWindowForeground";
import { useTraceStore } from "../services/gateway/traceStore";

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

export function LogsPage() {
  const { traces } = useTraceStore();
  const showCustomTooltip = true;
  const foregroundActive = useDocumentVisibility();

  const [cliKey, setCliKey] = useState<CliFilterKey>("all");
  const [statusFilter, setStatusFilter] = useState("");
  const [errorCodeFilter, setErrorCodeFilter] = useState("");
  const [pathFilter, setPathFilter] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);

  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const incrementalPollingEnabled = autoRefresh && foregroundActive;
  const requestLogsQuery = useRequestLogsListAllQuery(LOGS_PAGE_LIMIT);
  const incrementalPollQuery = useRequestLogsIncrementalPollQuery(LOGS_PAGE_LIMIT, {
    enabled: incrementalPollingEnabled,
    refetchIntervalMs: incrementalPollingEnabled ? AUTO_REFRESH_INTERVAL_MS : false,
  });

  useWindowForeground({
    enabled: true,
    throttleMs: 1000,
    onForeground: () => {
      if (autoRefresh) {
        void incrementalPollQuery.refetch();
      }
    },
  });

  const requestLogs = useMemo(() => requestLogsQuery.data ?? [], [requestLogsQuery.data]);
  const requestLogsLoading = requestLogsQuery.isLoading;
  const requestLogsRefreshing =
    (requestLogsQuery.isFetching && !requestLogsQuery.isLoading) || incrementalPollQuery.isFetching;
  const requestLogsAvailable: boolean | null = requestLogsQuery.isLoading
    ? null
    : requestLogsQuery.data != null;

  const statusPredicate = useMemo(() => buildStatusPredicate(statusFilter), [statusFilter]);
  const statusFilterValid = statusFilter.trim().length === 0 || statusPredicate != null;
  const activeFilterCount = [
    cliKey !== "all",
    statusFilter.trim().length > 0,
    errorCodeFilter.trim().length > 0,
    pathFilter.trim().length > 0,
  ].filter(Boolean).length;

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
  const logsSummaryText =
    requestLogsAvailable === false
      ? undefined
      : requestLogs.length === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsRefreshing
          ? `更新中… · 共 ${filteredLogs.length} / ${requestLogs.length} 条`
          : `共 ${filteredLogs.length} / ${requestLogs.length} 条`;

  function resetFilters() {
    setCliKey("all");
    setStatusFilter("");
    setErrorCodeFilter("");
    setPathFilter("");
  }

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader title="代理记录" />

      <Card padding="md" className="overflow-visible flex flex-col gap-6 pb-7">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">筛选条件</div>

          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
              <span>自动刷新</span>
              <Switch
                checked={autoRefresh}
                onCheckedChange={setAutoRefresh}
                size="sm"
                disabled={requestLogsAvailable === false}
              />
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={resetFilters}
              disabled={activeFilterCount === 0}
            >
              清空筛选
            </Button>
          </div>
        </div>

        <div className="grid items-start gap-5 md:grid-cols-2 xl:grid-cols-[1.35fr_1fr_1fr_1fr]">
          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
              CLI
            </div>
            <TabList
              ariaLabel="CLI 过滤"
              items={CLI_FILTER_ITEMS}
              value={cliKey}
              onChange={setCliKey}
              size="sm"
              className="w-full"
              buttonClassName="shrink-0 px-3 py-1.5 whitespace-nowrap"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
              Status
            </div>
            <Input
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value)}
              placeholder="例：499 / 524 / !200 / >=400"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-slate-500 dark:text-slate-400">
              支持 `499`、`!200`、`&gt;=400`、`&lt;=399`
            </div>
            {!statusFilterValid ? (
              <div className="text-[11px] leading-4 text-rose-600 dark:text-rose-400">
                表达式不合法：支持 499 / !200 / &gt;=400 / &lt;=399
              </div>
            ) : null}
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
              error_code
            </div>
            <Input
              value={errorCodeFilter}
              onChange={(e) => setErrorCodeFilter(e.target.value)}
              placeholder={`例：${GatewayErrorCodes.UPSTREAM_TIMEOUT}`}
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-slate-500 dark:text-slate-400">
              支持按错误码关键字模糊匹配
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
              Path
            </div>
            <Input
              value={pathFilter}
              onChange={(e) => setPathFilter(e.target.value)}
              placeholder="例：/v1/messages"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-slate-500 dark:text-slate-400">
              按请求路径或方法路径组合模糊匹配
            </div>
          </div>
        </div>
      </Card>

      <HomeRequestLogsPanel
        showCustomTooltip={showCustomTooltip}
        title="代理记录列表"
        summaryTextOverride={logsSummaryText}
        showOpenLogsPageButton={false}
        showCompactModeToggle={false}
        compactModeOverride={false}
        emptyStateTitle={activeFilterCount > 0 ? "没有符合筛选条件的代理记录" : "当前没有代理记录"}
        traces={traces}
        requestLogs={filteredLogs}
        requestLogsLoading={requestLogsLoading}
        requestLogsRefreshing={requestLogsRefreshing}
        requestLogsAvailable={requestLogsAvailable}
        onRefreshRequestLogs={() => void requestLogsQuery.refetch()}
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
      />

      <RequestLogDetailDialog selectedLogId={selectedLogId} onSelectLogId={setSelectedLogId} />
    </div>
  );
}
