import { useEffect, useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { FlaskConical, Pencil, Terminal, Trash2 } from "lucide-react";
import type { GatewayProviderCircuitStatus } from "../../services/gateway";
import type { ProviderSummary } from "../../services/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds, formatUnixSeconds, formatUsdRaw } from "../../utils/formatters";
import { providerBaseUrlSummary } from "./baseUrl";

export type SortableProviderCardProps = {
  provider: ProviderSummary;
  circuit: GatewayProviderCircuitStatus | null;
  circuitResetting: boolean;
  onToggleEnabled: (provider: ProviderSummary) => void;
  onResetCircuit: (provider: ProviderSummary) => void;
  onCopyTerminalLaunchCommand?: (provider: ProviderSummary) => void;
  terminalLaunchCopying?: boolean;
  onValidateModel?: (provider: ProviderSummary) => void;
  onEdit: (provider: ProviderSummary) => void;
  onDelete: (provider: ProviderSummary) => void;
};

export function SortableProviderCard({
  provider,
  circuit,
  circuitResetting,
  onToggleEnabled,
  onResetCircuit,
  onCopyTerminalLaunchCommand,
  terminalLaunchCopying = false,
  onValidateModel,
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

  const claudeModelsCount = Object.values(provider.claude_models ?? {}).filter((value) => {
    if (typeof value !== "string") return false;
    return Boolean(value.trim());
  }).length;
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

  const isOpen = circuit?.state === "OPEN";
  const cooldownUntil = circuit?.cooldown_until ?? null;
  const isUnavailable = isOpen || (cooldownUntil != null && Number.isFinite(cooldownUntil));
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    if (!isUnavailable) return;
    setNowUnix(Math.floor(Date.now() / 1000));
    const timer = window.setInterval(() => {
      setNowUnix(Math.floor(Date.now() / 1000));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [isUnavailable]);

  const unavailableUntil = isUnavailable
    ? (() => {
        const openUntil = isOpen ? (circuit?.open_until ?? null) : null;
        if (openUntil == null) return cooldownUntil;
        if (cooldownUntil == null) return openUntil;
        return Math.max(openUntil, cooldownUntil);
      })()
    : null;
  const unavailableRemaining =
    unavailableUntil != null ? Math.max(0, unavailableUntil - nowUnix) : null;
  const unavailableCountdown =
    unavailableRemaining != null ? formatCountdownSeconds(unavailableRemaining) : null;

  return (
    <div ref={setNodeRef} style={style} className="relative">
      <Card
        padding="sm"
        className={cn(
          "flex cursor-grab flex-col gap-2 transition-shadow duration-200 active:cursor-grabbing sm:flex-row sm:items-center sm:justify-between",
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
              <div className="truncate text-sm font-semibold">{provider.name}</div>
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
            <div className="mt-1 flex items-center gap-2">
              <span className="shrink-0 rounded-full bg-slate-50 px-2 py-0.5 font-mono text-[10px] text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                {provider.base_url_mode === "ping" ? "Ping" : "顺序"}
              </span>
              <span className="shrink-0 rounded-full bg-slate-50 px-2 py-0.5 font-mono text-[10px] text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                倍率 {provider.cost_multiplier}x
              </span>
              {provider.auth_type === "oauth" ? (
                <span
                  className={cn(
                    "shrink-0 rounded-full px-2 py-0.5 font-mono text-[10px]",
                    provider.oauth_last_error
                      ? "bg-rose-50 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400"
                      : provider.oauth_quota_exceeded
                        ? "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"
                        : "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
                  )}
                  title={
                    provider.oauth_last_error
                      ? `OAuth 错误: ${provider.oauth_last_error}`
                      : provider.oauth_quota_exceeded
                        ? "OAuth 配额超限"
                        : "OAuth 已认证"
                  }
                >
                  OAuth
                </span>
              ) : null}
              {provider.cli_key === "claude" && hasClaudeModels ? (
                <span
                  className="shrink-0 rounded-full bg-sky-50 px-2 py-0.5 font-mono text-[10px] text-sky-700 dark:bg-sky-900/30 dark:text-sky-400"
                  title={`已配置 Claude 模型映射（${claudeModelsCount}/5）`}
                >
                  Claude Models
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
            </div>
            <div
              className="mt-1 truncate font-mono text-xs text-slate-500 dark:text-slate-400 cursor-default"
              title={provider.base_urls.join("\n")}
            >
              {providerBaseUrlSummary(provider)}
            </div>
          </div>
        </div>

        <div
          className="flex flex-wrap items-center gap-3"
          onPointerDown={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-600 dark:text-slate-400">启用</span>
            <Switch checked={provider.enabled} onCheckedChange={() => onToggleEnabled(provider)} />
          </div>

          {isUnavailable ? (
            <Button
              onClick={() => onResetCircuit(provider)}
              variant="secondary"
              size="sm"
              disabled={circuitResetting}
            >
              {circuitResetting ? "处理中…" : "解除熔断"}
            </Button>
          ) : null}

          {onCopyTerminalLaunchCommand ? (
            <Button
              onClick={() => onCopyTerminalLaunchCommand(provider)}
              variant="secondary"
              size="sm"
              disabled={terminalLaunchCopying}
              title="复制终端启动命令"
            >
              <Terminal className="h-4 w-4" />
              {terminalLaunchCopying ? "复制中…" : "终端启动"}
            </Button>
          ) : null}

          {onValidateModel ? (
            <Button
              onClick={() => onValidateModel(provider)}
              variant="secondary"
              size="sm"
              title="模型验证"
            >
              <FlaskConical className="h-4 w-4" />
              模型验证
            </Button>
          ) : null}

          <Button onClick={() => onEdit(provider)} variant="secondary" size="sm" title="编辑">
            <Pencil className="h-4 w-4" />
            编辑
          </Button>

          <Button onClick={() => onDelete(provider)} variant="danger" size="sm" title="删除">
            <Trash2 className="h-4 w-4" />
            删除
          </Button>
        </div>
      </Card>
    </div>
  );
}
