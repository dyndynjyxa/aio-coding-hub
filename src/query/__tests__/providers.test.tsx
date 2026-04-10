import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ProviderSummary } from "../../services/providers/providers";
import {
  providerClaudeTerminalLaunchCommand,
  providerDelete,
  providerSetEnabled,
  providersList,
  providersReorder,
} from "../../services/providers/providers";
import {
  useProviderClaudeTerminalLaunchCommandMutation,
  useProviderDeleteMutation,
  useProviderSetEnabledMutation,
  useProvidersListQuery,
  useProvidersReorderMutation,
} from "../providers";
import { providersKeys } from "../keys";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";

vi.mock("../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../services/providers/providers")>(
    "../../services/providers/providers"
  );
  return {
    ...actual,
    providersList: vi.fn(),
    providerSetEnabled: vi.fn(),
    providerDelete: vi.fn(),
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
      await result.current.mutateAsync({ cliKey: "claude", providerId: 1 });
    });

    expect(providerDelete).toHaveBeenCalledWith(1);
    expect(client.getQueryData(providersKeys.list("claude"))).toEqual([providers[1]]);
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
      await result.current.mutateAsync({ cliKey: "claude", orderedProviderIds: [2, 1] });
    });

    expect(providersReorder).toHaveBeenCalledWith("claude", [2, 1]);
    expect(client.getQueryData(providersKeys.list("claude"))).toEqual(next);
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
});
