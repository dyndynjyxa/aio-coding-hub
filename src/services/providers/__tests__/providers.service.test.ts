import { describe, expect, it, vi } from "vitest";
import {
  baseUrlPingMs,
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
  providersList,
  providersReorder,
  providerUpsert,
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
      data: { id: 1, cli_key: "claude" } as any,
    });

    await providerUpsert({
      provider_id: null,
      cli_key: "claude",
      name: "P1",
      base_urls: ["https://example.com"],
      base_url_mode: "order",
      api_key: null,
      enabled: true,
      cost_multiplier: 1,
      priority: null,
      claude_models: null,
      limit_5h_usd: null,
      limit_daily_usd: null,
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      limit_weekly_usd: null,
      limit_monthly_usd: null,
      limit_total_usd: null,
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
        provider_id: null,
        cli_key: "claude",
        name: "P1",
        base_urls: ["https://example.com"],
        base_url_mode: "order",
        auth_mode: "api_key",
        api_key: "sk-test-secret",
        enabled: true,
        cost_multiplier: 1,
        priority: null,
        claude_models: null,
        limit_5h_usd: null,
        limit_daily_usd: null,
        daily_reset_mode: "fixed",
        daily_reset_time: "00:00:00",
        limit_weekly_usd: null,
        limit_monthly_usd: null,
        limit_total_usd: null,
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
      data: { id: 1, cli_key: "claude" } as any,
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

    await providersList("claude");
    await baseUrlPingMs("https://api.example.com");
    await providerSetEnabled(1, true);
    await providerDelete(1);
    await providersReorder("claude", [2, 1]);
    await providerClaudeTerminalLaunchCommand(5);

    expect(commands.providersList).toHaveBeenCalledWith("claude");
    expect(commands.baseUrlPingMs).toHaveBeenCalledWith("https://api.example.com");
    expect(commands.providerSetEnabled).toHaveBeenCalledWith(1, true);
    expect(commands.providerDelete).toHaveBeenCalledWith(1);
    expect(commands.providersReorder).toHaveBeenCalledWith("claude", [2, 1]);
    expect(commands.providerClaudeTerminalLaunchCommand).toHaveBeenCalledWith(5);
  });

  it("provider duplicate and clipboard copy both use generated ipc", async () => {
    vi.mocked(commands.providerDuplicate).mockResolvedValueOnce({
      status: "ok",
      data: { id: 42 } as any,
    });
    vi.mocked(commands.providerCopyApiKeyToClipboard).mockResolvedValueOnce({
      status: "ok",
      data: true as any,
    });

    const duplicated = await providerDuplicate(42);
    const copied = await providerCopyApiKeyToClipboard(42);

    expect(duplicated).toEqual({ id: 42 });
    expect(copied).toBe(true);
    expect(commands.providerDuplicate).toHaveBeenCalledWith(42);
    expect(commands.providerCopyApiKeyToClipboard).toHaveBeenCalledWith(42);
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
