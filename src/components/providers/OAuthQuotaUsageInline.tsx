import type { CliKey, OAuthLimitsResult } from "../../services/providers/providers";
import { cn } from "../../utils/cn";

export type OAuthQuotaUsageSegment = {
  key: "short" | "weekly" | "shortEstimate" | "weeklyEstimate" | "credits" | "resetCredit";
  text: string;
  title: string;
  resetCreditCount?: number;
};

function getOAuthWindowLabel(
  cliKey: CliKey,
  limits: OAuthLimitsResult,
  window: "short" | "weekly"
): string {
  if (cliKey === "gemini" && window === "short") return "短窗";
  const estimate = limits.quota_estimates?.[window];
  const seconds =
    limits.limit_window_seconds?.[window === "short" ? 0 : 1] ??
    (estimate ? estimate.window_end - estimate.window_start : null);
  if (seconds != null && seconds > 0 && Number.isFinite(seconds)) {
    return formatWindowLabel(seconds);
  }
  return window === "short" ? (limits.limit_short_label ?? "5h") : "周";
}

function formatWindowLabel(seconds: number): string {
  if (seconds === 7 * 24 * 3600) return "周";
  return seconds % 3600 === 0 ? `${seconds / 3600}h` : `${seconds / 60}m`;
}

function formatEstimateUsd(value: number): string {
  return value > 0 && value < 0.01 ? "<$0.01" : `$${value.toFixed(2)}`;
}

function formatOAuthQuotaResetCountdown(resetAt: number | null, nowUnix: number): string | null {
  if (resetAt == null) return null;
  const remaining = resetAt - nowUnix;
  if (remaining <= 0) return "已重置";

  const totalMinutes = Math.floor(remaining / 60);
  if (totalMinutes < 1) return "<1m";

  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;
  const parts: string[] = [];

  if (days > 0) parts.push(`${days}d`);
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0 || parts.length === 0) parts.push(`${minutes}m`);

  return parts.join(" ");
}

function buildOAuthQuotaUsageSegments({
  cliKey,
  limits,
  nowUnix,
}: {
  cliKey: CliKey;
  limits: OAuthLimitsResult | null;
  nowUnix: number;
}): OAuthQuotaUsageSegment[] {
  if (!limits) return [];

  const shortLabel = getOAuthWindowLabel(cliKey, limits, "short");
  const weeklyLabel = getOAuthWindowLabel(cliKey, limits, "weekly");
  const segments: OAuthQuotaUsageSegment[] = [];

  if (limits.limit_5h_text) {
    const shortReset = formatOAuthQuotaResetCountdown(limits.limit_5h_reset_at, nowUnix);
    segments.push({
      key: "short",
      text: shortReset
        ? `${shortLabel}: ${limits.limit_5h_text}(重置时间: ${shortReset})`
        : `${shortLabel}: ${limits.limit_5h_text}`,
      title: shortReset
        ? `${shortLabel} 用量: ${limits.limit_5h_text}，重置时间: ${shortReset}`
        : `${shortLabel} 用量: ${limits.limit_5h_text}`,
    });
  }

  if (limits.limit_weekly_text) {
    const weeklyReset = formatOAuthQuotaResetCountdown(limits.limit_weekly_reset_at, nowUnix);
    segments.push({
      key: "weekly",
      text: weeklyReset
        ? `${weeklyLabel}: ${limits.limit_weekly_text}(重置时间: ${weeklyReset})`
        : `${weeklyLabel}: ${limits.limit_weekly_text}`,
      title: weeklyReset
        ? `${weeklyLabel}用量: ${limits.limit_weekly_text}，重置时间: ${weeklyReset}`
        : `${weeklyLabel}用量: ${limits.limit_weekly_text}`,
    });
  }

  if (cliKey === "codex" || cliKey === "claude") {
    for (const [key, estimate] of [
      ["shortEstimate", limits.quota_estimates?.short],
      ["weeklyEstimate", limits.quota_estimates?.weekly],
    ] as const) {
      if (!estimate) continue;
      const seconds = estimate.window_end - estimate.window_start;
      if (seconds <= 0 || !Number.isFinite(seconds)) continue;
      const label = key === "shortEstimate" ? shortLabel : weeklyLabel;
      const prefix = `预估额度(${label})`;
      const hint = "根据本窗口本地费用与账号已用比例换算，仅供参考；外部使用或模型变化会影响预估。";
      const total = estimate.estimated_total_usd;
      const remaining = estimate.estimated_remaining_usd;
      const hasEstimate =
        total != null &&
        Number.isFinite(total) &&
        total > 0 &&
        remaining != null &&
        Number.isFinite(remaining) &&
        remaining >= 0 &&
        remaining <= total;
      const expired = estimate.window_end <= nowUnix;
      const unknownReason = expired
        ? "已重置，待刷新"
        : estimate.unpriced_count > 0 && estimate.used_usd == null
          ? "费用数据不足"
          : "待积累用量";
      const usedText =
        estimate.used_usd != null && Number.isFinite(estimate.used_usd)
          ? formatEstimateUsd(estimate.used_usd)
          : "未知";
      const missingCostHint =
        estimate.unpriced_count > 0
          ? `其中 ${estimate.unpriced_count} 次请求缺少费用，仅按已记录费用估算，结果可能偏低。`
          : "";
      segments.push({
        key,
        text:
          hasEstimate && !expired
            ? `${prefix}: ~${formatEstimateUsd(total)}(剩余 ~${formatEstimateUsd(remaining)})`
            : `${prefix}: ${unknownReason}`,
        title:
          hasEstimate && !expired
            ? `${prefix}: ~${formatEstimateUsd(total)}，预估剩余: ~${formatEstimateUsd(remaining)}，本地已用: ${usedText}，样本: ${estimate.sample_count} 次。${missingCostHint}${hint}`
            : `${prefix}: ${unknownReason}。${hint}`,
      });
    }
  }

  if (cliKey === "codex" && limits.credits != null) {
    const { has_credits, unlimited } = limits.credits;
    const balance = limits.credits.balance?.trim();
    const creditText = unlimited
      ? "无限"
      : !has_credits
        ? "0"
        : balance && /^\d+(?:\.\d+)?$/.test(balance)
          ? balance
          : "可用";
    segments.push({
      key: "credits",
      text: `可用点数: ${creditText}`,
      title: `Codex 可用点数: ${creditText}`,
    });
  }

  const resetCreditCount =
    cliKey === "codex" ? (limits.reset_credit_available_count ?? null) : null;
  if (resetCreditCount != null) {
    segments.push({
      key: "resetCredit",
      text: `可重置次数: ${resetCreditCount}`,
      title:
        resetCreditCount > 0 ? `Codex 可重置次数: ${resetCreditCount}` : "Codex 可重置次数不足",
      resetCreditCount,
    });
  }

  return segments;
}

export function OAuthQuotaUsageInline({
  cliKey,
  limits,
  nowUnix,
  className,
  segmentClassName,
  resetCreditDisabled = false,
  resetCreditLoading = false,
  onResetCreditClick,
}: {
  cliKey: CliKey;
  limits: OAuthLimitsResult | null;
  nowUnix: number;
  className?: string;
  segmentClassName?: string;
  resetCreditDisabled?: boolean;
  resetCreditLoading?: boolean;
  onResetCreditClick?: () => void;
}) {
  const segments = buildOAuthQuotaUsageSegments({ cliKey, limits, nowUnix });
  if (segments.length === 0) return null;

  return (
    <span className={cn("inline-flex min-w-0 flex-wrap items-center gap-2", className)}>
      {segments.map((segment) => {
        if (segment.key === "resetCredit") {
          const resetCreditCount = segment.resetCreditCount ?? 0;
          const canReset =
            Boolean(onResetCreditClick) &&
            !resetCreditDisabled &&
            !resetCreditLoading &&
            resetCreditCount > 0;
          const resetText =
            canReset || resetCreditLoading
              ? `${segment.text}(${resetCreditLoading ? "重置中..." : "点击重置"})`
              : segment.text;

          if (onResetCreditClick) {
            return (
              <button
                key={segment.key}
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  if (!canReset) return;
                  onResetCreditClick();
                }}
                disabled={!canReset}
                className={cn(
                  "shrink-0 rounded-sm font-mono text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-60",
                  segmentClassName,
                  canReset
                    ? "cursor-pointer text-sky-700 hover:text-sky-800 dark:text-sky-400 dark:hover:text-sky-300"
                    : "text-muted-foreground"
                )}
                title={segment.title}
              >
                {resetText}
              </button>
            );
          }

          return (
            <span
              key={segment.key}
              className={cn("shrink-0 font-mono text-xs text-muted-foreground", segmentClassName)}
              title={segment.title}
            >
              {resetText}
            </span>
          );
        }

        const isEstimate = segment.key === "shortEstimate" || segment.key === "weeklyEstimate";
        const breakAt = isEstimate ? segment.text.indexOf("(剩余") : -1;
        return (
          <span
            key={segment.key}
            className={cn(
              "font-mono text-xs text-muted-foreground",
              isEstimate ? "min-w-0 max-w-full break-words" : "shrink-0",
              segmentClassName
            )}
            title={segment.title}
          >
            {breakAt >= 0 ? (
              <>
                {segment.text.slice(0, breakAt)}
                <wbr />
                {segment.text.slice(breakAt)}
              </>
            ) : (
              segment.text
            )}
          </span>
        );
      })}
    </span>
  );
}
