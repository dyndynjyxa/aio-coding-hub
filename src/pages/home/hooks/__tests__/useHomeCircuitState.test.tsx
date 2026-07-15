import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useGatewayCircuitAutoRefresh,
  useGatewayCircuitResetProviderMutation,
  useGatewayCircuitStatusQuery,
} from "../../../../query/gateway";
import { useProvidersListQuery } from "../../../../query/providers";
import { useHomeCircuitState } from "../useHomeCircuitState";

vi.mock("../../../../query/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../../../query/gateway")>(
    "../../../../query/gateway"
  );
  return {
    ...actual,
    useGatewayCircuitAutoRefresh: vi.fn(),
    useGatewayCircuitResetProviderMutation: vi.fn(),
    useGatewayCircuitStatusQuery: vi.fn(),
  };
});

vi.mock("../../../../query/providers", () => ({
  useProvidersListQuery: vi.fn(),
}));

describe("useHomeCircuitState", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useGatewayCircuitResetProviderMutation).mockReturnValue({
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation(
      (cliKey) =>
        ({
          data:
            cliKey === "grok"
              ? [
                  {
                    provider_id: 44,
                    state: "OPEN",
                    failure_count: 3,
                    failure_threshold: 3,
                    open_until: 2_000,
                    cooldown_until: null,
                  },
                ]
              : [],
        }) as any
    );
    vi.mocked(useProvidersListQuery).mockImplementation(
      (cliKey) =>
        ({
          data: cliKey === "grok" ? [{ id: 44, name: "Grok upstream" }] : [],
        }) as any
    );
  });

  it("includes Grok providers in the Home open-circuit projection", () => {
    const { result } = renderHook(() => useHomeCircuitState());

    expect(useGatewayCircuitStatusQuery).toHaveBeenCalledWith("grok");
    expect(useProvidersListQuery).toHaveBeenCalledWith("grok");
    expect(useGatewayCircuitAutoRefresh).toHaveBeenCalledWith(
      "grok",
      expect.objectContaining({ hasUnavailable: true })
    );
    expect(result.current.openCircuits).toEqual([
      {
        cli_key: "grok",
        provider_id: 44,
        provider_name: "Grok upstream",
        displayState: "open",
        open_until: 2_000,
      },
    ]);
  });

  it("includes half-open rows without counting them as unavailable and sorts null until last", () => {
    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation(
      (cliKey) =>
        ({
          data:
            cliKey === "grok"
              ? [
                  {
                    provider_id: 45,
                    state: "HALF_OPEN",
                    failure_count: 3,
                    failure_threshold: 3,
                    open_until: null,
                    cooldown_until: null,
                  },
                  {
                    provider_id: 44,
                    state: "OPEN",
                    failure_count: 3,
                    failure_threshold: 3,
                    open_until: 2_000,
                    cooldown_until: null,
                  },
                ]
              : [],
        }) as any
    );
    vi.mocked(useProvidersListQuery).mockImplementation(
      (cliKey) =>
        ({
          data:
            cliKey === "grok"
              ? [
                  { id: 44, name: "Grok upstream" },
                  { id: 45, name: "Grok probe" },
                ]
              : [],
        }) as any
    );

    const { result } = renderHook(() => useHomeCircuitState());

    // 半开行进入结果集（displayState=half_open、open_until 恒 null），且排在有 until 的行之后。
    expect(result.current.openCircuits).toEqual([
      {
        cli_key: "grok",
        provider_id: 44,
        provider_name: "Grok upstream",
        displayState: "open",
        open_until: 2_000,
      },
      {
        cli_key: "grok",
        provider_id: 45,
        provider_name: "Grok probe",
        displayState: "half_open",
        open_until: null,
      },
    ]);
  });

  it("does not count half-open-only rows as unavailable", () => {
    vi.mocked(useGatewayCircuitStatusQuery).mockImplementation(
      (cliKey) =>
        ({
          data:
            cliKey === "grok"
              ? [
                  {
                    provider_id: 44,
                    state: "HALF_OPEN",
                    failure_count: 3,
                    failure_threshold: 3,
                    open_until: null,
                    cooldown_until: null,
                  },
                ]
              : [],
        }) as any
    );

    const { result } = renderHook(() => useHomeCircuitState());

    expect(useGatewayCircuitAutoRefresh).toHaveBeenCalledWith(
      "grok",
      expect.objectContaining({ hasUnavailable: false })
    );
    expect(result.current.openCircuits).toEqual([
      {
        cli_key: "grok",
        provider_id: 44,
        provider_name: "Grok upstream",
        displayState: "half_open",
        open_until: null,
      },
    ]);
  });
});
