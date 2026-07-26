import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import { openDesktopUrl } from "../../../services/desktop/opener";
import {
  providerOAuthDisconnect,
  providerOAuthFetchLimits,
  providerOAuthPollDeviceFlow,
  providerOAuthRefresh,
  providerOAuthStartDeviceFlow,
  providerOAuthStartFlow,
  type ProviderOAuthDeviceCodePollResult,
  type ProviderOAuthDeviceCodeStartResult,
  type ProviderOAuthStatusResult,
  type ProviderSummary,
} from "../../../services/providers/providers";
import type { OAuthActionContext } from "../providerEditorActionContext";
import {
  handleOAuthDeviceLogin,
  handleOAuthDisconnect,
  handleOAuthLogin,
  handleOAuthRefresh,
} from "../providerEditorOAuthActions";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../services/desktop/opener", () => ({ openDesktopUrl: vi.fn() }));
vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return {
    ...actual,
    providerOAuthStartFlow: vi.fn(),
    providerOAuthStartDeviceFlow: vi.fn(),
    providerOAuthPollDeviceFlow: vi.fn(),
    providerOAuthRefresh: vi.fn(),
    providerOAuthDisconnect: vi.fn(),
    providerOAuthFetchLimits: vi.fn(),
  };
});

function makeStatus(partial: Partial<ProviderOAuthStatusResult> = {}): ProviderOAuthStatusResult {
  return {
    connected: partial.connected ?? true,
    provider_type: partial.provider_type ?? "codex_oauth",
    email: partial.email ?? "user@example.com",
    expires_at: partial.expires_at ?? null,
    has_refresh_token: partial.has_refresh_token ?? true,
  };
}

function makeDeviceStart(
  partial: Partial<ProviderOAuthDeviceCodeStartResult> = {}
): ProviderOAuthDeviceCodeStartResult {
  return {
    provider_id: partial.provider_id ?? 9,
    provider_type: partial.provider_type ?? "codex_oauth",
    flow_id: partial.flow_id ?? "flow-1",
    device_code: partial.device_code ?? "device-1",
    user_code: partial.user_code ?? "USER-1",
    verification_uri: partial.verification_uri ?? "https://auth.example.com/device",
    expires_in: partial.expires_in ?? 60,
    interval: partial.interval ?? 0,
  };
}

function makeDevicePoll(
  partial: Partial<ProviderOAuthDeviceCodePollResult> = {}
): ProviderOAuthDeviceCodePollResult {
  return {
    completed: partial.completed ?? true,
    provider_id: partial.provider_id ?? 9,
    provider_type: partial.provider_type ?? "codex_oauth",
    expires_at: partial.expires_at ?? null,
  };
}

function makeProvider(partial: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: partial.id ?? 9,
    cli_key: partial.cli_key ?? "claude",
    name: partial.name ?? "OAuth Provider",
    base_urls: partial.base_urls ?? [],
    base_url_mode: partial.base_url_mode ?? "order",
    claude_models: partial.claude_models ?? {},
    enabled: partial.enabled ?? true,
    priority: partial.priority ?? 0,
    cost_multiplier: partial.cost_multiplier ?? 1,
    limit_5h_usd: partial.limit_5h_usd ?? null,
    limit_daily_usd: partial.limit_daily_usd ?? null,
    daily_reset_mode: partial.daily_reset_mode ?? "fixed",
    daily_reset_time: partial.daily_reset_time ?? "00:00:00",
    limit_weekly_usd: partial.limit_weekly_usd ?? null,
    limit_monthly_usd: partial.limit_monthly_usd ?? null,
    limit_total_usd: partial.limit_total_usd ?? null,
    tags: partial.tags ?? [],
    note: partial.note ?? "",
    created_at: partial.created_at ?? 1,
    updated_at: partial.updated_at ?? 1,
    auth_mode: partial.auth_mode ?? "oauth",
    oauth_provider_type: partial.oauth_provider_type ?? null,
    oauth_email: partial.oauth_email ?? null,
    oauth_expires_at: partial.oauth_expires_at ?? null,
    oauth_last_error: partial.oauth_last_error ?? null,
    source_provider_id: partial.source_provider_id ?? null,
    bridge_type: partial.bridge_type ?? null,
    api_key_configured: partial.api_key_configured ?? false,
    stream_idle_timeout_seconds: partial.stream_idle_timeout_seconds ?? null,
    extension_values: partial.extension_values ?? [],
  };
}

function makeCtx(overrides: Partial<OAuthActionContext> = {}) {
  let attempt = 0;
  let currentAttempt = 0;
  const values = {
    name: "OAuth Provider",
    api_key: "",
    auth_mode: "oauth" as const,
    cost_multiplier: "1",
    limit_5h_usd: "",
    limit_daily_usd: "",
    limit_weekly_usd: "",
    limit_monthly_usd: "",
    limit_total_usd: "",
    daily_reset_mode: "fixed" as const,
    daily_reset_time: "00:00:00",
    enabled: true,
    note: "",
  };
  const ctx: OAuthActionContext = {
    mode: "create",
    cliKey: "claude",
    editingProviderId: null,
    editProvider: null,
    open: true,
    onOpenChange: vi.fn(),
    onSaved: vi.fn(),
    authMode: "oauth",
    formValues: values,
    baseUrlMode: "order",
    baseUrlRows: [],
    tags: [],
    claudeModels: {},
    streamIdleTimeoutSeconds: "",
    apiKeyConfigured: false,
    isCodexGatewaySource: false,
    sourceProviderId: null,
    selectedCx2ccSourceProvider: null,
    form: {
      getValues: vi.fn(() => values),
      setValue: vi.fn(),
    },
    oauthStatus: null,
    setOauthStatus: vi.fn(),
    refreshOauthStatus: vi.fn().mockResolvedValue(makeStatus()),
    writeOauthStatusCache: vi.fn(),
    setOauthLoading: vi.fn(),
    oauthDeviceFlow: null,
    setOauthDeviceFlow: vi.fn(),
    oauthDevicePolling: false,
    setOauthDevicePolling: vi.fn(),
    oauthDeviceError: null,
    setOauthDeviceError: vi.fn(),
    persistProvider: vi.fn().mockResolvedValue(makeProvider({ id: 9 })),
    removeProvider: vi.fn().mockResolvedValue(true),
    beginOAuthLoginAttempt: vi.fn(() => {
      attempt += 1;
      currentAttempt = attempt;
      return attempt;
    }),
    isOAuthLoginAttemptCurrent: vi.fn((id: number) => id === currentAttempt),
    cancelOAuthDeviceFlow: vi.fn(),
    setActiveOAuthDeviceFlow: vi.fn(),
    clearActiveOAuthDeviceFlow: vi.fn(),
    ...overrides,
  };
  return { ctx, values, setCurrentAttempt: (next: number) => (currentAttempt = next) };
}

beforeEach(() => {
  vi.useRealTimers();
  vi.mocked(toast).mockReset();
  vi.mocked(logToConsole).mockReset();
  vi.mocked(openDesktopUrl).mockReset();
  vi.mocked(providerOAuthStartFlow).mockReset();
  vi.mocked(providerOAuthStartDeviceFlow).mockReset();
  vi.mocked(providerOAuthPollDeviceFlow).mockReset();
  vi.mocked(providerOAuthRefresh).mockReset();
  vi.mocked(providerOAuthDisconnect).mockReset();
  vi.mocked(providerOAuthFetchLimits).mockReset();
});

describe("providerEditorOAuthActions", () => {
  it("auto-saves a new provider and completes browser OAuth login", async () => {
    vi.mocked(providerOAuthStartFlow).mockResolvedValue({
      success: true,
      provider_id: 9,
      provider_type: "codex_oauth",
      expires_at: 123,
    });
    vi.mocked(providerOAuthFetchLimits).mockResolvedValue({
      limit_short_label: null,
      limit_5h_text: "5h $1",
      limit_weekly_text: "weekly $7",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
      reset_credit_available_count: null,
    });

    const { ctx } = makeCtx();
    await handleOAuthLogin(ctx);

    expect(ctx.persistProvider).toHaveBeenCalledWith(
      expect.objectContaining({ cliKey: "claude", name: "OAuth Provider", authMode: "oauth" })
    );
    expect(providerOAuthStartFlow).toHaveBeenCalledWith("claude", 9);
    expect(ctx.setOauthStatus).toHaveBeenCalledWith(makeStatus());
    expect(providerOAuthFetchLimits).toHaveBeenCalledWith(9);
    expect(toast).toHaveBeenCalledWith("OAuth 登录成功");
    expect(ctx.onSaved).toHaveBeenCalledWith("claude");
    expect(ctx.onOpenChange).toHaveBeenCalledWith(false);
    expect(ctx.removeProvider).not.toHaveBeenCalled();
    expect(ctx.setOauthLoading).toHaveBeenLastCalledWith(false);
  });

  it("falls back to login result and syncs cache when status fetch fails after login", async () => {
    vi.mocked(providerOAuthStartFlow).mockResolvedValue({
      success: true,
      provider_id: 9,
      provider_type: "codex_oauth",
      expires_at: 1234,
    });
    vi.mocked(providerOAuthFetchLimits).mockResolvedValue({
      limit_short_label: null,
      limit_5h_text: "5h $1",
      limit_weekly_text: "weekly $7",
      limit_5h_reset_at: null,
      limit_weekly_reset_at: null,
      reset_credit_available_count: null,
    });

    const { ctx } = makeCtx({
      refreshOauthStatus: vi.fn().mockRejectedValue(new Error("ipc down")),
    });
    await handleOAuthLogin(ctx);

    const fallback = {
      connected: true,
      provider_type: "codex_oauth",
      email: null,
      expires_at: 1234,
      has_refresh_token: null,
    };
    expect(ctx.setOauthStatus).toHaveBeenCalledWith(fallback);
    expect(ctx.writeOauthStatusCache).toHaveBeenCalledWith(fallback, 9);
    expect(toast).toHaveBeenCalledWith("OAuth 登录成功，但读取连接状态失败，可稍后重试");
    expect(toast).toHaveBeenCalledWith("OAuth 登录成功");
    expect(ctx.removeProvider).not.toHaveBeenCalled();
  });

  it("rolls back auto-saved provider when browser OAuth fails or becomes stale", async () => {
    vi.mocked(providerOAuthStartFlow).mockResolvedValueOnce({
      success: false,
      provider_id: 9,
      provider_type: "codex_oauth",
      expires_at: null,
    });

    const { ctx } = makeCtx();
    await handleOAuthLogin(ctx);

    expect(ctx.removeProvider).toHaveBeenCalledWith(9);
    expect(toast).toHaveBeenCalledWith("OAuth 登录失败");

    const stale = makeCtx({
      persistProvider: vi.fn().mockImplementation(async () => {
        stale.setCurrentAttempt(999);
        return makeProvider({ id: 11 });
      }),
    });
    await handleOAuthLogin(stale.ctx);

    expect(stale.ctx.removeProvider).toHaveBeenCalledWith(11);
    expect(providerOAuthStartFlow).toHaveBeenCalledTimes(1);
  });

  it("reports rollback cleanup failures during browser OAuth login", async () => {
    vi.mocked(providerOAuthStartFlow).mockResolvedValue({
      success: false,
      provider_id: 9,
      provider_type: "codex_oauth",
      expires_at: null,
    });

    const failedCleanup = makeCtx({ removeProvider: vi.fn().mockResolvedValue(false) });
    await handleOAuthLogin(failedCleanup.ctx);
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      expect.stringContaining("OAuth 登录失败后清理临时 Provider 失败"),
      expect.objectContaining({ provider_id: 9 })
    );

    vi.mocked(providerOAuthStartFlow).mockResolvedValueOnce({
      success: false,
      provider_id: 10,
      provider_type: "codex_oauth",
      expires_at: null,
    });
    const thrownCleanup = makeCtx({ removeProvider: vi.fn().mockRejectedValue(new Error("gone")) });
    await handleOAuthLogin(thrownCleanup.ctx);
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      expect.stringContaining("OAuth 登录失败后清理临时 Provider 异常"),
      expect.objectContaining({ provider_id: 9, error: "Error: gone" })
    );
  });

  it("handles browser OAuth validation errors and stale completion points", async () => {
    const missingName = makeCtx();
    missingName.values.name = "   ";
    await handleOAuthLogin(missingName.ctx);
    expect(toast).toHaveBeenCalledWith("请先填写 Provider 名称");
    expect(missingName.ctx.persistProvider).not.toHaveBeenCalled();

    vi.mocked(providerOAuthStartFlow).mockResolvedValueOnce({
      success: true,
      provider_id: 5,
      provider_type: "google",
      expires_at: null,
    });
    const staleStatus = makeCtx({
      mode: "edit",
      editingProviderId: 5,
      refreshOauthStatus: vi.fn().mockImplementation(async () => {
        staleStatus.setCurrentAttempt(99);
        return makeStatus();
      }),
    });
    await handleOAuthLogin(staleStatus.ctx);
    expect(staleStatus.ctx.setOauthStatus).not.toHaveBeenCalled();
    expect(toast).not.toHaveBeenCalledWith("OAuth 登录成功");

    vi.mocked(providerOAuthStartFlow).mockResolvedValueOnce({
      success: true,
      provider_id: 6,
      provider_type: "google",
      expires_at: null,
    });
    vi.mocked(providerOAuthFetchLimits).mockImplementationOnce(async () => {
      staleLimits.setCurrentAttempt(99);
      return null;
    });
    const staleLimits = makeCtx({ mode: "edit", editingProviderId: 6 });
    await handleOAuthLogin(staleLimits.ctx);
    expect(staleLimits.ctx.setOauthLoading).not.toHaveBeenLastCalledWith(false);
  });

  it("keeps login success but reports status and limit refresh failures", async () => {
    vi.mocked(providerOAuthStartFlow).mockResolvedValue({
      success: true,
      provider_id: 5,
      provider_type: "google",
      expires_at: null,
    });
    vi.mocked(providerOAuthFetchLimits).mockRejectedValue(new Error("limits down"));

    const { ctx } = makeCtx({
      mode: "edit",
      editingProviderId: 5,
      refreshOauthStatus: vi.fn().mockRejectedValue(new Error("status down")),
    });

    await handleOAuthLogin(ctx);

    expect(toast).toHaveBeenCalledWith("OAuth 登录成功，但读取连接状态失败，可稍后重试");
    expect(toast).toHaveBeenCalledWith("OAuth 登录成功，但获取用量失败，可稍后重试");
    expect(toast).toHaveBeenCalledWith("OAuth 登录成功");
    expect(ctx.persistProvider).not.toHaveBeenCalled();
    expect(ctx.onSaved).not.toHaveBeenCalled();
  });

  it("handles device OAuth completion, limit refresh warning, and expiration rollback", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);
    vi.mocked(openDesktopUrl).mockResolvedValue(true);
    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(makeDeviceStart());
    vi.mocked(providerOAuthPollDeviceFlow).mockResolvedValueOnce(
      makeDevicePoll({ completed: true })
    );
    vi.mocked(providerOAuthFetchLimits).mockRejectedValueOnce(new Error("limits down"));

    const { ctx } = makeCtx();
    const completePromise = handleOAuthDeviceLogin(ctx);
    await vi.runAllTimersAsync();
    await completePromise;

    expect(ctx.setActiveOAuthDeviceFlow).toHaveBeenCalledWith(1, "flow-1");
    expect(openDesktopUrl).toHaveBeenCalledWith("https://auth.example.com/device");
    expect(providerOAuthPollDeviceFlow).toHaveBeenCalledWith(9, "flow-1", "device-1", "USER-1");
    expect(ctx.clearActiveOAuthDeviceFlow).toHaveBeenCalledWith("flow-1");
    expect(ctx.setOauthStatus).toHaveBeenCalledWith(makeStatus());
    expect(toast).toHaveBeenCalledWith("设备码登录成功");
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      expect.stringContaining("设备码登录后获取用量异常"),
      expect.objectContaining({ provider_id: 9 })
    );

    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(
      makeDeviceStart({ flow_id: "flow-expired", expires_in: 0 })
    );
    vi.mocked(providerOAuthPollDeviceFlow).mockReset();
    const expired = makeCtx();
    const expiredPromise = handleOAuthDeviceLogin(expired.ctx);
    await vi.runAllTimersAsync();
    await expiredPromise;

    expect(expired.ctx.cancelOAuthDeviceFlow).toHaveBeenCalledWith("flow-expired");
    expect(expired.ctx.setOauthDeviceError).toHaveBeenCalledWith("设备码已过期，请重新开始登录。");
    expect(expired.ctx.removeProvider).toHaveBeenCalledWith(9);
    expect(toast).toHaveBeenCalledWith("设备码登录失败：设备码已过期");
  });

  it("cancels device flow and rolls back when the attempt becomes stale", async () => {
    vi.mocked(providerOAuthStartDeviceFlow).mockImplementation(async () => {
      stale.setCurrentAttempt(42);
      return makeDeviceStart({ flow_id: "flow-stale" });
    });

    const stale = makeCtx();
    await handleOAuthDeviceLogin(stale.ctx);

    expect(stale.ctx.cancelOAuthDeviceFlow).toHaveBeenCalledWith("flow-stale");
    expect(stale.ctx.removeProvider).toHaveBeenCalledWith(9);
    expect(openDesktopUrl).not.toHaveBeenCalled();
    expect(stale.ctx.setOauthLoading).not.toHaveBeenLastCalledWith(false);
  });

  it("handles device OAuth cleanup failures, polling retries, and runtime errors", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);
    vi.mocked(openDesktopUrl).mockResolvedValue(true);

    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(
      makeDeviceStart({ flow_id: "flow-cleanup", expires_in: 0 })
    );
    const cleanupFailed = makeCtx({ removeProvider: vi.fn().mockResolvedValue(false) });
    const cleanupFailedPromise = handleOAuthDeviceLogin(cleanupFailed.ctx);
    await vi.runAllTimersAsync();
    await cleanupFailedPromise;
    expect(logToConsole).toHaveBeenCalledWith(
      "warn",
      expect.stringContaining("设备码登录失败后清理临时 Provider 失败"),
      expect.objectContaining({ provider_id: 9 })
    );

    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(
      makeDeviceStart({ flow_id: "flow-cleanup-throw", expires_in: 0 })
    );
    const cleanupThrown = makeCtx({ removeProvider: vi.fn().mockRejectedValue(new Error("gone")) });
    const cleanupThrownPromise = handleOAuthDeviceLogin(cleanupThrown.ctx);
    await vi.runAllTimersAsync();
    await cleanupThrownPromise;
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      expect.stringContaining("设备码登录失败后清理临时 Provider 异常"),
      expect.objectContaining({ provider_id: 9, error: "Error: gone" })
    );

    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(
      makeDeviceStart({ flow_id: "flow-retry", interval: 1, expires_in: 3 })
    );
    vi.mocked(providerOAuthPollDeviceFlow)
      .mockResolvedValueOnce(makeDevicePoll({ completed: false }))
      .mockResolvedValueOnce(makeDevicePoll({ completed: true }));
    vi.mocked(providerOAuthFetchLimits).mockResolvedValueOnce(null);
    const retry = makeCtx({ mode: "edit", editingProviderId: 12 });
    const retryPromise = handleOAuthDeviceLogin(retry.ctx);
    await vi.advanceTimersByTimeAsync(1000);
    await vi.runAllTimersAsync();
    await retryPromise;
    expect(providerOAuthPollDeviceFlow).toHaveBeenCalledTimes(2);
    expect(retry.ctx.onSaved).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith("设备码登录成功");

    vi.mocked(providerOAuthStartDeviceFlow).mockResolvedValueOnce(
      makeDeviceStart({ flow_id: "flow-error" })
    );
    vi.mocked(providerOAuthPollDeviceFlow).mockRejectedValueOnce(new Error("poll down"));
    const runtimeError = makeCtx();
    const runtimeErrorPromise = handleOAuthDeviceLogin(runtimeError.ctx);
    await vi.runAllTimersAsync();
    await runtimeErrorPromise;
    expect(runtimeError.ctx.cancelOAuthDeviceFlow).toHaveBeenCalledWith("flow-error");
    expect(runtimeError.ctx.setOauthDeviceError).toHaveBeenCalledWith("Error: poll down");
    expect(toast).toHaveBeenCalledWith("设备码登录失败：Error: poll down");
  });

  it("handles OAuth refresh and disconnect success, failure, and missing provider guards", async () => {
    vi.mocked(providerOAuthRefresh)
      .mockResolvedValueOnce({ success: true, expires_at: 456 })
      .mockResolvedValueOnce({ success: false, expires_at: null })
      .mockRejectedValueOnce(new Error("refresh down"));
    vi.mocked(providerOAuthDisconnect)
      .mockResolvedValueOnce({ success: true })
      .mockResolvedValueOnce({ success: false })
      .mockRejectedValueOnce(new Error("disconnect down"));

    const { ctx } = makeCtx({ mode: "edit", editingProviderId: 7 });
    await handleOAuthRefresh(ctx);
    await handleOAuthRefresh(ctx);
    await handleOAuthRefresh(ctx);
    await handleOAuthDisconnect(ctx);
    await handleOAuthDisconnect(ctx);
    await handleOAuthDisconnect(ctx);

    expect(ctx.setOauthStatus).toHaveBeenCalledWith(makeStatus());
    expect(ctx.setOauthStatus).toHaveBeenCalledWith(null);
    expect(ctx.writeOauthStatusCache).toHaveBeenCalledWith(null, 7);
    expect(ctx.writeOauthStatusCache).toHaveBeenCalledTimes(1);
    expect(toast).toHaveBeenCalledWith("Token 刷新成功");
    expect(toast).toHaveBeenCalledWith("Token 刷新失败");
    expect(toast).toHaveBeenCalledWith("Token 刷新失败：Error: refresh down");
    expect(toast).toHaveBeenCalledWith("已断开 OAuth 连接");
    expect(toast).toHaveBeenCalledWith("断开 OAuth 连接失败");
    expect(toast).toHaveBeenCalledWith("断开 OAuth 连接失败：Error: disconnect down");

    const missing = makeCtx({ editingProviderId: null });
    await handleOAuthRefresh(missing.ctx);
    await handleOAuthDisconnect(missing.ctx);
    expect(providerOAuthRefresh).toHaveBeenCalledTimes(3);
    expect(providerOAuthDisconnect).toHaveBeenCalledTimes(3);
  });
});
