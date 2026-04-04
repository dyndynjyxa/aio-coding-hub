import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Copy, FlaskConical, Pencil, RefreshCw, Terminal, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import { FREE_TAG } from "../../constants/providers";
import type { GatewayProviderCircuitStatus } from "../../services/gateway";
import { getGatewayCircuitDerivedState } from "../../query/gateway";
import {
  providerOAuthFetchLimits,
  type ClaudeModels,
  type OAuthLimitsResult,
  type ProviderSummary,
} from "../../services/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { useNowUnix } from "../../hooks/useNowUnix";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds, formatUnixSeconds, formatUsdRaw } from "../../utils/formatters";
import { providerBaseUrlSummary } from "./baseUrl";

/** Module-level cache so OAuth limits survive tab switches / re-renders. */
const oauthLimitsCache = new Map<number, OAuthLimitsResult>();
const NOTE_URL_RE = /https?:\/\/[^\s]+/g;

function getOAuthShortWindowLabel(
  provider: ProviderSummary,
  oauthLimits: OAuthLimitsResult | null
) {
  if (provider.cli_key === "gemini") return "短窗";
  return oauthLimits?.limit_short_label ?? "5h";
}

function getConfiguredClaudeModelMappings(claudeModels: ClaudeModels | null | undefined) {
  const fields: Array<[label: string, value: string | null | undefined]> = [
    ["主模型", claudeModels?.main_model],
    ["推理模型(Thinking)", claudeModels?.reasoning_model],
    ["Haiku 默认模型", claudeModels?.haiku_model],
    ["Sonnet 默认模型", claudeModels?.sonnet_model],
    ["Opus 默认模型", claudeModels?.opus_model],
  ];

  return fields.flatMap(([label, value]) => {
    const trimmed = typeof value === "string" ? value.trim() : "";
    return trimmed ? [`${label}: ${trimmed}`] : [];
  });
}

function trimTrailingUrlPunctuation(url: string) {
  return url.replace(/[.,!?;:，。；：]+$/u, "");
}

function providerTagClassName(tag: string) {
  if (tag === FREE_TAG) {
    return "shrink-0 rounded-full bg-emerald-100 px-2 py-0.5 text-[10px] text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300";
  }
  return "shrink-0 rounded-full bg-slate-50 px-2 py-0.5 text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300";
}

function renderProviderNote(note: string) {
  const nodes: ReactNode[] = [];
  let lastIndex = 0;

  for (const match of note.matchAll(NOTE_URL_RE)) {
    const rawUrl = match[0];
    const start = match.index ?? 0;
    const normalizedUrl = trimTrailingUrlPunctuation(rawUrl);
    const trailingText = rawUrl.slice(normalizedUrl.length);

    if (start > lastIndex) {
      nodes.push(note.slice(lastIndex, start));
    }

    nodes.push(
      <a
        key={`${normalizedUrl}-${start}`}
        href={normalizedUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="text-sky-600 underline underline-offset-2 transition hover:text-sky-700 dark:text-sky-400 dark:hover:text-sky-300"
      >
        {normalizedUrl}
      </a>
    );

    if (trailingText) {
      nodes.push(trailingText);
    }

    lastIndex = start + rawUrl.length;
  }

  if (lastIndex < note.length) {
    nodes.push(note.slice(lastIndex));
  }

  return nodes.length > 0 ? nodes : [note];
}

export type SortableProviderCardProps = {
  provider: ProviderSummary;
  sourceProviderName?: string | null;
  circuit: GatewayProviderCircuitStatus | null;
  circuitResetting: boolean;
  onToggleEnabled: (provider: ProviderSummary) => void;
  onResetCircuit: (provider: ProviderSummary) => void;
  onCopyTerminalLaunchCommand?: (provider: ProviderSummary) => void;
  terminalLaunchCopying?: boolean;
  onValidateModel?: (provider: ProviderSummary) => void;
  onDuplicate?: (provider: ProviderSummary) => void;
  duplicateLoading?: boolean;
  onEdit: (provider: ProviderSummary) => void;
  onDelete: (provider: ProviderSummary) => void;
};

export function SortableProviderCard({
  provider,
  sourceProviderName = null,
  circuit,
  circuitResetting,
  onToggleEnabled,
  onResetCircuit,
  onCopyTerminalLaunchCommand,
  terminalLaunchCopying = false,
  onValidateModel,
  onDuplicate,
  duplicateLoading = false,
  onEdit,
  onDelete,
}: SortableProviderCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: provider.id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const claudeModelMappings = getConfiguredClaudeModelMappings(provider.claude_models);
  const claudeModelsCount = claudeModelMappings.length;
  const hasClaudeModels = claudeModelsCount > 0;

  const limitChips = [
    provider.limit_5h_usd != null ? `5h ≤ ${formatUsdRaw(provider.limit_5h_usd)}` : null,
    provider.limit_daily_usd != null
      ? `日 ≤ ${formatUsdRaw(provider.limit_daily_usd)}（${
          provider.daily_reset_mode === "fixed" ? `固定 ${provider.daily_reset_time}` : "滚动 24h"
        }）`
      : null,
    provider.limit_weekly_usd != null ? `周 ≤ ${formatUsdRaw(provider.limit_weekly_usd)}` : null,
    provider.limit_monthly_usd != null ? `月 ≤ ${formatUsdRaw(provider.limit_monthly_usd)}` : null,
    provider.limit_total_usd != null
      ? `总 ≤ ${formatUsdRaw(provider.limit_total_usd)}（无重置）`
      : null,
  ].filter((v): v is string => Boolean(v));
  const hasLimits = limitChips.length > 0;

  const circuitState = useMemo(() => getGatewayCircuitDerivedState(circuit), [circuit]);
  const { isUnavailable, unavailableUntil } = circuitState;
  const isOAuth = provider.auth_mode === "oauth";
  const [apiKeyDetailsVisible, setApiKeyDetailsVisible] = useState(false);
  const [oauthLimits, setOauthLimits] = useState<OAuthLimitsResult | null>(
    () => oauthLimitsCache.get(provider.id) ?? null
  );
  const [limitsLoading, setLimitsLoading] = useState(false);
  const oauthShortLabel = getOAuthShortWindowLabel(provider, oauthLimits);
  const shouldTrackNowUnix =
    isUnavailable ||
    (isOAuth &&
      oauthLimits != null &&
      (oauthLimits.limit_5h_reset_at != null || oauthLimits.limit_weekly_reset_at != null));
  const nowUnix = useNowUnix(shouldTrackNowUnix);
  const unavailableRemaining =
    unavailableUntil != null ? Math.max(0, unavailableUntil - nowUnix) : null;
  const unavailableCountdown =
    unavailableRemaining != null ? formatCountdownSeconds(unavailableRemaining) : null;

  // OAuth 限制重置倒计时
  const limitsResetCountdown = useMemo(() => {
    if (!isOAuth || !oauthLimits) return null;
    const reset5h = oauthLimits.limit_5h_reset_at;
    const resetWeekly = oauthLimits.limit_weekly_reset_at;
    if (!reset5h && !resetWeekly) return null;

    const formatReset = (timestamp: number) => {
      const diff = timestamp - nowUnix;
      if (diff <= 0) return "已重置";
      const totalMinutes = Math.floor(diff / 60);
      if (totalMinutes < 1) return "<1m";
      const days = Math.floor(totalMinutes / 1440);
      const hours = Math.floor((totalMinutes % 1440) / 60);
      const minutes = totalMinutes % 60;
      if (days > 0) return `${days}d ${hours}h ${minutes}m`;
      if (hours > 0) return `${hours}h ${minutes}m`;
      return `${minutes}m`;
    };

    return {
      reset5h: reset5h ? formatReset(reset5h) : null,
      resetWeekly: resetWeekly ? formatReset(resetWeekly) : null,
    };
  }, [isOAuth, oauthLimits, nowUnix]);

  useEffect(() => {
    // Disconnect switches auth_mode back to api_key; drop stale OAuth limits cache.
    if (isOAuth) {
      setApiKeyDetailsVisible(false);
      return;
    }
    oauthLimitsCache.delete(provider.id);
    setOauthLimits(null);
  }, [isOAuth, provider.id]);

  // 组件加载时获取 OAuth limits
  useEffect(() => {
    if (!isOAuth) return;
    const cached = oauthLimitsCache.get(provider.id);
    if (cached) {
      setOauthLimits(cached);
    } else {
      fetchLimits();
    }
    const interval = setInterval(fetchLimits, 60000);
    return () => clearInterval(interval);
  }, [isOAuth, provider.id]);

  async function fetchLimits() {
    if (!isOAuth) return;
    setLimitsLoading(true);
    try {
      const result = await providerOAuthFetchLimits(provider.id);
      if (!result) {
        const value: OAuthLimitsResult = { limit_5h_text: null, limit_weekly_text: null };
        oauthLimitsCache.set(provider.id, value);
        setOauthLimits(value);
        toast("获取 OAuth 用量失败");
        logToConsole("warn", `获取 OAuth 用量失败：${provider.name}`, {
          provider_id: provider.id,
          cli_key: provider.cli_key,
        });
        return;
      }
      const value = result;
      oauthLimitsCache.set(provider.id, value);
      setOauthLimits(value);
    } catch (err) {
      const value: OAuthLimitsResult = { limit_5h_text: null, limit_weekly_text: null };
      oauthLimitsCache.set(provider.id, value);
      setOauthLimits(value);
      toast(`获取 OAuth 用量失败：${String(err)}`);
      logToConsole("error", `获取 OAuth 用量异常：${provider.name}`, {
        provider_id: provider.id,
        cli_key: provider.cli_key,
        error: String(err),
      });
    } finally {
      setLimitsLoading(false);
    }
  }

  return (
    <div ref={setNodeRef} style={style} className="relative">
      <Card
        padding="sm"
        className={cn(
          "rounded-lg sm:rounded-xl flex cursor-grab flex-col gap-2 transition-shadow duration-200 active:cursor-grabbing sm:flex-row sm:items-center sm:justify-between",
          isDragging && "z-10 scale-[1.02] shadow-lg ring-2 ring-accent/30"
        )}
        {...attributes}
        {...listeners}
      >
        <div className="flex min-w-0 items-center gap-3">
          <div className="inline-flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-lg border border-slate-200 bg-white text-slate-400 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-500">
            ⠿
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 items-center gap-2">
              <div className="truncate text-base font-semibold">{provider.name}</div>
              {isUnavailable ? (
                <span
                  className="shrink-0 rounded-full bg-rose-50 px-2 py-0.5 font-mono text-[10px] text-rose-700 dark:bg-rose-900/30 dark:text-rose-400"
                  title={
                    unavailableUntil != null
                      ? `熔断至 ${formatUnixSeconds(unavailableUntil)}`
                      : "熔断"
                  }
                >
                  熔断{unavailableCountdown ? ` ${unavailableCountdown}` : ""}
                </span>
              ) : null}
            </div>
            <div className="mt-1 flex min-w-0 flex-wrap items-center gap-2">
              {isOAuth ? (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    fetchLimits();
                  }}
                  disabled={limitsLoading}
                  className={cn(
                    "inline-flex w-16 shrink-0 cursor-pointer items-center justify-center gap-1 rounded-full px-2 py-0.5 font-mono text-[10px] transition-opacity hover:opacity-80",
                    provider.oauth_last_error
                      ? "bg-rose-50 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400"
                      : "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
                  )}
                  title={
                    provider.oauth_last_error
                      ? `OAuth 错误: ${provider.oauth_last_error}（点击刷新用量）`
                      : provider.oauth_email
                        ? `OAuth: ${provider.oauth_email}（点击刷新用量）`
                        : "OAuth 已连接（点击刷新用量）"
                  }
                >
                  <RefreshCw className={cn("h-2.5 w-2.5", limitsLoading && "animate-spin")} />
                  OAuth
                </button>
              ) : provider.source_provider_id != null ? (
                <span
                  className="inline-flex w-16 shrink-0 items-center justify-center rounded-full px-2 py-0.5 font-mono text-[10px] bg-violet-50 text-violet-700 dark:bg-violet-900/30 dark:text-violet-400"
                  title="CX2CC 转译模式"
                >
                  CX2CC
                </span>
              ) : (
                <>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setApiKeyDetailsVisible((current) => !current);
                    }}
                    className="inline-flex w-16 shrink-0 cursor-pointer items-center justify-center rounded-full px-2 py-0.5 font-mono text-[10px] transition-opacity hover:opacity-80 bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400"
                    title="API Key 认证"
                  >
                    API Key
                  </button>
                  <span className="shrink-0 rounded-full bg-cyan-50 px-2 py-0.5 font-mono text-[10px] text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-300">
                    {provider.base_url_mode === "ping" ? "Ping" : "顺序"}
                  </span>
                </>
              )}
              {provider.cli_key === "claude" && hasClaudeModels ? (
                <span
                  className="shrink-0 rounded-full bg-sky-50 px-2 py-0.5 font-mono text-[10px] text-sky-700 dark:bg-sky-900/30 dark:text-sky-400"
                  title={[
                    `已配置 Claude 模型映射（${claudeModelsCount}/5）`,
                    ...claudeModelMappings,
                  ].join("\n")}
                >
                  模型映射 {claudeModelsCount}/5
                </span>
              ) : null}
              {hasLimits ? (
                <span
                  className="shrink-0 rounded-full bg-amber-50 px-2 py-0.5 font-mono text-[10px] text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                  title={limitChips.join("\n")}
                >
                  限额
                </span>
              ) : null}
              {(provider.tags ?? []).map((tag) => (
                <span key={tag} className={providerTagClassName(tag)} title={`标签: ${tag}`}>
                  {tag}
                </span>
              ))}
            </div>
            <div className="mt-1 flex min-w-0 flex-wrap items-center gap-2">
              {isOAuth ? (
                <>
                  {provider.oauth_email ? (
                    <span
                      className="truncate font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                      title={`OAuth: ${provider.oauth_email}`}
                    >
                      {provider.oauth_email}
                    </span>
                  ) : null}
                  {oauthLimits?.limit_5h_text ? (
                    <span
                      className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                      title={`${oauthShortLabel} 用量: ${oauthLimits.limit_5h_text}`}
                    >
                      {oauthShortLabel}: {oauthLimits.limit_5h_text}
                    </span>
                  ) : null}
                  {oauthLimits?.limit_weekly_text ? (
                    <span
                      className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                      title={`周用量: ${oauthLimits.limit_weekly_text}`}
                    >
                      周: {oauthLimits.limit_weekly_text}
                    </span>
                  ) : null}
                  {limitsResetCountdown?.reset5h && oauthLimits?.limit_5h_text ? (
                    <span
                      className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                      title={`${oauthShortLabel} 重置: ${limitsResetCountdown.reset5h}`}
                    >
                      重置: {limitsResetCountdown.reset5h}
                    </span>
                  ) : null}
                  {limitsResetCountdown?.resetWeekly && oauthLimits?.limit_weekly_text ? (
                    <span
                      className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                      title={`周重置: ${limitsResetCountdown.resetWeekly}`}
                    >
                      周重置: {limitsResetCountdown.resetWeekly}
                    </span>
                  ) : null}
                </>
              ) : provider.source_provider_id != null ? (
                <span
                  className="truncate font-mono text-xs text-violet-500 dark:text-violet-400 cursor-default"
                  title={`源 Codex 供应商: ${sourceProviderName ?? `#${provider.source_provider_id}`}`}
                >
                  源: {sourceProviderName ?? `#${provider.source_provider_id}`}
                </span>
              ) : apiKeyDetailsVisible ? (
                <span
                  className="truncate font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
                  title={provider.base_urls.join("\n")}
                >
                  {providerBaseUrlSummary(provider)}
                </span>
              ) : null}
            </div>
            {provider.note ? (
              <div
                className="mt-1 break-words text-xs text-slate-400 dark:text-slate-500 cursor-default"
                title={provider.note}
                onPointerDown={(e) => e.stopPropagation()}
              >
                {renderProviderNote(provider.note)}
              </div>
            ) : null}
          </div>
        </div>

        <div className="flex flex-col items-end gap-2" onPointerDown={(e) => e.stopPropagation()}>
          <div className="flex flex-wrap items-center justify-end gap-2">
            {isUnavailable ? (
              <Button
                onClick={() => onResetCircuit(provider)}
                variant="secondary"
                size="md"
                className="h-9"
                disabled={circuitResetting}
              >
                {circuitResetting ? "处理中…" : "解除熔断"}
              </Button>
            ) : null}

            <Button
              onClick={() => onEdit(provider)}
              variant="secondary"
              size="md"
              className="h-9"
              title="编辑"
            >
              <Pencil className="h-4 w-4" />
              编辑
            </Button>

            <div className="inline-flex h-9 items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 text-sm shadow-sm dark:border-slate-600 dark:bg-slate-800">
              <span className="text-sm font-medium text-slate-700 dark:text-slate-300">
                {provider.enabled ? "已启用" : "已关闭"}
              </span>
              <Switch
                checked={provider.enabled}
                onCheckedChange={() => onToggleEnabled(provider)}
              />
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-end gap-2">
            {onCopyTerminalLaunchCommand ? (
              <Button
                onClick={() => onCopyTerminalLaunchCommand(provider)}
                variant="secondary"
                size="sm"
                className="px-2 py-1 text-[11px] gap-1.5"
                disabled={terminalLaunchCopying}
                title="复制终端启动命令"
              >
                <Terminal className="h-3.5 w-3.5" />
                {terminalLaunchCopying ? "复制中…" : "终端启动"}
              </Button>
            ) : null}

            {onValidateModel ? (
              <Button
                onClick={() => onValidateModel(provider)}
                variant="secondary"
                size="sm"
                className="px-2 py-1 text-[11px] gap-1.5"
                title="模型验证"
              >
                <FlaskConical className="h-3.5 w-3.5" />
                模型验证
              </Button>
            ) : null}

            {onDuplicate ? (
              <Button
                onClick={() => onDuplicate(provider)}
                variant="secondary"
                size="sm"
                className="px-2 py-1 text-[11px] gap-1.5"
                disabled={duplicateLoading}
                title="复制"
              >
                <Copy className="h-3.5 w-3.5" />
                {duplicateLoading ? "复制中…" : "复制"}
              </Button>
            ) : null}

            <Button
              onClick={() => onDelete(provider)}
              variant="danger"
              size="sm"
              className="px-2 py-1 text-[11px] gap-1.5"
              title="删除"
            >
              <Trash2 className="h-3.5 w-3.5" />
              删除
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
