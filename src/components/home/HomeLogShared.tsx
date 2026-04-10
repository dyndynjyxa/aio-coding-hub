// Usage:
// - Import helpers/components from this module for Home "request logs" list and "realtime traces" cards.
// - Designed to keep status badge / error_code label / session reuse tooltip consistent across the Home page.

import { GatewayErrorCodes, getGatewayErrorShortLabel } from "../../constants/gatewayErrorCodes";
import type { CliKey } from "../../services/providers/providers";
import type { RequestLogRouteHop } from "../../services/gateway/requestLogs";
import type { TraceSession } from "../../services/gateway/traceStore";
import { Tooltip } from "../../ui/Tooltip";
import { FolderOpen } from "lucide-react";
import { RouteTooltipContent } from "./RouteTooltipContent";

const CLIENT_ABORT_ERROR_CODES: ReadonlySet<string> = new Set([
  GatewayErrorCodes.STREAM_ABORTED,
  GatewayErrorCodes.REQUEST_ABORTED,
]);

const STATUS_TEXT_UNKNOWN = "状态未知";

const SESSION_REUSE_TOOLTIP =
  "同一 session_id 在 5 分钟 TTL 内优先复用上一次成功 provider，减少抖动/提升缓存命中";

type RequestLogAuditInput = {
  cli_key: CliKey | string;
  path: string;
  excluded_from_stats?: boolean | null;
  special_settings_json?: string | null;
  error_code?: string | null;
};

type RequestLogProgressInput = {
  status: number | null;
  error_code?: string | null;
  created_at?: number;
  created_at_ms?: number | null;
};

export type RequestLogAuditTag = {
  label: string;
  className: string;
  title?: string;
};

export type RequestLogAuditMeta = {
  muted: boolean;
  summary: string | null;
  tags: RequestLogAuditTag[];
  providerFallbackText: string | null;
};

type ParsedRequestLogSpecialSetting = {
  type?: string;
  reason?: string;
};

function parseRequestLogSpecialSettings(
  specialSettingsJson: string | null | undefined
): ParsedRequestLogSpecialSetting[] {
  if (!specialSettingsJson) return [];

  try {
    const parsed = JSON.parse(specialSettingsJson) as unknown;
    if (Array.isArray(parsed)) {
      return parsed.filter(isParsedRequestLogSpecialSetting);
    }
    return isParsedRequestLogSpecialSetting(parsed) ? [parsed] : [];
  } catch {
    return [];
  }
}

function isParsedRequestLogSpecialSetting(value: unknown): value is ParsedRequestLogSpecialSetting {
  return typeof value === "object" && value !== null;
}

function auditTag(label: string, className: string, title?: string): RequestLogAuditTag {
  return { label, className, title };
}

export function buildRequestLogAuditMeta(log: RequestLogAuditInput): RequestLogAuditMeta {
  const settings = parseRequestLogSpecialSettings(log.special_settings_json);
  const settingTypes = new Set(settings.map((item) => item.type).filter(Boolean));
  const isWarmupIntercept = settingTypes.has("warmup_intercept");
  const isCliProxyGuard = settingTypes.has("cli_proxy_guard");
  const isClientAbort =
    !!(log.error_code && CLIENT_ABORT_ERROR_CODES.has(log.error_code)) ||
    settingTypes.has("client_abort");
  const excludedFromStats = !!log.excluded_from_stats;

  const tags: RequestLogAuditTag[] = [];

  if (isWarmupIntercept) {
    tags.push(
      auditTag(
        "Warmup",
        "bg-sky-50/80 text-sky-700 ring-1 ring-inset ring-sky-500/10 dark:bg-sky-500/15 dark:text-sky-200 dark:ring-sky-400/20",
        "Anthropic warmup 命中后直接由网关拦截"
      )
    );
  }

  if (isCliProxyGuard) {
    tags.push(
      auditTag(
        "代理守卫",
        "bg-orange-50/80 text-orange-700 ring-1 ring-inset ring-orange-500/10 dark:bg-orange-500/15 dark:text-orange-200 dark:ring-orange-400/20",
        "CLI 代理守卫提前处理了这次请求"
      )
    );
  }

  if (isClientAbort) {
    tags.push(
      auditTag(
        "客户端中断",
        "bg-amber-50/80 text-amber-700 ring-1 ring-inset ring-amber-500/10 dark:bg-amber-500/15 dark:text-amber-200 dark:ring-amber-400/20",
        "请求已被客户端主动中断"
      )
    );
  }

  if (excludedFromStats) {
    tags.push(
      auditTag(
        "不计统计",
        "bg-slate-100/90 text-slate-600 ring-1 ring-inset ring-slate-500/10 dark:bg-slate-700/70 dark:text-slate-200 dark:ring-slate-500/20",
        "这条记录保留在审计列表中，但不会进入 usage/cost/provider 聚合"
      )
    );
  }

  let summary: string | null = null;
  if (isWarmupIntercept) {
    summary = "Warmup 命中后由网关直接应答，仅保留审计记录，不进入统计。";
  } else if (isCliProxyGuard) {
    summary = "这次请求由 CLI 代理守卫提前处理，保留为审计行。";
  } else if (isClientAbort) {
    summary = "客户端中途中断了请求，系统保留这条审计记录但不计入统计。";
  } else if (excludedFromStats) {
    summary = "这条记录仅用于审计可见性，不参与统计聚合。";
  }

  return {
    muted: tags.length > 0,
    summary,
    tags,
    providerFallbackText: isWarmupIntercept ? "Warmup" : isCliProxyGuard ? "CLI 守卫" : null,
  };
}

/** Max age (ms) before a request log with no final status is treated as abandoned. */
const STALE_IN_PROGRESS_THRESHOLD_MS = 10 * 60 * 1000;

export function isPersistedRequestLogInProgress(log: RequestLogProgressInput) {
  if (log.status != null || (log.error_code ?? null) != null) return false;

  // Staleness guard: orphaned logs (crash / disconnect) stay status-null forever.
  // After the threshold they are no longer considered "in progress".
  const ms = log.created_at_ms && log.created_at_ms > 0 ? log.created_at_ms : 0;
  const createdAtMs = ms > 0 ? ms : log.created_at ? log.created_at * 1000 : 0;
  if (createdAtMs > 0 && Date.now() - createdAtMs > STALE_IN_PROGRESS_THRESHOLD_MS) {
    return false;
  }

  return true;
}

export type LiveTraceProvider = {
  providerId: number | null;
  providerName: string;
};

export function resolveLiveTraceProvider(
  trace: TraceSession | null | undefined
): LiveTraceProvider | null {
  const attempts = trace?.attempts;
  if (!attempts?.length) return null;

  let best: (typeof attempts)[number] | null = null;
  for (const attempt of attempts) {
    const name = attempt.provider_name?.trim();
    if (!name || name === "Unknown") continue;
    if (!best || attempt.attempt_index > best.attempt_index) {
      best = attempt;
    }
  }
  if (!best) return null;
  return {
    providerId: typeof best.provider_id === "number" ? best.provider_id : null,
    providerName: (best.provider_name ?? "").trim(),
  };
}

export function resolveLiveTraceDurationMs(
  trace: TraceSession | null | undefined,
  nowMs = Date.now()
) {
  if (!trace) return null;
  return Math.max(0, nowMs - trace.first_seen_ms);
}

export function getErrorCodeLabel(errorCode: string) {
  return getGatewayErrorShortLabel(errorCode);
}

export function SessionReuseBadge({ showCustomTooltip }: { showCustomTooltip: boolean }) {
  const className =
    "inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-indigo-50/80 px-2 py-0.5 text-[11px] font-semibold text-indigo-600 ring-1 ring-inset ring-indigo-500/10 dark:bg-indigo-500/15 dark:text-indigo-300 dark:ring-indigo-400/20 cursor-help";
  return showCustomTooltip ? (
    <Tooltip content={SESSION_REUSE_TOOLTIP}>
      <span className={className}>会话复用</span>
    </Tooltip>
  ) : (
    <span className={className} title={SESSION_REUSE_TOOLTIP}>
      会话复用
    </span>
  );
}

export function FreeBadge() {
  return (
    <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-emerald-50/80 px-2 py-0.5 text-[11px] font-semibold text-emerald-600 ring-1 ring-inset ring-emerald-500/10 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/20">
      免费
    </span>
  );
}

export function FolderBadge({
  folderName,
  folderPath,
  allowWrap = false,
}: {
  folderName: string;
  folderPath: string;
  allowWrap?: boolean;
}) {
  return (
    <span
      className="inline-flex min-w-0 items-center gap-1 rounded-md bg-slate-100/75 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:bg-slate-700/55 dark:text-slate-200"
      title={folderPath}
    >
      <FolderOpen className="h-3 w-3 shrink-0 text-slate-400 dark:text-slate-500" />
      <span className={allowWrap ? "whitespace-normal break-all" : "truncate"}>{folderName}</span>
    </span>
  );
}

export type StatusBadge = {
  text: string;
  semanticText: string;
  tone: string;
  title?: string;
  isError: boolean;
  isClientAbort: boolean;
  hasFailover: boolean;
};

export function computeStatusBadge(input: {
  status: number | null;
  errorCode: string | null;
  inProgress?: boolean;
  hasFailover?: boolean;
}): StatusBadge {
  if (input.inProgress) {
    return {
      text: "进行中",
      semanticText: "请求进行中",
      tone: "bg-accent/10 text-accent ring-1 ring-inset ring-accent/15",
      isError: false,
      isClientAbort: false,
      hasFailover: !!input.hasFailover,
    };
  }

  const isClientAbort = !!(input.errorCode && CLIENT_ABORT_ERROR_CODES.has(input.errorCode));
  const hasFailover = !!input.hasFailover;
  const isSuccessStatus = input.status != null && input.status >= 200 && input.status < 400;
  const isError = input.status != null ? input.status >= 400 : input.errorCode != null;

  let text = STATUS_TEXT_UNKNOWN;
  let semanticText = STATUS_TEXT_UNKNOWN;

  if (isClientAbort) {
    text = input.status == null ? "已中断" : `${input.status} 已中断`;
    semanticText = "客户端已中断";
  } else if (isSuccessStatus && hasFailover) {
    text = input.status == null ? "切换后成功" : `${input.status} 切换后成功`;
    semanticText = "切换供应商后成功";
  } else if (isSuccessStatus) {
    text = input.status == null ? "成功" : `${input.status} 成功`;
    semanticText = "请求成功";
  } else if (isError) {
    text = input.status == null ? "失败" : `${input.status} 失败`;
    semanticText = "请求失败";
  }

  const tone = isClientAbort
    ? "bg-amber-50 text-amber-600 ring-1 ring-inset ring-amber-500/15 dark:bg-amber-500/15 dark:text-amber-400 dark:ring-amber-400/25"
    : isSuccessStatus
      ? hasFailover
        ? "text-emerald-600 bg-emerald-50/60 ring-1 ring-inset ring-amber-400/30 dark:text-emerald-400 dark:bg-emerald-500/15 dark:ring-amber-500/30"
        : "text-emerald-600 bg-emerald-50/60 ring-1 ring-inset ring-emerald-500/10 dark:text-emerald-400 dark:bg-emerald-500/15 dark:ring-emerald-400/20"
      : isError
        ? "text-rose-600 bg-rose-50/60 ring-1 ring-inset ring-rose-500/10 dark:text-rose-400 dark:bg-rose-500/15 dark:ring-rose-400/20"
        : "text-slate-500 bg-slate-100 ring-1 ring-inset ring-slate-500/10 dark:text-slate-400 dark:bg-slate-700 dark:ring-slate-500/20";

  const title = input.errorCode
    ? `${semanticText} · ${getErrorCodeLabel(input.errorCode)} (${input.errorCode})`
    : semanticText;

  return { text, semanticText, tone, title, isError, isClientAbort, hasFailover };
}

export function computeEffectiveInputTokens(
  cliKey: CliKey | string,
  inputTokens: number | null,
  cacheReadInputTokens: number | null
) {
  if (inputTokens == null) return null;
  const cacheRead = cacheReadInputTokens ?? 0;
  if (cliKey === "codex" || cliKey === "gemini") return Math.max(inputTokens - cacheRead, 0);
  return inputTokens;
}

export function buildRequestRouteMeta(input: {
  route: RequestLogRouteHop[] | null | undefined;
  status: number | null;
  hasFailover: boolean;
  attemptCount: number;
}) {
  const hops = input.route ?? [];
  if (hops.length === 0) {
    return {
      hasRoute: false,
      label: "链路",
      summary: "暂无链路信息",
      tooltipText: null as string | null,
      tooltipContent: null as React.ReactNode,
    };
  }

  const totalHopAttempts = hops.reduce((sum, h) => sum + (h.attempts ?? 1), 0);
  const skippedCount = Math.max(0, input.attemptCount - totalHopAttempts);
  const hasRetry = hops.some((h) => (h.attempts ?? 1) > 1);

  const summary = input.hasFailover
    ? `切换 ${input.attemptCount} 次后${input.status != null && input.status < 400 ? "成功" : "结束"}`
    : skippedCount > 0 && hasRetry
      ? `跳过 ${skippedCount} 个候选，并重试 ${input.attemptCount} 次`
      : skippedCount > 0
        ? `跳过 ${skippedCount} 个候选`
        : hasRetry
          ? `同一供应商重试 ${input.attemptCount} 次`
          : "直连完成";

  // 纯文本 fallback（用于 title 属性）
  const tooltipText = hops
    .map((hop, idx) => {
      const rawProviderName = hop.provider_name?.trim();
      const providerName =
        !rawProviderName || rawProviderName === "Unknown" ? "未知" : rawProviderName;
      const status = hop.status ?? (idx === hops.length - 1 ? input.status : null) ?? null;
      const statusText = status == null ? "状态未知" : String(status);
      const attemptsSuffix = hop.attempts && hop.attempts > 1 ? `，尝试 ${hop.attempts} 次` : "";
      if (hop.ok) return `${providerName}（${statusText}，成功${attemptsSuffix}）`;
      if (hop.skipped) return `${providerName}（已跳过${attemptsSuffix}）`;
      const errorCode = hop.error_code ?? null;
      const errorLabel = errorCode ? getErrorCodeLabel(errorCode) : "失败";
      return `${providerName}（${statusText}，${errorLabel}${attemptsSuffix}）`;
    })
    .join(" → ");

  let label = summary;
  if (input.hasFailover) {
    label = `切换 ${input.attemptCount} 次`;
  } else if (skippedCount > 0 && hasRetry) {
    label = `跳过 ${skippedCount} 个 + 重试`;
  } else if (skippedCount > 0) {
    label = `跳过 ${skippedCount} 个`;
  } else if (hasRetry) {
    label = `重试 ${input.attemptCount} 次`;
  }

  const tooltipContent = (
    <RouteTooltipContent
      hops={hops}
      finalStatus={input.status}
      summary={summary}
      skippedCount={skippedCount}
    />
  );

  return {
    hasRoute: true,
    label,
    summary,
    tooltipText,
    tooltipContent,
  };
}
