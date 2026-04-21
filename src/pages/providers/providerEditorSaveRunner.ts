import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import type { SaveActionContext } from "./providerEditorActionContext";
import { presentProviderEditorPayloadBuildError } from "./providerEditorFeedback";
import { buildProviderEditorUpsertInput } from "./providerEditorSubmitModel";

export async function runProviderEditorSave(ctx: SaveActionContext) {
  if (ctx.saving) return;

  const built = buildProviderEditorUpsertInput({
    ...ctx,
    formValues: ctx.form.getValues(),
  });
  if (!built.ok) {
    presentProviderEditorPayloadBuildError(ctx.mode, built.error);
    return;
  }

  if (ctx.authMode === "oauth") {
    let effectiveOauthStatus = ctx.oauthStatus;
    if (!effectiveOauthStatus?.connected && ctx.editingProviderId) {
      try {
        const latestStatus = await ctx.refreshOauthStatus(ctx.editingProviderId);
        ctx.setOauthStatus(latestStatus);
        effectiveOauthStatus = latestStatus;
      } catch (err) {
        logToConsole("warn", "保存前刷新 OAuth 状态失败", {
          cli_key: ctx.cliKey,
          provider_id: ctx.editingProviderId,
          error: String(err),
        });
      }
    }

    if (!effectiveOauthStatus?.connected) {
      toast("请先完成 OAuth 登录");
      return;
    }
  }

  ctx.setSaving(true);
  try {
    const saved = await ctx.persistProvider(built.value.payload);
    if (!saved) {
      return;
    }

    ctx.form.setValue("api_key", "", { shouldDirty: false, shouldValidate: false });
    logToConsole("info", ctx.mode === "create" ? "保存 Provider" : "更新 Provider", {
      cli: saved.cli_key,
      provider_id: saved.id,
      name: saved.name,
      base_urls: saved.base_urls,
      base_url_mode: saved.base_url_mode,
      enabled: saved.enabled,
      cost_multiplier: saved.cost_multiplier,
      claude_models: saved.claude_models,
      limit_5h_usd: saved.limit_5h_usd,
      limit_daily_usd: saved.limit_daily_usd,
      daily_reset_mode: saved.daily_reset_mode,
      daily_reset_time: saved.daily_reset_time,
      limit_weekly_usd: saved.limit_weekly_usd,
      limit_monthly_usd: saved.limit_monthly_usd,
      limit_total_usd: saved.limit_total_usd,
      tags: saved.tags,
      note: saved.note,
      stream_idle_timeout_seconds: saved.stream_idle_timeout_seconds,
    });
    toast(ctx.mode === "create" ? "Provider 已保存" : "Provider 已更新");

    ctx.onSaved(saved.cli_key);
    ctx.onOpenChange(false);
  } catch (err) {
    logToConsole(
      "error",
      ctx.mode === "create" ? "保存 Provider 失败" : "更新 Provider 失败",
      {
        error: String(err),
        cli: ctx.cliKey,
        provider_id: ctx.mode === "edit" && ctx.editProvider ? ctx.editProvider.id : undefined,
      }
    );
    toast(`${ctx.mode === "create" ? "保存" : "更新"}失败：${String(err)}`);
  } finally {
    ctx.setSaving(false);
  }
}
