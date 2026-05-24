import { describe, expect, it, vi } from "vitest";
import {
  baseUrlPingMs,
  MAX_PROVIDER_ORDER_IDS,
  type ProviderSummary,
  providerClaudeTerminalLaunchCommand,
  providerCopyApiKeyToClipboard,
  providerDelete,
  providerDuplicate,
  providerOAuthDisconnect,
  providerOAuthFetchLimits,
  providerOAuthRefresh,
  providerOAuthStartFlow,
  providerOAuthStatus,
  providerSetEnabled,
  providerTestAvailability,
  providersList,
  providersReorder,
  providerUpsert,
  validateProviderCliKey,
  validateProviderId,
} from "../providers";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      providersList: vi.fn(),
      providerUpsert: vi.fn(),
      providerDuplicate: vi.fn(),
      providerSetEnabled: vi.fn(),
      providerDelete: vi.fn(),
      providersReorder: vi.fn(),
      providerClaudeTerminalLaunchCommand: vi.fn(),
      providerCopyApiKeyToClipboard: vi.fn(),
      baseUrlPingMs: vi.fn(),
      providerOauthStartFlow: vi.fn(),
      providerOauthRefresh: vi.fn(),
      providerOauthDisconnect: vi.fn(),
      providerOauthStatus: vi.fn(),
      providerOauthFetchLimits: vi.fn(),
      providerTestAvailability: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

function createProviderSummary(overrides: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: 1,
    cli_key: "claude",
    name: "P1",
    base_urls: ["https://example.com"],
    base_url_mode: "order",
    claude_models: {},
    enabled: true,
    priority: 0,
    cost_multiplier: 1,
    limit_5h_usd: null,
    limit_daily_usd: null,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00:00",
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    tags: [],
    note: "",
    created_at: 0,
    updated_at: 0,
    auth_mode: "api_key",
    oauth_provider_type: null,
    oauth_email: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    source_provider_id: null,
    bridge_type: null,
    stream_idle_timeout_seconds: null,
    api_key_configured: false,
    ...overrides,
  };
}

describe("services/providers/providers", () => {
  it("rethrows and logs when invoke fails", async () => {
    vi.mocked(commands.providersList).mockRejectedValueOnce(new Error("providers boom"));

    await expect(providersList("claude")).rejects.toThrow("providers boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取供应商列表失败",
      expect.objectContaining({
        cmd: "providers_list",
        error: expect.stringContaining("providers boom"),
      })
    );
  });

  it("treats null invoke result as error when runtime exists", async () => {
    vi.mocked(commands.providersList).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(providersList("claude")).rejects.toThrow("IPC_NULL_RESULT: providers_list");
  });

  it("builds provider_upsert args as before", async () => {
    vi.mocked(commands.providerUpsert).mockResolvedValueOnce({
      status: "ok",
      data: createProviderSummary(),
    });

    await providerUpsert({
      providerId: null,
      cliKey: "claude",
      name: "P1",
      baseUrls: ["https://example.com"],
      baseUrlMode: "order",
      apiKey: null,
      enabled: true,
      costMultiplier: 1,
      priority: null,
      claudeModels: null,
      limit5hUsd: null,
      limitDailyUsd: null,
      dailyResetMode: "fixed",
      dailyResetTime: "00:00:00",
      limitWeeklyUsd: null,
      limitMonthlyUsd: null,
      limitTotalUsd: null,
    });

    expect(commands.providerUpsert).toHaveBeenCalledWith(
      expect.objectContaining({
        providerId: null,
        cliKey: "claude",
        name: "P1",
        baseUrlMode: "order",
        limit5hUsd: null,
        dailyResetMode: "fixed",
      })
    );
  });

  it("redacts provider secrets before logging save failures", async () => {
    vi.mocked(commands.providerUpsert).mockRejectedValueOnce(new Error("save failed"));

    await expect(
      providerUpsert({
        providerId: null,
        cliKey: "claude",
        name: "P1",
        baseUrls: ["https://example.com"],
        baseUrlMode: "order",
        authMode: "api_key",
        apiKey: "sk-test-secret",
        enabled: true,
        costMultiplier: 1,
        priority: null,
        claudeModels: null,
        limit5hUsd: null,
        limitDailyUsd: null,
        dailyResetMode: "fixed",
        dailyResetTime: "00:00:00",
        limitWeeklyUsd: null,
        limitMonthlyUsd: null,
        limitTotalUsd: null,
      })
    ).rejects.toThrow("save failed");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存供应商失败",
      expect.objectContaining({
        cmd: "provider_upsert",
        args: {
          input: expect.objectContaining({
            apiKey: "[REDACTED]",
            name: "P1",
          }),
        },
      })
    );
  });

  it("passes providers command args with stable contract fields", async () => {
    vi.mocked(commands.providersList).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.baseUrlPingMs).mockResolvedValueOnce({ status: "ok", data: 120 as any });
    vi.mocked(commands.providerSetEnabled).mockResolvedValueOnce({
      status: "ok",
      data: createProviderSummary(),
    });
    vi.mocked(commands.providerDelete).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });
    vi.mocked(commands.providersReorder).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.providerClaudeTerminalLaunchCommand).mockResolvedValueOnce({
      status: "ok",
      data: "bash '/tmp/aio.sh'" as any,
    });
    vi.mocked(commands.providerTestAvailability).mockResolvedValueOnce({
      status: "ok",
      data: {
        ok: true,
        provider_id: 5,
        provider_name: "P1",
        base_url: "https://api.example.com",
        status: 200,
        latency_ms: 42,
        error: null,
        response_preview: null,
      } as any,
    });

    await providersList("claude");
    await baseUrlPingMs("https://api.example.com");
    await providerSetEnabled(1, true);
    await providerDelete(1);
    await providersReorder("claude", [2, 1]);
    await providerClaudeTerminalLaunchCommand(5);
    await providerTestAvailability(5);

    expect(commands.providersList).toHaveBeenCalledWith("claude");
    expect(commands.baseUrlPingMs).toHaveBeenCalledWith("https://api.example.com");
    expect(commands.providerSetEnabled).toHaveBeenCalledWith(1, true);
    expect(commands.providerDelete).toHaveBeenCalledWith(1);
    expect(commands.providersReorder).toHaveBeenCalledWith("claude", [2, 1]);
    expect(commands.providerClaudeTerminalLaunchCommand).toHaveBeenCalledWith(5);
    expect(commands.providerTestAvailability).toHaveBeenCalledWith(5);
  });

  it("normalizes provider cli keys before IPC", async () => {
    vi.mocked(commands.providersList).mockClear();
    vi.mocked(commands.providerUpsert).mockClear();
    vi.mocked(commands.providersReorder).mockClear();
    vi.mocked(commands.providerOauthStartFlow).mockClear();

    vi.mocked(commands.providersList).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.providerUpsert).mockResolvedValue({
      status: "ok",
      data: createProviderSummary({ cli_key: "codex" }),
    });
    vi.mocked(commands.providersReorder).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.providerOauthStartFlow).mockResolvedValue({
      status: "ok",
      data: {
        success: true,
        provider_type: "google",
        expires_at: 1700000000,
        provider_id: 10,
      } as any,
    });

    expect(validateProviderCliKey(" claude ")).toBe("claude");

    await providersList(" claude " as never);
    await providerUpsert({
      providerId: null,
      cliKey: " codex " as never,
      name: "P1",
      baseUrls: ["https://example.com"],
      baseUrlMode: "order",
      apiKey: null,
      enabled: true,
      costMultiplier: 1,
      priority: null,
      claudeModels: null,
      limit5hUsd: null,
      limitDailyUsd: null,
      dailyResetMode: "fixed",
      dailyResetTime: "00:00:00",
      limitWeeklyUsd: null,
      limitMonthlyUsd: null,
      limitTotalUsd: null,
    });
    await providersReorder(" gemini " as never, [2, 1]);
    await providerOAuthStartFlow(" claude ", 10);

    expect(commands.providersList).toHaveBeenCalledWith("claude");
    expect(commands.providerUpsert).toHaveBeenCalledWith(
      expect.objectContaining({ cliKey: "codex" })
    );
    expect(commands.providersReorder).toHaveBeenCalledWith("gemini", [2, 1]);
    expect(commands.providerOauthStartFlow).toHaveBeenCalledWith("claude", 10);
  });

  it("rejects invalid provider reorder ids before IPC", async () => {
    vi.mocked(commands.providersReorder).mockClear();

    await expect(providersReorder("claude", [2, 0])).rejects.toThrow(
      "SEC_INVALID_INPUT: invalid providerId=0"
    );
    await expect(providersReorder("claude", [2, 2])).rejects.toThrow(
      "SEC_INVALID_INPUT: duplicate providerId=2"
    );
    await expect(
      providersReorder(
        "claude",
        Array.from({ length: MAX_PROVIDER_ORDER_IDS + 1 }, (_, index) => index + 1)
      )
    ).rejects.toThrow("orderedProviderIds must contain at most");

    expect(commands.providersReorder).not.toHaveBeenCalled();
  });

  it("rejects invalid provider ids before IPC", async () => {
    vi.mocked(commands.providerUpsert).mockClear();
    vi.mocked(commands.providerSetEnabled).mockClear();
    vi.mocked(commands.providerDelete).mockClear();
    vi.mocked(commands.providerDuplicate).mockClear();
    vi.mocked(commands.providerOauthStartFlow).mockClear();
    vi.mocked(commands.providerOauthRefresh).mockClear();
    vi.mocked(commands.providerOauthDisconnect).mockClear();
    vi.mocked(commands.providerOauthStatus).mockClear();
    vi.mocked(commands.providerOauthFetchLimits).mockClear();
    vi.mocked(commands.providerTestAvailability).mockClear();

    expect(validateProviderId(42)).toBe(42);
    expect(() => validateProviderId(0)).toThrow("SEC_INVALID_INPUT");
    expect(() => validateProviderId(Number.NaN)).toThrow("SEC_INVALID_INPUT");

    await expect(
      providerUpsert({
        providerId: 0,
        cliKey: "claude",
        name: "P1",
        baseUrls: ["https://example.com"],
        baseUrlMode: "order",
        apiKey: null,
        enabled: true,
        costMultiplier: 1,
        priority: null,
        claudeModels: null,
        limit5hUsd: null,
        limitDailyUsd: null,
        dailyResetMode: "fixed",
        dailyResetTime: "00:00:00",
        limitWeeklyUsd: null,
        limitMonthlyUsd: null,
        limitTotalUsd: null,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      providerUpsert({
        providerId: null,
        cliKey: "claude",
        name: "P1",
        baseUrls: ["https://example.com"],
        baseUrlMode: "order",
        apiKey: null,
        enabled: true,
        costMultiplier: 1,
        priority: null,
        claudeModels: null,
        limit5hUsd: null,
        limitDailyUsd: null,
        dailyResetMode: "fixed",
        dailyResetTime: "00:00:00",
        limitWeeklyUsd: null,
        limitMonthlyUsd: null,
        limitTotalUsd: null,
        sourceProviderId: 0,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerSetEnabled(0, true)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerDelete(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerDuplicate(1.5)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerOAuthStartFlow("not-a-cli", 1)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerOAuthRefresh(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerOAuthDisconnect(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerOAuthStatus(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerOAuthFetchLimits(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(providerTestAvailability(0)).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.providerUpsert).not.toHaveBeenCalled();
    expect(commands.providerSetEnabled).not.toHaveBeenCalled();
    expect(commands.providerDelete).not.toHaveBeenCalled();
    expect(commands.providerDuplicate).not.toHaveBeenCalled();
    expect(commands.providerOauthStartFlow).not.toHaveBeenCalled();
    expect(commands.providerOauthRefresh).not.toHaveBeenCalled();
    expect(commands.providerOauthDisconnect).not.toHaveBeenCalled();
    expect(commands.providerOauthStatus).not.toHaveBeenCalled();
    expect(commands.providerOauthFetchLimits).not.toHaveBeenCalled();
    expect(commands.providerTestAvailability).not.toHaveBeenCalled();
  });

  it("provider duplicate and clipboard copy both use generated ipc", async () => {
    vi.mocked(commands.providerDuplicate).mockResolvedValueOnce({
      status: "ok",
      data: createProviderSummary({ id: 42 }),
    });
    vi.mocked(commands.providerCopyApiKeyToClipboard).mockResolvedValueOnce({
      status: "ok",
      data: true as any,
    });

    const duplicated = await providerDuplicate(42);
    const copied = await providerCopyApiKeyToClipboard(42);

    expect(duplicated).toEqual(createProviderSummary({ id: 42 }));
    expect(copied).toBe(true);
    expect(commands.providerDuplicate).toHaveBeenCalledWith(42);
    expect(commands.providerCopyApiKeyToClipboard).toHaveBeenCalledWith(
      42,
      expect.objectContaining({
        confirm: expect.objectContaining({
          action: "provider_copy_api_key_to_clipboard",
          resource: "provider:42:api_key",
          nonce: expect.any(String),
        }),
      })
    );
  });

  it("providerOAuthStartFlow uses generated ipc", async () => {
    vi.mocked(commands.providerOauthStartFlow).mockResolvedValueOnce({
      status: "ok",
      data: {
        success: true,
        provider_type: "google",
        expires_at: 1700000000,
        provider_id: 10,
      } as any,
    });

    const result = await providerOAuthStartFlow("claude", 10);
    expect(result).toEqual({
      success: true,
      provider_type: "google",
      expires_at: 1700000000,
      provider_id: 10,
    });
    expect(commands.providerOauthStartFlow).toHaveBeenCalledWith("claude", 10);
  });

  it("providerOAuthRefresh uses generated ipc", async () => {
    vi.mocked(commands.providerOauthRefresh).mockResolvedValueOnce({
      status: "ok",
      data: { success: true, expires_at: 1700001000 } as any,
    });

    const result = await providerOAuthRefresh(20);
    expect(result).toEqual({ success: true, expires_at: 1700001000 });
    expect(commands.providerOauthRefresh).toHaveBeenCalledWith(20);
  });

  it("providerOAuthDisconnect uses generated ipc", async () => {
    vi.mocked(commands.providerOauthDisconnect).mockResolvedValueOnce({
      status: "ok",
      data: { success: true } as any,
    });

    const result = await providerOAuthDisconnect(30);
    expect(result).toEqual({ success: true });
    expect(commands.providerOauthDisconnect).toHaveBeenCalledWith(30);
  });

  it("providerOAuthStatus uses generated ipc", async () => {
    vi.mocked(commands.providerOauthStatus).mockResolvedValueOnce({
      status: "ok",
      data: {
        connected: true,
        provider_type: "google",
        email: "test@example.com",
        expires_at: 1700002000,
        has_refresh_token: true,
      } as any,
    });

    const result = await providerOAuthStatus(40);
    expect(result).toEqual({
      connected: true,
      provider_type: "google",
      email: "test@example.com",
      expires_at: 1700002000,
      has_refresh_token: true,
    });
    expect(commands.providerOauthStatus).toHaveBeenCalledWith(40);
  });

  it("providerOAuthFetchLimits uses generated ipc", async () => {
    vi.mocked(commands.providerOauthFetchLimits).mockResolvedValueOnce({
      status: "ok",
      data: {
        limit_short_label: "1h",
        limit_5h_text: "100 requests",
        limit_weekly_text: "1000 requests",
      } as any,
    });

    const result = await providerOAuthFetchLimits(50);
    expect(result).toEqual({
      limit_short_label: "1h",
      limit_5h_text: "100 requests",
      limit_weekly_text: "1000 requests",
    });
    expect(commands.providerOauthFetchLimits).toHaveBeenCalledWith(50);
  });
});
