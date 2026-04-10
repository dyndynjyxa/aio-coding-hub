// Usage: 用量页面 KPI 汇总卡片区域 — 5 张卡片。

import type { UsageSummary } from "../../services/usage/usage";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";
import { formatInteger, formatPercent, formatUsd, formatDurationMs } from "../../utils/formatters";
import { StatCard, StatCardSkeleton } from "./StatCard";

const SUMMARY_CARD_COUNT = 5;
const SUMMARY_SKELETON_KEYS = Array.from({ length: SUMMARY_CARD_COUNT }, (_, i) => i);

function computeCacheStats(summary: UsageSummary) {
  const total = summary.cache_read_input_tokens + summary.cache_creation_input_tokens;
  const hitRate = computeCacheHitRate(
    summary.input_tokens,
    summary.cache_creation_input_tokens,
    summary.cache_read_input_tokens
  );
  return { total, hitRate };
}

type UsageSummaryCardsProps = {
  summary: UsageSummary | null;
  /** 来自排行榜 rows 的费用合计（由父组件计算，避免重复） */
  totalCostUsd: number;
  /** 排行榜维度数量 */
  leaderboardCount: number;
  loading: boolean;
};

type CardConfig = {
  key: string;
  title: string;
  value: string;
  accent: Parameters<typeof StatCard>[0]["accent"];
  hint?: string;
};

function UsageSummaryCardsSkeleton() {
  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
      {SUMMARY_SKELETON_KEYS.map((key) => (
        <StatCardSkeleton key={key} />
      ))}
    </div>
  );
}

function buildCardConfigs({
  summary,
  totalCostUsd,
  leaderboardCount,
}: Pick<UsageSummaryCardsProps, "summary" | "totalCostUsd" | "leaderboardCount">): CardConfig[] {
  const cacheStats = summary ? computeCacheStats(summary) : null;

  return [
    {
      key: "requests_total",
      title: "总请求数",
      value: formatInteger(summary?.requests_total),
      accent: "blue",
      hint: summary
        ? `成功 ${formatInteger(summary.requests_success)} / 失败 ${formatInteger(summary.requests_failed)}`
        : undefined,
    },
    {
      key: "total_cost_usd",
      title: "总消耗金额",
      value: formatUsd(totalCostUsd),
      accent: "orange",
      hint: leaderboardCount > 0 ? `${leaderboardCount} 个维度合计` : undefined,
    },
    {
      key: "io_total_tokens",
      title: "总 Token 数",
      value: formatInteger(summary?.io_total_tokens),
      accent: "blue",
      hint: summary
        ? `输入 ${formatInteger(summary.input_tokens)} / 输出 ${formatInteger(summary.output_tokens)}`
        : undefined,
    },
    {
      key: "cache_hit_rate",
      title: "缓存命中率",
      value: cacheStats ? formatPercent(cacheStats.hitRate, 1) : "—",
      accent: "purple",
      hint: cacheStats ? `总缓存 Token ${formatInteger(cacheStats.total)}` : undefined,
    },
    {
      key: "avg_duration_ms",
      title: "平均延迟",
      value: formatDurationMs(summary?.avg_duration_ms),
      accent: "slate",
      hint:
        summary?.avg_ttfb_ms != null ? `首字 ${formatDurationMs(summary.avg_ttfb_ms)}` : undefined,
    },
  ];
}

export function UsageSummaryCards({
  summary,
  totalCostUsd,
  leaderboardCount,
  loading,
}: UsageSummaryCardsProps) {
  if (loading && !summary) return <UsageSummaryCardsSkeleton />;

  const cards = buildCardConfigs({ summary, totalCostUsd, leaderboardCount });

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-5">
      {cards.map((card) => (
        <StatCard
          key={card.key}
          title={card.title}
          value={card.value}
          accent={card.accent}
          hint={card.hint}
        />
      ))}
    </div>
  );
}
