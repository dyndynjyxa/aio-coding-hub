import type { CliKey, OAuthLimitsResult } from "../../services/providers/providers";
import { cn } from "../../utils/cn";

export type OAuthQuotaUsageSegment = {
  key: "short" | "weekly" | "resetCredit";
  text: string;
  title: string;
  resetCreditCount?: number;
};

function getOAuthShortWindowLabel(cliKey: CliKey, limits: OAuthLimitsResult | null): string {
  if (cliKey === "gemini") return "短窗";
  return limits?.limit_short_label ?? "5h";
}

export function formatOAuthQuotaResetCountdown(
  resetAt: number | null,
  nowUnix: number
): string | null {
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

export function buildOAuthQuotaUsageSegments({
  cliKey,
  limits,
  nowUnix,
}: {
  cliKey: CliKey;
  limits: OAuthLimitsResult | null;
  nowUnix: number;
}): OAuthQuotaUsageSegment[] {
  if (!limits) return [];

  const shortLabel = getOAuthShortWindowLabel(cliKey, limits);
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
        ? `周: ${limits.limit_weekly_text}(重置时间: ${weeklyReset})`
        : `周: ${limits.limit_weekly_text}`,
      title: weeklyReset
        ? `周用量: ${limits.limit_weekly_text}，重置时间: ${weeklyReset}`
        : `周用量: ${limits.limit_weekly_text}`,
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

        return (
          <span
            key={segment.key}
            className={cn("shrink-0 font-mono text-xs text-muted-foreground", segmentClassName)}
            title={segment.title}
          >
            {segment.text}
          </span>
        );
      })}
    </span>
  );
}
