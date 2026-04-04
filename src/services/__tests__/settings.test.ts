import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../test/mocks/tauri";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";

describe("services/settings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("passes update as a single named parameter", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.mocked(tauriInvoke).mockResolvedValue({ schema_version: 1 } as any);

    const { settingsSet } = await import("../settings");

    await settingsSet({
      preferredPort: 37123,
      showHomeHeatmap: false,
      showHomeUsage: true,
      homeUsagePeriod: "last7",
      cliPriorityOrder: ["codex", "claude", "gemini"],
      autoStart: false,
      trayEnabled: true,
      logRetentionDays: 30,
      providerCooldownSeconds: 30,
      providerBaseUrlPingCacheTtlSeconds: 60,
      upstreamFirstByteTimeoutSeconds: 0,
      upstreamStreamIdleTimeoutSeconds: 0,
      upstreamRequestTimeoutNonStreamingSeconds: 0,
      failoverMaxAttemptsPerProvider: 5,
      failoverMaxProvidersToTry: 5,
      circuitBreakerFailureThreshold: 5,
      circuitBreakerOpenDurationMinutes: 30,
    });

    expect(tauriInvoke).toHaveBeenCalledWith(
      "settings_set",
      expect.objectContaining({
        update: expect.objectContaining({
          preferredPort: 37123,
          showHomeHeatmap: false,
          showHomeUsage: true,
          homeUsagePeriod: "last7",
          cliPriorityOrder: ["codex", "claude", "gemini"],
          autoStart: false,
        }),
      })
    );

    vi.mocked(tauriInvoke).mockClear();

    await settingsSet({
      preferredPort: 37123,
      gatewayListenMode: "custom",
      gatewayCustomListenAddress: "0.0.0.0:37123",
      autoStart: false,
      trayEnabled: true,
      logRetentionDays: 30,
      providerCooldownSeconds: 30,
      providerBaseUrlPingCacheTtlSeconds: 60,
      upstreamFirstByteTimeoutSeconds: 0,
      upstreamStreamIdleTimeoutSeconds: 0,
      upstreamRequestTimeoutNonStreamingSeconds: 0,
      interceptAnthropicWarmupRequests: true,
      enableThinkingSignatureRectifier: false,
      enableResponseFixer: true,
      responseFixerFixEncoding: true,
      responseFixerFixSseFormat: false,
      responseFixerFixTruncatedJson: true,
      cliPriorityOrder: ["gemini", "claude", "codex"],
      enableCacheAnomalyMonitor: true,
      updateReleasesUrl: "https://example.invalid/releases.json",
      failoverMaxAttemptsPerProvider: 5,
      failoverMaxProvidersToTry: 5,
      circuitBreakerFailureThreshold: 5,
      circuitBreakerOpenDurationMinutes: 30,
      wslAutoConfig: true,
      wslTargetCli: { claude: true, codex: false, gemini: true },
      codexHomeOverride: "D:\\CodexHome",
    } as any);

    expect(tauriInvoke).toHaveBeenCalledWith(
      "settings_set",
      expect.objectContaining({
        update: expect.objectContaining({
          gatewayListenMode: "custom",
          gatewayCustomListenAddress: "0.0.0.0:37123",
          interceptAnthropicWarmupRequests: true,
          enableThinkingSignatureRectifier: false,
          enableCacheAnomalyMonitor: true,
          enableResponseFixer: true,
          responseFixerFixEncoding: true,
          responseFixerFixSseFormat: false,
          responseFixerFixTruncatedJson: true,
          cliPriorityOrder: ["gemini", "claude", "codex"],
          updateReleasesUrl: "https://example.invalid/releases.json",
          wslAutoConfig: true,
          wslTargetCli: { claude: true, codex: false, gemini: true },
          codexHomeOverride: "D:\\CodexHome",
        }),
      })
    );
  });
});
