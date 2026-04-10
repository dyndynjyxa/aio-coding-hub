import { beforeEach, describe, expect, it } from "vitest";
import { providerUpsert } from "../providers";
import { getProvidersState, setProvidersState } from "../../../test/msw/state";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/providers via MSW bridge", () => {
  beforeEach(() => {
    setTauriRuntime();
    setProvidersState("claude", []);
  });

  it("persists provider_upsert with nested input payload through tauri bridge", async () => {
    const saved = await providerUpsert({
      cli_key: "claude",
      name: "Bridge Provider",
      base_urls: ["https://api.example.com"],
      base_url_mode: "order",
      auth_mode: "api_key",
      api_key: "sk-test",
      enabled: true,
      cost_multiplier: 1.5,
      priority: 8,
      claude_models: null,
      limit_5h_usd: 5,
      limit_daily_usd: 10,
      daily_reset_mode: "fixed",
      daily_reset_time: "01:02:03",
      limit_weekly_usd: 15,
      limit_monthly_usd: 20,
      limit_total_usd: 25,
      tags: ["a", "b"],
      note: "hello",
    });

    expect(saved).toMatchObject({
      cli_key: "claude",
      name: "Bridge Provider",
      base_urls: ["https://api.example.com"],
      base_url_mode: "order",
      limit_5h_usd: 5,
      daily_reset_mode: "fixed",
      daily_reset_time: "01:02:03",
      auth_mode: "api_key",
      tags: ["a", "b"],
      note: "hello",
    });

    expect(getProvidersState("claude")).toHaveLength(1);
    expect(getProvidersState("claude")[0]).toMatchObject({
      name: "Bridge Provider",
      limit_5h_usd: 5,
    });
  });

  it("preserves stream idle timeout when omitted and clears it when null is submitted", async () => {
    const baseInput = {
      cli_key: "claude" as const,
      name: "Timeout Provider",
      base_urls: ["https://api.example.com"],
      base_url_mode: "order" as const,
      auth_mode: "api_key" as const,
      api_key: "sk-test",
      enabled: true,
      cost_multiplier: 1,
      priority: 1,
      claude_models: null,
      limit_5h_usd: null,
      limit_daily_usd: null,
      daily_reset_mode: "fixed" as const,
      daily_reset_time: "00:00:00",
      limit_weekly_usd: null,
      limit_monthly_usd: null,
      limit_total_usd: null,
      tags: [],
      note: "",
    };

    const created = await providerUpsert({
      ...baseInput,
      stream_idle_timeout_seconds: 120,
    });
    expect(created?.stream_idle_timeout_seconds).toBe(120);

    const preserved = await providerUpsert({
      ...baseInput,
      provider_id: created?.id,
      name: "Timeout Provider Updated",
      api_key: undefined,
    });
    expect(preserved?.stream_idle_timeout_seconds).toBe(120);

    const cleared = await providerUpsert({
      ...baseInput,
      provider_id: created?.id,
      name: "Timeout Provider Cleared",
      api_key: undefined,
      stream_idle_timeout_seconds: null,
    });
    expect(cleared?.stream_idle_timeout_seconds).toBeNull();
  });
});
