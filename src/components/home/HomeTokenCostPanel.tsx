import { Fragment, useCallback, useMemo, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Check,
  ChevronDown,
  ChevronRight,
  FolderOpen,
  X,
} from "lucide-react";
import { toast } from "sonner";
import type {
  UsageDayDetailV1,
  UsageDayFolderRow,
  UsageDayHourRow,
  UsageFolderOptionV1,
  UsageLeaderboardRow,
  UsagePeriod,
  UsageSummary,
} from "../../services/usage/usage";
import { useCustomDateRange, type CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useUsageDayDetailV1Query, useUsageFolderOptionsV1Query } from "../../query/usage";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Popover } from "../../ui/Popover";
import { Spinner } from "../../ui/Spinner";
import { Switch } from "../../ui/Switch";
import { TabList, type TabListItem } from "../../ui/TabList";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import { cn } from "../../utils/cn";
import { formatUnknownError } from "../../utils/errors";
import {
  formatInteger,
  formatPercent,
  formatTokensPerSecond,
  formatUsdCompact,
} from "../../utils/formatters";
import { StatCard, StatCardSkeleton } from "../usage/StatCard";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import { buildPreviewTokenDayDetail, PREVIEW_TOKEN_FOLDER_OPTIONS } from "./previewTokenData";
import { useHomeTokenCostDataModel } from "./useHomeTokenCostDataModel";

type TokenCostScope = "provider" | "model" | "day";
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
  | "cacheTokens"
  | "cost"
  | "requests"
  | "successRate"
  | "tokenShare"
  | "outputSpeed";
type DayFolderSortKey = "folder" | "totalTokens" | "ioTokens" | "cacheTokens" | "cost";
type IndexedLeaderboardRow = { row: UsageLeaderboardRow; originalIndex: number };

function scopeLabel(scope: TokenCostScope) {
  if (scope === "provider") return "供应商";
  if (scope === "model") return "模型";
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

function cacheTokens(row: UsageTokenMetricRow) {
  return row.cache_creation_input_tokens + row.cache_read_input_tokens;
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

function startOfLocalDay(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
}

function addLocalDays(date: Date, days: number) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days, 0, 0, 0, 0);
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
  now = new Date()
): TokenCostQueryConfig {
  const todayStart = startOfLocalDay(now);
  const tomorrowStart = addLocalDays(todayStart, 1);

  switch (range) {
    case "custom":
      return {
        label: customApplied
          ? `${customApplied.startDate} 至 ${customApplied.endDate}`
          : rangeLabel(range),
        period: "custom",
        input: {
          ...emptyTokenCostQueryInput(),
          startTs: customApplied?.startTs ?? null,
          endTs: customApplied?.endTs ?? null,
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
        },
        previewFactor: 3,
      };
    case "last7":
      return {
        label: rangeLabel(range),
        period: "weekly",
        input: emptyTokenCostQueryInput(),
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
        },
        previewFactor: 30,
      };
    case "month":
      return {
        label: rangeLabel(range),
        period: "monthly",
        input: emptyTokenCostQueryInput(),
        previewFactor: Math.max(1, now.getDate()),
      };
    case "today":
    default:
      return {
        label: rangeLabel("today"),
        period: "daily",
        input: emptyTokenCostQueryInput(),
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

function summaryCostCoverage(summary: UsageSummary | null) {
  if (!summary) return null;
  const denom = summary.requests_success;
  if (!Number.isFinite(denom) || denom <= 0) return null;
  const covered = summary.cost_covered_success;
  if (!Number.isFinite(covered) || covered < 0) return null;
  return covered / denom;
}

function trimCompactZero(value: string) {
  return value.replace(/\.0([KM])$/, "$1").replace(/\.0%$/, "%");
}

function sortLeaderboardRows(
  rows: UsageLeaderboardRow[],
  sortState: SortState<LeaderboardSortKey> | null,
  summary: UsageSummary | null
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
        case "cacheTokens":
          return compareNumberValue(
            cacheTokens(left.row),
            cacheTokens(right.row),
            sortState.direction
          );
        case "cost":
          return compareNumberValue(left.row.cost_usd, right.row.cost_usd, sortState.direction);
        case "requests":
          return compareNumberValue(
            left.row.requests_total,
            right.row.requests_total,
            sortState.direction
          );
        case "successRate":
          return compareNumberValue(
            successRate(left.row),
            successRate(right.row),
            sortState.direction
          );
        case "tokenShare":
          return compareNumberValue(
            tokenShare(left.row, summary),
            tokenShare(right.row, summary),
            sortState.direction
          );
        case "outputSpeed":
          return compareNumberValue(
            left.row.avg_output_tokens_per_second,
            right.row.avg_output_tokens_per_second,
            sortState.direction
          );
      }
    },
    (item) => item.originalIndex
  );
}

function sortDayFolderRows(
  folders: UsageDayFolderRow[],
  sortState: SortState<DayFolderSortKey> | null
) {
  const indexedFolders = folders.map((folder, originalIndex) => ({ folder, originalIndex }));
  const sorted = sortState
    ? stableSort(
        indexedFolders,
        (left, right) => {
          switch (sortState.key) {
            case "folder":
              return compareTextValue(left.folder.name, right.folder.name, sortState.direction);
            case "totalTokens":
              return compareNumberValue(
                left.folder.total_tokens,
                right.folder.total_tokens,
                sortState.direction
              );
            case "ioTokens":
              return compareNumberValue(
                left.folder.io_total_tokens,
                right.folder.io_total_tokens,
                sortState.direction
              );
            case "cacheTokens":
              return compareNumberValue(
                cacheTokens(left.folder),
                cacheTokens(right.folder),
                sortState.direction
              );
            case "cost":
              return compareNumberValue(
                left.folder.cost_usd,
                right.folder.cost_usd,
                sortState.direction
              );
          }
        },
        (item) => item.originalIndex
      )
    : indexedFolders;
  return sorted.map((item) => item.folder);
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
  sortKey,
  sortState,
  onSort,
}: {
  label: string;
  note?: string;
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

  return (
    <th scope="col" className={TABLE_TH_CLASS} aria-sort={ariaSort}>
      <button
        type="button"
        onClick={() => onSort(sortKey)}
        className={cn(
          "-mx-1 inline-flex items-center gap-1 rounded px-1 py-0.5 text-left transition hover:text-foreground focus:outline-none focus:ring-2 focus:ring-accent/30 dark:hover:text-foreground",
          active && "text-sky-700 dark:text-sky-300"
        )}
      >
        <TableHeaderLabel label={label} note={note} />
        <SortIcon
          aria-hidden="true"
          className={cn(
            "h-3.5 w-3.5 shrink-0",
            active ? "text-sky-600 dark:text-sky-300" : "text-muted-foreground"
          )}
        />
      </button>
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

function InputOutputTokenValue({ row }: { row: Pick<UsageTokenMetricRow, "io_total_tokens"> }) {
  return (
    <span className="whitespace-nowrap tabular-nums">
      {trimCompactZero(formatTokensMillions(row.io_total_tokens))}
    </span>
  );
}

function TotalTokenValue({ row }: { row: Pick<UsageTokenMetricRow, "total_tokens"> }) {
  return (
    <span className="whitespace-nowrap tabular-nums">
      {trimCompactZero(formatTokensMillions(row.total_tokens))}
    </span>
  );
}

function CacheHitRateBreakdown({ row }: { row: UsageTokenMetricRow }) {
  const totalWithCache = row.total_tokens;
  const hasValidTotal = Number.isFinite(totalWithCache) && totalWithCache > 0;
  const cacheTokens = row.cache_creation_input_tokens + row.cache_read_input_tokens;
  const hitRate = computeCacheHitRate(
    row.input_tokens,
    row.cache_creation_input_tokens,
    row.cache_read_input_tokens
  );

  const cacheText = hasValidTotal ? trimCompactZero(formatTokensMillions(cacheTokens)) : "—";
  const hitRateText =
    hasValidTotal && Number.isFinite(hitRate) ? trimCompactZero(formatPercent(hitRate)) : "—";

  return <TokenBreakdownInline parts={[cacheText, hitRateText]} />;
}

function TokenShareBar({ percent }: { percent: number }) {
  const pct = Number.isFinite(percent) ? Math.max(0, Math.min(1, percent)) : 0;
  const displayPct = (pct * 100).toFixed(1);

  return (
    <div
      className="flex items-center gap-1.5"
      role="progressbar"
      aria-valuenow={Number(displayPct)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={`Token 占比 ${displayPct}%`}
    >
      <div className="h-1.5 flex-1 rounded-full bg-secondary">
        <div
          className="h-full rounded-full bg-sky-500 transition-all duration-300"
          style={{ width: `${pct * 100}%` }}
        />
      </div>
      <span className="w-10 text-right text-[10px] tabular-nums text-muted-foreground">
        {displayPct}%
      </span>
    </div>
  );
}

function DayDetailLoading() {
  return (
    <div className="flex items-center justify-center gap-2 py-8 text-sm text-muted-foreground">
      <Spinner size="sm" />
      <span>加载日期详情中…</span>
    </div>
  );
}

function DayFolderUsageTable({ folders }: { folders: UsageDayFolderRow[] }) {
  const [sortState, setSortState] = useState<SortState<DayFolderSortKey> | null>(null);
  const sortedFolders = useMemo(() => sortDayFolderRows(folders, sortState), [folders, sortState]);
  const handleSort = useCallback((key: DayFolderSortKey) => {
    setSortState((current) => nextSortState(current, key));
  }, []);

  if (folders.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-muted-foreground">
        当天暂无可展示的文件夹用量。
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-separate border-spacing-0 text-left text-xs">
        <caption className="sr-only">日期文件夹用量明细</caption>
        <thead>
          <tr>
            <SortableColumnHeader
              label="文件夹"
              sortKey="folder"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="总Token"
              sortKey="totalTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="输入+输出"
              sortKey="ioTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="缓存情况"
              sortKey="cacheTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="花费"
              sortKey="cost"
              sortState={sortState}
              onSort={handleSort}
            />
          </tr>
        </thead>
        <tbody>
          {sortedFolders.map((folder) => (
            <tr key={folder.key} className="align-top">
              <td className={TABLE_TD_CLASS}>
                <div className="flex min-w-[180px] items-start gap-2">
                  <FolderOpen className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  <div className="min-w-0">
                    <div className="truncate font-medium text-foreground">{folder.name}</div>
                    {folder.folder_path ? (
                      <div
                        className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground"
                        title={folder.folder_path}
                      >
                        {folder.folder_path}
                      </div>
                    ) : null}
                  </div>
                </div>
              </td>
              <td className={TABLE_MONO_TD_CLASS}>
                <TotalTokenValue row={folder} />
              </td>
              <td className={TABLE_MONO_TD_CLASS}>
                <InputOutputTokenValue row={folder} />
              </td>
              <td className={TABLE_MONO_TD_CLASS}>
                <CacheHitRateBreakdown row={folder} />
              </td>
              <td className={TABLE_MONO_TD_CLASS}>{formatCostValue(folder.cost_usd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function hourLabel(hour: number) {
  return `${String(hour).padStart(2, "0")}:00`;
}

function DayHourlyMiniBarChart({ hours }: { hours: UsageDayHourRow[] }) {
  const maxTokens = Math.max(1, ...hours.map((row) => row.total_tokens));
  const totalTokens = hours.reduce((sum, row) => sum + row.total_tokens, 0);
  const totalRequests = hours.reduce((sum, row) => sum + row.requests_total, 0);
  const activeHours = hours.filter((row) => row.total_tokens > 0 || row.requests_total > 0);
  const firstActiveHour = activeHours[0]?.hour ?? null;
  const lastActiveHour = activeHours[activeHours.length - 1]?.hour ?? null;
  const activeRangeText =
    firstActiveHour == null || lastActiveHour == null
      ? "最早 — · 最晚 —"
      : `最早 ${hourLabel(firstActiveHour)} · 最晚 ${hourLabel(lastActiveHour)}`;

  return (
    <div>
      <div className="mb-3 flex items-baseline justify-between gap-3">
        <div>
          <div className="text-sm font-semibold text-foreground">24 小时分布</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            {formatTokenValue(totalTokens)} · {formatInteger(totalRequests)} 次请求
          </div>
          <div className="mt-0.5 text-xs text-muted-foreground">{activeRangeText}</div>
        </div>
      </div>
      <div
        className="flex h-28 items-end gap-1 rounded-md border border-border bg-white px-2 py-2 dark:border-border dark:bg-card/50"
        role="img"
        aria-label="24 小时 Token 分布"
      >
        {hours.map((row) => {
          const ratio = maxTokens > 0 ? row.total_tokens / maxTokens : 0;
          const height = row.total_tokens > 0 ? Math.max(8, Math.round(ratio * 100)) : 2;
          return (
            <div
              key={row.hour}
              className="flex h-full min-w-[5px] flex-1 items-end"
              title={`${hourLabel(row.hour)} · ${formatTokenValue(row.total_tokens)} · ${formatInteger(row.requests_total)} 次请求`}
            >
              <div
                data-testid="day-hour-bar"
                className={cn(
                  "w-full rounded-sm transition-colors",
                  row.total_tokens > 0
                    ? "bg-sky-500 hover:bg-sky-600 dark:bg-sky-400 dark:hover:bg-sky-300"
                    : "bg-muted dark:bg-secondary"
                )}
                style={{ height: `${height}%` }}
              />
            </div>
          );
        })}
      </div>
      <div className="mt-2 grid grid-cols-5 text-[10px] tabular-nums text-muted-foreground">
        <span>00</span>
        <span className="text-center">06</span>
        <span className="text-center">12</span>
        <span className="text-center">18</span>
        <span className="text-right">23</span>
      </div>
    </div>
  );
}

function DayDetailPanel({
  detail,
  loading,
  errorText,
}: {
  detail: UsageDayDetailV1 | null;
  loading: boolean;
  errorText: string | null;
}) {
  if (loading) return <DayDetailLoading />;

  if (errorText) {
    return (
      <div className="py-6 text-sm text-rose-600 dark:text-rose-300">
        日期详情加载失败：{errorText}
      </div>
    );
  }

  if (!detail) {
    return <div className="py-6 text-sm text-muted-foreground">暂无日期详情。</div>;
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1.45fr)_minmax(280px,0.85fr)]">
      <div className="min-w-0">
        <div className="mb-3 text-sm font-semibold text-foreground">文件夹 Token 明细</div>
        <DayFolderUsageTable folders={detail.folders} />
      </div>
      <DayHourlyMiniBarChart hours={detail.hours} />
    </div>
  );
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
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-7">
        {SUMMARY_SKELETON_KEYS.map((key) => (
          <StatCardSkeleton key={key} />
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-7">
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
        title="成本覆盖率"
        value={formatPercent(summaryCostCoverage(summary))}
        accent="orange"
      />
      <StatCard title="成功请求" value={formatInteger(summary?.requests_success)} accent="green" />
      <StatCard
        title="缓存命中率"
        value={formatPercent(summaryCacheHitRate(summary))}
        accent="purple"
      />
      <StatCard
        title={`${scopeLabel(scope)}数`}
        value={formatInteger(rows.length)}
        accent="slate"
      />
    </div>
  );
}

function TokenLeaderboardTable({
  scope,
  rows,
  summary,
  loading,
  customPending,
  expandedDay,
  dayDetail,
  dayDetailLoading,
  dayDetailErrorText,
  onToggleDay,
}: {
  scope: TokenCostScope;
  rows: UsageLeaderboardRow[];
  summary: UsageSummary | null;
  loading: boolean;
  customPending: boolean;
  expandedDay: string | null;
  dayDetail: UsageDayDetailV1 | null;
  dayDetailLoading: boolean;
  dayDetailErrorText: string | null;
  onToggleDay: (day: string) => void;
}) {
  const [sortState, setSortState] = useState<SortState<LeaderboardSortKey> | null>(null);
  const sortedRows = useMemo(
    () => sortLeaderboardRows(rows, sortState, summary),
    [rows, sortState, summary]
  );
  const handleSort = useCallback((key: LeaderboardSortKey) => {
    setSortState((current) => nextSortState(current, key));
  }, []);

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

  return (
    <div className="min-h-0 flex-1 overflow-auto scrollbar-overlay">
      <table className="w-full border-separate border-spacing-0 text-left text-sm">
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
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="总Token"
              sortKey="totalTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="输入+输出 Token"
              sortKey="ioTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="缓存情况"
              note="缓存/命中率"
              sortKey="cacheTokens"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="总花费"
              sortKey="cost"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="请求数"
              sortKey="requests"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="成功率"
              sortKey="successRate"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="Token 占比"
              sortKey="tokenShare"
              sortState={sortState}
              onSort={handleSort}
            />
            <SortableColumnHeader
              label="平均输出速度"
              sortKey="outputSpeed"
              sortState={sortState}
              onSort={handleSort}
            />
          </tr>
        </thead>
        <tbody>
          {sortedRows.map(({ row }, index) => {
            const expanded = scope === "day" && expandedDay === row.key;
            return (
              <Fragment key={row.key}>
                <tr
                  className={cn(
                    "align-top transition-colors hover:bg-secondary/60 dark:hover:bg-secondary/50",
                    expanded && "bg-secondary/80 dark:bg-secondary/60"
                  )}
                >
                  <td className={`${TABLE_TD_CLASS} text-xs tabular-nums text-muted-foreground`}>
                    {index + 1}
                  </td>
                  <td className={TABLE_TD_CLASS}>
                    {scope === "day" ? (
                      <button
                        type="button"
                        aria-expanded={expanded}
                        aria-label={`${expanded ? "收起" : "展开"} ${row.name} 日期详情`}
                        onClick={() => onToggleDay(row.key)}
                        className="group flex min-w-[130px] items-center gap-1.5 text-left"
                      >
                        <ChevronRight
                          aria-hidden="true"
                          className={cn(
                            "h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform dark:text-muted-foreground",
                            expanded && "rotate-90 text-sky-500 dark:text-sky-300"
                          )}
                        />
                        <span className="font-medium text-foreground group-hover:text-sky-700 dark:text-foreground dark:group-hover:text-sky-300">
                          {row.name}
                        </span>
                      </button>
                    ) : (
                      <div className="font-medium text-foreground">{row.name}</div>
                    )}
                  </td>
                  <td className={TABLE_MONO_TD_CLASS}>
                    <TotalTokenValue row={row} />
                  </td>
                  <td className={TABLE_MONO_TD_CLASS}>
                    <InputOutputTokenValue row={row} />
                  </td>
                  <td className={TABLE_MONO_TD_CLASS}>
                    <CacheHitRateBreakdown row={row} />
                  </td>
                  <td className={TABLE_MONO_TD_CLASS}>{formatCostValue(row.cost_usd)}</td>
                  <td className={TABLE_MONO_TD_CLASS}>{formatInteger(row.requests_total)}</td>
                  <td className={TABLE_MONO_TD_CLASS}>{formatPercent(successRate(row))}</td>
                  <td className={`${TABLE_TD_CLASS} min-w-[120px]`}>
                    <TokenShareBar percent={tokenShare(row, summary)} />
                  </td>
                  <td className={TABLE_MONO_TD_CLASS}>
                    {formatTokensPerSecond(row.avg_output_tokens_per_second)}
                  </td>
                </tr>
                {expanded ? (
                  <tr>
                    <td
                      colSpan={10}
                      className="border-b border-border bg-secondary/70 px-4 py-4 dark:border-border dark:bg-card/40"
                    >
                      <DayDetailPanel
                        detail={dayDetail?.day === row.key ? dayDetail : null}
                        loading={dayDetailLoading}
                        errorText={dayDetailErrorText}
                      />
                    </td>
                  </tr>
                ) : null}
              </Fragment>
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
    const missingSelected = selectedKeys
      .filter((key) => !optionsByKey.has(key))
      .map<UsageFolderOptionV1>((key) => ({
        key,
        name: key,
        folder_path: null,
        requests_total: 0,
        total_tokens: 0,
      }));
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

export function HomeTokenCostPanel({ devPreviewEnabled = false }: HomeTokenCostPanelProps) {
  const [scope, setScope] = useState<TokenCostScope>("provider");
  const [range, setRange] = useState<TokenCostRange>("today");
  const [expandedDay, setExpandedDay] = useState<string | null>(null);
  const [selectedFolderKeys, setSelectedFolderKeys] = useState<string[]>([]);
  const [excludeCx2CcGatewayBridge, setExcludeCx2CcGatewayBridge] = useState(true);
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
    () => buildTokenCostQueryConfig(range, customApplied),
    [customApplied, range]
  );
  const customPending = range === "custom" && !customApplied;
  const selectedFolderKeysForQuery = useMemo(
    () => (selectedFolderKeys.length > 0 ? selectedFolderKeys : null),
    [selectedFolderKeys]
  );
  const filteredQueryConfig = useMemo(
    () => ({
      ...queryConfig,
      input: {
        ...queryConfig.input,
        folderKeys: selectedFolderKeysForQuery,
        excludeCx2CcGatewayBridge,
      },
    }),
    [excludeCx2CcGatewayBridge, queryConfig, selectedFolderKeysForQuery]
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
  const expandedVisibleDay = useMemo(() => {
    if (scope !== "day" || customPending || !expandedDay) return null;
    return displayRows.some((row) => row.key === expandedDay) ? expandedDay : null;
  }, [customPending, displayRows, expandedDay, scope]);
  const dayDetailParams = useMemo(
    () => ({
      day: expandedVisibleDay ?? "",
      cliKey: filteredQueryConfig.input.cliKey,
      providerId: filteredQueryConfig.input.providerId,
      folderLimit: 8,
      folderKeys: selectedFolderKeysForQuery,
      excludeCx2CcGatewayBridge,
    }),
    [
      excludeCx2CcGatewayBridge,
      expandedVisibleDay,
      filteredQueryConfig.input.cliKey,
      filteredQueryConfig.input.providerId,
      selectedFolderKeysForQuery,
    ]
  );
  const dayDetailQueryEnabled = Boolean(expandedVisibleDay) && !model.previewActive;
  const dayDetailQuery = useUsageDayDetailV1Query(dayDetailParams, {
    enabled: dayDetailQueryEnabled,
  });
  const previewDayDetail = useMemo(
    () =>
      expandedVisibleDay && model.previewActive
        ? buildPreviewTokenDayDetail(
            expandedVisibleDay,
            queryConfig.previewFactor,
            selectedFolderKeysForQuery
          )
        : null,
    [expandedVisibleDay, model.previewActive, queryConfig.previewFactor, selectedFolderKeysForQuery]
  );
  const fetchedDayDetail =
    dayDetailQuery.data?.day === expandedVisibleDay ? dayDetailQuery.data : null;
  const displayDayDetail = previewDayDetail ?? fetchedDayDetail;
  const dayDetailLoading =
    Boolean(expandedVisibleDay) &&
    !displayDayDetail &&
    dayDetailQueryEnabled &&
    (dayDetailQuery.isLoading || dayDetailQuery.isFetching);
  const dayDetailErrorText =
    dayDetailQueryEnabled && !displayDayDetail && dayDetailQuery.error
      ? formatUnknownError(dayDetailQuery.error)
      : null;
  const handleToggleDay = useCallback(
    (day: string) => {
      if (customPending) return;
      setExpandedDay((current) => (current === day ? null : day));
    },
    [customPending]
  );
  const handleToggleFolderKey = useCallback((key: string) => {
    setSelectedFolderKeys((current) =>
      current.includes(key) ? current.filter((item) => item !== key) : [...current, key]
    );
  }, []);
  const handleClearFolderKeys = useCallback(() => {
    setSelectedFolderKeys([]);
  }, []);
  const handleApplyCustomRange = useCallback(() => {
    if (applyCustomRange()) {
      setRange("custom");
    }
  }, [applyCustomRange]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-5 overflow-hidden">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="用量筛选">
          <FolderMultiSelect
            options={folderOptions}
            selectedKeys={selectedFolderKeys}
            loading={folderOptionsLoading}
            disabled={folderSelectDisabled}
            onToggleKey={handleToggleFolderKey}
            onClear={handleClearFolderKeys}
          />
          <label className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-white px-2.5 text-xs text-muted-foreground shadow-sm dark:border-border dark:bg-card dark:text-secondary-foreground">
            <span className="whitespace-nowrap">转接去重</span>
            <Switch
              checked={excludeCx2CcGatewayBridge}
              onCheckedChange={setExcludeCx2CcGatewayBridge}
              size="sm"
              aria-label="过滤转接重复用量"
            />
          </label>
          {TOKEN_COST_RANGE_ITEMS.map((item) => {
            const active = range === item.key;
            return (
              <Button
                key={item.key}
                size="sm"
                variant={active ? "primary" : "secondary"}
                aria-pressed={active}
                onClick={() => setRange(item.key)}
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
        <div className="flex flex-wrap items-center gap-3 lg:justify-end">
          <TabList
            ariaLabel="用量维度切换"
            items={TOKEN_COST_SCOPE_ITEMS}
            value={scope}
            onChange={setScope}
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

      <Card padding="none" className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="shrink-0 border-b border-border px-6 pb-4 pt-5 dark:border-border">
          <div className="text-base font-semibold text-foreground">{scopeLabel(scope)}排行</div>
        </div>
        <TokenLeaderboardTable
          scope={scope}
          rows={displayRows}
          summary={displaySummary}
          loading={displayLoading}
          customPending={customPending}
          expandedDay={expandedVisibleDay}
          dayDetail={displayDayDetail}
          dayDetailLoading={dayDetailLoading}
          dayDetailErrorText={dayDetailErrorText}
          onToggleDay={handleToggleDay}
        />
      </Card>
    </div>
  );
}
