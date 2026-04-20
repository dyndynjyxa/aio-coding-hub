import {
  createProviderEditorDialogSchema,
  validateProviderClaudeModels,
} from "../../schemas/providerEditorDialog";
import type {
  ProviderEditorPayloadBuildError,
  ProviderEditorPayloadBuildSuccess,
  ProviderEditorPayloadContext,
} from "./providerEditorActionContext";
import { normalizeBaseUrlRows } from "./baseUrl";
import { resolveStreamIdleTimeoutSeconds } from "./providerEditorTimeout";

export function buildProviderEditorUpsertInput(
  ctx: ProviderEditorPayloadContext
):
  | { ok: true; value: ProviderEditorPayloadBuildSuccess }
  | { ok: false; error: ProviderEditorPayloadBuildError } {
  const parsed = createProviderEditorDialogSchema({
    mode: ctx.mode,
    skipApiKeyCheck: ctx.authMode === "cx2cc",
  }).safeParse({
    ...ctx.formValues,
    auth_mode: ctx.authMode === "cx2cc" ? "api_key" : ctx.authMode,
  });

  if (!parsed.success) {
    return {
      ok: false,
      error: {
        kind: "schema",
        issues: parsed.error.issues,
      },
    };
  }

  const parsedTimeout = resolveStreamIdleTimeoutSeconds(ctx.streamIdleTimeoutSeconds);
  if (parsedTimeout === undefined) {
    return {
      ok: false,
      error: {
        kind: "message",
        message: "流式空闲超时必须为 0-3600 秒",
      },
    };
  }

  if (ctx.authMode === "api_key" && ctx.mode === "edit") {
    const nextApiKey = parsed.data.api_key.trim();
    if (!nextApiKey && !ctx.apiKeyConfigured) {
      return {
        ok: false,
        error: {
          kind: "message",
          message: "请输入 API Key",
        },
      };
    }
  }

  let finalBaseUrls: string[] = [];
  let finalBaseUrlMode = ctx.baseUrlMode;

  if (ctx.authMode === "oauth") {
    finalBaseUrls = [];
  } else if (ctx.authMode === "cx2cc") {
    finalBaseUrls = [];
    finalBaseUrlMode = "order";

    if (!ctx.sourceProviderId && !ctx.isCodexGatewaySource) {
      return {
        ok: false,
        error: {
          kind: "message",
          message: "请选择源 Codex 来源",
        },
      };
    }
  } else {
    const normalized = normalizeBaseUrlRows(ctx.baseUrlRows);
    if (!normalized.ok) {
      return {
        ok: false,
        error: {
          kind: "message",
          message: normalized.message,
        },
      };
    }
    finalBaseUrls = normalized.baseUrls;
  }

  if (ctx.cliKey === "claude" && ctx.authMode !== "oauth") {
    const modelError = validateProviderClaudeModels(ctx.claudeModels);
    if (modelError) {
      return {
        ok: false,
        error: {
          kind: "message",
          message: modelError,
        },
      };
    }
  }

  const effectiveCostMultiplier =
    ctx.authMode === "cx2cc" && ctx.isCodexGatewaySource
      ? 0
      : ctx.authMode === "cx2cc" && ctx.selectedCx2ccSourceProvider
        ? ctx.selectedCx2ccSourceProvider.cost_multiplier
        : parsed.data.cost_multiplier;

  const payload = {
    ...(ctx.editingProviderId ? { provider_id: ctx.editingProviderId } : {}),
    cli_key: ctx.cliKey,
    name: parsed.data.name,
    base_urls: finalBaseUrls,
    base_url_mode: finalBaseUrlMode,
    auth_mode: ctx.authMode === "cx2cc" ? "api_key" : ctx.authMode,
    api_key:
      ctx.authMode === "oauth" || ctx.authMode === "cx2cc"
        ? null
        : parsed.data.api_key.trim() || null,
    enabled: parsed.data.enabled,
    cost_multiplier: effectiveCostMultiplier,
    limit_5h_usd: parsed.data.limit_5h_usd,
    limit_daily_usd: parsed.data.limit_daily_usd,
    daily_reset_mode: parsed.data.daily_reset_mode,
    daily_reset_time: parsed.data.daily_reset_time,
    limit_weekly_usd: parsed.data.limit_weekly_usd,
    limit_monthly_usd: parsed.data.limit_monthly_usd,
    limit_total_usd: parsed.data.limit_total_usd,
    tags: ctx.tags,
    note: parsed.data.note,
    stream_idle_timeout_seconds: parsedTimeout,
    ...(ctx.cliKey === "claude" && ctx.authMode !== "oauth"
      ? { claude_models: ctx.claudeModels }
      : {}),
    source_provider_id:
      ctx.authMode === "cx2cc" && !ctx.isCodexGatewaySource ? ctx.sourceProviderId : null,
    bridge_type: ctx.authMode === "cx2cc" ? "cx2cc" : null,
  };

  return {
    ok: true,
    value: {
      payload,
      parsedName: parsed.data.name,
    },
  };
}
