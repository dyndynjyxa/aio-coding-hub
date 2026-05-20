import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { createTestAppSettings } from "../../../test/fixtures/settings";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/settings/settings", () => {
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

  it("maps cached settings back into the generated update contract", async () => {
    const { createSettingsSetInput } = await import("../settings");

    const input = createSettingsSetInput(createTestAppSettings(), {
      upstream_proxy_password: { mode: "clear" },
    });

    expect(input).toMatchObject({
      preferredPort: 37123,
      gatewayListenMode: "localhost",
      wslTargetCli: { claude: true, codex: true, gemini: true },
      cx2CcFallbackModelMain: "gpt-5.4",
      upstreamProxyPassword: { mode: "clear" },
    });
    expect(input).not.toHaveProperty("cx2ccFallbackModelMain");
  });

  it("rejects invalid settings at the frontend boundary before IPC", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.mocked(tauriInvoke).mockResolvedValue({ schema_version: 1 } as any);

    const { settingsSet } = await import("../settings");
    const required = {
      preferredPort: 37123,
      autoStart: false,
      logRetentionDays: 30,
      failoverMaxAttemptsPerProvider: 5,
      failoverMaxProvidersToTry: 5,
    };

    await expect(
      settingsSet({
        ...required,
        gatewayListenMode: "custom",
        gatewayCustomListenAddress: "http://127.0.0.1:37123",
      } as any)
    ).rejects.toThrow("自定义地址仅支持 host 或 host:port");

    await expect(
      settingsSet({
        ...required,
        upstreamProxyUsername: "x".repeat(257),
      } as any)
    ).rejects.toThrow("代理用户名必须 <= 256 字符");

    await expect(
      settingsSet({
        ...required,
        cx2CcFallbackModelMain: "x".repeat(129),
      } as any)
    ).rejects.toThrow("CX2CC 主模型默认必须 <= 128 字符");

    await expect(
      settingsSet({
        ...required,
        preferredPort: 80,
      } as any)
    ).rejects.toThrow("首选端口必须 >= 1024");

    await expect(
      settingsSet({
        ...required,
        upstreamStreamIdleTimeoutSeconds: 30,
      } as any)
    ).rejects.toThrow("流式空闲超时必须为 0");

    await expect(
      settingsSet({
        ...required,
        failoverMaxAttemptsPerProvider: 20,
        failoverMaxProvidersToTry: 20,
      } as any)
    ).rejects.toThrow("Failover 总尝试次数必须 <= 100");

    await expect(
      settingsSet({
        ...required,
        circuitBreakerOpenDurationMinutes: 1441,
      } as any)
    ).rejects.toThrow("熔断打开时长必须 <= 1440");

    expect(tauriInvoke).not.toHaveBeenCalled();
  });

  it("rejects missing required settings before generated IPC", async () => {
    setTauriRuntime();
    vi.resetModules();
    vi.mocked(tauriInvoke).mockResolvedValue({ schema_version: 1 } as any);

    const { settingsSet } = await import("../settings");

    await expect(
      settingsSet({
        autoStart: false,
        logRetentionDays: 30,
        failoverMaxAttemptsPerProvider: 5,
        failoverMaxProvidersToTry: 5,
      } as any)
    ).rejects.toThrow("SEC_INVALID_INPUT: preferredPort is required");

    expect(tauriInvoke).not.toHaveBeenCalled();
  });
});
