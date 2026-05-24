// Usage:
// - Import helpers/components from this module for Home "request logs" list and "realtime traces" cards.
// - Designed to keep status badge / error_code label / session reuse tooltip consistent across the Home page.

import { GatewayErrorCodes, getGatewayErrorShortLabel } from "../../constants/gatewayErrorCodes";
import {
  normalizeClaudeModelMapping,
  type ClaudeModelMapping,
} from "../../services/gateway/claudeModelMapping";
import type { CliKey } from "../../services/providers/providers";
import type { RequestLogRouteHop } from "../../services/gateway/requestLogs";
import type { TraceSession } from "../../services/gateway/traceStore";
import { Tooltip } from "../../ui/Tooltip";
import { computeEffectiveInputTokens as computeSharedEffectiveInputTokens } from "../../utils/cacheRateMetrics";
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
  status?: number | null;
  excluded_from_stats?: boolean | null;
  special_settings_json?: string | null;
  error_code?: string | null;
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
} & Record<string, unknown>;

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

function parsedSettingString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function parsedSettingNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
}

function parsedSettingBoolean(value: unknown): boolean {
  return typeof value === "boolean" ? value : false;
}

export function resolveClaudeModelMappingFromSpecialSettings(
  specialSettingsJson: string | null | undefined,
  finalProviderId?: number | null
): ClaudeModelMapping | null {
  const settings = parseRequestLogSpecialSettings(specialSettingsJson);
  const mappings = settings
    .map((setting) => {
      if (setting.type !== "claude_model_mapping") return null;
      return normalizeClaudeModelMapping({
        requestedModel: parsedSettingString(setting.requestedModel),
        effectiveModel: parsedSettingString(setting.effectiveModel),
        mappingKind: parsedSettingString(setting.mappingKind),
        providerId: parsedSettingNumber(setting.providerId),
        providerName: parsedSettingString(setting.providerName),
        applied: parsedSettingBoolean(setting.applied),
      });
    })
    .filter((mapping): mapping is ClaudeModelMapping => mapping !== null);

  if (mappings.length === 0) return null;

  if (finalProviderId != null) {
    const finalProviderMapping = mappings
      .slice()
      .reverse()
      .find((mapping) => mapping.providerId === finalProviderId);
    if (finalProviderMapping) return finalProviderMapping;
  }

  return mappings[mappings.length - 1] ?? null;
}

export function hasClaudeModelMappingSpecialSetting(
  specialSettingsJson: string | null | undefined
): boolean {
  const settings = parseRequestLogSpecialSettings(specialSettingsJson);
  for (const setting of settings) {
    if (setting.type !== "claude_model_mapping") continue;
    return true;
  }
  return false;
}

export function formatClaudeModelMappingText(
  requestedModel: string | null | undefined,
  mapping: ClaudeModelMapping | null | undefined
) {
  const normalized = normalizeClaudeModelMapping(mapping);
  if (normalized) {
    return `${normalized.requestedModel} → ${normalized.effectiveModel}`;
  }

  const fallback = requestedModel?.trim();
  return fallback || "未知";
}

type CodexServiceTierResultSetting = {
  type: "codex_service_tier_result";
  requestedServiceTier?: string | null;
  actualServiceTier?: string | null;
  billingSourcePreference?: string | null;
  resolvedFrom?: string | null;
  effectivePriority?: boolean;
};

function isCodexServiceTierResultSetting(value: unknown): value is CodexServiceTierResultSetting {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as Record<string, unknown>).type === "codex_service_tier_result"
  );
}

/**
 * Check if the request has priority service tier applied (Codex fast mode).
 */
export function hasPriorityServiceTierSpecialSetting(
  specialSettingsJson: string | null | undefined
): boolean {
  if (!specialSettingsJson) return false;

  try {
    const settings = JSON.parse(specialSettingsJson) as unknown;
    if (!Array.isArray(settings)) return false;

    const codexTierSetting = [...settings].reverse().find(isCodexServiceTierResultSetting);
    if (!codexTierSetting) return false;

    // Legacy compatibility: if no billingSourcePreference, check actualServiceTier
    if (
      codexTierSetting.billingSourcePreference == null &&
      codexTierSetting.resolvedFrom == null &&
      codexTierSetting.actualServiceTier != null
    ) {
      return codexTierSetting.actualServiceTier === "priority";
    }

    return codexTierSetting.effectivePriority === true;
  } catch {
    return false;
  }
}

function auditTag(label: string, className: string, title?: string): RequestLogAuditTag {
  return { label, className, title };
}

export function buildRequestLogAuditMeta(log: RequestLogAuditInput): RequestLogAuditMeta {
  const settings = parseRequestLogSpecialSettings(log.special_settings_json);
  const settingTypes = new Set(settings.map((item) => item.type).filter(Boolean));
  const isWarmupIntercept = settingTypes.has("warmup_intercept");
  const isCliProxyGuard = settingTypes.has("cli_proxy_guard");
  const isSuccessful = typeof log.status === "number" && log.status >= 200 && log.status < 300;
  const isClientAbort =
    !isSuccessful &&
    (!!(log.error_code && CLIENT_ABORT_ERROR_CODES.has(log.error_code)) ||
      settingTypes.has("client_abort"));
  const isAllProvidersUnavailable = log.error_code === GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE;
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

  if (isAllProvidersUnavailable) {
    tags.push(
      auditTag(
        "全部不可用",
        "bg-rose-50/80 text-rose-700 ring-1 ring-inset ring-rose-500/10 dark:bg-rose-500/15 dark:text-rose-200 dark:ring-rose-400/20",
        "当前没有可用 Provider，通常是全部处于熔断、冷却或限流状态"
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
  } else if (isAllProvidersUnavailable) {
    summary = "当前没有可用 Provider，网关未继续向已熔断或冷却中的供应商发起上游请求。";
  } else if (isClientAbort) {
    summary = "客户端中途中断了请求，系统保留这条审计记录但不计入统计。";
  } else if (excludedFromStats) {
    summary = "这条记录仅用于审计可见性，不参与统计聚合。";
  }

  return {
    muted: tags.length > 0,
    summary,
    tags,
    providerFallbackText: isWarmupIntercept
      ? "Warmup"
      : isCliProxyGuard
        ? "CLI 守卫"
        : isAllProvidersUnavailable
          ? "无可用供应商"
          : null,
  };
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

const FAST_MODE_TOOLTIP = "Codex 优先服务层 (fast mode) - 使用更高优先级资源，费率更高";

export function FastModeBadge({ showCustomTooltip }: { showCustomTooltip: boolean }) {
  const className =
    "inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-orange-50/80 px-2 py-0.5 text-[11px] font-semibold text-orange-600 ring-1 ring-inset ring-orange-500/10 dark:bg-orange-500/15 dark:text-orange-300 dark:ring-orange-400/20 cursor-help";

  return showCustomTooltip ? (
    <Tooltip content={FAST_MODE_TOOLTIP}>
      <span className={className}>fast</span>
    </Tooltip>
  ) : (
    <span className={className} title={FAST_MODE_TOOLTIP}>
      fast
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
  return computeSharedEffectiveInputTokens(cliKey, inputTokens, cacheReadInputTokens);
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

  const skippedCount = hops
    .filter((h) => h.skipped)
    .reduce((sum, h) => sum + (h.attempts ?? 1), 0);
  const activeAttemptCount = hops
    .filter((h) => !h.skipped)
    .reduce((sum, h) => sum + (h.attempts ?? 1), 0);
  const hasRetry = hops.some((h) => !h.skipped && (h.attempts ?? 1) > 1);

  const summary = input.hasFailover
    ? `切换 ${input.attemptCount} 次后${input.status != null && input.status < 400 ? "成功" : "结束"}`
    : skippedCount > 0 && hasRetry
      ? `跳过 ${skippedCount} 个候选，并重试 ${activeAttemptCount} 次`
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
