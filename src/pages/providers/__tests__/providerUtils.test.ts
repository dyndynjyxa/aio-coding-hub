import { describe, expect, it } from "vitest";
import { normalizeBaseUrlRows, providerBaseUrlSummary, providerPrimaryBaseUrl } from "../baseUrl";
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
});

describe("validateProviderClaudeModels", () => {
  it("validates Claude model mapping length", () => {
    expect(validateProviderClaudeModels({ main_model: "x".repeat(201) })).toMatch(/过长/);
    expect(validateProviderClaudeModels({ main_model: "ok" })).toBeNull();
  });
});
