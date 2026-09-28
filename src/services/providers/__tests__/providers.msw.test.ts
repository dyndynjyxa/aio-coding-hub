import { beforeEach, describe, expect, it } from "vitest";
import { providerUpsert } from "../providers";
import { getProvidersState, setProvidersState } from "../../../test/msw/state";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/providers via MSW bridge", () => {
  beforeEach(() => {
    setTauriRuntime();
    setProvidersState("claude", []);
    setProvidersState("grok", []);
  });

  it("persists a Grok API key provider through the MSW bridge", async () => {
    const saved = await providerUpsert({
      cliKey: "grok",
      name: "Grok Provider",
      baseUrls: ["https://api.example.test/v1"],
      baseUrlMode: "order",
      authMode: "api_key",
      apiKey: "test-api-key",
      enabled: true,
      costMultiplier: 1,
      priority: 100,
      claudeModels: null,
      limit5hUsd: null,
      limitDailyUsd: null,
      dailyResetMode: "fixed",
      dailyResetTime: "00:00:00",
      limitWeeklyUsd: null,
      limitMonthlyUsd: null,
      limitTotalUsd: null,
      tags: [],
      note: "",
    });

    expect(saved).toMatchObject({ cli_key: "grok", auth_mode: "api_key" });
    expect(getProvidersState("grok")).toHaveLength(1);
  });

  it("persists provider_upsert with nested input payload through tauri bridge", async () => {
    const saved = await providerUpsert({
      cliKey: "claude",
      name: "Bridge Provider",
      baseUrls: ["https://api.example.com"],
      baseUrlMode: "order",
      authMode: "api_key",
      apiKey: "sk-test",
      enabled: true,
      costMultiplier: 1.5,
      priority: 8,
      claudeModels: null,
      limit5hUsd: 5,
      limitDailyUsd: 10,
      dailyResetMode: "fixed",
      dailyResetTime: "01:02:03",
      limitWeeklyUsd: 15,
      limitMonthlyUsd: 20,
      limitTotalUsd: 25,
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
      cliKey: "claude" as const,
      name: "Timeout Provider",
      baseUrls: ["https://api.example.com"],
      baseUrlMode: "order" as const,
      authMode: "api_key" as const,
      apiKey: "sk-test",
      enabled: true,
      costMultiplier: 1,
      priority: 1,
      claudeModels: null,
      limit5hUsd: null,
      limitDailyUsd: null,
      dailyResetMode: "fixed" as const,
      dailyResetTime: "00:00:00",
      limitWeeklyUsd: null,
      limitMonthlyUsd: null,
      limitTotalUsd: null,
      tags: [],
      note: "",
    };

    const created = await providerUpsert({
      ...baseInput,
      streamIdleTimeoutSeconds: 120,
    });
    expect(created?.stream_idle_timeout_seconds).toBe(120);

    const preserved = await providerUpsert({
      ...baseInput,
      providerId: created?.id,
      name: "Timeout Provider Updated",
      apiKey: undefined,
    });
    expect(preserved?.stream_idle_timeout_seconds).toBe(120);

    const cleared = await providerUpsert({
      ...baseInput,
      providerId: created?.id,
      name: "Timeout Provider Cleared",
      apiKey: undefined,
      streamIdleTimeoutSeconds: null,
    });
    expect(cleared?.stream_idle_timeout_seconds).toBeNull();
  });
});
