import {
  memo,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import { useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Copy, GripVertical, Pencil, RefreshCw, Terminal, Trash2, Zap } from "lucide-react";
import { FREE_TAG } from "../../constants/providers";
import type { GatewayProviderCircuitStatus } from "../../services/gateway/gateway";
import { getGatewayCircuitDerivedState } from "../../query/gateway";
import {
  refreshProviderOAuthLimits,
  resetProviderOAuthCodexQuota,
  useOAuthLimitsQuery,
} from "../../query/providers";
import {
  getProviderTypeInfo,
  type ClaudeModels,
  type ProviderSummary,
} from "../../services/providers/providers";
import { OAuthQuotaUsageInline } from "../../components/providers/OAuthQuotaUsageInline";
import { openDesktopUrl } from "../../services/desktop/opener";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { Switch } from "../../ui/Switch";
import { useNowUnix } from "../../hooks/useNowUnix";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds, formatUnixSeconds, formatUsdRaw } from "../../utils/formatters";
import { providerBaseUrlSummary } from "./baseUrl";

const NOTE_URL_RE = /https?:\/\/[^\s]+/g;

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

async function openProviderNoteUrl(url: string) {
  try {
    await openDesktopUrl(url);
  } catch {
    try {
      window.open(url, "_blank", "noopener,noreferrer");
    } catch {}
  }
}

function handleProviderNoteUrlClick(event: ReactMouseEvent<HTMLAnchorElement>, url: string) {
  event.preventDefault();
  event.stopPropagation();
  void openProviderNoteUrl(url);
}

function providerTagClassName(tag: string) {
  if (tag === FREE_TAG) {
    return "shrink-0 rounded-full bg-emerald-100 px-2 py-0.5 text-[10px] text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300";
  }
  return "shrink-0 rounded-full bg-secondary px-2 py-0.5 text-[10px] text-muted-foreground dark:bg-secondary dark:text-secondary-foreground";
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
        onClick={(event) => handleProviderNoteUrlClick(event, normalizedUrl)}
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
  sourceProvider?: ProviderSummary | null;
  trailing?: ReactNode;
  circuit: GatewayProviderCircuitStatus | null;
  circuitResetting: boolean;
  onToggleEnabled: (provider: ProviderSummary) => void;
  onResetCircuit: (provider: ProviderSummary) => void;
  onCopyTerminalLaunchCommand?: (provider: ProviderSummary) => void;
  terminalLaunchCopying?: boolean;
  onTestAvailability?: (provider: ProviderSummary) => void;
  testAvailabilityLoading?: boolean;
  onDuplicate?: (provider: ProviderSummary) => void;
  duplicateLoading?: boolean;
  onEdit: (provider: ProviderSummary) => void;
  onDelete: (provider: ProviderSummary) => void;
};

type ProviderCardProps = SortableProviderCardProps & {
  className?: string;
  dragHandleProps?: ButtonHTMLAttributes<HTMLButtonElement>;
} & HTMLAttributes<HTMLDivElement>;

export const ProviderCard = memo(function ProviderCard({
  provider,
  sourceProviderName = null,
  sourceProvider = null,
  trailing = null,
  circuit,
  circuitResetting,
  onToggleEnabled,
  onResetCircuit,
  onCopyTerminalLaunchCommand,
  terminalLaunchCopying = false,
  onTestAvailability,
  testAvailabilityLoading = false,
  onDuplicate,
  duplicateLoading = false,
  onEdit,
  onDelete,
  className,
  dragHandleProps,
  ...cardProps
}: ProviderCardProps) {
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
  const { isOAuth, isCx2cc, isCx2ccGateway } = getProviderTypeInfo(provider);
  const [apiKeyDetailsVisible, setApiKeyDetailsVisible] = useState(false);
  const [limitsRefreshing, setLimitsRefreshing] = useState(false);
  const [resetConfirmOpen, setResetConfirmOpen] = useState(false);
  const [resettingCodexQuota, setResettingCodexQuota] = useState(false);
  const [resetError, setResetError] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { data: oauthLimits = null, isLoading: limitsQueryLoading } = useOAuthLimitsQuery(
    provider.id,
    isOAuth
  );
  const limitsLoading = limitsQueryLoading || limitsRefreshing;
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
  const cx2ccSourceName =
    sourceProviderName ??
    sourceProvider?.name ??
    (provider.source_provider_id != null
      ? `#${provider.source_provider_id}`
      : "当前 AIO 服务 Codex 网关");
  const cx2ccRouteLabel = isCx2ccGateway
    ? "跟随当前 Codex 分流"
    : (sourceProvider?.base_urls[0] ?? "跟随网关默认路由");
  const visibleTags = provider.tags ?? [];
  const resetCreditCount =
    isOAuth && provider.cli_key === "codex"
      ? (oauthLimits?.reset_credit_available_count ?? null)
      : null;
  const showResetCredit = resetCreditCount != null;
  const canResetCredit = Boolean(
    showResetCredit && resetCreditCount > 0 && !limitsLoading && !resettingCodexQuota
  );

  async function handleConfirmCodexReset() {
    if (!canResetCredit) return;
    setResettingCodexQuota(true);
    setResetError(null);
    try {
      const result = await resetProviderOAuthCodexQuota(queryClient, provider.id, {
        resetCircuitAfterRefresh: true,
      });
      if (result.refresh_error) {
        setResetError(`已重置，但刷新用量失败：${result.refresh_error}`);
      }
    } catch (error) {
      setResetError(`重置失败：${String(error)}`);
    } finally {
      setResettingCodexQuota(false);
      setResetConfirmOpen(false);
    }
  }

  return (
    <>
      <Card
        padding="sm"
        className={cn(
          "rounded-lg sm:rounded-xl flex flex-col gap-2 transition-shadow duration-200 sm:flex-row sm:items-center sm:justify-between",
          className
        )}
        {...cardProps}
      >
        <div className="flex min-w-0 items-center gap-3">
          {dragHandleProps ? (
            <button
              type="button"
              className="inline-flex h-8 w-8 shrink-0 cursor-grab items-center justify-center rounded-lg border border-border bg-white text-muted-foreground transition-colors hover:bg-secondary active:cursor-grabbing dark:border-border dark:bg-secondary dark:text-muted-foreground"
              title="拖拽排序"
              aria-label={`拖拽调整 ${provider.name} 顺序`}
              {...dragHandleProps}
            >
              <GripVertical className="h-4 w-4" aria-hidden="true" />
            </button>
          ) : null}
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
                <>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      if (limitsRefreshing) return;
                      setLimitsRefreshing(true);
                      void refreshProviderOAuthLimits(queryClient, provider.id, {
                        resetCircuitAfterRefresh: true,
                      })
                        .catch(() => {})
                        .finally(() => setLimitsRefreshing(false));
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
                </>
              ) : isCx2cc ? (
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
              {isCx2cc && provider.cost_multiplier !== 0 ? (
                <span
                  className={cn(
                    "shrink-0 rounded-full px-2 py-0.5 font-mono text-[10px]",
                    "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                  )}
                  title={`价格倍率: x${provider.cost_multiplier.toFixed(2)}`}
                >
                  x{provider.cost_multiplier.toFixed(2)}
                </span>
              ) : null}
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
              {visibleTags.map((tag) => (
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
                      className="truncate font-mono text-xs text-muted-foreground cursor-default"
                      title={`OAuth: ${provider.oauth_email}`}
                    >
                      {provider.oauth_email}
                    </span>
                  ) : null}
                  <OAuthQuotaUsageInline
                    cliKey={provider.cli_key}
                    limits={oauthLimits}
                    nowUnix={nowUnix}
                    className="contents"
                    segmentClassName="cursor-default"
                    resetCreditDisabled={!canResetCredit}
                    resetCreditLoading={resettingCodexQuota}
                    onResetCreditClick={
                      showResetCredit
                        ? () => {
                            if (!canResetCredit) return;
                            setResetConfirmOpen(true);
                          }
                        : undefined
                    }
                  />
                  {resetError ? (
                    <span className="shrink-0 text-xs text-rose-600 dark:text-rose-400">
                      {resetError}
                    </span>
                  ) : null}
                </>
              ) : isCx2cc ? (
                <>
                  <span
                    className="truncate font-mono text-xs text-violet-500 dark:text-violet-400 cursor-default"
                    title={`来源: ${cx2ccSourceName}`}
                  >
                    来源: {cx2ccSourceName}
                  </span>
                  <span
                    className="truncate font-mono text-xs text-muted-foreground cursor-default"
                    title={cx2ccRouteLabel}
                  >
                    {cx2ccRouteLabel}
                  </span>
                </>
              ) : apiKeyDetailsVisible ? (
                <span
                  className="truncate font-mono text-xs text-muted-foreground cursor-default"
                  title={provider.base_urls.join("\n")}
                >
                  {providerBaseUrlSummary(provider)}
                </span>
              ) : null}
            </div>
            {provider.note ? (
              <div
                className="mt-1 break-words text-xs text-muted-foreground cursor-default"
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

            <div className="inline-flex h-9 items-center gap-2 rounded-lg border border-border bg-white px-3 text-sm shadow-sm dark:border-border dark:bg-secondary">
              <span className="text-sm font-medium text-secondary-foreground">
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

            {trailing}

            {onTestAvailability ? (
              <Button
                onClick={() => onTestAvailability(provider)}
                variant="secondary"
                size="sm"
                className="px-2 py-1 text-[11px] gap-1.5"
                disabled={testAvailabilityLoading}
                title="测试供应商可用性"
              >
                <Zap className="h-3.5 w-3.5" />
                {testAvailabilityLoading ? "测试中…" : "测试"}
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
      <ConfirmDialog
        open={resetConfirmOpen}
        title="确认重置 Codex 额度"
        description="使用 1 次 Codex 重置次数刷新该账号额度？"
        onClose={() => {
          if (resettingCodexQuota) return;
          setResetConfirmOpen(false);
        }}
        onConfirm={() => {
          void handleConfirmCodexReset();
        }}
        confirmLabel="确认重置"
        confirmingLabel="重置中…"
        confirming={resettingCodexQuota}
        disabled={!canResetCredit}
        confirmVariant="danger"
      />
    </>
  );
});

export const SortableProviderCard = memo(function SortableProviderCard(
  props: SortableProviderCardProps
) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: props.provider.id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} className="relative">
      <ProviderCard
        {...props}
        className={cn(isDragging && "z-10 scale-[1.02] shadow-lg ring-2 ring-accent/30")}
        dragHandleProps={{ ...attributes, ...listeners }}
      />
    </div>
  );
});
