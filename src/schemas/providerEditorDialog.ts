// Usage: Zod schema for ProviderEditorDialog (RHF + toast-based submit validation).

import { z } from "zod";

const MAX_LIMIT_USD = 1_000_000_000;

function parseCostMultiplier() {
  return z.string().transform((raw, ctx) => {
    const value = Number(raw);
    if (!Number.isFinite(value)) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: "价格倍率必须是数字" });
      return z.NEVER;
    }
    if (value < 0) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: "价格倍率必须大于等于 0" });
      return z.NEVER;
    }
    if (value > 1000) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: "价格倍率不能大于 1000" });
      return z.NEVER;
    }
    return value;
  });
}

function parseLimitUsd(label: string) {
  return z.string().transform((raw, ctx) => {
    const trimmed = raw.trim();
    if (!trimmed) return null;
    const value = Number(trimmed);
    if (!Number.isFinite(value)) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: `${label} 必须是数字` });
      return z.NEVER;
    }
    if (value < 0) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: `${label} 必须大于等于 0` });
      return z.NEVER;
    }
    if (value > MAX_LIMIT_USD) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, message: `${label} 不能大于 ${MAX_LIMIT_USD}` });
      return z.NEVER;
    }
    return value;
  });
}

function pad2(value: number) {
  return String(value).padStart(2, "0");
}

function parseResetTimeHms() {
  return z.string().transform((raw, ctx) => {
    const trimmed = raw.trim();
    if (!trimmed) return "00:00:00";

    const match = /^([0-9]{1,2}):([0-9]{2})(?::([0-9]{2}))?$/.exec(trimmed);
    if (!match) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "固定重置时间格式必须为 HH:mm:ss（或 HH:mm）",
      });
      return z.NEVER;
    }

    const hours = Number(match[1]);
    const minutes = Number(match[2]);
    const seconds = match[3] ? Number(match[3]) : 0;

    if (
      !Number.isInteger(hours) ||
      !Number.isInteger(minutes) ||
      !Number.isInteger(seconds) ||
      hours < 0 ||
      hours > 23 ||
      minutes < 0 ||
      minutes > 59 ||
      seconds < 0 ||
      seconds > 59
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "固定重置时间必须在 00:00:00 到 23:59:59 之间",
      });
      return z.NEVER;
    }

    return `${pad2(hours)}:${pad2(minutes)}:${pad2(seconds)}`;
  });
}

export function createProviderEditorDialogSchema(options: {
  mode: "create" | "edit";
  skipApiKeyCheck?: boolean;
}) {
  return z
    .object({
      name: z.string().trim().min(1, { message: "名称不能为空" }),
      api_key: z.string(),
      auth_mode: z.enum(["api_key", "oauth"]),
      cost_multiplier: parseCostMultiplier(),
      limit_5h_usd: parseLimitUsd("5 小时消费上限"),
      limit_daily_usd: parseLimitUsd("每日消费上限"),
      limit_weekly_usd: parseLimitUsd("周消费上限"),
      limit_monthly_usd: parseLimitUsd("月消费上限"),
      limit_total_usd: parseLimitUsd("总消费上限"),
      daily_reset_mode: z.enum(["fixed", "rolling"]),
      daily_reset_time: parseResetTimeHms(),
      enabled: z.boolean(),
      note: z.string().trim().max(500, { message: "备注不能超过 500 字符" }),
    })
    .superRefine((values, ctx) => {
      if (options.mode !== "create") return;
      if (options.skipApiKeyCheck) return;
      if (values.auth_mode === "oauth") return;
      if (values.api_key.trim()) return;
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["api_key"],
        message: "API Key 不能为空（新增 Provider 必填）",
      });
    });
}

export const providerEditorDialogSchemaCreate = createProviderEditorDialogSchema({
  mode: "create",
});
export type ProviderEditorDialogFormInput = z.input<typeof providerEditorDialogSchemaCreate>;
export type ProviderEditorDialogFormOutput = z.output<typeof providerEditorDialogSchemaCreate>;

const MAX_MODEL_NAME_LEN = 200;

export function validateProviderClaudeModels(input: {
  main_model?: string | null;
  reasoning_model?: string | null;
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
}) {
  const fields: Array<[label: string, value: string | null | undefined]> = [
    ["主模型", input.main_model],
    ["推理模型(Thinking)", input.reasoning_model],
    ["Haiku 默认模型", input.haiku_model],
    ["Sonnet 默认模型", input.sonnet_model],
    ["Opus 默认模型", input.opus_model],
  ];

  for (const [label, value] of fields) {
    const trimmed = (value ?? "").trim();
    if (!trimmed) continue;
    if (trimmed.length > MAX_MODEL_NAME_LEN) {
      return `${label} 过长（最多 ${MAX_MODEL_NAME_LEN} 字符）`;
    }
  }

  return null;
}
