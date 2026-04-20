import { describe, expect, it } from "vitest";
import {
  buildDuplicatedProviderInitialValues,
  buildDuplicatedProviderName,
} from "../providerDuplicate";

function createProvider(overrides: Record<string, unknown> = {}) {
  return {
    id: 1,
    cli_key: "claude",
    name: "Alpha",
    base_urls: ["https://example.com"],
    base_url_mode: "order",
    claude_models: { main_model: "claude-sonnet" },
    enabled: true,
    priority: 1,
    cost_multiplier: 1,
    limit_5h_usd: 1,
    limit_daily_usd: 2,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00",
    limit_weekly_usd: 3,
    limit_monthly_usd: 4,
    limit_total_usd: 5,
    tags: ["core"],
    note: "provider note",
    created_at: 1,
    updated_at: 1,
    auth_mode: "api_key",
    oauth_provider_type: null,
    oauth_email: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    source_provider_id: null,
    bridge_type: null,
    ...overrides,
  } as any;
}

describe("pages/providers/providerDuplicate", () => {
  it("builds duplicate names with trim, case-insensitive matching, and incremented suffixes", () => {
    const existingProviders = [
      createProvider({ name: "Provider 副本" }),
      createProvider({ id: 2, name: " provider 副本 2 " }),
    ];

    expect(buildDuplicatedProviderName("   ", existingProviders)).toBe("Provider 副本 3");
    expect(buildDuplicatedProviderName("Alpha", [createProvider({ name: "alpha 副本" })])).toBe(
      "Alpha 副本 2"
    );
  });

  it("duplicates editable values for api-key providers and still clears sensitive api key", () => {
    const provider = createProvider({
      name: "Alpha",
      base_urls: ["https://a.example.com", "https://b.example.com"],
      claude_models: { main_model: "main", reasoning_model: "reasoning" },
      tags: ["tag-a", "tag-b"],
    });

    const duplicated = buildDuplicatedProviderInitialValues(
      provider,
      [provider, createProvider({ id: 9, name: "Alpha 副本" })],
      "sk-live"
    );

    expect(duplicated).toMatchObject({
      name: "Alpha 副本 2",
      api_key: "",
      auth_mode: "api_key",
      base_url_mode: "order",
      enabled: true,
      note: "provider note",
      source_provider_id: null,
      bridge_type: null,
      stream_idle_timeout_seconds: null,
    });
    expect(duplicated.base_urls).toEqual(["https://a.example.com", "https://b.example.com"]);
    expect(duplicated.tags).toEqual(["tag-a", "tag-b"]);
    expect(duplicated.claude_models).toEqual({
      main_model: "main",
      reasoning_model: "reasoning",
    });

    expect(duplicated.base_urls).not.toBe(provider.base_urls);
    expect(duplicated.tags).not.toBe(provider.tags);
    expect(duplicated.claude_models).not.toBe(provider.claude_models);
  });

  it("clears api key for bridge or oauth providers and falls back optional values safely", () => {
    const bridgeProvider = createProvider({
      name: "Bridge",
      auth_mode: "oauth",
      tags: null,
      note: null,
      claude_models: null,
      source_provider_id: 42,
      bridge_type: "proxy",
    });

    const duplicated = buildDuplicatedProviderInitialValues(bridgeProvider, [], "sk-should-clear");

    expect(duplicated).toMatchObject({
      name: "Bridge 副本",
      api_key: "",
      auth_mode: "oauth",
      tags: [],
      note: "",
      source_provider_id: 42,
      bridge_type: "proxy",
    });
    expect(duplicated.claude_models).toEqual({});
  });
});
