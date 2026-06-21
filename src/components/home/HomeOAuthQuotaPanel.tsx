import { useMemo, useState } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { cliLongLabel } from "../../constants/clis";
import { useNowUnix } from "../../hooks/useNowUnix";
import { OAuthQuotaUsageInline } from "../providers/OAuthQuotaUsageInline";
import { Card } from "../../ui/Card";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { CliBrandIcon } from "./CliBrandIcon";
import {
  hasHomeOAuthQuotaText,
  hasInsufficientHomeOAuthQuota,
  type HomeOAuthQuotaRow,
} from "./homeOAuthQuotaTypes";

export type HomeOAuthQuotaPanelContentProps = {
  rows: HomeOAuthQuotaRow[];
  hasProviders: boolean;
  hasRefreshed: boolean;
  refreshing: boolean;
  onRefresh?: () => void;
  onRefreshRow?: (providerId: number) => void;
  onResetRow?: (providerId: number) => void | Promise<void>;
};

type HomeOAuthQuotaPanelProps = HomeOAuthQuotaPanelContentProps & {
  onRefresh: () => void;
};

function OAuthQuotaProviderCard({
  row,
  onRefreshRow,
  onRequestReset,
}: {
  row: HomeOAuthQuotaRow;
  onRefreshRow?: (providerId: number) => void;
  onRequestReset?: (row: HomeOAuthQuotaRow) => void;
}) {
  const shouldTrackNowUnix =
    row.limits != null &&
    (row.limits?.limit_5h_reset_at != null || row.limits?.limit_weekly_reset_at != null);
  const nowUnix = useNowUnix(shouldTrackNowUnix);
  const showInsufficientQuota =
    row.state === "success" && hasInsufficientHomeOAuthQuota(row.limits);
  const resetCreditCount =
    row.cliKey === "codex" ? (row.limits?.reset_credit_available_count ?? null) : null;
  const showResetCredit = resetCreditCount != null;
  const canResetCredit = Boolean(
    showResetCredit && resetCreditCount > 0 && row.state !== "loading" && !row.resetting
  );
  const hasQuotaDisplay = hasHomeOAuthQuotaText(row.limits) || showResetCredit;
  const requestReset = () => {
    if (!canResetCredit || !onRequestReset) return;
    onRequestReset(row);
  };

  return (
    <div className="rounded-lg border border-border bg-white px-3 py-2.5 shadow-sm dark:border-border dark:bg-secondary">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2 text-xs text-secondary-foreground">
            <span
              className="inline-flex h-5 w-5 shrink-0 items-center justify-center text-secondary-foreground"
              title={cliLongLabel(row.cliKey)}
            >
              <CliBrandIcon
                cliKey={row.cliKey}
                className="h-3.5 w-3.5 rounded-[3px] object-contain"
              />
            </span>
            <span className="truncate font-medium">{row.providerName}</span>
          </div>

          <div className="flex shrink-0 items-center gap-1">
            {!row.enabled ? (
              <span className="whitespace-nowrap rounded-full bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground dark:bg-secondary dark:text-muted-foreground">
                已禁用
              </span>
            ) : null}
            {onRefreshRow ? (
              <button
                type="button"
                onClick={() => onRefreshRow(row.providerId)}
                disabled={row.state === "loading"}
                className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-secondary dark:hover:text-indigo-400"
                title={`刷新 ${row.providerName} OAuth 配额`}
                aria-label={`刷新 ${row.providerName} OAuth 配额`}
              >
                <RefreshCw
                  className={cn("h-3.5 w-3.5", row.state === "loading" && "animate-spin")}
                />
              </button>
            ) : null}
          </div>
        </div>

        {row.state === "loading" ? (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Spinner size="sm" />
            刷新中...
          </div>
        ) : row.state === "error" ? (
          <div className="space-y-1.5" data-testid={`oauth-quota-status-${row.providerId}`}>
            <div className="flex items-start gap-2 text-xs text-rose-600 dark:text-rose-400">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>刷新失败，请重试</span>
            </div>
            {hasQuotaDisplay ? (
              <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
                <OAuthQuotaUsageInline
                  cliKey={row.cliKey}
                  limits={row.limits}
                  nowUnix={nowUnix}
                  resetCreditDisabled={!canResetCredit || !onRequestReset}
                  resetCreditLoading={row.resetting}
                  onResetCreditClick={showResetCredit && onRequestReset ? requestReset : undefined}
                />
                <span className="shrink-0 whitespace-nowrap rounded-full bg-rose-50 px-1.5 py-0.5 text-[10px] text-rose-700 dark:bg-rose-900/30 dark:text-rose-400">
                  刷新失败
                </span>
              </div>
            ) : (
              <span className="inline-flex shrink-0 whitespace-nowrap rounded-full bg-rose-50 px-1.5 py-0.5 text-[10px] text-rose-700 dark:bg-rose-900/30 dark:text-rose-400">
                刷新失败
              </span>
            )}
          </div>
        ) : row.state === "idle" ? (
          <div className="text-xs text-muted-foreground">点击右上角刷新获取 OAuth 配额</div>
        ) : !hasQuotaDisplay ? (
          <div className="text-xs text-muted-foreground">暂无 OAuth 配额信息</div>
        ) : (
          <div
            className="flex items-center justify-between gap-2 text-xs text-muted-foreground"
            data-testid={`oauth-quota-status-${row.providerId}`}
          >
            <OAuthQuotaUsageInline
              cliKey={row.cliKey}
              limits={row.limits}
              nowUnix={nowUnix}
              resetCreditDisabled={!canResetCredit || !onRequestReset}
              resetCreditLoading={row.resetting}
              onResetCreditClick={showResetCredit && onRequestReset ? requestReset : undefined}
            />
            {showInsufficientQuota ? (
              <span className="shrink-0 rounded-full bg-amber-50 px-1.5 py-0.5 text-[10px] text-amber-700 dark:bg-amber-900/30 dark:text-amber-300">
                配额不足
              </span>
            ) : null}
          </div>
        )}
        {row.resetError ? (
          <div className="flex items-start gap-2 text-xs text-rose-600 dark:text-rose-400">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <span>{row.resetError}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function HomeOAuthQuotaPanelContent({
  rows,
  hasProviders,
  hasRefreshed,
  refreshing,
  onRefresh,
  onRefreshRow,
  onResetRow,
}: HomeOAuthQuotaPanelContentProps) {
  const [resetTargetId, setResetTargetId] = useState<number | null>(null);
  const [confirmingResetId, setConfirmingResetId] = useState<number | null>(null);
  const resetTarget = useMemo(
    () => rows.find((row) => row.providerId === resetTargetId) ?? null,
    [resetTargetId, rows]
  );
  const showNoQuotaNotice =
    hasRefreshed &&
    rows.length > 0 &&
    rows.every((row) => row.state === "success" && !hasHomeOAuthQuotaText(row.limits));

  if (!hasProviders) {
    return <EmptyState title="当前没有 OAuth 供应商" />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-center justify-between shrink-0">
        <span className="text-xs text-muted-foreground">{rows.length} 个 OAuth 供应商</span>
        {onRefresh ? (
          <button
            type="button"
            onClick={onRefresh}
            disabled={refreshing}
            className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-secondary dark:hover:text-indigo-400"
            title="刷新 OAuth 配额"
            aria-label="刷新 OAuth 配额"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
          </button>
        ) : null}
      </div>

      <>
        {showNoQuotaNotice ? (
          <div className="rounded-lg border border-dashed border-border bg-secondary/80 px-3 py-2 text-xs text-muted-foreground dark:border-border dark:bg-secondary/40 dark:text-muted-foreground">
            当前暂无 OAuth 配额信息
          </div>
        ) : null}

        <div className="min-h-0 flex-1 space-y-2 overflow-auto pr-1 scrollbar-overlay">
          {rows.map((row) => (
            <OAuthQuotaProviderCard
              key={`${row.cliKey}:${row.providerId}`}
              row={row}
              onRefreshRow={onRefreshRow}
              onRequestReset={
                onResetRow ? (target) => setResetTargetId(target.providerId) : undefined
              }
            />
          ))}
        </div>
      </>
      <ConfirmDialog
        open={resetTarget != null}
        title="确认重置 Codex 额度"
        description="使用 1 次 Codex 重置次数刷新该账号额度？"
        onClose={() => {
          if (confirmingResetId != null) return;
          setResetTargetId(null);
        }}
        onConfirm={() => {
          if (!resetTarget || !onResetRow) return;
          setConfirmingResetId(resetTarget.providerId);
          void Promise.resolve(onResetRow(resetTarget.providerId)).finally(() => {
            setConfirmingResetId(null);
            setResetTargetId(null);
          });
        }}
        confirmLabel="确认重置"
        confirmingLabel="重置中…"
        confirming={confirmingResetId != null}
        disabled={!resetTarget || resetTarget.resetting}
        confirmVariant="danger"
      />
    </div>
  );
}

export function HomeOAuthQuotaPanel({
  rows,
  hasProviders,
  hasRefreshed,
  refreshing,
  onRefresh,
  onRefreshRow,
  onResetRow,
}: HomeOAuthQuotaPanelProps) {
  return (
    <Card padding="sm" className="flex h-full flex-col">
      <div className="mb-3 text-sm font-semibold">OAuth 配额</div>
      <HomeOAuthQuotaPanelContent
        rows={rows}
        hasProviders={hasProviders}
        hasRefreshed={hasRefreshed}
        refreshing={refreshing}
        onRefresh={onRefresh}
        onRefreshRow={onRefreshRow}
        onResetRow={onResetRow}
      />
    </Card>
  );
}
