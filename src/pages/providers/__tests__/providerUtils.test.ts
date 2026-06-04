import { describe, expect, it } from "vitest";
import {
  MAX_PROVIDER_BASE_URLS,
  MAX_PROVIDER_BASE_URL_CHARS,
  normalizeBaseUrlRows,
  providerBaseUrlSummary,
  providerPrimaryBaseUrl,
} from "../baseUrl";
import { validateProviderClaudeModels } from "../../../schemas/providerEditorDialog";

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

  it("rejects base url boundary overflows before submit", () => {
    const tooManyRows = Array.from({ length: MAX_PROVIDER_BASE_URLS + 1 }, (_, index) => ({
      id: String(index),
      url: `https://api-${index}.example.com`,
      ping: { status: "idle" },
    }));
    const tooMany = normalizeBaseUrlRows(tooManyRows as any);
    expect(tooMany.ok).toBe(false);
    if (!tooMany.ok) expect(tooMany.message).toMatch(/最多支持/);

    const tooLong = normalizeBaseUrlRows([
      {
        id: "long",
        url: `https://example.com/${"a".repeat(MAX_PROVIDER_BASE_URL_CHARS)}`,
        ping: { status: "idle" },
      },
    ] as any);
    expect(tooLong.ok).toBe(false);
    if (!tooLong.ok) expect(tooLong.message).toMatch(/不能超过/);
  });
});

describe("validateProviderClaudeModels", () => {
  it("validates Claude model mapping length", () => {
    expect(validateProviderClaudeModels({ main_model: "x".repeat(201) })).toMatch(/过长/);
    expect(validateProviderClaudeModels({ main_model: "ok" })).toBeNull();
  });
});
