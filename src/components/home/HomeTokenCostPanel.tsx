import { useCallback, useMemo, useReducer, useRef, useState, useSyncExternalStore } from "react";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Check,
  ChevronDown,
  CircleHelp,
  Download,
  FolderOpen,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { usageLeaderboardCsvExport } from "../../services/usage/usage";
import type {
  UsageFolderOptionV1,
  UsageLeaderboardRow,
  UsagePeriod,
  UsageSummary,
} from "../../services/usage/usage";
import { useCustomDateRange, type CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useUsageFolderOptionsV1Query } from "../../query/usage";
import { saveDesktopFilePath } from "../../services/desktop/dialog";
import {
  HOME_USAGE_DAY_START_HOUR_OPTIONS,
  HOME_USAGE_DEFAULT_DAY_START_HOUR,
  addLocalDays,
  dayStartHourLabel,
  formatUsageDayHourMinuteFromMs,
  localDateHour,
  normalizeHomeUsageDayStartHour,
  readHomeUsageDayStartHourFromStorage,
  startOfLocalUsageDay,
  subscribeHomeUsageDayStartHour,
  writeHomeUsageDayStartHourToStorage,
} from "../../services/home/homeUsageDayBoundary";
import {
  HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
  HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
  HOME_USAGE_FULL_IDLE_GAP_MINUTES_OPTIONS,
  HOME_USAGE_SESSION_BREAK_GAP_MINUTES_OPTIONS,
  readHomeUsageFullIdleGapMinutesFromStorage,
  readHomeUsageSessionBreakGapMinutesFromStorage,
  subscribeHomeUsageDevelopmentTimeThresholds,
  writeHomeUsageFullIdleGapMinutesToStorage,
  writeHomeUsageSessionBreakGapMinutesToStorage,
} from "../../services/home/homeUsageDevelopmentTime";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Popover } from "../../ui/Popover";
import { Select } from "../../ui/Select";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { TabList, type TabListItem } from "../../ui/TabList";
import { Tooltip } from "../../ui/Tooltip";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import { cn } from "../../utils/cn";
import { formatUnknownError } from "../../utils/errors";
import {
  formatCompactDurationMs,
  formatInteger,
  formatPercent,
  formatUsdCompact,
} from "../../utils/formatters";
import { StatCard, StatCardSkeleton } from "../usage/StatCard";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import { PREVIEW_TOKEN_FOLDER_OPTIONS } from "./previewTokenData";
import { useHomeTokenCostDataModel } from "./useHomeTokenCostDataModel";
import {
  developmentTimeEstimateTooltip,
  FOLDER_DEVELOPMENT_TIME_NOTE,
  FULL_IDLE_GAP_TOOLTIP,
  SESSION_BREAK_GAP_TOOLTIP,
} from "./developmentTimeEstimate";

type TokenCostScope = "provider" | "model" | "folder" | "day";
type TokenCostRange =
  | "today"
  | "yesterday"
  | "last3"
  | "last7"
  | "last15"
  | "last30"
  | "month"
  | "custom";

const TOKEN_COST_SCOPE_ITEMS = [
  { key: "provider", label: "供应商" },
  { key: "model", label: "模型" },
  { key: "folder", label: "文件夹" },
  { key: "day", label: "日期" },
] satisfies Array<TabListItem<TokenCostScope>>;

const TOKEN_COST_RANGE_ITEMS = [
  { key: "today", label: "今天" },
  { key: "yesterday", label: "昨天" },
  { key: "last3", label: "最近 3 天" },
  { key: "last7", label: "最近 7 天" },
  { key: "last15", label: "最近 15 天" },
  { key: "last30", label: "最近 30 天" },
  { key: "month", label: "当月" },
] as const satisfies ReadonlyArray<{ key: Exclude<TokenCostRange, "custom">; label: string }>;

const TABLE_TH_CLASS =
  "border-b border-border bg-secondary/70 dark:bg-secondary/70 px-3 py-2.5 text-left text-xs font-medium uppercase tracking-wide text-muted-foreground";
const TABLE_TD_CLASS = "border-b border-border px-3 py-3";
const TABLE_MONO_TD_CLASS =
  "border-b border-border px-3 py-3 font-mono text-xs tabular-nums text-secondary-foreground";

const SUMMARY_SKELETON_KEYS = [0, 1, 2, 3, 4, 5, 6];
const EMPTY_LEADERBOARD_ROWS: UsageLeaderboardRow[] = [];

type TokenCostQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: null;
  providerId: null;
  folderKeys?: string[] | null;
  dayStartHour?: number | null;
  fullIdleGapMinutes?: number | null;
  sessionBreakGapMinutes?: number | null;
  excludeCx2CcGatewayBridge?: boolean | null;
};

type TokenCostQueryConfig = {
  label: string;
  period: UsagePeriod;
  input: TokenCostQueryInput;
  previewFactor: number;
};

type UsageRequestMetricRow = Pick<UsageLeaderboardRow, "requests_total" | "requests_success">;
type UsageTokenMetricRow = Pick<
  UsageLeaderboardRow,
  | "total_tokens"
  | "io_total_tokens"
  | "input_tokens"
  | "output_tokens"
  | "cache_creation_input_tokens"
  | "cache_read_input_tokens"
>;
type SortDirection = "asc" | "desc";
type SortState<T extends string> = { key: T; direction: SortDirection };
type LeaderboardSortKey =
  | "name"
  | "totalTokens"
  | "ioTokens"
  | "cost"
  | "totalDuration"
  | "requests"
  | "activityStart"
  | "activityEnd"
  | "estimatedDevelopmentTime";
type IndexedLeaderboardRow = { row: UsageLeaderboardRow; originalIndex: number };

function scopeLabel(scope: TokenCostScope) {
  if (scope === "provider") return "供应商";
  if (scope === "model") return "模型";
  if (scope === "folder") return "文件夹";
  return "日期";
}

function rangeLabel(range: TokenCostRange) {
  if (range === "custom") return "自定义";
  return TOKEN_COST_RANGE_ITEMS.find((item) => item.key === range)?.label ?? "今天";
}

function formatTokenValue(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  return formatTokensMillions(value);
}

function formatCostValue(value: number | null | undefined) {
  return formatUsdCompact(value);
}

function successRate(row: UsageRequestMetricRow) {
  if (row.requests_total <= 0) return NaN;
  return row.requests_success / row.requests_total;
}

function tokenShare(row: UsageLeaderboardRow, summary: UsageSummary | null) {
  if (!summary || summary.io_total_tokens <= 0) return 0;
  return row.io_total_tokens / summary.io_total_tokens;
}

function nextSortState<T extends string>(current: SortState<T> | null, key: T): SortState<T> {
  if (current?.key === key) {
    return {
      key,
      direction: current.direction === "desc" ? "asc" : "desc",
    };
  }
  return { key, direction: "desc" };
}

function compareTextValue(
  left: string | null | undefined,
  right: string | null | undefined,
  direction: SortDirection
) {
  const leftText = left?.trim() ?? "";
  const rightText = right?.trim() ?? "";
  if (!leftText && !rightText) return 0;
  if (!leftText) return 1;
  if (!rightText) return -1;
  const comparison = leftText.localeCompare(rightText, "zh-CN");
  return direction === "asc" ? comparison : -comparison;
}

function compareNumberValue(
  left: number | null | undefined,
  right: number | null | undefined,
  direction: SortDirection
) {
  const leftValid = left != null && Number.isFinite(left);
  const rightValid = right != null && Number.isFinite(right);
  if (!leftValid && !rightValid) return 0;
  if (!leftValid) return 1;
  if (!rightValid) return -1;
  const leftNumber = Number(left);
  const rightNumber = Number(right);
  return direction === "asc" ? leftNumber - rightNumber : rightNumber - leftNumber;
}

function stableSort<T>(
  items: T[],
  compare: (left: T, right: T) => number,
  originalIndex: (item: T) => number
) {
  return [...items].sort(
    (left, right) => compare(left, right) || originalIndex(left) - originalIndex(right)
  );
}

function unixSecondsFromDate(date: Date) {
  return Math.floor(date.getTime() / 1000);
}

function emptyTokenCostQueryInput(): TokenCostQueryInput {
  return {
    startTs: null,
    endTs: null,
    cliKey: null,
    providerId: null,
  };
}

function customPreviewFactor(customApplied: CustomDateRangeApplied | null) {
  if (!customApplied) return 1;
  const seconds = customApplied.endTs - customApplied.startTs;
  if (!Number.isFinite(seconds) || seconds <= 0) return 1;
  return Math.max(1, Math.ceil(seconds / 86_400));
}

function buildTokenCostQueryConfig(
  range: TokenCostRange,
  customApplied: CustomDateRangeApplied | null,
  dayStartHour = HOME_USAGE_DEFAULT_DAY_START_HOUR,
  now = new Date()
): TokenCostQueryConfig {
  const normalizedDayStartHour = normalizeHomeUsageDayStartHour(dayStartHour);
  const todayStart = startOfLocalUsageDay(now, normalizedDayStartHour);
  const tomorrowStart = addLocalDays(todayStart, 1);
  const customStart = customApplied
    ? localDateHour(customApplied.startDate, normalizedDayStartHour)
    : null;
  const customEnd = customApplied
    ? localDateHour(customApplied.endDate, normalizedDayStartHour, 1)
    : null;

  switch (range) {
    case "custom":
      return {
        label: customApplied
          ? `${customApplied.startDate} 至 ${customApplied.endDate}`
          : rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: customStart ? unixSecondsFromDate(customStart) : null,
          endTs: customEnd ? unixSecondsFromDate(customEnd) : null,
          dayStartHour: normalizedDayStartHour,
        },
        previewFactor: customPreviewFactor(customApplied),
      };
    case "yesterday":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -1)),
          endTs: unixSecondsFromDate(todayStart),
          dayStartHour: normalizedDayStartHour,
        },
        previewFactor: 1,
      };
    case "last3":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -2)),
          endTs: unixSecondsFromDate(tomorrowStart),
          dayStartHour: normalizedDayStartHour,
        },
        previewFactor: 3,
      };
    case "last7":
      return {
        label: rangeLabel(range),
        period: "weekly",
        input: { ...emptyTokenCostQueryInput(), dayStartHour: normalizedDayStartHour },
        previewFactor: 7,
      };
    case "last15":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -14)),
          endTs: unixSecondsFromDate(tomorrowStart),
          dayStartHour: normalizedDayStartHour,
        },
        previewFactor: 15,
      };
    case "last30":
      return {
        label: rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: unixSecondsFromDate(addLocalDays(todayStart, -29)),
          endTs: unixSecondsFromDate(tomorrowStart),
          dayStartHour: normalizedDayStartHour,
        },
        previewFactor: 30,
      };
    case "month":
      return {
        label: rangeLabel(range),
        period: "monthly",
        input: { ...emptyTokenCostQueryInput(), dayStartHour: normalizedDayStartHour },
        previewFactor: Math.max(1, now.getDate()),
      };
    case "today":
    default:
      return {
        label: rangeLabel("today"),
        period: "daily",
        input: { ...emptyTokenCostQueryInput(), dayStartHour: normalizedDayStartHour },
        previewFactor: 1,
      };
  }
}

function summaryCacheHitRate(summary: UsageSummary | null) {
  if (!summary) return null;
  return computeCacheHitRate(
    summary.input_tokens,
    summary.cache_creation_input_tokens,
    summary.cache_read_input_tokens
  );
}

function trimCompactZero(value: string) {
  return value.replace(/\.0([KM])$/, "$1").replace(/\.0%$/, "%");
}

function activityTimeOffsetMs(
  row: UsageLeaderboardRow,
  value: number | null | undefined,
  dayStartHour: number
) {
  if (value == null || !Number.isFinite(value)) return null;
  const dayStart = localDateHour(row.key, dayStartHour);
  if (!dayStart) return null;
  return value - dayStart.getTime();
}

function sortLeaderboardRows(
  rows: UsageLeaderboardRow[],
  sortState: SortState<LeaderboardSortKey> | null,
  dayStartHour: number
): IndexedLeaderboardRow[] {
  const indexedRows = rows.map((row, originalIndex) => ({ row, originalIndex }));
  if (!sortState) return indexedRows;

  return stableSort(
    indexedRows,
    (left, right) => {
      switch (sortState.key) {
        case "name":
          return compareTextValue(left.row.name, right.row.name, sortState.direction);
        case "totalTokens":
          return compareNumberValue(
            left.row.total_tokens,
            right.row.total_tokens,
            sortState.direction
          );
        case "ioTokens":
          return compareNumberValue(
            left.row.io_total_tokens,
            right.row.io_total_tokens,
            sortState.direction
          );
        case "cost":
          return compareNumberValue(left.row.cost_usd, right.row.cost_usd, sortState.direction);
        case "totalDuration":
          return compareNumberValue(
            left.row.total_duration_ms,
            right.row.total_duration_ms,
            sortState.direction
          );
        case "requests":
          return compareNumberValue(
            left.row.requests_total,
            right.row.requests_total,
            sortState.direction
          );
        case "estimatedDevelopmentTime":
          return compareNumberValue(
            left.row.estimated_development_time_ms,
            right.row.estimated_development_time_ms,
            sortState.direction
          );
        case "activityStart":
          return compareNumberValue(
            activityTimeOffsetMs(left.row, left.row.first_request_created_at_ms, dayStartHour),
            activityTimeOffsetMs(right.row, right.row.first_request_created_at_ms, dayStartHour),
            sortState.direction
          );
        case "activityEnd":
          return compareNumberValue(
            activityTimeOffsetMs(left.row, left.row.last_request_completed_at_ms, dayStartHour),
            activityTimeOffsetMs(right.row, right.row.last_request_completed_at_ms, dayStartHour),
            sortState.direction
          );
      }
    },
    (item) => item.originalIndex
  );
}

function TableHeaderLabel({ label, note }: { label: string; note?: string }) {
  return (
    <div className="inline-flex items-baseline gap-1 whitespace-nowrap normal-case">
      <span>{label}</span>
      {note ? (
        <span className="text-[10px] font-normal tracking-normal text-muted-foreground">
          （{note}）
        </span>
      ) : null}
    </div>
  );
}

function SortableColumnHeader<T extends string>({
  label,
  note,
  tooltip,
  sortKey,
  sortState,
  onSort,
}: {
  label: string;
  note?: string;
  tooltip?: string;
  sortKey: T;
  sortState: SortState<T> | null;
  onSort: (key: T) => void;
}) {
  const activeDirection = sortState?.key === sortKey ? sortState.direction : null;
  const active = activeDirection != null;
  const ariaSort = activeDirection
    ? activeDirection === "asc"
      ? "ascending"
      : "descending"
    : "none";
  const SortIcon = activeDirection
    ? activeDirection === "asc"
      ? ArrowUp
      : ArrowDown
    : ArrowUpDown;

  const button = (
    <button
      type="button"
      onClick={() => onSort(sortKey)}
      className={cn(
        "-mx-1 inline-flex items-center gap-1 rounded px-1 py-0.5 text-left transition hover:text-foreground focus:outline-none focus:ring-2 focus:ring-accent/30 dark:hover:text-foreground",
        active && "text-sky-700 dark:text-sky-300"
      )}
    >
      <TableHeaderLabel label={label} note={note} />
      {tooltip ? <CircleHelp aria-hidden="true" className="h-3.5 w-3.5 shrink-0" /> : null}
      <SortIcon
        aria-hidden="true"
        className={cn(
          "h-3.5 w-3.5 shrink-0",
          active ? "text-sky-600 dark:text-sky-300" : "text-muted-foreground"
        )}
      />
    </button>
  );

  return (
    <th scope="col" className={TABLE_TH_CLASS} aria-sort={ariaSort}>
      {tooltip ? (
        <Tooltip content={tooltip} contentClassName="max-w-[320px] normal-case leading-5">
          {button}
        </Tooltip>
      ) : (
        button
      )}
    </th>
  );
}

function ActivityRangeColumnHeader({
  sortState,
  onSort,
}: {
  sortState: SortState<LeaderboardSortKey> | null;
  onSort: (key: LeaderboardSortKey) => void;
}) {
  const sortControl = (key: "activityStart" | "activityEnd", label: string) => {
    const activeDirection = sortState?.key === key ? sortState.direction : null;
    const SortIcon = activeDirection
      ? activeDirection === "asc"
        ? ArrowUp
        : ArrowDown
      : ArrowUpDown;
    return (
      <button
        type="button"
        aria-label={label}
        title={label}
        onClick={() => onSort(key)}
        className={cn(
          "rounded p-0.5 transition hover:text-foreground focus:outline-none focus:ring-2 focus:ring-accent/30 dark:hover:text-foreground",
          activeDirection && "text-sky-700 dark:text-sky-300"
        )}
      >
        <SortIcon aria-hidden="true" className="h-3.5 w-3.5" />
      </button>
    );
  };

  return (
    <th scope="col" className={TABLE_TH_CLASS}>
      <div className="inline-flex items-center gap-1 whitespace-nowrap normal-case">
        {sortControl("activityStart", "按活动开始时间排序")}
        <span>活动范围</span>
        {sortControl("activityEnd", "按活动结束时间排序")}
      </div>
    </th>
  );
}

function TokenBreakdownInline({ parts }: { parts: string[] }) {
  return (
    <span aria-label={parts.join("/")} className="inline-flex items-baseline gap-0.5 tabular-nums">
      {parts.map((part, index) => (
        <span key={`${part}-${index}`} className="inline-flex items-baseline gap-0.5">
          {index > 0 ? (
            <span className="text-muted-foreground" aria-hidden="true">
              /
            </span>
          ) : null}
          <span>{part}</span>
        </span>
      ))}
    </span>
  );
}

function inputOutputTokenText(row: Pick<UsageTokenMetricRow, "io_total_tokens">) {
  return trimCompactZero(formatTokensMillions(row.io_total_tokens));
}

function cacheHitRateText(row: UsageTokenMetricRow) {
  const totalWithCache = row.total_tokens;
  const hasValidTotal = Number.isFinite(totalWithCache) && totalWithCache > 0;
  const hitRate = computeCacheHitRate(
    row.input_tokens,
    row.cache_creation_input_tokens,
    row.cache_read_input_tokens
  );
  return hasValidTotal && Number.isFinite(hitRate) ? trimCompactZero(formatPercent(hitRate)) : "—";
}

function requestCountText(row: UsageRequestMetricRow) {
  return formatInteger(row.requests_total);
}

function successRateText(row: UsageRequestMetricRow) {
  return trimCompactZero(formatPercent(successRate(row)));
}

function tokenShareText(percent: number) {
  const pct = Number.isFinite(percent) ? Math.max(0, Math.min(1, percent)) : 0;
  return trimCompactZero(formatPercent(pct));
}

function totalTokenText(row: Pick<UsageTokenMetricRow, "total_tokens">) {
  return trimCompactZero(formatTokensMillions(row.total_tokens));
}

function InputOutputCacheValue({ row }: { row: UsageTokenMetricRow }) {
  return <TokenBreakdownInline parts={[inputOutputTokenText(row), cacheHitRateText(row)]} />;
}

function RequestSuccessRateValue({ row }: { row: UsageRequestMetricRow }) {
  return <TokenBreakdownInline parts={[requestCountText(row), successRateText(row)]} />;
}

function activityRangeText(row: UsageLeaderboardRow, dayStartHour: number) {
  const first = row.first_request_created_at_ms;
  const last = row.last_request_completed_at_ms;
  if (first == null || last == null || !Number.isFinite(first) || !Number.isFinite(last)) {
    return "—";
  }
  const firstText = formatUsageDayHourMinuteFromMs(first, row.key, dayStartHour);
  let lastText = formatUsageDayHourMinuteFromMs(last, row.key, dayStartHour);
  if (!firstText || !lastText) {
    return "—";
  }
  const firstDate = new Date(first);
  const lastDate = new Date(last);
  if (
    !lastText.startsWith("次日") &&
    (firstDate.getFullYear() !== lastDate.getFullYear() ||
      firstDate.getMonth() !== lastDate.getMonth() ||
      firstDate.getDate() !== lastDate.getDate())
  ) {
    lastText = `次日${lastText}`;
  }
  return `${firstText}–${lastText}`;
}

function TotalTokenShareValue({
  row,
  summary,
}: {
  row: UsageLeaderboardRow;
  summary: UsageSummary | null;
}) {
  return (
    <TokenBreakdownInline parts={[totalTokenText(row), tokenShareText(tokenShare(row, summary))]} />
  );
}

function csvCell(value: string | number | null | undefined) {
  const text = value == null ? "" : String(value);
  const normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  if (!/[",\n]/.test(normalized)) return normalized;
  return `"${normalized.replace(/"/g, '""')}"`;
}

function buildCsvContent(headers: string[], rows: string[][]) {
  const lines = [headers, ...rows].map((row) => row.map(csvCell).join(","));
  return `\uFEFF${lines.join("\r\n")}\r\n`;
}

function timestampForCsvFileName(now = new Date()) {
  const pad2 = (value: number) => String(value).padStart(2, "0");
  return [
    now.getFullYear(),
    pad2(now.getMonth() + 1),
    pad2(now.getDate()),
    "-",
    pad2(now.getHours()),
    pad2(now.getMinutes()),
    pad2(now.getSeconds()),
  ].join("");
}

function homeUsageCsvDefaultFileName(scope: TokenCostScope, now = new Date()) {
  return `aio-coding-hub-home-usage-${scope}-${timestampForCsvFileName(now)}.csv`;
}

function buildHomeUsageLeaderboardCsv(
  scope: TokenCostScope,
  sortedRows: IndexedLeaderboardRow[],
  summary: UsageSummary | null,
  dayStartHour: number
) {
  if (scope === "day") {
    const headers = [
      "排名",
      "日期",
      "总 Token/占比",
      "输入+出/缓存率",
      "请求数/成功率",
      "请求总耗时",
      "活动范围",
      "预估开发时间",
      "总花费",
    ];
    const rows = sortedRows.map(({ row }, index) => [
      String(index + 1),
      row.name,
      `${totalTokenText(row)}/${tokenShareText(tokenShare(row, summary))}`,
      `${inputOutputTokenText(row)}/${cacheHitRateText(row)}`,
      `${requestCountText(row)}/${successRateText(row)}`,
      formatCompactDurationMs(row.total_duration_ms),
      activityRangeText(row, dayStartHour),
      formatCompactDurationMs(row.estimated_development_time_ms),
      formatCostValue(row.cost_usd),
    ]);
    return buildCsvContent(headers, rows);
  }

  if (scope === "folder") {
    const headers = [
      "排名",
      "文件夹名称",
      "完整路径",
      "总 Token/占比",
      "输入+出/缓存率",
      "请求数/成功率",
      "请求总耗时",
      "预估开发时间",
      "总花费",
    ];
    const rows = sortedRows.map(({ row }, index) => [
      String(index + 1),
      row.name,
      row.folder_path ?? "",
      `${totalTokenText(row)}/${tokenShareText(tokenShare(row, summary))}`,
      `${inputOutputTokenText(row)}/${cacheHitRateText(row)}`,
      `${requestCountText(row)}/${successRateText(row)}`,
      formatCompactDurationMs(row.total_duration_ms),
      formatCompactDurationMs(row.estimated_development_time_ms),
      formatCostValue(row.cost_usd),
    ]);
    return buildCsvContent(headers, rows);
  }

  const headers = [
    "排名",
    scopeLabel(scope),
    "总 Token/占比",
    "输入+出/缓存率",
    "请求数/成功率",
    "请求总耗时",
    "总花费",
  ];
  const rows = sortedRows.map(({ row }, index) => [
    String(index + 1),
    row.name,
    `${totalTokenText(row)}/${tokenShareText(tokenShare(row, summary))}`,
    `${inputOutputTokenText(row)}/${cacheHitRateText(row)}`,
    `${requestCountText(row)}/${successRateText(row)}`,
    formatCompactDurationMs(row.total_duration_ms),
    formatCostValue(row.cost_usd),
  ]);
  return buildCsvContent(headers, rows);
}

function TokenSummaryCards({
  summary,
  rows,
  totalCostUsd,
  scope,
  loading,
}: {
  summary: UsageSummary | null;
  rows: UsageLeaderboardRow[];
  totalCostUsd: number | null;
  scope: TokenCostScope;
  loading: boolean;
}) {
  if (loading && !summary) {
    return (
      <div className="grid shrink-0 grid-cols-2 gap-3 lg:grid-cols-4 xl:grid-cols-7">
        {SUMMARY_SKELETON_KEYS.map((key) => (
          <StatCardSkeleton key={key} />
        ))}
      </div>
    );
  }

  return (
    <div className="grid shrink-0 grid-cols-2 gap-3 lg:grid-cols-4 xl:grid-cols-7">
      <StatCard
        title="含缓存总 Token"
        value={formatTokenValue(summary?.total_tokens)}
        accent="purple"
      />
      <StatCard
        title="输入+输出 Token"
        value={formatTokenValue(summary?.io_total_tokens)}
        accent="blue"
      />
      <StatCard title="总花费" value={formatCostValue(totalCostUsd)} accent="orange" />
      <StatCard
        title="请求总耗时"
        value={formatCompactDurationMs(summary?.total_duration_ms)}
        accent="cyan"
      />
      <StatCard title="成功请求" value={formatInteger(summary?.requests_success)} accent="green" />
      <StatCard
        title="缓存命中率"
        value={formatPercent(summaryCacheHitRate(summary))}
        accent="purple"
      />
      <StatCard title={`${scopeLabel(scope)}数`} value={formatInteger(rows.length)} accent="rose" />
    </div>
  );
}

function TokenLeaderboardTable({
  scope,
  rows,
  sortedRows,
  summary,
  loading,
  customPending,
  dayStartHour,
  developmentTimeTooltip,
  sortState,
  onSort,
}: {
  scope: TokenCostScope;
  rows: UsageLeaderboardRow[];
  sortedRows: IndexedLeaderboardRow[];
  summary: UsageSummary | null;
  loading: boolean;
  customPending: boolean;
  dayStartHour: number;
  developmentTimeTooltip: string;
  sortState: SortState<LeaderboardSortKey> | null;
  onSort: (key: LeaderboardSortKey) => void;
}) {
  if (loading && rows.length === 0) {
    return (
      <div className="flex items-center justify-center gap-3 px-6 py-14 text-sm text-muted-foreground">
        <Spinner />
        <span>加载用量中…</span>
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="px-6 py-14 text-center text-sm text-muted-foreground">
        {customPending ? "请选择开始日期和结束日期后点击“自定义”。" : "当前时间范围暂无用量数据。"}
      </div>
    );
  }

  const dayScope = scope === "day";
  const folderScope = scope === "folder";
  const developmentTimeScope = dayScope || folderScope;

  return (
    <div className="min-h-0 flex-1 overflow-auto scrollbar-overlay">
      <table
        className={cn(
          "w-full border-separate border-spacing-0 text-left text-sm",
          dayScope ? "min-w-[980px]" : developmentTimeScope ? "min-w-[880px]" : "min-w-[760px]"
        )}
      >
        <caption className="sr-only">用量排行榜</caption>
        <thead className="sticky top-0 z-10">
          <tr>
            <th scope="col" className={TABLE_TH_CLASS}>
              排名
            </th>
            <SortableColumnHeader
              label={scopeLabel(scope)}
              sortKey="name"
              sortState={sortState}
              onSort={onSort}
            />
            <SortableColumnHeader
              label="总 Token/占比"
              sortKey="totalTokens"
              sortState={sortState}
              onSort={onSort}
            />
            <SortableColumnHeader
              label="输入+出/缓存率"
              sortKey="ioTokens"
              sortState={sortState}
              onSort={onSort}
            />
            <SortableColumnHeader
              label="请求数/成功率"
              sortKey="requests"
              sortState={sortState}
              onSort={onSort}
            />
            <SortableColumnHeader
              label="请求总耗时"
              sortKey="totalDuration"
              sortState={sortState}
              onSort={onSort}
            />
            {dayScope ? <ActivityRangeColumnHeader sortState={sortState} onSort={onSort} /> : null}
            {developmentTimeScope ? (
              <SortableColumnHeader
                label="预估开发时间"
                tooltip={
                  folderScope
                    ? `${developmentTimeTooltip}${FOLDER_DEVELOPMENT_TIME_NOTE}`
                    : developmentTimeTooltip
                }
                sortKey="estimatedDevelopmentTime"
                sortState={sortState}
                onSort={onSort}
              />
            ) : null}
            <SortableColumnHeader
              label="总花费"
              sortKey="cost"
              sortState={sortState}
              onSort={onSort}
            />
          </tr>
        </thead>
        <tbody>
          {sortedRows.map(({ row }, index) => {
            const emptyDay = dayScope && row.requests_total === 0;
            return (
              <tr
                key={row.key}
                className="align-top transition-colors hover:bg-secondary/60 dark:hover:bg-secondary/50"
              >
                <td className={`${TABLE_TD_CLASS} text-xs tabular-nums text-muted-foreground`}>
                  {index + 1}
                </td>
                <td className={TABLE_TD_CLASS}>
                  <div className="min-w-[130px] font-medium text-foreground">{row.name}</div>
                  {folderScope ? (
                    <div
                      className="mt-0.5 max-w-[280px] truncate font-mono text-[10px] text-muted-foreground"
                      title={row.folder_path ?? undefined}
                    >
                      {row.folder_path ?? "—"}
                    </div>
                  ) : null}
                </td>
                <td className={TABLE_MONO_TD_CLASS}>
                  {emptyDay ? (
                    <TokenBreakdownInline parts={["—", "—"]} />
                  ) : (
                    <TotalTokenShareValue row={row} summary={summary} />
                  )}
                </td>
                <td className={TABLE_MONO_TD_CLASS}>
                  {emptyDay ? (
                    <TokenBreakdownInline parts={["—", "—"]} />
                  ) : (
                    <InputOutputCacheValue row={row} />
                  )}
                </td>
                <td className={TABLE_MONO_TD_CLASS}>
                  {emptyDay ? (
                    <TokenBreakdownInline parts={["—", "—"]} />
                  ) : (
                    <RequestSuccessRateValue row={row} />
                  )}
                </td>
                <td className={TABLE_MONO_TD_CLASS}>
                  {emptyDay ? "—" : formatCompactDurationMs(row.total_duration_ms)}
                </td>
                {dayScope ? (
                  <td className={TABLE_MONO_TD_CLASS}>{activityRangeText(row, dayStartHour)}</td>
                ) : null}
                {developmentTimeScope ? (
                  <td className={TABLE_MONO_TD_CLASS}>
                    {emptyDay ? "—" : formatCompactDurationMs(row.estimated_development_time_ms)}
                  </td>
                ) : null}
                <td className={TABLE_MONO_TD_CLASS}>
                  {emptyDay ? "—" : formatCostValue(row.cost_usd)}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function CustomRangeForm({
  customStartDate,
  customEndDate,
  onCustomStartDateChange,
  onCustomEndDateChange,
  onApplyCustomRange,
  active,
}: {
  customStartDate: string;
  customEndDate: string;
  onCustomStartDateChange: (value: string) => void;
  onCustomEndDateChange: (value: string) => void;
  onApplyCustomRange: () => void;
  active: boolean;
}) {
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <input
        type="date"
        value={customStartDate}
        onChange={(event) => onCustomStartDateChange(event.currentTarget.value)}
        aria-label="开始日期"
        className="h-8 rounded-md border border-border bg-white px-2 text-xs text-foreground outline-none transition focus:border-accent focus:ring-2 focus:ring-accent/20 dark:border-border dark:bg-secondary dark:text-foreground"
      />
      <span className="text-xs text-muted-foreground">→</span>
      <input
        type="date"
        value={customEndDate}
        onChange={(event) => onCustomEndDateChange(event.currentTarget.value)}
        aria-label="结束日期"
        className="h-8 rounded-md border border-border bg-white px-2 text-xs text-foreground outline-none transition focus:border-accent focus:ring-2 focus:ring-accent/20 dark:border-border dark:bg-secondary dark:text-foreground"
      />
      <Button
        size="sm"
        variant={active ? "primary" : "secondary"}
        aria-pressed={active}
        onClick={onApplyCustomRange}
        className="whitespace-nowrap"
      >
        自定义
      </Button>
    </div>
  );
}

function FolderMultiSelect({
  options,
  selectedKeys,
  loading,
  disabled,
  onToggleKey,
  onClear,
}: {
  options: UsageFolderOptionV1[];
  selectedKeys: string[];
  loading: boolean;
  disabled: boolean;
  onToggleKey: (key: string) => void;
  onClear: () => void;
}) {
  const selectedSet = useMemo(() => new Set(selectedKeys), [selectedKeys]);
  const optionsByKey = useMemo(
    () => new Map(options.map((option) => [option.key, option])),
    [options]
  );
  const displayOptions = useMemo(() => {
    const missingSelected: UsageFolderOptionV1[] = [];
    for (const key of selectedKeys) {
      if (optionsByKey.has(key)) continue;
      missingSelected.push({
        key,
        name: key,
        folder_path: null,
        requests_total: 0,
        total_tokens: 0,
      });
    }
    return [...options, ...missingSelected];
  }, [options, optionsByKey, selectedKeys]);
  const selectedLabel =
    selectedKeys.length === 0
      ? "全部文件夹"
      : selectedKeys.length === 1
        ? (optionsByKey.get(selectedKeys[0])?.name ?? selectedKeys[0])
        : `${selectedKeys.length} 个文件夹`;

  const trigger = (
    <span
      className={cn(
        "inline-flex h-8 items-center gap-1.5 rounded-lg border border-border bg-card px-2.5 text-xs font-medium text-foreground transition hover:bg-secondary",
        disabled && "cursor-not-allowed opacity-50"
      )}
    >
      <FolderOpen className="h-3.5 w-3.5 text-muted-foreground" />
      <span className="max-w-[150px] truncate">{selectedLabel}</span>
      {loading ? <Spinner size="sm" /> : <ChevronDown className="h-3.5 w-3.5" />}
    </span>
  );

  if (disabled) {
    return (
      <Button size="sm" variant="secondary" disabled className="whitespace-nowrap">
        <FolderOpen className="h-3.5 w-3.5" />
        全部文件夹
      </Button>
    );
  }

  return (
    <Popover
      align="end"
      trigger={trigger}
      contentClassName="w-80 p-0"
      className="whitespace-nowrap"
    >
      <div className="border-b border-border px-3 py-2 dark:border-border">
        <div className="flex items-center justify-between gap-2">
          <div className="text-sm font-semibold text-foreground">文件夹</div>
          <Button
            size="sm"
            variant="ghost"
            onClick={onClear}
            disabled={selectedKeys.length === 0}
            aria-label="清空文件夹筛选"
            className="h-7 px-2"
          >
            <X className="h-3.5 w-3.5" />
            清空
          </Button>
        </div>
      </div>
      <div className="max-h-72 overflow-y-auto py-1">
        {loading && displayOptions.length === 0 ? (
          <div className="flex items-center justify-center gap-2 px-3 py-6 text-sm text-muted-foreground">
            <Spinner size="sm" />
            <span>加载文件夹中…</span>
          </div>
        ) : null}
        {!loading && displayOptions.length === 0 ? (
          <div className="px-3 py-6 text-center text-sm text-muted-foreground">
            当前范围暂无文件夹。
          </div>
        ) : null}
        {displayOptions.map((option) => {
          const selected = selectedSet.has(option.key);
          return (
            <button
              key={option.key}
              type="button"
              role="checkbox"
              aria-checked={selected}
              onClick={() => onToggleKey(option.key)}
              className="flex w-full items-start gap-2 px-3 py-2 text-left transition hover:bg-secondary dark:hover:bg-secondary"
            >
              <span
                className={cn(
                  "mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded border border-border dark:border-border",
                  selected && "border-sky-500 bg-sky-500 text-white"
                )}
              >
                {selected ? <Check className="h-3 w-3" /> : null}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-foreground">
                  {option.name}
                </span>
                <span className="mt-0.5 block truncate font-mono text-[10px] text-muted-foreground">
                  {option.folder_path ?? "未知文件夹"} · {formatTokenValue(option.total_tokens)}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </Popover>
  );
}

type HomeTokenCostPanelProps = {
  devPreviewEnabled?: boolean;
};

type HomeTokenCostPanelState = {
  scope: TokenCostScope;
  range: TokenCostRange;
  selectedFolderKeys: string[];
  excludeCx2CcGatewayBridge: boolean;
};

type HomeTokenCostPanelAction =
  | { type: "setScope"; scope: TokenCostScope }
  | { type: "setRange"; range: TokenCostRange }
  | { type: "toggleFolderKey"; key: string }
  | { type: "clearFolderKeys" }
  | { type: "setExcludeCx2CcGatewayBridge"; exclude: boolean };

function createInitialHomeTokenCostPanelState(): HomeTokenCostPanelState {
  return {
    scope: "day",
    range: "last7",
    selectedFolderKeys: [],
    excludeCx2CcGatewayBridge: true,
  };
}

function homeTokenCostPanelReducer(
  state: HomeTokenCostPanelState,
  action: HomeTokenCostPanelAction
): HomeTokenCostPanelState {
  switch (action.type) {
    case "setScope":
      return { ...state, scope: action.scope };
    case "setRange":
      return { ...state, range: action.range };
    case "toggleFolderKey":
      return {
        ...state,
        selectedFolderKeys: state.selectedFolderKeys.includes(action.key)
          ? state.selectedFolderKeys.filter((item) => item !== action.key)
          : [...state.selectedFolderKeys, action.key],
      };
    case "clearFolderKeys":
      return { ...state, selectedFolderKeys: [] };
    case "setExcludeCx2CcGatewayBridge":
      return { ...state, excludeCx2CcGatewayBridge: action.exclude };
  }
}

export function HomeTokenCostPanel({ devPreviewEnabled = false }: HomeTokenCostPanelProps) {
  const initialState = useMemo(createInitialHomeTokenCostPanelState, []);
  const [state, dispatch] = useReducer(homeTokenCostPanelReducer, initialState);
  const [leaderboardSortState, setLeaderboardSortState] =
    useState<SortState<LeaderboardSortKey> | null>(null);
  const [exportingCsv, setExportingCsv] = useState(false);
  const exportingCsvRef = useRef(false);
  const { scope, range, selectedFolderKeys, excludeCx2CcGatewayBridge } = state;
  const dayStartHour = useSyncExternalStore(
    subscribeHomeUsageDayStartHour,
    readHomeUsageDayStartHourFromStorage,
    () => HOME_USAGE_DEFAULT_DAY_START_HOUR
  );
  const fullIdleGapMinutes = useSyncExternalStore(
    subscribeHomeUsageDevelopmentTimeThresholds,
    readHomeUsageFullIdleGapMinutesFromStorage,
    () => HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES
  );
  const sessionBreakGapMinutes = useSyncExternalStore(
    subscribeHomeUsageDevelopmentTimeThresholds,
    readHomeUsageSessionBreakGapMinutesFromStorage,
    () => HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES
  );
  const onInvalidCustomRange = useCallback((message: string) => toast(message), []);
  const customDateRangeOptions = useMemo(
    () => ({ onInvalid: onInvalidCustomRange }),
    [onInvalidCustomRange]
  );
  const {
    customStartDate,
    setCustomStartDate,
    customEndDate,
    setCustomEndDate,
    customApplied,
    applyCustomRange,
  } = useCustomDateRange(range, customDateRangeOptions);

  const queryConfig = useMemo(
    () => buildTokenCostQueryConfig(range, customApplied, dayStartHour),
    [customApplied, dayStartHour, range]
  );
  const customPending = range === "custom" && !customApplied;
  const selectedFolderKeysForQuery = selectedFolderKeys.length > 0 ? selectedFolderKeys : null;
  const filteredQueryConfig = useMemo(
    () => ({
      ...queryConfig,
      input: {
        ...queryConfig.input,
        folderKeys: selectedFolderKeysForQuery,
        fullIdleGapMinutes,
        sessionBreakGapMinutes,
        excludeCx2CcGatewayBridge,
      },
    }),
    [
      excludeCx2CcGatewayBridge,
      fullIdleGapMinutes,
      queryConfig,
      selectedFolderKeysForQuery,
      sessionBreakGapMinutes,
    ]
  );
  const queryRefreshConfig = useMemo(
    () =>
      customPending
        ? {
            summary: { enabled: false },
            leaderboard: { enabled: false },
          }
        : undefined,
    [customPending]
  );

  const model = useHomeTokenCostDataModel({
    scope,
    queryConfig: filteredQueryConfig,
    devPreviewEnabled,
    queryRefreshConfig,
  });
  const folderOptionsInput = useMemo(
    () => ({
      ...queryConfig.input,
      excludeCx2CcGatewayBridge,
    }),
    [excludeCx2CcGatewayBridge, queryConfig.input]
  );
  const folderOptionsQuery = useUsageFolderOptionsV1Query(queryConfig.period, folderOptionsInput, {
    enabled: !customPending,
  });
  const folderOptions =
    model.previewActive && !customPending
      ? PREVIEW_TOKEN_FOLDER_OPTIONS
      : (folderOptionsQuery.data ?? []);
  const folderOptionsLoading =
    !model.previewActive &&
    !customPending &&
    (folderOptionsQuery.isLoading || folderOptionsQuery.isFetching);
  const folderSelectDisabled =
    customPending ||
    (!folderOptionsLoading && folderOptions.length === 0 && selectedFolderKeys.length === 0);
  const displaySummary = customPending ? null : model.summary;
  const displayRows = customPending ? EMPTY_LEADERBOARD_ROWS : model.rows;
  const displayTotalCostUsd = customPending ? null : model.totalCostUsd;
  const displayLoading = customPending ? false : model.loading;
  const sortedDisplayRows = useMemo(
    () => sortLeaderboardRows(displayRows, leaderboardSortState, dayStartHour),
    [dayStartHour, displayRows, leaderboardSortState]
  );
  const exportCsvDisabled =
    customPending || displayLoading || sortedDisplayRows.length === 0 || exportingCsv;
  const handleToggleFolderKey = useCallback((key: string) => {
    dispatch({ type: "toggleFolderKey", key });
  }, []);
  const handleClearFolderKeys = useCallback(() => {
    dispatch({ type: "clearFolderKeys" });
  }, []);
  const handleDayStartHourChange = useCallback((dayStartHour: number) => {
    writeHomeUsageDayStartHourToStorage(dayStartHour);
  }, []);
  const handleFullIdleGapMinutesChange = useCallback((minutes: number) => {
    writeHomeUsageFullIdleGapMinutesToStorage(minutes);
  }, []);
  const handleSessionBreakGapMinutesChange = useCallback((minutes: number) => {
    writeHomeUsageSessionBreakGapMinutesToStorage(minutes);
  }, []);
  const developmentTimeTooltip = developmentTimeEstimateTooltip(
    fullIdleGapMinutes,
    sessionBreakGapMinutes
  );
  const handleApplyCustomRange = useCallback(() => {
    if (applyCustomRange()) {
      dispatch({ type: "setRange", range: "custom" });
    }
  }, [applyCustomRange]);
  const handleLeaderboardSort = useCallback((key: LeaderboardSortKey) => {
    setLeaderboardSortState((current) => nextSortState(current, key));
  }, []);
  const handleScopeChange = useCallback((nextScope: TokenCostScope) => {
    dispatch({ type: "setScope", scope: nextScope });
    setLeaderboardSortState(
      nextScope === "folder" ? { key: "totalTokens", direction: "desc" } : null
    );
  }, []);
  const handleExportCsv = useCallback(async () => {
    if (
      exportingCsvRef.current ||
      customPending ||
      displayLoading ||
      sortedDisplayRows.length === 0
    ) {
      return;
    }

    exportingCsvRef.current = true;
    setExportingCsv(true);

    try {
      const filePath = await saveDesktopFilePath({
        title: "导出用量排行 CSV",
        defaultPath: homeUsageCsvDefaultFileName(scope),
        filters: [{ name: "CSV", extensions: ["csv"] }],
        canCreateDirectories: true,
      });
      if (!filePath) {
        return;
      }

      const csv = buildHomeUsageLeaderboardCsv(
        scope,
        sortedDisplayRows,
        displaySummary,
        dayStartHour
      );
      await usageLeaderboardCsvExport(filePath, csv);
      toast("用量排行 CSV 已导出");
    } catch (error) {
      toast(`导出 CSV 失败：${formatUnknownError(error)}`);
    } finally {
      exportingCsvRef.current = false;
      setExportingCsv(false);
    }
  }, [customPending, dayStartHour, displayLoading, displaySummary, scope, sortedDisplayRows]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-5 overflow-y-auto lg:overflow-hidden">
      <div className="flex shrink-0 flex-col gap-3 2xl:flex-row 2xl:items-start 2xl:justify-between">
        <fieldset className="flex min-w-0 flex-col gap-2 border-0 p-0">
          <legend className="sr-only">用量筛选</legend>
          <div
            role="group"
            aria-label="用量筛选设置"
            className="flex flex-wrap items-center gap-1.5"
          >
            <FolderMultiSelect
              options={folderOptions}
              selectedKeys={selectedFolderKeys}
              loading={folderOptionsLoading}
              disabled={folderSelectDisabled}
              onToggleKey={handleToggleFolderKey}
              onClear={handleClearFolderKeys}
            />
            <div className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-white px-2.5 text-xs text-muted-foreground shadow-sm dark:border-border dark:bg-card dark:text-secondary-foreground">
              <span className="whitespace-nowrap">转接去重</span>
              <Switch
                checked={excludeCx2CcGatewayBridge}
                onCheckedChange={(exclude) =>
                  dispatch({ type: "setExcludeCx2CcGatewayBridge", exclude })
                }
                size="sm"
                aria-label="过滤转接重复用量"
              />
            </div>
            <label className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-white px-2.5 text-xs text-muted-foreground shadow-sm dark:border-border dark:bg-card dark:text-secondary-foreground">
              <span className="whitespace-nowrap">统计日开始</span>
              <Select
                aria-label="统计日开始"
                value={String(dayStartHour)}
                onChange={(event) => handleDayStartHourChange(Number(event.currentTarget.value))}
                className="h-6 w-auto rounded border-0 bg-transparent px-1 py-0 text-xs shadow-none focus:bg-transparent focus:ring-0 focus:ring-offset-0"
              >
                {HOME_USAGE_DAY_START_HOUR_OPTIONS.map((hour) => (
                  <option key={hour} value={hour}>
                    {dayStartHourLabel(hour)}
                  </option>
                ))}
              </Select>
            </label>
            <label className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-white px-2.5 text-xs text-muted-foreground shadow-sm dark:border-border dark:bg-card dark:text-secondary-foreground">
              <span className="whitespace-nowrap">完整计入</span>
              <Tooltip content={FULL_IDLE_GAP_TOOLTIP} contentClassName="max-w-[320px] leading-5">
                <span
                  aria-label="完整计入说明"
                  className="inline-flex cursor-help items-center text-muted-foreground"
                >
                  <CircleHelp aria-hidden="true" className="h-3.5 w-3.5" />
                </span>
              </Tooltip>
              <Select
                aria-label="完整计入时间"
                value={String(fullIdleGapMinutes)}
                onChange={(event) =>
                  handleFullIdleGapMinutesChange(Number(event.currentTarget.value))
                }
                className="h-6 w-auto rounded border-0 bg-transparent px-1 py-0 text-xs shadow-none focus:bg-transparent focus:ring-0 focus:ring-offset-0"
              >
                {HOME_USAGE_FULL_IDLE_GAP_MINUTES_OPTIONS.map((minutes) => (
                  <option key={minutes} value={minutes}>
                    {minutes} 分钟
                  </option>
                ))}
              </Select>
            </label>
            <label className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-white px-2.5 text-xs text-muted-foreground shadow-sm dark:border-border dark:bg-card dark:text-secondary-foreground">
              <span className="whitespace-nowrap">停止计入</span>
              <Tooltip
                content={SESSION_BREAK_GAP_TOOLTIP}
                contentClassName="max-w-[320px] leading-5"
              >
                <span
                  aria-label="停止计入说明"
                  className="inline-flex cursor-help items-center text-muted-foreground"
                >
                  <CircleHelp aria-hidden="true" className="h-3.5 w-3.5" />
                </span>
              </Tooltip>
              <Select
                aria-label="停止计入时间"
                value={String(sessionBreakGapMinutes)}
                onChange={(event) =>
                  handleSessionBreakGapMinutesChange(Number(event.currentTarget.value))
                }
                className="h-6 w-auto rounded border-0 bg-transparent px-1 py-0 text-xs shadow-none focus:bg-transparent focus:ring-0 focus:ring-offset-0"
              >
                {HOME_USAGE_SESSION_BREAK_GAP_MINUTES_OPTIONS.map((minutes) => (
                  <option key={minutes} value={minutes}>
                    {minutes} 分钟
                  </option>
                ))}
              </Select>
            </label>
          </div>
          <div
            role="group"
            aria-label="用量时间范围"
            className="flex flex-wrap items-center gap-1.5"
          >
            {TOKEN_COST_RANGE_ITEMS.map((item) => {
              const active = range === item.key;
              return (
                <Button
                  key={item.key}
                  size="sm"
                  variant={active ? "primary" : "secondary"}
                  aria-pressed={active}
                  onClick={() => dispatch({ type: "setRange", range: item.key })}
                  className="whitespace-nowrap"
                >
                  {item.label}
                </Button>
              );
            })}
            <CustomRangeForm
              customStartDate={customStartDate}
              customEndDate={customEndDate}
              onCustomStartDateChange={setCustomStartDate}
              onCustomEndDateChange={setCustomEndDate}
              onApplyCustomRange={handleApplyCustomRange}
              active={range === "custom" && Boolean(customApplied)}
            />
          </div>
        </fieldset>
        <div className="flex flex-wrap items-center gap-3 2xl:justify-end">
          <TabList
            ariaLabel="用量维度切换"
            items={TOKEN_COST_SCOPE_ITEMS}
            value={scope}
            onChange={handleScopeChange}
            size="sm"
          />
        </div>
      </div>

      <TokenSummaryCards
        summary={displaySummary}
        rows={displayRows}
        totalCostUsd={displayTotalCostUsd}
        scope={scope}
        loading={displayLoading}
      />

      <QueryErrorCard
        errorText={customPending ? null : model.errorText}
        loading={customPending ? false : model.fetching}
        onRetry={model.refresh}
      />

      <Card
        padding="none"
        className="flex min-h-[280px] shrink-0 flex-col overflow-hidden lg:min-h-0 lg:flex-1"
      >
        <div className="shrink-0 border-b border-border px-6 pb-4 pt-5 dark:border-border">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="text-base font-semibold text-foreground">{scopeLabel(scope)}排行</div>
            <Button
              size="sm"
              variant="secondary"
              disabled={exportCsvDisabled}
              onClick={() => void handleExportCsv()}
              className="whitespace-nowrap"
            >
              {exportingCsv ? (
                <Spinner size="sm" />
              ) : (
                <Download aria-hidden="true" className="h-3.5 w-3.5" />
              )}
              导出 CSV
            </Button>
          </div>
        </div>
        <TokenLeaderboardTable
          scope={scope}
          rows={displayRows}
          sortedRows={sortedDisplayRows}
          summary={displaySummary}
          loading={displayLoading}
          customPending={customPending}
          dayStartHour={dayStartHour}
          developmentTimeTooltip={developmentTimeTooltip}
          sortState={leaderboardSortState}
          onSort={handleLeaderboardSort}
        />
      </Card>
    </div>
  );
}
