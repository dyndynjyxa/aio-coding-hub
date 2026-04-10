import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { providerLimitUsageV1 } from "../../services/providers/providerLimitUsage";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { useProviderLimitUsageV1Query } from "../providerLimitUsage";

vi.mock("../../services/providers/providerLimitUsage", async () => {
  const actual = await vi.importActual<
    typeof import("../../services/providers/providerLimitUsage")
  >("../../services/providers/providerLimitUsage");
  return {
    ...actual,
    providerLimitUsageV1: vi.fn(),
  };
});

describe("query/providerLimitUsage", () => {
  it("calls providerLimitUsageV1 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query("claude"), { wrapper });

    await waitFor(() => {
      expect(providerLimitUsageV1).toHaveBeenCalledWith("claude");
    });
  });

  it("useProviderLimitUsageV1Query enters error state when providerLimitUsageV1 rejects", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockRejectedValue(new Error("provider limit usage query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderLimitUsageV1Query("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query("claude", { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(providerLimitUsageV1).not.toHaveBeenCalled();
  });

  it("supports refetchInterval option without altering query function args", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query(null, { refetchIntervalMs: 5000 }), { wrapper });

    await waitFor(() => {
      expect(providerLimitUsageV1).toHaveBeenCalledWith(null);
    });
  });
});
