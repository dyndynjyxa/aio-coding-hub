// Usage: Unit tests for ProviderEditorDialog zod schema (no DOM).

import { describe, expect, it } from "vitest";
import { createProviderEditorDialogSchema } from "../providerEditorDialog";

describe("schemas/providerEditorDialog", () => {
  it("requires api_key only in create mode", () => {
    const createSchema = createProviderEditorDialogSchema({ mode: "create" });
    const editSchema = createProviderEditorDialogSchema({ mode: "edit" });

    const base = {
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
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
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "01:02",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(true);
    if (res.success) {
      expect(res.data.daily_reset_time).toBe("01:02:00");
    }

    const bad = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "25:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
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
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: " 12.5 ",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(ok.success).toBe(true);
    if (ok.success) {
      expect(ok.data.limit_daily_usd).toBe(12.5);
      expect(ok.data.limit_5h_usd).toBeNull();
    }

    const badCost = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "-1",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(badCost.success).toBe(false);
    if (!badCost.success) {
      expect(badCost.error.issues.some((issue) => issue.message === "价格倍率必须大于等于 0")).toBe(
        true
      );
    }

    const badLimit = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "NaN",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(badLimit.success).toBe(false);
    if (!badLimit.success) {
      expect(
        badLimit.error.issues.some((issue) => issue.message === "每日消费上限 必须是数字")
      ).toBe(true);
    }
  });

  it("rejects cost_multiplier above 1000", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1001",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(false);
    if (!res.success) {
      expect(res.error.issues.some((i) => i.message === "价格倍率不能大于 1000")).toBe(true);
    }
  });

  it("rejects non-finite cost_multiplier", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "abc",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(false);
    if (!res.success) {
      expect(res.error.issues.some((i) => i.message === "价格倍率必须是数字")).toBe(true);
    }
  });

  it("rejects negative limit_usd values", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "-1",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(false);
    if (!res.success) {
      expect(res.error.issues.some((i) => i.message === "5 小时消费上限 必须大于等于 0")).toBe(
        true
      );
    }
  });

  it("rejects limit_usd exceeding max", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "2000000000",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(false);
    if (!res.success) {
      expect(res.error.issues.some((i) => i.message.includes("5 小时消费上限 不能大于"))).toBe(
        true
      );
    }
  });

  it("rejects invalid daily_reset_time format", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "not-a-time",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(false);
    if (!res.success) {
      expect(
        res.error.issues.some((i) => i.message.includes("固定重置时间格式必须为 HH:mm:ss"))
      ).toBe(true);
    }
  });

  it("defaults empty daily_reset_time to 00:00:00", () => {
    const schema = createProviderEditorDialogSchema({ mode: "edit" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "api_key",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(true);
    if (res.success) {
      expect(res.data.daily_reset_time).toBe("00:00:00");
    }
  });

  it("skips api_key check in create mode when auth_mode is oauth", () => {
    const schema = createProviderEditorDialogSchema({ mode: "create" });
    const res = schema.safeParse({
      name: "n",
      api_key: "",
      auth_mode: "oauth",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
      supports_websockets: false,
      note: "",
    });
    expect(res.success).toBe(true);
  });
});
