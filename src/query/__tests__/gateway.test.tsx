import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { GatewayProviderCircuitStatus } from "../../services/gateway/gateway";
import {
  GATEWAY_SESSIONS_DEFAULT_LIMIT,
  GATEWAY_SESSIONS_MAX_LIMIT,
  gatewayCircuitResetCli,
  gatewayCircuitResetProvider,
  gatewayCircuitStatus,
  gatewaySessionsList,
  gatewayStatus,
} from "../../services/gateway/gateway";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { gatewayKeys } from "../keys";
import {
  getGatewayCircuitDerivedState,
  summarizeGatewayCircuitRows,
  useGatewayCircuitByProviderId,
  useGatewayCircuitStatusQuery,
  useGatewayCircuitResetCliMutation,
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitAutoRefresh,
  useGatewaySessionsListQuery,
  useGatewayStatusQuery,
} from "../gateway";

vi.mock("../../services/gateway/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../services/gateway/gateway")>(
    "../../services/gateway/gateway"
  );
  return {
    ...actual,
    gatewayStatus: vi.fn(),
    gatewayCircuitStatus: vi.fn(),
    gatewaySessionsList: vi.fn(),
    gatewayCircuitResetProvider: vi.fn(),
    gatewayCircuitResetCli: vi.fn(),
  };
});

type GatewayQueryCliKey = Parameters<typeof useGatewayCircuitStatusQuery>[0];

describe("query/gateway", () => {
  it("getGatewayCircuitDerivedState derives unavailable state and max unavailableUntil", () => {
    expect(
      getGatewayCircuitDerivedState({
        provider_id: 9,
        state: "OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: 120,
        cooldown_until: 180,
      })
    ).toEqual({
      isOpen: true,
      isUnavailable: true,
      unavailableUntil: 180,
    });

    expect(
      getGatewayCircuitDerivedState({
        provider_id: 10,
        state: "CLOSED",
        failure_count: 0,
        failure_threshold: 5,
        open_until: 500,
        cooldown_until: 150,
      })
    ).toEqual({
      isOpen: false,
      isUnavailable: true,
      unavailableUntil: 150,
    });
  });

  it("getGatewayCircuitDerivedState treats HALF_OPEN as probe-available instead of unavailable", () => {
    expect(
      getGatewayCircuitDerivedState({
        provider_id: 11,
        state: "HALF_OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
      })
    ).toEqual({
      isOpen: false,
      isUnavailable: false,
      unavailableUntil: null,
    });
  });

  it("summarizeGatewayCircuitRows builds provider lookup and refresh summary", () => {
    const summary = summarizeGatewayCircuitRows([
      {
        provider_id: 1,
        state: "OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
      },
      {
        provider_id: 2,
        state: "CLOSED",
        failure_count: 0,
        failure_threshold: 5,
        open_until: 999,
        cooldown_until: 140,
      },
      {
        provider_id: 3,
        state: "CLOSED",
        failure_count: 0,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
      },
      {
        provider_id: 4,
        state: "HALF_OPEN",
        failure_count: 5,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
      },
    ]);

    expect(summary.byProviderId[1]?.provider_id).toBe(1);
    expect(summary.byProviderId[2]?.provider_id).toBe(2);
    expect(summary.byProviderId[4]?.provider_id).toBe(4);
    expect(summary.unavailableRows.map(({ row }) => row.provider_id)).toEqual([1, 2]);
    expect(summary.hasUnavailable).toBe(true);
    expect(summary.hasUnavailableWithoutUntil).toBe(true);
    expect(summary.earliestUnavailableUntil).toBe(140);
  });

  it("useGatewayCircuitByProviderId builds a provider_id -> status map", async () => {
    setTauriRuntime();

    const rows: GatewayProviderCircuitStatus[] = [
      {
        provider_id: 1,
        state: "closed",
        failure_count: 0,
        failure_threshold: 5,
        open_until: null,
        cooldown_until: null,
      },
      {
        provider_id: 2,
        state: "open",
        failure_count: 5,
        failure_threshold: 5,
        open_until: 123,
        cooldown_until: 456,
      },
    ];
    vi.mocked(gatewayCircuitStatus).mockResolvedValue(rows);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () => useGatewayCircuitByProviderId(" claude " as unknown as GatewayQueryCliKey),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(gatewayCircuitStatus).toHaveBeenCalledWith("claude");
    expect(result.current.circuitByProviderId[1]).toEqual(rows[0]);
    expect(result.current.circuitByProviderId[2]).toEqual(rows[1]);
  });

  it("useGatewayCircuitStatusQuery normalizes cliKey before cache key and service call", async () => {
    setTauriRuntime();
    vi.mocked(gatewayCircuitStatus).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useGatewayCircuitStatusQuery(" codex " as unknown as GatewayQueryCliKey), {
      wrapper,
    });

    await waitFor(() => {
      expect(gatewayCircuitStatus).toHaveBeenCalledWith("codex");
    });
    expect(client.getQueryState(gatewayKeys.circuitStatus("codex"))).toBeTruthy();
    expect(
      client.getQueryState(gatewayKeys.circuitStatus(" codex " as unknown as GatewayQueryCliKey))
    ).toBeUndefined();
  });

  it("rejects invalid gateway circuit cliKey before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(() => useGatewayCircuitStatusQuery("opencode" as unknown as GatewayQueryCliKey), {
        wrapper,
      })
    ).toThrow("SEC_INVALID_INPUT");
    expect(gatewayCircuitStatus).not.toHaveBeenCalled();
  });

  it("useGatewaySessionsListQuery respects options.enabled", async () => {
    setTauriRuntime();

    vi.mocked(gatewaySessionsList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useGatewaySessionsListQuery(10, { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(gatewaySessionsList).not.toHaveBeenCalled();
  });

  it("useGatewaySessionsListQuery fetches with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(gatewaySessionsList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useGatewaySessionsListQuery(10), { wrapper });

    await waitFor(() => {
      expect(gatewaySessionsList).toHaveBeenCalledWith(10);
    });
  });

  it("useGatewaySessionsListQuery normalizes limit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(gatewaySessionsList).mockClear();
    vi.mocked(gatewaySessionsList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewaySessionsListQuery(999), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(gatewaySessionsList).toHaveBeenCalledWith(GATEWAY_SESSIONS_MAX_LIMIT);
    expect(client.getQueryState(gatewayKeys.sessionsList(GATEWAY_SESSIONS_MAX_LIMIT))).toBeTruthy();
    expect(client.getQueryState(gatewayKeys.sessionsList(999))).toBeUndefined();
  });

  it("useGatewaySessionsListQuery uses the backend default limit when omitted", async () => {
    setTauriRuntime();

    vi.mocked(gatewaySessionsList).mockClear();
    vi.mocked(gatewaySessionsList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useGatewaySessionsListQuery(), { wrapper });

    await waitFor(() => {
      expect(gatewaySessionsList).toHaveBeenCalledWith(GATEWAY_SESSIONS_DEFAULT_LIMIT);
    });
    expect(
      client.getQueryState(gatewayKeys.sessionsList(GATEWAY_SESSIONS_DEFAULT_LIMIT))
    ).toBeTruthy();
  });

  it("useGatewaySessionsListQuery enters error state when service rejects", async () => {
    setTauriRuntime();

    vi.mocked(gatewaySessionsList).mockRejectedValue(new Error("sessions boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewaySessionsListQuery(10), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useGatewayStatusQuery respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useGatewayStatusQuery({ enabled: false }), { wrapper });
    await Promise.resolve();

    expect(gatewayStatus).not.toHaveBeenCalled();
  });

  it("useGatewayStatusQuery fetches gateway status", async () => {
    setTauriRuntime();

    vi.mocked(gatewayStatus).mockResolvedValue({
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewayStatusQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(gatewayStatus).toHaveBeenCalledTimes(1);
    expect(result.current.data?.running).toBe(true);
  });

  it("useGatewayStatusQuery enters error state when service rejects", async () => {
    setTauriRuntime();

    vi.mocked(gatewayStatus).mockRejectedValue(new Error("status boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewayStatusQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useGatewayCircuitResetProviderMutation invalidates cliKey circuit status when provided", async () => {
    vi.mocked(gatewayCircuitResetProvider).mockResolvedValue(true);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewayCircuitResetProviderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        cliKey: " claude " as unknown as GatewayQueryCliKey,
        providerId: 1,
      });
    });

    expect(gatewayCircuitResetProvider).toHaveBeenCalledWith(1);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuitStatus("claude") });
  });

  it("useGatewayCircuitResetProviderMutation invalidates all circuits when cliKey is absent", async () => {
    vi.mocked(gatewayCircuitResetProvider).mockResolvedValue(true);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewayCircuitResetProviderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ providerId: 1 });
    });

    expect(gatewayCircuitResetProvider).toHaveBeenCalledWith(1);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuits() });
  });

  it("useGatewayCircuitResetCliMutation invalidates cli circuit status", async () => {
    vi.mocked(gatewayCircuitResetCli).mockResolvedValue(1);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useGatewayCircuitResetCliMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        cliKey: " claude " as unknown as GatewayQueryCliKey,
      });
    });

    expect(gatewayCircuitResetCli).toHaveBeenCalledWith("claude");
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: gatewayKeys.circuitStatus("claude") });
  });

  it("rejects invalid gateway circuit reset cliKey before mutation side effects", async () => {
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const providerResult = renderHook(() => useGatewayCircuitResetProviderMutation(), { wrapper });
    await expect(
      providerResult.result.current.mutateAsync({
        cliKey: "opencode" as unknown as GatewayQueryCliKey,
        providerId: 1,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    const cliResult = renderHook(() => useGatewayCircuitResetCliMutation(), { wrapper });
    await expect(
      cliResult.result.current.mutateAsync({
        cliKey: "opencode" as unknown as GatewayQueryCliKey,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(gatewayCircuitResetProvider).not.toHaveBeenCalled();
    expect(gatewayCircuitResetCli).not.toHaveBeenCalled();
  });

  it("useGatewayCircuitAutoRefresh invalidates normalized circuit keys", async () => {
    vi.useFakeTimers();
    try {
      const client = createTestQueryClient();
      const invalidateSpy = vi.spyOn(client, "invalidateQueries");
      const wrapper = createQueryWrapper(client);

      renderHook(
        () =>
          useGatewayCircuitAutoRefresh(
            " claude " as unknown as GatewayQueryCliKey,
            {
              byProviderId: {},
              unavailableRows: [],
              hasUnavailable: true,
              hasUnavailableWithoutUntil: true,
              earliestUnavailableUntil: null,
            },
            { enabled: true }
          ),
        { wrapper }
      );

      await act(async () => {
        vi.advanceTimersByTime(250);
      });

      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: gatewayKeys.circuitStatus("claude"),
      });
      expect(invalidateSpy).not.toHaveBeenCalledWith({
        queryKey: gatewayKeys.circuitStatus(" claude " as unknown as GatewayQueryCliKey),
      });
    } finally {
      vi.useRealTimers();
    }
  });
});
