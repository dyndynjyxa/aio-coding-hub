// Usage:
// - Import helpers/components from this module for Home "request logs" list and "realtime traces" cards.
// - Designed to keep status badge / error_code label / session reuse tooltip consistent across the Home page.

import { GatewayErrorCodes } from "../../constants/gatewayErrorCodes";
import type { CliKey } from "../../services/providers";
import type { RequestLogRouteHop } from "../../services/requestLogs";
import { Tooltip } from "../../ui/Tooltip";
import { RouteTooltipContent } from "./RouteTooltipContent";

const ERROR_CODE_LABELS: Record<string, string> = {
  [GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE]: "全部不可用",
  [GatewayErrorCodes.UPSTREAM_ALL_FAILED]: "全部失败",
  [GatewayErrorCodes.NO_ENABLED_PROVIDER]: "无供应商",
  [GatewayErrorCodes.UPSTREAM_TIMEOUT]: "上游超时",
  [GatewayErrorCodes.UPSTREAM_CONNECT_FAILED]: "连接失败",
  [GatewayErrorCodes.UPSTREAM_5XX]: "上游5XX",
  [GatewayErrorCodes.UPSTREAM_4XX]: "上游4XX",
  [GatewayErrorCodes.UPSTREAM_READ_ERROR]: "读取错误",
  [GatewayErrorCodes.STREAM_ERROR]: "流错误",
  [GatewayErrorCodes.STREAM_ABORTED]: "流中断",
  [GatewayErrorCodes.STREAM_IDLE_TIMEOUT]: "流空闲超时",
  [GatewayErrorCodes.REQUEST_ABORTED]: "请求中断",
  [GatewayErrorCodes.INTERNAL_ERROR]: "内部错误",
  [GatewayErrorCodes.BODY_TOO_LARGE]: "请求过大",
  [GatewayErrorCodes.INVALID_CLI_KEY]: "无效CLI",
  [GatewayErrorCodes.INVALID_BASE_URL]: "无效URL",
  [GatewayErrorCodes.PORT_IN_USE]: "端口占用",
  [GatewayErrorCodes.RESPONSE_BUILD_ERROR]: "响应构建错误",
  [GatewayErrorCodes.PROVIDER_RATE_LIMITED]: "供应商限额",
  [GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN]: "供应商熔断",
};

const CLIENT_ABORT_ERROR_CODES: ReadonlySet<string> = new Set([
  GatewayErrorCodes.STREAM_ABORTED,
  GatewayErrorCodes.REQUEST_ABORTED,
]);

const SESSION_REUSE_TOOLTIP =
  "同一 session_id 在 5 分钟 TTL 内优先复用上一次成功 provider，减少抖动/提升缓存命中";

export function getErrorCodeLabel(errorCode: string) {
  return ERROR_CODE_LABELS[errorCode] ?? errorCode;
}

export function SessionReuseBadge({ showCustomTooltip }: { showCustomTooltip: boolean }) {
  const className =
    "inline-flex items-center rounded-md bg-indigo-50/75 px-2 py-0.5 text-[11px] font-medium text-indigo-600 dark:bg-indigo-900/25 dark:text-indigo-200 cursor-help";
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
    <span className="inline-flex items-center rounded-md bg-emerald-50/75 px-2 py-0.5 text-[11px] font-medium text-emerald-600 dark:bg-emerald-900/25 dark:text-emerald-200">
      免费
    </span>
  );
}

export type StatusBadge = {
  text: string;
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
      tone: "bg-accent/10 text-accent",
      isError: false,
      isClientAbort: false,
      hasFailover: !!input.hasFailover,
    };
  }

  const isClientAbort = !!(input.errorCode && CLIENT_ABORT_ERROR_CODES.has(input.errorCode));
  const isError = input.status != null ? input.status >= 400 : input.errorCode != null;
  const hasFailover = !!input.hasFailover;

  const text = input.status == null ? "—" : String(input.status);
  const tone = isClientAbort
    ? "bg-amber-50 text-amber-600 border border-amber-200/60 dark:bg-amber-900/30 dark:text-amber-400 dark:border-amber-700/60"
    : input.status != null && input.status >= 200 && input.status < 400
      ? hasFailover
        ? "text-emerald-600 bg-emerald-50/50 border border-amber-300/60 dark:text-emerald-400 dark:bg-emerald-900/30 dark:border-amber-600/60"
        : "text-emerald-600 bg-emerald-50/50 dark:text-emerald-400 dark:bg-emerald-900/30"
      : isError
        ? "text-rose-600 bg-rose-50/50 dark:text-rose-400 dark:bg-rose-900/30"
        : "text-slate-500 bg-slate-100 dark:text-slate-400 dark:bg-slate-700";

  const title = input.errorCode
    ? `${getErrorCodeLabel(input.errorCode)} (${input.errorCode})`
    : undefined;

  return { text, tone, title, isError, isClientAbort, hasFailover };
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
      tooltipText: null as string | null,
      tooltipContent: null as React.ReactNode,
    };
  }

  // 纯文本 fallback（用于 title 属性）
  const tooltipText = hops
    .map((hop, idx) => {
      const rawProviderName = hop.provider_name?.trim();
      const providerName =
        !rawProviderName || rawProviderName === "Unknown" ? "未知" : rawProviderName;
      const status = hop.status ?? (idx === hops.length - 1 ? input.status : null) ?? null;
      const statusText = status == null ? "—" : String(status);
      const attemptsSuffix = hop.attempts && hop.attempts > 1 ? ` x${hop.attempts}` : "";
      if (hop.ok) return `${providerName}(${statusText})${attemptsSuffix}`;
      const errorCode = hop.error_code ?? null;
      const errorLabel = errorCode ? getErrorCodeLabel(errorCode) : "失败";
      return `${providerName}(${statusText} ${errorLabel})${attemptsSuffix}`;
    })
    .join(" -> ");

  // 标签: 区分"降级"（切换 provider）、"重试"（同一 provider 多次尝试）、"跳过"（有 provider 被 skip）
  // skipped 的 provider 不在 hops 中，通过 attemptCount 与 hop attempts 差值推算
  const totalHopAttempts = hops.reduce((sum, h) => sum + (h.attempts ?? 1), 0);
  const skippedCount = input.attemptCount - totalHopAttempts;
  const hasRetry = hops.some((h) => (h.attempts ?? 1) > 1);

  let label = "链路";
  if (input.hasFailover) {
    // 真正切换了 provider（route 中有多个 hop）
    label = `链路[降级*${input.attemptCount}]`;
  } else if (skippedCount > 0 && hasRetry) {
    label = `链路[跳过*${skippedCount}+重试]`;
  } else if (skippedCount > 0) {
    label = `链路[跳过*${skippedCount}]`;
  } else if (hasRetry) {
    label = `链路[重试*${input.attemptCount}]`;
  }

  // 富文本 tooltip 内容
  const tooltipContent = <RouteTooltipContent hops={hops} finalStatus={input.status} />;

  return {
    hasRoute: true,
    label,
    tooltipText,
    tooltipContent,
  };
}
