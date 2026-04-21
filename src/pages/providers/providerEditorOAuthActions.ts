import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import {
  providerOAuthStartFlow,
  providerOAuthRefresh,
  providerOAuthDisconnect,
  providerOAuthFetchLimits,
} from "../../services/providers/providers";
import type { OAuthActionContext } from "./providerEditorActionContext";
import { presentProviderEditorPayloadBuildError } from "./providerEditorFeedback";
import { buildProviderEditorUpsertInput } from "./providerEditorSubmitModel";

export async function handleOAuthLogin(ctx: OAuthActionContext) {
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
      if (!saved) {
        toast("自动保存 Provider 失败");
        return;
      }
      targetProviderId = saved.id;
      autoSavedProviderId = saved.id;
      shouldRollbackAutoSavedProvider = true;
    }

    const result = await providerOAuthStartFlow(ctx.cliKey, targetProviderId);
    if (result.success) {
      shouldRollbackAutoSavedProvider = false;

      let status: Awaited<ReturnType<OAuthActionContext["refreshOauthStatus"]>> = null;
      try {
        status = await ctx.refreshOauthStatus(targetProviderId);
        ctx.setOauthStatus(status);
      } catch (statusErr) {
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
        limits = await providerOAuthFetchLimits(targetProviderId);
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

      toast("OAuth 登录成功");
      logToConsole(
        "info",
        `OAuth 登录成功：${ctx.form.getValues().name || "OAuth Provider"}`,
        {
          cli_key: ctx.cliKey,
          provider_id: targetProviderId,
          provider_type: result.provider_type,
          email: status?.email,
          expires_at: result.expires_at,
          limit_5h: limits?.limit_5h_text,
          limit_weekly: limits?.limit_weekly_text,
        }
      );
      if (!ctx.editingProviderId) {
        ctx.onSaved(ctx.cliKey);
        ctx.onOpenChange(false);
      }
    } else {
      await rollbackAutoSavedProvider();
      toast("OAuth 登录失败");
      logToConsole(
        "warn",
        `OAuth 登录失败：${ctx.form.getValues().name || "OAuth Provider"}`,
        { cli_key: ctx.cliKey, provider_id: targetProviderId }
      );
    }
  } catch (err) {
    await rollbackAutoSavedProvider();
    toast(`OAuth 登录失败：${String(err)}`);
    logToConsole(
      "error",
      `OAuth 登录异常：${ctx.form.getValues().name || "OAuth Provider"}`,
      { cli_key: ctx.cliKey, error: String(err) }
    );
  } finally {
    ctx.setOauthLoading(false);
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
