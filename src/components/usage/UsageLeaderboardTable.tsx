// Usage: 用量排行表格 — 支持费用占比、$/1K Token 列。

import { memo } from "react";
import type { UsageSummary, UsageLeaderboardRow } from "../../services/usage/usage";
import {
  formatDurationMs,
  formatInteger,
  formatPercent,
  formatTokensPerSecond,
  formatUsd,
} from "../../utils/formatters";
import { TokenBreakdown } from "./TokenBreakdown";
import { CacheBreakdown } from "./CacheBreakdown";
import { CostBar } from "./CostBar";
import { TABLE_COLUMNS } from "./UsageTableColumns";

const TH_CLASS =
  "border-b border-slate-200 dark:border-slate-700 bg-slate-50/60 dark:bg-slate-800/60 px-3 py-2.5 backdrop-blur-sm";
const TD_CLASS = "border-b border-slate-100 dark:border-slate-700 px-3 py-3";
const MONO_TD = `${TD_CLASS} font-mono text-xs tabular-nums text-slate-700 dark:text-slate-300`;

function successRate(row: UsageLeaderboardRow) {
  if (row.requests_total <= 0) return NaN;
  return row.requests_success / row.requests_total;
}

function costPer1kTokens(row: UsageLeaderboardRow) {
  if (row.io_total_tokens <= 0 || row.cost_usd == null || !Number.isFinite(row.cost_usd))
    return null;
  return (row.cost_usd / row.io_total_tokens) * 1000;
}

type UsageLeaderboardTableProps = {
  rows: UsageLeaderboardRow[];
  summary: UsageSummary | null;
  totalCostUsd: number;
  errorText: string | null;
};

function UsageTableEmptyState({ errorText }: { errorText: string | null }) {
  return (
    <div className="px-6 pb-5 text-sm text-slate-600 dark:text-slate-400">
      {errorText
        ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
        : "暂无用量数据。请先通过网关发起请求。"}
    </div>
  );
}

function UsageTableHeader() {
  return (
    <thead className="sticky top-0 z-10">
      <tr className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
        {TABLE_COLUMNS.map((col) => (
          <th key={col.key} scope="col" className={TH_CLASS}>
            {col.label}
          </th>
        ))}
      </tr>
    </thead>
  );
}

function UsageLeaderboardEmptyRow({
  errorText,
  hasSummary,
}: {
  errorText: string | null;
  hasSummary: boolean;
}) {
  return (
    <tr className="align-top">
      <td
        colSpan={TABLE_COLUMNS.length}
        className="border-b border-slate-100 dark:border-slate-700 px-3 py-4 text-sm text-slate-600 dark:text-slate-400"
      >
        {errorText
          ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
          : hasSummary
            ? "暂无 Leaderboard 数据。"
            : "暂无可展示的数据。"}
      </td>
    </tr>
  );
}

const UsageLeaderboardDataRow = memo(function UsageLeaderboardDataRow({
  index,
  row,
  totalCostUsd,
}: {
  index: number;
  row: UsageLeaderboardRow;
  totalCostUsd: number;
}) {
  const costPercent = totalCostUsd > 0 && row.cost_usd != null ? row.cost_usd / totalCostUsd : 0;
  const per1k = costPer1kTokens(row);

  return (
    <tr
      key={row.key}
      className="align-top transition-colors hover:bg-slate-50/50 dark:hover:bg-slate-800/50"
    >
      <td className={`${TD_CLASS} text-xs tabular-nums text-slate-400 dark:text-slate-500`}>
        {index + 1}
      </td>
      <td className={TD_CLASS}>
        <div className="font-medium text-slate-900 dark:text-slate-100">{row.name}</div>
      </td>
      <td className={MONO_TD}>{formatInteger(row.requests_total)}</td>
      <td className={MONO_TD}>{formatPercent(successRate(row))}</td>
      <td className={MONO_TD}>
        <TokenBreakdown
          totalTokens={row.io_total_tokens}
          inputTokens={row.input_tokens}
          outputTokens={row.output_tokens}
          totalTokensWithCache={row.total_tokens}
        />
      </td>
      <td className={MONO_TD}>
        <CacheBreakdown
          inputTokens={row.input_tokens}
          cacheCreationInputTokens={row.cache_creation_input_tokens}
          cacheReadInputTokens={row.cache_read_input_tokens}
        />
      </td>
      <td className={MONO_TD}>{formatUsd(row.cost_usd)}</td>
      <td className={`${TD_CLASS} min-w-[120px]`}>
        <CostBar percent={costPercent} />
      </td>
      <td className={MONO_TD}>{per1k != null ? formatUsd(per1k) : "—"}</td>
      <td className={MONO_TD}>{formatDurationMs(row.avg_duration_ms)}</td>
      <td className={MONO_TD}>{formatDurationMs(row.avg_ttfb_ms)}</td>
      <td className={MONO_TD}>{formatTokensPerSecond(row.avg_output_tokens_per_second)}</td>
    </tr>
  );
});

function UsageLeaderboardSummaryRow({
  summary,
  totalCostUsd,
  rowsTotalIoTokens,
}: {
  summary: UsageSummary;
  totalCostUsd: number;
  rowsTotalIoTokens: number;
}) {
  const successRateText = formatPercent(
    summary.requests_total > 0 ? summary.requests_success / summary.requests_total : NaN
  );
  const costPer1kText =
    rowsTotalIoTokens > 0 && totalCostUsd > 0
      ? formatUsd((totalCostUsd / rowsTotalIoTokens) * 1000)
      : "—";

  return (
    <tr className="align-top bg-slate-100/80 dark:bg-slate-800/80">
      <td className={`${TD_CLASS} text-sm font-semibold text-slate-500 dark:text-slate-400`}>Σ</td>
      <td className={TD_CLASS}>
        <div className="font-semibold text-slate-900 dark:text-slate-100">总计</div>
        <div className="mt-1 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
          {formatInteger(summary.requests_total)} 请求 ·{" "}
          {formatInteger(summary.requests_with_usage)} 有用量
        </div>
        <div className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
          仅统计成功请求（{formatInteger(summary.requests_success)}）
        </div>
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {formatInteger(summary.requests_total)}
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {successRateText}
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        <TokenBreakdown
          totalTokens={summary.io_total_tokens}
          inputTokens={summary.input_tokens}
          outputTokens={summary.output_tokens}
          totalTokensWithCache={summary.total_tokens}
        />
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        <CacheBreakdown
          inputTokens={summary.input_tokens}
          cacheCreationInputTokens={summary.cache_creation_input_tokens}
          cacheReadInputTokens={summary.cache_read_input_tokens}
        />
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {formatUsd(totalCostUsd)}
      </td>
      <td className={TD_CLASS}>
        <span className="text-xs text-slate-500 dark:text-slate-400">100%</span>
      </td>
      <td className={`${MONO_TD} font-medium text-slate-500 dark:text-slate-400`}>
        {costPer1kText}
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {formatDurationMs(summary.avg_duration_ms)}
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {formatDurationMs(summary.avg_ttfb_ms)}
      </td>
      <td className={`${MONO_TD} font-medium text-slate-900 dark:text-slate-100`}>
        {formatTokensPerSecond(summary.avg_output_tokens_per_second)}
      </td>
    </tr>
  );
}

export function UsageLeaderboardTable({
  rows,
  summary,
  totalCostUsd,
  errorText,
}: UsageLeaderboardTableProps) {
  if (rows.length === 0 && !summary) return <UsageTableEmptyState errorText={errorText} />;

  const rowsTotalIoTokens = rows.reduce((sum, row) => sum + row.io_total_tokens, 0);

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-separate border-spacing-0 text-left text-sm">
        <caption className="sr-only">用量排行榜</caption>
        <UsageTableHeader />
        <tbody>
          {rows.length === 0 ? (
            <UsageLeaderboardEmptyRow errorText={errorText} hasSummary={summary != null} />
          ) : (
            rows.map((row, index) => (
              <UsageLeaderboardDataRow
                key={row.key}
                index={index}
                row={row}
                totalCostUsd={totalCostUsd}
              />
            ))
          )}
        </tbody>

        {/* 汇总行 */}
        {summary ? (
          <tfoot>
            <UsageLeaderboardSummaryRow
              summary={summary}
              totalCostUsd={totalCostUsd}
              rowsTotalIoTokens={rowsTotalIoTokens}
            />
          </tfoot>
        ) : null}
      </table>
    </div>
  );
}
