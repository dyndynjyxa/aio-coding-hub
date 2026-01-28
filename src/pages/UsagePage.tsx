// Usage: Usage analytics page. Backend commands: `usage_summary_v2`, `usage_leaderboard_v2` (and related `usage_*`).

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  usageLeaderboardV2,
  usageSummaryV2,
  type UsageLeaderboardRow,
  type UsagePeriod,
  type UsageScope,
  type UsageSummary,
} from "../services/usage";
import { CLI_FILTER_ITEMS, type CliFilterKey } from "../constants/clis";
import { PERIOD_ITEMS } from "../constants/periods";
import { useCustomDateRange } from "../hooks/useCustomDateRange";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { PageHeader } from "../ui/PageHeader";
import { cn } from "../utils/cn";
import { formatUnknownError } from "../utils/errors";
import {
  formatDurationMs,
  formatInteger,
  formatPercent,
  formatTokensPerSecond,
  formatUsd,
} from "../utils/formatters";

type ScopeItem = { key: UsageScope; label: string };

const SCOPE_ITEMS: ScopeItem[] = [
  { key: "provider", label: "供应商" },
  { key: "cli", label: "CLI" },
  { key: "model", label: "模型" },
];

const FILTER_LABEL_CLASS = "w-16 shrink-0 pt-1.5 text-right text-xs font-medium text-slate-600";
const FILTER_OPTIONS_CLASS = "min-w-0 flex flex-1 flex-wrap items-center gap-2";
const FILTER_OPTION_BUTTON_CLASS = "w-24 whitespace-nowrap";

function StatCard({
  title,
  value,
  hint,
  className,
}: {
  title: string;
  value: string;
  hint?: string;
  className?: string;
}) {
  return (
    <Card
      padding="md"
      className={cn("flex h-full flex-col transition-shadow hover:shadow-lg", className)}
    >
      <div className="text-xs font-medium uppercase tracking-wide text-slate-500">{title}</div>
      <div className="mt-3 text-xl font-semibold tracking-tight text-slate-900 xl:text-2xl">
        {value}
      </div>
      {hint ? (
        <div className="mt-auto pt-3 text-xs leading-relaxed text-slate-500">{hint}</div>
      ) : null}
    </Card>
  );
}

function StatCardSkeleton({ className }: { className?: string }) {
  return (
    <Card padding="md" className={cn("h-full animate-pulse", className)}>
      <div className="h-3 w-20 rounded-md bg-slate-200" />
      <div className="mt-4 h-7 w-28 rounded-md bg-slate-200" />
      <div className="mt-4 h-3 w-36 rounded-md bg-slate-100" />
    </Card>
  );
}

function TokenBreakdown({
  totalTokens,
  inputTokens,
  outputTokens,
  totalTokensWithCache,
}: {
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  totalTokensWithCache?: number;
}) {
  return (
    <div className="space-y-0.5">
      <div>{formatInteger(totalTokens)}</div>
      <div className="text-[10px] leading-4 text-slate-500">
        输入 <span className="text-slate-700">{formatInteger(inputTokens)}</span>
      </div>
      <div className="text-[10px] leading-4 text-slate-500">
        输出 <span className="text-slate-700">{formatInteger(outputTokens)}</span>
      </div>
      {totalTokensWithCache != null && Number.isFinite(totalTokensWithCache) ? (
        <div className="text-[10px] leading-4 text-slate-500">
          含缓存 <span className="text-slate-700">{formatInteger(totalTokensWithCache)}</span>
        </div>
      ) : null}
    </div>
  );
}

function CacheBreakdown({
  inputTokens,
  outputTokens,
  cacheCreationInputTokens,
  cacheReadInputTokens,
}: {
  inputTokens: number;
  outputTokens: number;
  cacheCreationInputTokens: number;
  cacheReadInputTokens: number;
}) {
  const denom = inputTokens + outputTokens + cacheCreationInputTokens + cacheReadInputTokens;
  const rate = denom > 0 ? cacheReadInputTokens / denom : NaN;

  return (
    <div className="space-y-0.5 text-[10px] leading-4">
      <div className="text-slate-500">
        创建 <span className="text-slate-700">{formatInteger(cacheCreationInputTokens)}</span>
      </div>
      <div className="text-slate-500">
        读取 <span className="text-slate-700">{formatInteger(cacheReadInputTokens)}</span>
      </div>
      <div className="text-slate-500">
        缓存率 <span className="text-slate-700">{formatPercent(rate, 2)}</span>
      </div>
    </div>
  );
}

export function UsagePage() {
  const [scope, setScope] = useState<UsageScope>("provider");
  const [period, setPeriod] = useState<UsagePeriod>("daily");
  const [cliKey, setCliKey] = useState<CliFilterKey>("all");
  const [reloadSeq, setReloadSeq] = useState(0);

  const {
    customStartDate,
    setCustomStartDate,
    customEndDate,
    setCustomEndDate,
    customApplied,
    bounds,
    showCustomForm,
    applyCustomRange,
    clearCustomRange,
  } = useCustomDateRange(period, { onInvalid: (message) => toast(message) });

  const [tauriAvailable, setTauriAvailable] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorText, setErrorText] = useState<string | null>(null);

  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [rows, setRows] = useState<UsageLeaderboardRow[]>([]);

  const summaryCards = useMemo(() => {
    if (!summary) return [];

    const requestsHint = `${formatInteger(summary.requests_total)} 请求 · ${formatInteger(
      summary.requests_with_usage
    )} 有用量`;

    return [
      {
        title: "总 Token（输入+输出）",
        value: formatInteger(summary.io_total_tokens),
        hint: requestsHint,
      },
      {
        title: "输入 Token",
        value: formatInteger(summary.input_tokens),
        hint: requestsHint,
      },
      {
        title: "输出 Token",
        value: formatInteger(summary.output_tokens),
        hint: requestsHint,
      },
      {
        title: "缓存创建（输入）",
        value: formatInteger(summary.cache_creation_input_tokens),
        hint: "缓存创建总计",
      },
      {
        title: "缓存读取（输入）",
        value: formatInteger(summary.cache_read_input_tokens),
        hint: "上游返回",
      },
      {
        title: "缓存创建（5m）",
        value: formatInteger(summary.cache_creation_5m_input_tokens),
        hint: "上游返回",
      },
    ];
  }, [summary]);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (period === "custom" && !customApplied) {
        setErrorText(null);
        setSummary(null);
        setRows([]);
        setLoading(false);
        setTauriAvailable(null);
        return;
      }

      setErrorText(null);
      setLoading(true);
      try {
        const filterCliKey = cliKey === "all" ? null : cliKey;
        const sum = await usageSummaryV2(period, { ...bounds, cliKey: filterCliKey });
        if (cancelled) return;

        if (sum === null) {
          setTauriAvailable(false);
          setErrorText(null);
          setSummary(null);
          setRows([]);
          return;
        }

        setTauriAvailable(true);
        setSummary(sum);

        const list = await usageLeaderboardV2(scope, period, {
          ...bounds,
          cliKey: filterCliKey,
          limit: 50,
        });
        if (cancelled) return;
        setRows(list ?? []);
      } catch (err) {
        if (cancelled) return;
        setTauriAvailable(true);
        setErrorText(formatUnknownError(err));
        toast("加载用量失败：请重试（详情见页面错误信息）");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    load();
    return () => {
      cancelled = true;
    };
  }, [bounds, cliKey, customApplied, period, reloadSeq, scope]);

  function successRate(row: UsageLeaderboardRow) {
    if (row.requests_total <= 0) return NaN;
    return row.requests_success / row.requests_total;
  }

  const tableTitle = useMemo(() => {
    switch (scope) {
      case "cli":
        return "CLI";
      case "provider":
        return "供应商";
      case "model":
        return "模型";
      default:
        return "Leaderboard";
    }
  }, [scope]);

  return (
    <div className="space-y-6">
      <PageHeader title="用量" />

      <Card padding="md" className="space-y-4">
        <div className="flex items-start gap-3">
          <span className={FILTER_LABEL_CLASS}>CLI：</span>
          <div className={FILTER_OPTIONS_CLASS}>
            {CLI_FILTER_ITEMS.map((item) => (
              <Button
                key={item.key}
                size="sm"
                variant={cliKey === item.key ? "primary" : "secondary"}
                onClick={() => setCliKey(item.key)}
                disabled={loading}
                className={FILTER_OPTION_BUTTON_CLASS}
              >
                {item.label}
              </Button>
            ))}
          </div>
        </div>

        <div className="flex items-start gap-3">
          <span className={FILTER_LABEL_CLASS}>维度：</span>
          <div className={FILTER_OPTIONS_CLASS}>
            {SCOPE_ITEMS.map((item) => (
              <Button
                key={item.key}
                size="sm"
                variant={scope === item.key ? "primary" : "secondary"}
                onClick={() => setScope(item.key)}
                disabled={loading}
                className={FILTER_OPTION_BUTTON_CLASS}
              >
                {item.label}
              </Button>
            ))}
          </div>
        </div>

        <div className="flex items-start gap-3">
          <span className={FILTER_LABEL_CLASS}>时间窗：</span>
          <div className={FILTER_OPTIONS_CLASS}>
            {PERIOD_ITEMS.map((item) => (
              <Button
                key={item.key}
                size="sm"
                variant={period === item.key ? "primary" : "secondary"}
                onClick={() => setPeriod(item.key)}
                disabled={loading}
                className={FILTER_OPTION_BUTTON_CLASS}
              >
                {item.label}
              </Button>
            ))}
            {period === "custom" ? (
              <span className="w-full pt-1 text-xs text-slate-500">
                endDate 包含（按本地日期边界计算）
              </span>
            ) : null}
          </div>
        </div>

        {showCustomForm ? (
          <div className="flex items-start gap-3 border-t border-slate-100 pt-4">
            <div className="w-16 shrink-0" aria-hidden="true" />
            <div className="min-w-0 flex flex-1 flex-col gap-3 md:flex-row md:items-end">
              <div className="flex flex-col gap-1.5">
                <div className="text-xs font-medium text-slate-600">开始日期</div>
                <input
                  type="date"
                  value={customStartDate}
                  onChange={(e) => setCustomStartDate(e.currentTarget.value)}
                  aria-label="开始日期"
                  className="h-10 rounded-lg border border-slate-200 bg-white px-3 text-sm text-slate-900 shadow-sm outline-none transition focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <div className="text-xs font-medium text-slate-600">结束日期</div>
                <input
                  type="date"
                  value={customEndDate}
                  onChange={(e) => setCustomEndDate(e.currentTarget.value)}
                  aria-label="结束日期"
                  className="h-10 rounded-lg border border-slate-200 bg-white px-3 text-sm text-slate-900 shadow-sm outline-none transition focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
                />
              </div>
              <div className="flex flex-wrap items-center gap-2 md:pb-0.5">
                <Button size="sm" variant="primary" onClick={applyCustomRange} disabled={loading}>
                  应用
                </Button>
                <Button size="sm" variant="secondary" onClick={clearCustomRange} disabled={loading}>
                  清空
                </Button>
                {customApplied ? (
                  <span className="text-xs font-medium text-slate-600">
                    已应用：{customApplied.startDate} → {customApplied.endDate}
                  </span>
                ) : (
                  <span className="text-xs text-slate-500">请选择日期范围后点击"应用"</span>
                )}
              </div>
            </div>
          </div>
        ) : null}
      </Card>

      {errorText ? (
        <Card padding="md" className="border-rose-200 bg-rose-50">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className="text-sm font-semibold text-rose-900">加载失败</div>
              <div className="mt-1 text-sm text-rose-800">
                用量数据刷新失败，请重试；必要时查看 Console 日志定位 error_code。
              </div>
            </div>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setReloadSeq((v) => v + 1)}
              disabled={loading}
              className="border-rose-200 bg-white text-rose-800 hover:bg-rose-50"
            >
              重试
            </Button>
          </div>
          <div className="mt-3 rounded-lg border border-rose-200 bg-white/60 p-3 font-mono text-xs text-slate-800">
            {errorText}
          </div>
        </Card>
      ) : null}

      {tauriAvailable === false ? (
        <Card padding="md">
          <div className="text-sm text-slate-600">
            当前环境未检测到 Tauri Runtime。请通过桌面端运行（`pnpm tauri dev`）后查看用量。
          </div>
        </Card>
      ) : null}

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-6">
        {loading ? (
          Array.from({ length: 6 }).map((_, idx) => <StatCardSkeleton key={idx} />)
        ) : summaryCards.length === 0 ? (
          <Card padding="md" className="col-span-full">
            <div className="text-sm text-slate-600">
              {errorText
                ? '加载失败：暂无可展示的用量摘要。请点击上方"重试"。'
                : period === "custom" && !customApplied
                  ? '自定义范围：请选择日期后点击"应用"。'
                  : "暂无用量数据。请先通过网关发起请求。"}
            </div>
          </Card>
        ) : (
          summaryCards.map((card) => (
            <StatCard key={card.title} title={card.title} value={card.value} hint={card.hint} />
          ))
        )}
      </div>

      <Card padding="none">
        <div className="flex items-center justify-between gap-4 px-6 pb-0 pt-5">
          <div className="text-sm font-semibold text-slate-900">{tableTitle}</div>
        </div>

        <div className="mt-4">
          {loading ? (
            <div className="overflow-x-auto">
              <table className="w-full border-separate border-spacing-0 text-left text-sm">
                <thead>
                  <tr className="text-xs font-medium uppercase tracking-wide text-slate-500">
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">#</th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">名称</th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">请求数</th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">成功率</th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      总 Token
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      缓存 / 缓存率
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      平均耗时
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      平均首字
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      平均速率
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5">
                      花费金额
                    </th>
                  </tr>
                </thead>
                <tbody className="animate-pulse">
                  {Array.from({ length: 5 }).map((_, idx) => (
                    <tr key={idx} className="align-top">
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-5 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-32 rounded-md bg-slate-200" />
                        <div className="mt-2 h-3 w-48 rounded-md bg-slate-100" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-14 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-12 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-16 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-20 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-14 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-14 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-16 rounded-md bg-slate-200" />
                      </td>
                      <td className="border-b border-slate-100 px-3 py-3.5">
                        <div className="h-3 w-14 rounded-md bg-slate-200" />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : rows.length === 0 && !summary ? (
            <div className="px-6 pb-5 text-sm text-slate-600">
              {errorText
                ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
                : period === "custom" && !customApplied
                  ? '自定义范围：请选择日期后点击"应用"。'
                  : "暂无用量数据。请先通过网关发起请求。"}
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-separate border-spacing-0 text-left text-sm">
                <thead className="sticky top-0 z-10">
                  <tr className="text-xs font-medium uppercase tracking-wide text-slate-500">
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      #
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      名称
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      请求数
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      成功率
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      总 Token
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      缓存 / 缓存率
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      平均耗时
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      平均首字
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      平均速率
                    </th>
                    <th className="border-b border-slate-200 bg-slate-50/60 px-3 py-2.5 backdrop-blur-sm">
                      花费金额
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {rows.length === 0 ? (
                    <tr className="align-top">
                      <td
                        colSpan={10}
                        className="border-b border-slate-100 px-3 py-4 text-sm text-slate-600"
                      >
                        {errorText
                          ? '加载失败：暂无可展示的数据。请点击上方"重试"。'
                          : summary
                            ? "暂无 Leaderboard 数据。"
                            : "暂无可展示的数据。"}
                      </td>
                    </tr>
                  ) : (
                    rows.map((row, index) => (
                      <tr
                        key={row.key}
                        className="align-top transition-colors hover:bg-slate-50/50"
                      >
                        <td className="border-b border-slate-100 px-3 py-3 text-xs tabular-nums text-slate-400">
                          {index + 1}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3">
                          <div className="font-medium text-slate-900">{row.name}</div>
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatInteger(row.requests_total)}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatPercent(successRate(row))}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          <TokenBreakdown
                            totalTokens={row.io_total_tokens}
                            inputTokens={row.input_tokens}
                            outputTokens={row.output_tokens}
                            totalTokensWithCache={row.total_tokens}
                          />
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          <CacheBreakdown
                            inputTokens={row.input_tokens}
                            outputTokens={row.output_tokens}
                            cacheCreationInputTokens={row.cache_creation_input_tokens}
                            cacheReadInputTokens={row.cache_read_input_tokens}
                          />
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatDurationMs(row.avg_duration_ms)}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatDurationMs(row.avg_ttfb_ms)}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatTokensPerSecond(row.avg_output_tokens_per_second)}
                        </td>
                        <td className="border-b border-slate-100 px-3 py-3 font-mono text-xs tabular-nums text-slate-700">
                          {formatUsd(row.cost_usd)}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
                {summary ? (
                  <tfoot>
                    <tr className="align-top bg-slate-100/80">
                      <td className="border-b border-slate-200 px-3 py-3 text-sm font-semibold text-slate-500">
                        Σ
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3">
                        <div className="font-semibold text-slate-900">总计</div>
                        <div className="mt-1 text-xs leading-relaxed text-slate-500">
                          {formatInteger(summary.requests_total)} 请求 ·{" "}
                          {formatInteger(summary.requests_with_usage)} 有用量
                        </div>
                        <div className="mt-0.5 text-xs text-slate-500">
                          仅统计成功请求（{formatInteger(summary.requests_success)}）
                        </div>
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        {formatInteger(summary.requests_total)}
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        {formatPercent(
                          summary.requests_total > 0
                            ? summary.requests_success / summary.requests_total
                            : NaN
                        )}
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        <TokenBreakdown
                          totalTokens={summary.io_total_tokens}
                          inputTokens={summary.input_tokens}
                          outputTokens={summary.output_tokens}
                          totalTokensWithCache={summary.total_tokens}
                        />
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        <CacheBreakdown
                          inputTokens={summary.input_tokens}
                          outputTokens={summary.output_tokens}
                          cacheCreationInputTokens={summary.cache_creation_input_tokens}
                          cacheReadInputTokens={summary.cache_read_input_tokens}
                        />
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        {formatDurationMs(summary.avg_duration_ms)}
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        {formatDurationMs(summary.avg_ttfb_ms)}
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-900">
                        {formatTokensPerSecond(summary.avg_output_tokens_per_second)}
                      </td>
                      <td className="border-b border-slate-200 px-3 py-3 font-mono text-xs font-medium tabular-nums text-slate-500">
                        —
                      </td>
                    </tr>
                  </tfoot>
                ) : null}
              </table>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
