import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { toast } from "sonner";
import { RefreshCw, ShieldCheck, ShieldOff } from "lucide-react";
import {
  providerOAuthDisconnect,
  providerOAuthRefresh,
  providerOAuthStartFlow,
  providerOAuthStatus,
  type ProviderSummary,
} from "../../../services/providers/providers";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";

export type ClaudeOAuthCardProps = {
  providers: ProviderSummary[] | null;
};

type OAuthStatus = NonNullable<Awaited<ReturnType<typeof providerOAuthStatus>>>;

function buildInitialStatus(provider: ProviderSummary): OAuthStatus {
  return {
    connected: Boolean(provider.oauth_email),
    provider_type: provider.oauth_provider_type ?? undefined,
    email: provider.oauth_email ?? undefined,
    expires_at: provider.oauth_expires_at ?? undefined,
    has_refresh_token: undefined,
  };
}

function formatExpiresAt(expiresAt: number | undefined) {
  if (!expiresAt) return "—";
  const ts = expiresAt > 1_000_000_000_000 ? expiresAt : expiresAt * 1000;
  return new Date(ts).toLocaleString("zh-CN", { hour12: false });
}

export function ClaudeOAuthCard({ providers }: ClaudeOAuthCardProps) {
  const oauthProvider = useMemo(
    () =>
      providers?.find(
        (provider) => provider.cli_key === "claude" && provider.auth_mode === "oauth"
      ) ?? null,
    [providers]
  );
  const [status, setStatus] = useState<OAuthStatus | null>(
    oauthProvider ? buildInitialStatus(oauthProvider) : null
  );
  const [statusLoading, setStatusLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<"login" | "refresh" | "disconnect" | null>(
    null
  );
  const [statusError, setStatusError] = useState<string | null>(null);

  useEffect(() => {
    if (!oauthProvider) {
      setStatus(null);
      setStatusError(null);
      setStatusLoading(false);
      return;
    }

    let cancelled = false;
    setStatus(buildInitialStatus(oauthProvider));
    setStatusError(null);
    setStatusLoading(true);

    void providerOAuthStatus(provider.id)
      .then((next) => {
        if (cancelled || !next) return;
        setStatus(next);
      })
      .catch((error) => {
        if (cancelled) return;
        setStatusError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        if (!cancelled) {
          setStatusLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [
    oauthProvider?.id,
    oauthProvider?.oauth_email,
    oauthProvider?.oauth_expires_at,
    oauthProvider?.oauth_provider_type,
  ]);

  async function reloadStatus(providerId: number) {
    const next = await providerOAuthStatus(providerId);
    setStatus(next);
    setStatusError(null);
    return next;
  }

  if (!oauthProvider) {
    return (
      <Card className="p-5">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              Claude OAuth
            </h3>
            <p className="mt-1 text-sm text-slate-600 dark:text-slate-400">
              请先在供应商页面创建一个 Claude OAuth 供应商
            </p>
          </div>
          <Button asChild variant="secondary" size="sm">
            <Link to="/providers">前往供应商页面</Link>
          </Button>
        </div>
      </Card>
    );
  }

  // After early return, oauthProvider is guaranteed non-null.
  // TypeScript doesn't narrow useMemo results across early returns, so re-bind.
  const provider = oauthProvider;

  const connected = status?.connected ?? false;
  const busy = actionLoading !== null;
  const effectiveError = statusError ?? provider.oauth_last_error ?? null;

  async function handleLogin() {
    setActionLoading("login");
    try {
      const result = await providerOAuthStartFlow("claude", provider.id);
      if (!result?.success) {
        toast.error("Claude OAuth 登录失败");
        return;
      }
      const next = await reloadStatus(provider.id);
      toast.success(next?.connected ? "Claude OAuth 登录成功" : "已打开浏览器，请完成 Claude 授权");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Claude OAuth 登录失败");
    } finally {
      setActionLoading(null);
    }
  }

  async function handleRefresh() {
    setActionLoading("refresh");
    try {
      const result = await providerOAuthRefresh(provider.id);
      if (!result?.success) {
        toast.error("刷新令牌失败");
        return;
      }
      await reloadStatus(provider.id);
      toast.success("令牌已刷新");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "刷新令牌失败");
    } finally {
      setActionLoading(null);
    }
  }

  async function handleDisconnect() {
    setActionLoading("disconnect");
    try {
      const result = await providerOAuthDisconnect(provider.id);
      if (!result?.success) {
        toast.error("断开连接失败");
        return;
      }
      setStatus({
        connected: false,
        provider_type: status?.provider_type ?? provider.oauth_provider_type ?? undefined,
        email: undefined,
        expires_at: undefined,
        has_refresh_token: undefined,
      });
      setStatusError(null);
      toast.success("已断开 Claude OAuth 连接");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "断开连接失败");
    } finally {
      setActionLoading(null);
    }
  }

  return (
    <Card className="p-5">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              Claude OAuth
            </h3>
            <p className="mt-1 text-sm text-slate-600 dark:text-slate-400">
              使用浏览器完成 Claude 官方 OAuth 授权，适用于 OAuth 模式的 Claude 供应商。
            </p>
          </div>
          <span
            className={[
              "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset",
              connected
                ? "bg-green-50 text-green-700 ring-green-600/20 dark:bg-green-900/30 dark:text-green-400"
                : "bg-slate-100 text-slate-600 ring-slate-500/10 dark:bg-slate-700 dark:text-slate-300",
            ].join(" ")}
          >
            {connected ? <ShieldCheck className="h-3 w-3" /> : <ShieldOff className="h-3 w-3" />}
            {statusLoading ? "读取状态中..." : connected ? "已连接" : "未连接"}
          </span>
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800">
            <div className="text-xs text-slate-500 dark:text-slate-400">供应商</div>
            <div className="mt-1 text-sm font-medium text-slate-900 dark:text-slate-100">
              {oauthProvider.name}
            </div>
          </div>
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800">
            <div className="text-xs text-slate-500 dark:text-slate-400">邮箱</div>
            <div className="mt-1 text-sm font-medium text-slate-900 dark:text-slate-100">
              {connected ? (status?.email ?? "—") : "—"}
            </div>
          </div>
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800">
            <div className="text-xs text-slate-500 dark:text-slate-400">到期时间</div>
            <div className="mt-1 text-sm font-medium text-slate-900 dark:text-slate-100">
              {connected ? formatExpiresAt(status?.expires_at) : "—"}
            </div>
          </div>
        </div>

        {effectiveError ? (
          <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:border-amber-700 dark:bg-amber-900/20 dark:text-amber-300">
            最近状态：{effectiveError}
          </div>
        ) : null}

        <div className="flex flex-wrap items-center gap-2">
          {!connected ? (
            <Button onClick={() => void handleLogin()} disabled={busy || statusLoading}>
              {actionLoading === "login" ? <RefreshCw className="h-4 w-4 animate-spin" /> : null}
              登录 Claude
            </Button>
          ) : (
            <>
              <Button
                variant="secondary"
                onClick={() => void handleRefresh()}
                disabled={busy || statusLoading}
              >
                {actionLoading === "refresh" ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : null}
                刷新令牌
              </Button>
              <Button
                variant="danger"
                onClick={() => void handleDisconnect()}
                disabled={busy || statusLoading}
              >
                {actionLoading === "disconnect" ? (
                  <RefreshCw className="h-4 w-4 animate-spin" />
                ) : null}
                断开连接
              </Button>
            </>
          )}

          <Button asChild variant="ghost" size="sm">
            <Link to="/providers">管理供应商</Link>
          </Button>
        </div>
      </div>
    </Card>
  );
}
