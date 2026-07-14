// Usage:
// - Entry: Home "代理记录" button -> `/#/logs`.
// - Backend commands: `request_logs_list_all`, `request_logs_list_after_id_all`, `request_log_get`, `request_attempt_logs_by_trace_id`.

import { useMemo, useReducer } from "react";
import { HomeRequestLogsPanel } from "../components/home/HomeRequestLogsPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { cliFilterItemsWith, type CliFilterKey } from "../constants/clis";
import { GatewayErrorCodes } from "../constants/gatewayErrorCodes";
import { useRequestLogsFeed } from "../hooks/useRequestLogsFeed";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import { useTraceStore } from "../services/gateway/traceStore";

const LOGS_PAGE_LIMIT = 200;
const AUTO_REFRESH_INTERVAL_MS = 2000;
const LOG_CLI_FILTER_ITEMS = cliFilterItemsWith("logs");

type StatusPredicate = (status: number | null) => boolean;

type LogsPageState = {
  cliKey: CliFilterKey;
  statusFilter: string;
  errorCodeFilter: string;
  pathFilter: string;
  autoRefresh: boolean;
  selectedLogId: number | null;
};

type LogsPageAction =
  | { type: "setCliKey"; cliKey: CliFilterKey }
  | { type: "setStatusFilter"; statusFilter: string }
  | { type: "setErrorCodeFilter"; errorCodeFilter: string }
  | { type: "setPathFilter"; pathFilter: string }
  | { type: "setAutoRefresh"; autoRefresh: boolean }
  | { type: "setSelectedLogId"; selectedLogId: number | null }
  | { type: "resetFilters" };

const initialLogsPageState: LogsPageState = {
  cliKey: "all",
  statusFilter: "",
  errorCodeFilter: "",
  pathFilter: "",
  autoRefresh: true,
  selectedLogId: null,
};

function logsPageReducer(state: LogsPageState, action: LogsPageAction): LogsPageState {
  switch (action.type) {
    case "setCliKey":
      return { ...state, cliKey: action.cliKey };
    case "setStatusFilter":
      return { ...state, statusFilter: action.statusFilter };
    case "setErrorCodeFilter":
      return { ...state, errorCodeFilter: action.errorCodeFilter };
    case "setPathFilter":
      return { ...state, pathFilter: action.pathFilter };
    case "setAutoRefresh":
      return { ...state, autoRefresh: action.autoRefresh };
    case "setSelectedLogId":
      return { ...state, selectedLogId: action.selectedLogId };
    case "resetFilters":
      return {
        ...state,
        cliKey: "all",
        statusFilter: "",
        errorCodeFilter: "",
        pathFilter: "",
      };
  }
}

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

  const [state, dispatch] = useReducer(logsPageReducer, initialLogsPageState);
  const { cliKey, statusFilter, errorCodeFilter, pathFilter, autoRefresh, selectedLogId } = state;
  const setSelectedLogId = (selectedLogId: number | null) =>
    dispatch({ type: "setSelectedLogId", selectedLogId });
  const {
    requestLogs,
    activeRequests,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsAvailable,
    refreshRequestLogs,
  } = useRequestLogsFeed({
    limit: LOGS_PAGE_LIMIT,
    liveUpdatesEnabled: autoRefresh,
    liveUpdateIntervalMs: AUTO_REFRESH_INTERVAL_MS,
    refreshOnForeground: autoRefresh,
  });

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
  const filteredActiveRequests = useMemo(() => {
    const errorNeedle = errorCodeFilter.trim().toLowerCase();
    const pathNeedle = pathFilter.trim().toLowerCase();

    return activeRequests.filter((request) => {
      if (cliKey !== "all" && request.cli_key !== cliKey) return false;
      if (statusPredicate) return false;
      if (errorNeedle) return false;
      if (pathNeedle) {
        const haystack = `${request.method} ${request.path}`.toLowerCase();
        if (!haystack.includes(pathNeedle)) return false;
      }
      return true;
    });
  }, [activeRequests, cliKey, errorCodeFilter, pathFilter, statusPredicate]);
  const logsSummaryText =
    requestLogsAvailable === false
      ? undefined
      : requestLogs.length === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsRefreshing
          ? `更新中… · 共 ${filteredLogs.length} / ${requestLogs.length} 条`
          : `共 ${filteredLogs.length} / ${requestLogs.length} 条`;

  function resetFilters() {
    dispatch({ type: "resetFilters" });
  }

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader title="代理记录" />

      <Card padding="sm" className="overflow-visible flex flex-col gap-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="text-sm font-semibold text-foreground">筛选条件</div>

          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>自动刷新</span>
              <Switch
                checked={autoRefresh}
                onCheckedChange={(autoRefresh) => dispatch({ type: "setAutoRefresh", autoRefresh })}
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

        <div className="grid items-start gap-4 md:grid-cols-2 xl:grid-cols-[1.35fr_1fr_1fr_1fr]">
          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              CLI
            </div>
            <TabList
              ariaLabel="CLI 过滤"
              items={LOG_CLI_FILTER_ITEMS}
              value={cliKey}
              onChange={(cliKey) => dispatch({ type: "setCliKey", cliKey })}
              size="sm"
              className="w-full"
              buttonClassName="shrink-0 px-3 py-1.5 whitespace-nowrap"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Status
            </div>
            <Input
              value={statusFilter}
              onChange={(e) => dispatch({ type: "setStatusFilter", statusFilter: e.target.value })}
              placeholder="例：499 / 524 / !200 / >=400"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              支持 `499`、`!200`、`&gt;=400`、`&lt;=399`
            </div>
            {!statusFilterValid ? (
              <div className="text-[11px] leading-4 text-rose-600 dark:text-rose-400">
                表达式不合法：支持 499 / !200 / &gt;=400 / &lt;=399
              </div>
            ) : null}
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              error_code
            </div>
            <Input
              value={errorCodeFilter}
              onChange={(e) =>
                dispatch({ type: "setErrorCodeFilter", errorCodeFilter: e.target.value })
              }
              placeholder={`例：${GatewayErrorCodes.UPSTREAM_TIMEOUT}`}
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              支持按错误码关键字模糊匹配
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Path
            </div>
            <Input
              value={pathFilter}
              onChange={(e) => dispatch({ type: "setPathFilter", pathFilter: e.target.value })}
              placeholder="例：/v1/messages"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              按请求路径或方法路径组合模糊匹配
            </div>
          </div>
        </div>
      </Card>

      <HomeRequestLogsPanel
        displayOptions={{
          customTooltip: showCustomTooltip,
          openLogsPageButton: false,
          compactModeToggle: false,
        }}
        title="代理记录列表"
        summaryTextOverride={logsSummaryText}
        compactModeOverride={false}
        emptyStateTitle={activeFilterCount > 0 ? "没有符合筛选条件的代理记录" : "当前没有代理记录"}
        traces={traces}
        activeRequests={filteredActiveRequests}
        requestLogs={filteredLogs}
        requestLogsLoading={requestLogsLoading}
        requestLogsRefreshing={requestLogsRefreshing}
        requestLogsAvailable={requestLogsAvailable}
        onRefreshRequestLogs={() => void refreshRequestLogs()}
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
      />

      <RequestLogDetailDialog selectedLogId={selectedLogId} onSelectLogId={setSelectedLogId} />
    </div>
  );
}
