import { describe, expect, it } from "vitest";
import {
  applyPersistedSettingsPatch,
  buildPersistedSettingsMutationInput,
  buildPersistedSettingsSnapshot,
  diffPersistedSettings,
  replacePersistedSettingsKeys,
  validatePersistedSettings,
  DEFAULT_PERSISTED_SETTINGS,
} from "../settingsPersistenceModel";

describe("pages/settings/settingsPersistenceModel", () => {
  it("builds a normalized persisted snapshot from settings query data", () => {
    const snapshot = buildPersistedSettingsSnapshot({
      ...DEFAULT_PERSISTED_SETTINGS,
      cli_priority_order: ["gemini", "claude", "codex", "claude"],
      show_home_heatmap: undefined as never,
      show_home_usage: undefined as never,
      start_minimized: undefined as never,
      tray_enabled: undefined as never,
    } as any);

    expect(snapshot.show_home_heatmap).toBe(DEFAULT_PERSISTED_SETTINGS.show_home_heatmap);
    expect(snapshot.show_home_usage).toBe(DEFAULT_PERSISTED_SETTINGS.show_home_usage);
    expect(snapshot.start_minimized).toBe(DEFAULT_PERSISTED_SETTINGS.start_minimized);
    expect(snapshot.tray_enabled).toBe(DEFAULT_PERSISTED_SETTINGS.tray_enabled);
    expect(snapshot.cli_priority_order).toEqual(["gemini", "claude", "codex"]);
  });

  it("diffs and selectively replaces persisted keys", () => {
    const desired = applyPersistedSettingsPatch(DEFAULT_PERSISTED_SETTINGS, {
      preferred_port: 38000,
      cli_priority_order: ["codex", "claude", "gemini"],
      auto_start: true,
    });

    expect(diffPersistedSettings(DEFAULT_PERSISTED_SETTINGS, desired)).toEqual([
      "preferred_port",
      "cli_priority_order",
      "auto_start",
    ]);

    expect(
      replacePersistedSettingsKeys(desired, DEFAULT_PERSISTED_SETTINGS, [
        "preferred_port",
        "auto_start",
      ])
    ).toEqual({
      ...desired,
      preferred_port: DEFAULT_PERSISTED_SETTINGS.preferred_port,
      auto_start: DEFAULT_PERSISTED_SETTINGS.auto_start,
    });
  });

  it("validates numeric persisted fields with feature-specific rules", () => {
    expect(
      validatePersistedSettings(
        applyPersistedSettingsPatch(DEFAULT_PERSISTED_SETTINGS, {
          preferred_port: 80,
        }),
        ["preferred_port"]
      )
    ).toBe("端口号必须为 1024-65535");

    expect(
      validatePersistedSettings(
        applyPersistedSettingsPatch(DEFAULT_PERSISTED_SETTINGS, {
          upstream_stream_idle_timeout_seconds: 10,
        }),
        ["upstream_stream_idle_timeout_seconds"]
      )
    ).toBe("上游流式空闲超时必须为 0（禁用）或 60-3600 秒");

    expect(
      validatePersistedSettings(
        applyPersistedSettingsPatch(DEFAULT_PERSISTED_SETTINGS, {
          circuit_breaker_failure_threshold: 8,
        }),
        ["circuit_breaker_failure_threshold"]
      )
    ).toBeNull();
  });

  it("builds the generated settings mutation payload from persisted draft state", () => {
    const input = buildPersistedSettingsMutationInput(
      applyPersistedSettingsPatch(DEFAULT_PERSISTED_SETTINGS, {
        preferred_port: 38080,
        show_home_heatmap: false,
        cli_priority_order: ["codex", "gemini", "claude"],
      })
    );

    expect(input).toEqual({
      preferredPort: 38080,
      showHomeHeatmap: false,
      showHomeUsage: true,
      homeUsagePeriod: "last15",
      cliPriorityOrder: ["codex", "gemini", "claude"],
      autoStart: false,
      startMinimized: false,
      trayEnabled: true,
      logRetentionDays: 7,
      providerCooldownSeconds: 30,
      providerBaseUrlPingCacheTtlSeconds: 60,
      upstreamFirstByteTimeoutSeconds: 0,
      upstreamStreamIdleTimeoutSeconds: 0,
      upstreamRequestTimeoutNonStreamingSeconds: 0,
      interceptAnthropicWarmupRequests: false,
      enableThinkingSignatureRectifier: true,
      enableResponseFixer: true,
      responseFixerFixEncoding: true,
      responseFixerFixSseFormat: true,
      responseFixerFixTruncatedJson: true,
      failoverMaxAttemptsPerProvider: 5,
      failoverMaxProvidersToTry: 5,
      circuitBreakerFailureThreshold: 5,
      circuitBreakerOpenDurationMinutes: 30,
    });
  });
});
