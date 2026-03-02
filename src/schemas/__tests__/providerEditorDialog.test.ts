// Usage: Unit tests for ProviderEditorDialog zod schema (no DOM).

import { describe, expect, it } from "vitest";
import { createProviderEditorDialogSchema } from "../providerEditorDialog";

describe("schemas/providerEditorDialog", () => {
  it("requires api_key only in create mode", () => {
    const createSchema = createProviderEditorDialogSchema({ mode: "create" });
    const editSchema = createProviderEditorDialogSchema({ mode: "edit" });

    const base = {
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
    } as const;

    const createRes = createSchema.safeParse(base);
    expect(createRes.success).toBe(false);
    if (!createRes.success) {
      expect(
        createRes.error.issues.some(
          (issue) => issue.message === "API Key 不能为空（新增 Provider 必填）"
        )
      ).toBe(true);
    }

    const editRes = editSchema.safeParse(base);
    expect(editRes.success).toBe(true);
  });

  it("normalizes daily_reset_time and validates range", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });

    const res = schema.safeParse({
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "01:02",
      enabled: true,
    });
    expect(res.success).toBe(true);
    if (res.success) {
      expect(res.data.daily_reset_time).toBe("01:02:00");
    }

    const bad = schema.safeParse({
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "25:00:00",
      enabled: true,
    });
    expect(bad.success).toBe(false);
    if (!bad.success) {
      expect(
        bad.error.issues.some(
          (issue) => issue.message === "固定重置时间必须在 00:00:00 到 23:59:59 之间"
        )
      ).toBe(true);
    }
  });

  it("parses limits and validates key messages", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });

    const ok = schema.safeParse({
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: " 12.5 ",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
    });
    expect(ok.success).toBe(true);
    if (ok.success) {
      expect(ok.data.limit_daily_usd).toBe(12.5);
      expect(ok.data.limit_5h_usd).toBeNull();
    }

    const badCost = schema.safeParse({
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
    });
    expect(badCost.success).toBe(false);
    if (!badCost.success) {
      expect(badCost.error.issues.some((issue) => issue.message === "价格倍率必须大于 0")).toBe(
        true
      );
    }

    const badLimit = schema.safeParse({
      name: "n",
      auth_type: "api_key",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "NaN",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
    });
    expect(badLimit.success).toBe(false);
    if (!badLimit.success) {
      expect(
        badLimit.error.issues.some((issue) => issue.message === "每日消费上限 必须是数字")
      ).toBe(true);
    }
  });
});
