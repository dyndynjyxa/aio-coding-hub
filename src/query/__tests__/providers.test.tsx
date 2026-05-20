import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ProviderSummary } from "../../services/providers/providers";
import {
  providerOAuthFetchLimits,
  providerOAuthStatus,
  providerClaudeTerminalLaunchCommand,
  providerDelete,
  providerTestAvailability,
  providerUpsert,
  providerSetEnabled,
  providersList,
  providersReorder,
} from "../../services/providers/providers";
import {
  fetchProviderOAuthStatus,
  readProviderOAuthLimitsCache,
  refreshProviderOAuthLimits,
  useOAuthLimitsQuery,
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderOAuthStatusQuery,
  useProviderSetEnabledMutation,
  useProviderTestAvailabilityMutation,
  useProviderUpsertMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../providers";
import { gatewayKeys, oauthLimitsKeys, providersKeys } from "../keys";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";

vi.mock("../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../services/providers/providers")>(
    "../../services/providers/providers"
  );
  return {
    ...actual,
    providersList: vi.fn(),
    providerOAuthStatus: vi.fn(),
    providerOAuthFetchLimits: vi.fn(),
    providerUpsert: vi.fn(),
    providerSetEnabled: vi.fn(),
    providerDelete: vi.fn(),
    providerTestAvailability: vi.fn(),
    providersReorder: vi.fn(),
    providerClaudeTerminalLaunchCommand: vi.fn(),
  };
});

function makeProvider(
  partial: Partial<ProviderSummary> & Pick<ProviderSummary, "id" | "cli_key" | "name">
): ProviderSummary {
  return {
    id: partial.id,
    cli_key: partial.cli_key,
    name: partial.name,
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
    created_at: partial.created_at ?? 0,
    updated_at: partial.updated_at ?? 0,
    auth_mode: partial.auth_mode ?? "api_key",
    oauth_provider_type: partial.oauth_provider_type ?? null,
    oauth_email: partial.oauth_email ?? null,
    oauth_expires_at: partial.oauth_expires_at ?? null,
    oauth_last_error: partial.oauth_last_error ?? null,
    source_provider_id: partial.source_provider_id ?? null,
    bridge_type: partial.bridge_type ?? null,
    stream_idle_timeout_seconds: partial.stream_idle_timeout_seconds ?? null,
    api_key_configured: partial.api_key_configured ?? false,
  };
}

describe("query/providers", () => {
  it("calls providersList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(providersList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProvidersListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(providersList).toHaveBeenCalledWith("claude");
    });
  });

  it("useProvidersListQuery normalizes cliKey before cache key and service call", async () => {
    setTauriRuntime();

    vi.mocked(providersList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProvidersListQuery(" claude " as never), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(providersList).toHaveBeenCalledWith("claude");
    expect(client.getQueryState(providersKeys.list("claude"))).toBeTruthy();
    expect(client.getQueryState(providersKeys.list(" claude " as never))).toBeUndefined();
  });

  it("rejects invalid provider list cliKey before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() => renderHook(() => useProvidersListQuery("opencode" as never), { wrapper })).toThrow(
      "SEC_INVALID_INPUT"
    );
    expect(providersList).not.toHaveBeenCalled();
  });

  it("useProvidersListQuery enters error state when providersList rejects", async () => {
    setTauriRuntime();

    vi.mocked(providersList).mockRejectedValue(new Error("providers query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProvidersListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProvidersListQuery("claude", { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(providersList).not.toHaveBeenCalled();
  });

  it("normalizes OAuth status providerId before cache key and service call", async () => {
    setTauriRuntime();

    const status = {
      connected: true,
      provider_type: "google",
      email: "test@example.com",
      expires_at: 1700000000,
      has_refresh_token: true,
    };
    vi.mocked(providerOAuthStatus).mockResolvedValue(status);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderOAuthStatusQuery(7), { wrapper });

    await waitFor(() => {
      expect(result.current.data).toEqual(status);
    });

    expect(providerOAuthStatus).toHaveBeenCalledWith(7);
    expect(client.getQueryState(providersKeys.oauthStatus(7))).toBeTruthy();
  });

  it("rejects invalid OAuth status providerId before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() => renderHook(() => useProviderOAuthStatusQuery(0), { wrapper })).toThrow(
      "SEC_INVALID_INPUT"
    );
    expect(providerOAuthStatus).not.toHaveBeenCalled();
    expect(client.getQueryState(providersKeys.oauthStatus(0))).toBeUndefined();
  });

  it("fetchProviderOAuthStatus normalizes providerId before cache key and service call", async () => {
    setTauriRuntime();

    const status = {
      connected: false,
      provider_type: null,
      email: null,
      expires_at: null,
      has_refresh_token: false,
    };
    vi.mocked(providerOAuthStatus).mockResolvedValue(status);

    const client = createTestQueryClient();

    await expect(fetchProviderOAuthStatus(client as never, null)).resolves.toBeNull();
    await expect(fetchProviderOAuthStatus(client as never, 9)).resolves.toEqual(status);
    await expect(fetchProviderOAuthStatus(client as never, Number.NaN)).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );

    expect(providerOAuthStatus).toHaveBeenCalledWith(9);
    expect(client.getQueryData(providersKeys.oauthStatus(9))).toEqual(status);
    expect(client.getQueryState(providersKeys.oauthStatus(Number.NaN))).toBeUndefined();
  });

  it("normalizes OAuth limits providerId before cache reads, refreshes, and query calls", async () => {
    setTauriRuntime();

    const limits = {
      limit_short_label: "5h",
      limit_5h_text: "100 requests",
      limit_weekly_text: null,
      limit_5h_reset_at: 1700000000,
      limit_weekly_reset_at: null,
    };
    vi.mocked(providerOAuthFetchLimits).mockResolvedValue(limits);

    const client = createTestQueryClient();
    client.setQueryData(oauthLimitsKeys.detail(11), limits);

    expect(readProviderOAuthLimitsCache(client, 11)).toEqual(limits);
    expect(() => readProviderOAuthLimitsCache(client, 0)).toThrow("SEC_INVALID_INPUT");

    await expect(refreshProviderOAuthLimits(client, 11)).resolves.toEqual(limits);
    await expect(refreshProviderOAuthLimits(client, 0)).rejects.toThrow("SEC_INVALID_INPUT");

    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useOAuthLimitsQuery(11, true), { wrapper });

    await waitFor(() => {
      expect(result.current.data).toEqual(limits);
    });

    expect(providerOAuthFetchLimits).toHaveBeenCalledWith(11);
    expect(client.getQueryData(oauthLimitsKeys.detail(11))).toEqual(limits);
    expect(() => renderHook(() => useOAuthLimitsQuery(0, false), { wrapper })).toThrow(
      "SEC_INVALID_INPUT"
    );
  });

  it("useProviderSetEnabledMutation updates cached providers list", async () => {
    setTauriRuntime();

    const provider: ProviderSummary = makeProvider({
      id: 1,
      cli_key: "claude",
      name: "P1",
      enabled: false,
    });
    const updated: ProviderSummary = { ...provider, enabled: true };

    vi.mocked(providerSetEnabled).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), [provider]);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderSetEnabledMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 1, enabled: true });
    });

    expect(providerSetEnabled).toHaveBeenCalledWith(1, true);
    expect(client.getQueryData(providersKeys.list("claude"))).toEqual([updated]);
  });

  it("useProviderUpsertMutation updates the cached providers list and invalidates related queries", async () => {
    setTauriRuntime();

    const existing = makeProvider({
      id: 1,
      cli_key: "claude",
      name: "Existing",
      enabled: true,
    });
    const saved = { ...existing, name: "Updated" };

    vi.mocked(providerUpsert).mockResolvedValue(saved);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), [existing]);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderUpsertMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        input: {
          providerId: 1,
          cliKey: "claude",
          name: "Updated",
          baseUrls: [],
          baseUrlMode: "order",
          enabled: true,
          costMultiplier: 1,
          limit5hUsd: null,
          limitDailyUsd: null,
          dailyResetMode: "fixed",
          dailyResetTime: "00:00:00",
          limitWeeklyUsd: null,
          limitMonthlyUsd: null,
          limitTotalUsd: null,
        },
      });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toEqual([saved]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: providersKeys.list("claude") });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: gatewayKeys.circuitStatus("claude"),
    });
  });

  it("useProviderSetEnabledMutation is a no-op when service returns null", async () => {
    setTauriRuntime();

    const provider: ProviderSummary = makeProvider({
      id: 1,
      cli_key: "claude",
      name: "P1",
      enabled: false,
    });

    vi.mocked(providerSetEnabled).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), [provider]);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderSetEnabledMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 1, enabled: true });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toEqual([provider]);
  });

  it("useProviderSetEnabledMutation does not update when list cache is missing", async () => {
    setTauriRuntime();

    const provider: ProviderSummary = makeProvider({
      id: 1,
      cli_key: "claude",
      name: "P1",
      enabled: true,
    });

    vi.mocked(providerSetEnabled).mockResolvedValue(provider);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), null);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderSetEnabledMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 1, enabled: true });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toBeNull();
  });

  it("useProviderDeleteMutation removes provider from cached list", async () => {
    setTauriRuntime();

    const providers: ProviderSummary[] = [
      makeProvider({ id: 1, cli_key: "claude", name: "P1" }),
      makeProvider({ id: 2, cli_key: "claude", name: "P2" }),
    ];

    vi.mocked(providerDelete).mockResolvedValue(true);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), providers);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: " claude " as never, providerId: 1 });
    });

    expect(providerDelete).toHaveBeenCalledWith(1);
    expect(client.getQueryData(providersKeys.list("claude"))).toEqual([providers[1]]);
    expect(client.getQueryData(providersKeys.list(" claude " as never))).toBeUndefined();
  });

  it("useProviderDeleteMutation is a no-op when service returns false", async () => {
    setTauriRuntime();

    const providers: ProviderSummary[] = [makeProvider({ id: 1, cli_key: "claude", name: "P1" })];

    vi.mocked(providerDelete).mockResolvedValue(false);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), providers);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", providerId: 1 });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toEqual(providers);
  });

  it("useProviderDeleteMutation does not update when list cache is missing", async () => {
    setTauriRuntime();

    vi.mocked(providerDelete).mockResolvedValue(true);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), null);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", providerId: 1 });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toBeNull();
  });

  it("useProvidersReorderMutation sets cached list when service returns next order", async () => {
    setTauriRuntime();

    const providers: ProviderSummary[] = [
      makeProvider({ id: 1, cli_key: "claude", name: "P1" }),
      makeProvider({ id: 2, cli_key: "claude", name: "P2" }),
    ];
    const next: ProviderSummary[] = [providers[1], providers[0]];

    vi.mocked(providersReorder).mockResolvedValue(next);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), providers);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProvidersReorderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: " claude " as never, orderedProviderIds: [2, 1] });
    });

    expect(providersReorder).toHaveBeenCalledWith("claude", [2, 1]);
    expect(client.getQueryData(providersKeys.list("claude"))).toEqual(next);
    expect(client.getQueryData(providersKeys.list(" claude " as never))).toBeUndefined();
  });

  it("useProvidersReorderMutation is a no-op when service returns null", async () => {
    setTauriRuntime();

    const providers: ProviderSummary[] = [makeProvider({ id: 1, cli_key: "claude", name: "P1" })];

    vi.mocked(providersReorder).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(providersKeys.list("claude"), providers);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProvidersReorderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", orderedProviderIds: [1] });
    });

    expect(client.getQueryData(providersKeys.list("claude"))).toEqual(providers);
  });

  it("useProviderClaudeTerminalLaunchCommandMutation calls service with provider id", async () => {
    setTauriRuntime();

    vi.mocked(providerClaudeTerminalLaunchCommand).mockResolvedValue("bash '/tmp/aio.sh'");

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderClaudeTerminalLaunchCommandMutation(), {
      wrapper,
    });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 8 });
    });

    expect(providerClaudeTerminalLaunchCommand).toHaveBeenCalledWith(8);
  });

  it("useProviderTestAvailabilityMutation calls service with provider id", async () => {
    setTauriRuntime();

    vi.mocked(providerTestAvailability).mockResolvedValue({
      ok: true,
      provider_id: 8,
      provider_name: "P1",
      base_url: "https://api.example.com",
      status: 200,
      latency_ms: 42,
      error: null,
      response_preview: null,
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderTestAvailabilityMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 8 });
    });

    expect(providerTestAvailability).toHaveBeenCalledWith(8);
  });
});
