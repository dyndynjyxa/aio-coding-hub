import { toast } from "sonner";
import { openDesktopUrl } from "../../services/desktop/opener";
import { logToConsole } from "../../services/consoleLog";
import {
  providerOAuthStartDeviceFlow,
  providerOAuthPollDeviceFlow,
  providerOAuthStartFlow,
  providerOAuthRefresh,
  providerOAuthDisconnect,
  providerOAuthFetchLimits,
} from "../../services/providers/providers";
import type { OAuthActionContext } from "./providerEditorActionContext";
import { presentProviderEditorPayloadBuildError } from "./providerEditorFeedback";
import { buildProviderEditorUpsertInput } from "./providerEditorSubmitModel";

class OAuthAttemptStaleError extends Error {
  constructor() {
    super("OAuth attempt is no longer current");
  }
}

function isOAuthAttemptStaleError(error: unknown) {
  return error instanceof OAuthAttemptStaleError;
}

function ensureCurrentOAuthAttempt(
  isCurrentAttempt: () => boolean,
  onStale?: () => Promise<void> | void
) {
  if (isCurrentAttempt()) return Promise.resolve();
  return Promise.resolve(onStale?.()).then(() => {
    throw new OAuthAttemptStaleError();
  });
}

function whenOAuthAttemptCurrent<T>(
  value: T | PromiseLike<T>,
  isCurrentAttempt: () => boolean,
  onStale?: () => Promise<void> | void
) {
  return Promise.resolve(value).then((resolvedValue) => {
    if (isCurrentAttempt()) return resolvedValue;
    return Promise.resolve(onStale?.()).then(() => {
      throw new OAuthAttemptStaleError();
    });
  });
}

async function waitForOAuthDevicePollInterval(
  ctx: OAuthActionContext,
  attemptId: number,
  ms: number
) {
  const deadline = Date.now() + ms;
  await new Promise<void>((resolve) => {
    const check = () => {
      if (!ctx.isOAuthLoginAttemptCurrent(attemptId)) {
        resolve();
        return;
      }

      const remainingMs = deadline - Date.now();
      if (remainingMs <= 0) {
        resolve();
        return;
      }

      window.setTimeout(check, Math.min(remainingMs, 250));
    };

    check();
  });
}

export async function handleOAuthLogin(ctx: OAuthActionContext) {
  const attemptId = ctx.beginOAuthLoginAttempt();
  const isCurrentAttempt = () => ctx.isOAuthLoginAttemptCurrent(attemptId);
  ctx.setOauthLoading(true);
  let autoSavedProviderId: number | null = null;
  let shouldRollbackAutoSavedProvider = false;

  const rollbackAutoSavedProvider = async () => {
    if (!shouldRollbackAutoSavedProvider || !autoSavedProviderId) return;
    try {
      const deleted = await ctx.removeProvider(autoSavedProviderId);
      if (!deleted) {
        logToConsole(
          "warn",
          `OAuth 登录失败后清理临时 Provider 失败：${ctx.form.getValues().name || "OAuth Provider"}`,
          { cli_key: ctx.cliKey, provider_id: autoSavedProviderId }
        );
      }
    } catch (cleanupErr) {
      logToConsole(
        "error",
        `OAuth 登录失败后清理临时 Provider 异常：${ctx.form.getValues().name || "OAuth Provider"}`,
        { cli_key: ctx.cliKey, provider_id: autoSavedProviderId, error: String(cleanupErr) }
      );
    }
  };

  try {
    let targetProviderId = ctx.editingProviderId;
    if (!targetProviderId) {
      if (!ctx.form.getValues().name?.trim()) {
        toast("请先填写 Provider 名称");
        return;
      }

      const built = buildProviderEditorUpsertInput({
        ...ctx,
        formValues: ctx.form.getValues(),
      });
      if (!built.ok) {
        presentProviderEditorPayloadBuildError(ctx.mode, built.error);
        return;
      }

      const saved = await ctx.persistProvider(built.value.payload);
      targetProviderId = saved.id;
      autoSavedProviderId = saved.id;
      shouldRollbackAutoSavedProvider = true;
      if (!isCurrentAttempt()) {
        await rollbackAutoSavedProvider();
        return;
      }
    }

    if (!isCurrentAttempt()) {
      await rollbackAutoSavedProvider();
      return;
    }
    const result = await whenOAuthAttemptCurrent(
      providerOAuthStartFlow(ctx.cliKey, targetProviderId),
      isCurrentAttempt,
      rollbackAutoSavedProvider
    );
    if (result.success) {
      shouldRollbackAutoSavedProvider = false;

      let status: Awaited<ReturnType<OAuthActionContext["refreshOauthStatus"]>> = null;
      try {
        const nextStatus = await whenOAuthAttemptCurrent(
          ctx.refreshOauthStatus(targetProviderId),
          isCurrentAttempt
        );
        status = nextStatus;
        ctx.setOauthStatus(status);
      } catch (statusErr) {
        if (isOAuthAttemptStaleError(statusErr)) {
          return;
        }
        if (!isCurrentAttempt()) {
          return;
        }
        // Token 已保存成功，只是状态读取失败：用登录结果兜底展示，
        // 并同步写入缓存，避免编辑器 effect 用旧快照盖回去。
        status = {
          connected: true,
          provider_type: result.provider_type,
          email: null,
          expires_at: result.expires_at,
          has_refresh_token: null,
        };
        ctx.setOauthStatus(status);
        ctx.writeOauthStatusCache(status, targetProviderId);
        toast("OAuth 登录成功，但读取连接状态失败，可稍后重试");
        logToConsole(
          "warn",
          `OAuth 登录后读取状态失败：${ctx.form.getValues().name || "OAuth Provider"}`,
          {
            cli_key: ctx.cliKey,
            provider_id: targetProviderId,
            provider_type: result.provider_type,
            error: String(statusErr),
          }
        );
      }

      let limits: Awaited<ReturnType<typeof providerOAuthFetchLimits>> = null;
      try {
        limits = await whenOAuthAttemptCurrent(
          providerOAuthFetchLimits(targetProviderId),
          isCurrentAttempt
        );
        if (!limits) {
          toast("OAuth 登录成功，但获取用量失败，可稍后重试");
          logToConsole(
            "warn",
            `OAuth 登录后获取用量失败：${ctx.form.getValues().name || "OAuth Provider"}`,
            {
              cli_key: ctx.cliKey,
              provider_id: targetProviderId,
              provider_type: result.provider_type,
              email: status?.email,
            }
          );
        }
      } catch (err) {
        if (isOAuthAttemptStaleError(err)) {
          return;
        }
        if (!isCurrentAttempt()) {
          return;
        }
        toast("OAuth 登录成功，但获取用量失败，可稍后重试");
        logToConsole(
          "warn",
          `OAuth 登录后获取用量异常：${ctx.form.getValues().name || "OAuth Provider"}`,
          {
            cli_key: ctx.cliKey,
            provider_id: targetProviderId,
            provider_type: result.provider_type,
            email: status?.email,
            error: String(err),
          }
        );
      }

      if (!isCurrentAttempt()) {
        return;
      }
      toast("OAuth 登录成功");
      logToConsole("info", `OAuth 登录成功：${ctx.form.getValues().name || "OAuth Provider"}`, {
        cli_key: ctx.cliKey,
        provider_id: targetProviderId,
        provider_type: result.provider_type,
        email: status?.email,
        expires_at: result.expires_at,
        limit_5h: limits?.limit_5h_text,
        limit_weekly: limits?.limit_weekly_text,
      });
      if (!ctx.editingProviderId) {
        ctx.onSaved(ctx.cliKey);
        ctx.onOpenChange(false);
      }
    } else {
      await rollbackAutoSavedProvider();
      toast("OAuth 登录失败");
      logToConsole("warn", `OAuth 登录失败：${ctx.form.getValues().name || "OAuth Provider"}`, {
        cli_key: ctx.cliKey,
        provider_id: targetProviderId,
      });
    }
  } catch (err) {
    if (isOAuthAttemptStaleError(err)) {
      return;
    }
    if (!isCurrentAttempt()) {
      await rollbackAutoSavedProvider();
      return;
    }
    await rollbackAutoSavedProvider();
    toast(`OAuth 登录失败：${String(err)}`);
    logToConsole("error", `OAuth 登录异常：${ctx.form.getValues().name || "OAuth Provider"}`, {
      cli_key: ctx.cliKey,
      error: String(err),
    });
  } finally {
    if (isCurrentAttempt()) {
      ctx.setOauthLoading(false);
    }
  }
}

export async function handleOAuthDeviceLogin(ctx: OAuthActionContext) {
  const attemptId = ctx.beginOAuthLoginAttempt();
  const isCurrentAttempt = () => ctx.isOAuthLoginAttemptCurrent(attemptId);
  ctx.setOauthLoading(true);
  ctx.setOauthDeviceError(null);
  ctx.setOauthDeviceFlow(null);
  ctx.setOauthDevicePolling(false);
  let autoSavedProviderId: number | null = null;
  let shouldRollbackAutoSavedProvider = false;
  let activeFlowId: string | null = null;

  const rollbackAutoSavedProvider = async () => {
    if (!shouldRollbackAutoSavedProvider || !autoSavedProviderId) return;
    try {
      const deleted = await ctx.removeProvider(autoSavedProviderId);
      if (!deleted) {
        logToConsole(
          "warn",
          `设备码登录失败后清理临时 Provider 失败：${ctx.form.getValues().name || "OAuth Provider"}`,
          { cli_key: ctx.cliKey, provider_id: autoSavedProviderId }
        );
      }
    } catch (cleanupErr) {
      logToConsole(
        "error",
        `设备码登录失败后清理临时 Provider 异常：${ctx.form.getValues().name || "OAuth Provider"}`,
        {
          cli_key: ctx.cliKey,
          provider_id: autoSavedProviderId,
          error: String(cleanupErr),
        }
      );
    }
  };

  try {
    let targetProviderId = ctx.editingProviderId;
    if (!targetProviderId) {
      if (!ctx.form.getValues().name?.trim()) {
        toast("请先填写 Provider 名称");
        return;
      }

      const built = buildProviderEditorUpsertInput({
        ...ctx,
        formValues: ctx.form.getValues(),
      });
      if (!built.ok) {
        presentProviderEditorPayloadBuildError(ctx.mode, built.error);
        return;
      }

      const saved = await ctx.persistProvider(built.value.payload);
      targetProviderId = saved.id;
      autoSavedProviderId = saved.id;
      shouldRollbackAutoSavedProvider = true;
      if (!isCurrentAttempt()) {
        await rollbackAutoSavedProvider();
        return;
      }
    }

    const start = await providerOAuthStartDeviceFlow(targetProviderId);
    activeFlowId = start.flow_id;
    if (!isCurrentAttempt()) {
      ctx.cancelOAuthDeviceFlow(start.flow_id);
      await rollbackAutoSavedProvider();
      return;
    }
    ctx.setActiveOAuthDeviceFlow(attemptId, start.flow_id);
    ctx.setOauthDeviceFlow(start);
    ctx.setOauthDevicePolling(true);
    if (!isCurrentAttempt()) {
      await rollbackAutoSavedProvider();
      return;
    }
    await whenOAuthAttemptCurrent(
      openDesktopUrl(start.verification_uri),
      isCurrentAttempt,
      rollbackAutoSavedProvider
    );

    const deadline = Date.now() + start.expires_in * 1000;
    const pollDeviceFlowUntilComplete = async (): Promise<boolean> => {
      if (Date.now() >= deadline) return false;

      const result = await whenOAuthAttemptCurrent(
        providerOAuthPollDeviceFlow(
          targetProviderId,
          start.flow_id,
          start.device_code,
          start.user_code
        ),
        isCurrentAttempt,
        rollbackAutoSavedProvider
      );
      if (result.completed) {
        shouldRollbackAutoSavedProvider = false;
        ctx.clearActiveOAuthDeviceFlow(start.flow_id);
        ctx.setOauthDevicePolling(false);
        ctx.setOauthDeviceFlow(null);
        ctx.setOauthDeviceError(null);

        const status = await whenOAuthAttemptCurrent(
          ctx.refreshOauthStatus(targetProviderId),
          isCurrentAttempt
        );
        ctx.setOauthStatus(status);

        try {
          await whenOAuthAttemptCurrent(
            providerOAuthFetchLimits(targetProviderId),
            isCurrentAttempt
          );
        } catch (err) {
          if (isOAuthAttemptStaleError(err)) {
            throw err;
          }
          logToConsole(
            "warn",
            `设备码登录后获取用量异常：${ctx.form.getValues().name || "OAuth Provider"}`,
            {
              cli_key: ctx.cliKey,
              provider_id: targetProviderId,
              error: String(err),
            }
          );
        }

        toast("设备码登录成功");
        if (!ctx.editingProviderId) {
          ctx.onSaved(ctx.cliKey);
          ctx.onOpenChange(false);
        }
        return true;
      }

      await whenOAuthAttemptCurrent(
        waitForOAuthDevicePollInterval(ctx, attemptId, start.interval * 1000),
        isCurrentAttempt,
        rollbackAutoSavedProvider
      );
      return pollDeviceFlowUntilComplete();
    };

    const completed = await pollDeviceFlowUntilComplete();
    if (!completed) {
      await ensureCurrentOAuthAttempt(isCurrentAttempt, rollbackAutoSavedProvider);
      if (activeFlowId) {
        ctx.cancelOAuthDeviceFlow(activeFlowId);
        ctx.clearActiveOAuthDeviceFlow(activeFlowId);
      }
      ctx.setOauthDevicePolling(false);
      ctx.setOauthDeviceError("设备码已过期，请重新开始登录。");
      await rollbackAutoSavedProvider();
      toast("设备码登录失败：设备码已过期");
    }
  } catch (err) {
    if (isOAuthAttemptStaleError(err)) {
      return;
    }
    if (!isCurrentAttempt()) {
      await rollbackAutoSavedProvider();
      return;
    }
    if (activeFlowId) {
      ctx.cancelOAuthDeviceFlow(activeFlowId);
      ctx.clearActiveOAuthDeviceFlow(activeFlowId);
    }
    ctx.setOauthDevicePolling(false);
    ctx.setOauthDeviceError(String(err));
    await rollbackAutoSavedProvider();
    toast(`设备码登录失败：${String(err)}`);
    logToConsole("error", `设备码登录异常：${ctx.form.getValues().name || "OAuth Provider"}`, {
      cli_key: ctx.cliKey,
      error: String(err),
    });
  } finally {
    if (isCurrentAttempt()) {
      ctx.setOauthLoading(false);
    }
  }
}

export async function handleOAuthRefresh(ctx: OAuthActionContext) {
  if (!ctx.editingProviderId) return;
  ctx.setOauthLoading(true);
  try {
    const result = await providerOAuthRefresh(ctx.editingProviderId);
    if (result.success) {
      const status = await ctx.refreshOauthStatus(ctx.editingProviderId);
      ctx.setOauthStatus(status);
      toast("Token 刷新成功");
      logToConsole("info", `OAuth Token 刷新成功：${ctx.form.getValues().name}`, {
        provider_id: ctx.editingProviderId,
        expires_at: result.expires_at,
      });
    } else {
      toast("Token 刷新失败");
      logToConsole("warn", `OAuth Token 刷新失败：${ctx.form.getValues().name}`, {
        provider_id: ctx.editingProviderId,
      });
    }
  } catch (err) {
    toast(`Token 刷新失败：${String(err)}`);
    logToConsole("error", `OAuth Token 刷新异常：${ctx.form.getValues().name}`, {
      provider_id: ctx.editingProviderId,
      error: String(err),
    });
  } finally {
    ctx.setOauthLoading(false);
  }
}

export async function handleOAuthDisconnect(ctx: OAuthActionContext) {
  if (!ctx.editingProviderId) return;
  ctx.setOauthLoading(true);
  try {
    const result = await providerOAuthDisconnect(ctx.editingProviderId);
    if (result.success) {
      ctx.setOauthStatus(null);
      // 缓存也写为未连接：否则 5 分钟内重开弹窗会命中"仍已连接"的旧快照。
      ctx.writeOauthStatusCache(null, ctx.editingProviderId);
      toast("已断开 OAuth 连接");
      logToConsole("info", `OAuth 已断开连接：${ctx.form.getValues().name}`, {
        provider_id: ctx.editingProviderId,
      });
    } else {
      toast("断开 OAuth 连接失败");
      logToConsole("warn", `OAuth 断开连接失败：${ctx.form.getValues().name}`, {
        provider_id: ctx.editingProviderId,
      });
    }
  } catch (err) {
    toast(`断开 OAuth 连接失败：${String(err)}`);
    logToConsole("error", `OAuth 断开连接异常：${ctx.form.getValues().name}`, {
      provider_id: ctx.editingProviderId,
      error: String(err),
    });
  } finally {
    ctx.setOauthLoading(false);
  }
}
