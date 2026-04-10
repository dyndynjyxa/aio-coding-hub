import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  usageHourlySeries,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageSummary,
  usageSummaryV2,
} from "../../services/usage/usage";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import {
  useUsageHourlySeriesQuery,
  useUsageLeaderboardV2Query,
  useUsageProviderCacheRateTrendV1Query,
  useUsageSummaryQuery,
  useUsageSummaryV2Query,
} from "../usage";

vi.mock("../../services/usage/usage", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/usage")>(
    "../../services/usage/usage"
  );
  return {
    ...actual,
    usageHourlySeries: vi.fn(),
    usageSummary: vi.fn(),
    usageSummaryV2: vi.fn(),
    usageLeaderboardV2: vi.fn(),
    usageProviderCacheRateTrendV1: vi.fn(),
  };
});

describe("query/usage", () => {
  it("calls usageHourlySeries with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageHourlySeries).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useUsageHourlySeriesQuery(7), { wrapper });

    await waitFor(() => {
      expect(usageHourlySeries).toHaveBeenCalledWith(7);
    });
  });

  it("useUsageHourlySeriesQuery enters error state when usageHourlySeries rejects", async () => {
    setTauriRuntime();

    vi.mocked(usageHourlySeries).mockRejectedValue(new Error("usage hourly query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useUsageHourlySeriesQuery(7), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useUsageHourlySeriesQuery(7, { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(usageHourlySeries).not.toHaveBeenCalled();
  });

  it("calls usageSummary with tauri runtime and respects options.enabled + refetchIntervalMs branches", async () => {
    setTauriRuntime();

    vi.mocked(usageSummary).mockResolvedValue(null);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useUsageSummaryQuery("today", { cliKey: "claude" }), { wrapper });

    await waitFor(() => {
      expect(usageSummary).toHaveBeenCalledWith("today", { cliKey: "claude" });
    });

    vi.mocked(usageSummary).mockClear();

    renderHook(
      () => useUsageSummaryQuery("today", { cliKey: "claude" }, { refetchIntervalMs: false }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageSummary).toHaveBeenCalledWith("today", { cliKey: "claude" });
    });

    vi.mocked(usageSummary).mockClear();

    renderHook(() => useUsageSummaryQuery("today", { cliKey: "claude" }, { enabled: false }), {
      wrapper,
    });
    await Promise.resolve();

    expect(usageSummary).not.toHaveBeenCalled();
  });

  it("calls usageSummaryV2 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageSummaryV2).mockResolvedValue(null);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageSummaryV2Query("daily", {
          startTs: 1,
          endTs: 2,
          cliKey: "claude",
          providerId: 7,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageSummaryV2).toHaveBeenCalledWith("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 7,
      });
    });
  });

  it("does not call usageSummaryV2 when disabled", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageSummaryV2Query(
          "daily",
          { startTs: 1, endTs: 2, cliKey: "claude", providerId: null },
          { enabled: false }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageSummaryV2).not.toHaveBeenCalled();
  });

  it("calls usageLeaderboardV2 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageLeaderboardV2).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageLeaderboardV2Query("provider", "weekly", {
          startTs: 1,
          endTs: 2,
          cliKey: "claude",
          providerId: 9,
          limit: null,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageLeaderboardV2).toHaveBeenCalledWith("provider", "weekly", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 9,
        limit: null,
      });
    });
  });

  it("does not call usageLeaderboardV2 when disabled", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageLeaderboardV2Query(
          "provider",
          "weekly",
          {
            startTs: 1,
            endTs: 2,
            cliKey: "claude",
            providerId: null,
            limit: null,
          },
          { enabled: false }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageLeaderboardV2).not.toHaveBeenCalled();
  });

  it("calls usageProviderCacheRateTrendV1 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageProviderCacheRateTrendV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageProviderCacheRateTrendV1Query("daily", {
          startTs: 1,
          endTs: 2,
          cliKey: "claude",
          providerId: 11,
          limit: 20,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageProviderCacheRateTrendV1).toHaveBeenCalledWith("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 11,
        limit: 20,
      });
    });
  });

  it("does not call usageProviderCacheRateTrendV1 when disabled", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageProviderCacheRateTrendV1Query(
          "daily",
          {
            startTs: 1,
            endTs: 2,
            cliKey: "claude",
            providerId: null,
            limit: 20,
          },
          { enabled: false }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageProviderCacheRateTrendV1).not.toHaveBeenCalled();
  });
});
