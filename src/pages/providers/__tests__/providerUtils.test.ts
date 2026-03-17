import { describe, expect, it } from "vitest";
import { normalizeBaseUrlRows, providerBaseUrlSummary, providerPrimaryBaseUrl } from "../baseUrl";
import {
  parseAndValidateCostMultiplier,
  parseAndValidateLimitUsd,
  parseAndNormalizeResetTimeHms,
  validateProviderApiKeyForCreate,
  validateProviderClaudeModels,
  validateProviderName,
} from "../validators";

describe("pages/providers/baseUrl helpers", () => {
  it("summarizes provider base urls", () => {
    expect(providerPrimaryBaseUrl(null)).toBe("—");
    expect(providerPrimaryBaseUrl({ base_urls: ["https://a"] } as any)).toBe("https://a");
    expect(providerBaseUrlSummary({ base_urls: ["https://a"] } as any)).toBe("https://a");
    expect(providerBaseUrlSummary({ base_urls: ["https://a", "https://b"] } as any)).toBe(
      "https://a · https://b"
    );
    expect(
      providerBaseUrlSummary({ base_urls: ["https://a", "https://b", "https://c"] } as any)
    ).toBe("https://a · https://b (+1)");
  });

  it("normalizes base url rows with validation", () => {
    expect(normalizeBaseUrlRows([] as any).ok).toBe(false);
    expect(
      normalizeBaseUrlRows([{ id: "1", url: "   ", ping: { status: "idle" } }] as any).ok
    ).toBe(false);
    expect(
      normalizeBaseUrlRows([{ id: "1", url: "ftp://x", ping: { status: "idle" } }] as any).ok
    ).toBe(false);
    expect(
      normalizeBaseUrlRows([{ id: "1", url: "not-a-url", ping: { status: "idle" } }] as any).ok
    ).toBe(false);
    expect(
      normalizeBaseUrlRows([
        { id: "1", url: "https://a", ping: { status: "idle" } },
        { id: "2", url: "https://a", ping: { status: "idle" } },
      ] as any).ok
    ).toBe(false);

    const ok = normalizeBaseUrlRows([
      { id: "1", url: "https://a", ping: { status: "idle" } },
      { id: "2", url: " https://b ", ping: { status: "idle" } },
    ] as any);
    expect(ok.ok).toBe(true);
    if (ok.ok) expect(ok.baseUrls).toEqual(["https://a", "https://b"]);
  });
});

describe("pages/providers/validators", () => {
  it("validates required fields", () => {
    expect(validateProviderName("")).toBeTruthy();
    expect(validateProviderName("ok")).toBeNull();
    expect(validateProviderApiKeyForCreate("")).toBeTruthy();
    expect(validateProviderApiKeyForCreate("sk-xxx")).toBeNull();
  });

  it("validates cost multiplier range", () => {
    expect(parseAndValidateCostMultiplier("NaN").ok).toBe(false);
    expect(parseAndValidateCostMultiplier("0").ok).toBe(false);
    expect(parseAndValidateCostMultiplier("1001").ok).toBe(false);
    const ok = parseAndValidateCostMultiplier("1.5");
    expect(ok.ok).toBe(true);
    if (ok.ok) expect(ok.value).toBe(1.5);
  });

  it("parses limit USD with validation", () => {
    expect(parseAndValidateLimitUsd("", "上限")).toEqual({ ok: true, value: null });
    expect(parseAndValidateLimitUsd("   ", "上限")).toEqual({ ok: true, value: null });

    expect(parseAndValidateLimitUsd("NaN", "上限")).toEqual({
      ok: false,
      message: "上限 必须是数字",
    });
    expect(parseAndValidateLimitUsd("-1", "上限")).toEqual({
      ok: false,
      message: "上限 必须大于等于 0",
    });
    expect(parseAndValidateLimitUsd("1000000001", "上限")).toEqual({
      ok: false,
      message: "上限 不能大于 1000000000",
    });

    expect(parseAndValidateLimitUsd("0", "上限")).toEqual({ ok: true, value: 0 });
    expect(parseAndValidateLimitUsd(" 12.5 ", "上限")).toEqual({ ok: true, value: 12.5 });
  });

  it("parses and normalizes reset time (HH:mm or HH:mm:ss)", () => {
    expect(parseAndNormalizeResetTimeHms("")).toEqual({ ok: true, value: "00:00:00" });
    expect(parseAndNormalizeResetTimeHms("   ")).toEqual({ ok: true, value: "00:00:00" });

    expect(parseAndNormalizeResetTimeHms("1:2")).toEqual({
      ok: false,
      message: "固定重置时间格式必须为 HH:mm:ss（或 HH:mm）",
    });
    expect(parseAndNormalizeResetTimeHms("24:00")).toEqual({
      ok: false,
      message: "固定重置时间必须在 00:00:00 到 23:59:59 之间",
    });
    expect(parseAndNormalizeResetTimeHms("23:60")).toEqual({
      ok: false,
      message: "固定重置时间必须在 00:00:00 到 23:59:59 之间",
    });

    expect(parseAndNormalizeResetTimeHms("1:02")).toEqual({ ok: true, value: "01:02:00" });
    expect(parseAndNormalizeResetTimeHms(" 1:02:03 ")).toEqual({ ok: true, value: "01:02:03" });
  });

  it("validates Claude model mapping length", () => {
    expect(validateProviderClaudeModels({ main_model: "x".repeat(201) })).toMatch(/过长/);
    expect(validateProviderClaudeModels({ main_model: "ok" })).toBeNull();
  });
});
