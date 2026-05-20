import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageSummary } from "../../services/usage/usage";
import {
  USAGE_DAY_DETAIL_FOLDER_MAX_LIMIT,
  USAGE_HOURLY_SERIES_MAX_DAYS,
  USAGE_LEADERBOARD_V2_DEFAULT_LIMIT,
  USAGE_LEADERBOARD_V2_MAX_LIMIT,
  USAGE_PROVIDER_CACHE_RATE_TREND_MAX_LIMIT,
  usageDayDetailV1,
  usageFolderOptionsV1,
  usageHourlySeries,
  usageLeaderboardV2,
  usageProviderCacheRateTrendV1,
  usageSummary,
  usageSummaryV2,
} from "../../services/usage/usage";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { usageKeys } from "../keys";
import {
  useUsageHourlySeriesQuery,
  useUsageDayDetailV1Query,
  useUsageFolderOptionsV1Query,
  useUsageLeaderboardV2Query,
  useUsageProviderCacheRateTrendV1Query,
  useUsageSummaryQuery,
  useUsageSummaryV2Query,
} from "../usage";

function queryRefreshOptions(query: { options?: unknown } | undefined) {
  return (query?.options ?? {}) as {
    refetchInterval?: number | false;
    refetchOnMount?: boolean | "always";
  };
}

vi.mock("../../services/usage/usage", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/usage")>(
    "../../services/usage/usage"
  );
  return {
    ...actual,
    usageHourlySeries: vi.fn(),
    usageDayDetailV1: vi.fn(),
    usageFolderOptionsV1: vi.fn(),
    usageSummary: vi.fn(),
    usageSummaryV2: vi.fn(),
    usageLeaderboardV2: vi.fn(),
    usageProviderCacheRateTrendV1: vi.fn(),
  };
});

function makeUsageSummary(overrides: Partial<UsageSummary> = {}): UsageSummary {
  return {
    requests_total: 0,
    requests_with_usage: 0,
    requests_success: 0,
    requests_failed: 0,
    cost_covered_success: 0,
    avg_duration_ms: null,
    avg_ttfb_ms: null,
    avg_output_tokens_per_second: null,
    input_tokens: 0,
    output_tokens: 0,
    io_total_tokens: 0,
    total_tokens: 0,
    cache_read_input_tokens: 0,
    cache_creation_input_tokens: 0,
    cache_creation_5m_input_tokens: 0,
    cache_creation_1h_input_tokens: 0,
    ...overrides,
  };
}

describe("query/usage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

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

  it("normalizes hourly series days for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(usageHourlySeries).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useUsageHourlySeriesQuery(999), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageHourlySeries).toHaveBeenCalledWith(USAGE_HOURLY_SERIES_MAX_DAYS);
    expect(client.getQueryState(usageKeys.hourlySeries(USAGE_HOURLY_SERIES_MAX_DAYS))).toBeTruthy();
    expect(client.getQueryState(usageKeys.hourlySeries(999))).toBeUndefined();
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

    vi.mocked(usageSummary).mockResolvedValue(makeUsageSummary());

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

  it("normalizes usage summary cliKey before cache key and service call", async () => {
    setTauriRuntime();

    vi.mocked(usageSummary).mockResolvedValue(makeUsageSummary());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = { cliKey: " claude " } as never;
    const normalizedInput = { cliKey: "claude" as const };

    const { result } = renderHook(() => useUsageSummaryQuery("today", input), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageSummary).toHaveBeenCalledWith("today", normalizedInput);
    expect(client.getQueryState(usageKeys.summary("today", normalizedInput))).toBeTruthy();
    expect(client.getQueryState(usageKeys.summary("today", input))).toBeUndefined();
  });

  it("rejects invalid usage summary cliKey before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(() => useUsageSummaryQuery("today", { cliKey: "opencode" } as never), { wrapper })
    ).toThrow("SEC_INVALID_INPUT");
    expect(usageSummary).not.toHaveBeenCalled();
  });

  it("calls usageSummaryV2 with tauri runtime and forwards refresh options", async () => {
    setTauriRuntime();

    vi.mocked(usageSummaryV2).mockResolvedValue(makeUsageSummary());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 7,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    };

    renderHook(
      () =>
        useUsageSummaryV2Query("daily", input, {
          refetchIntervalMs: 60_000,
          refetchOnMount: "always",
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageSummaryV2).toHaveBeenCalledWith("daily", input);
    });

    const query = client.getQueryCache().find({ queryKey: usageKeys.summaryV2("daily", input) });
    const options = queryRefreshOptions(query);
    expect(options.refetchInterval).toBe(60_000);
    expect(options.refetchOnMount).toBe("always");
  });

  it("normalizes usage v2 filters before cache key and service call", async () => {
    setTauriRuntime();

    vi.mocked(usageSummaryV2).mockResolvedValue(makeUsageSummary());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: " gemini ",
      providerId: 7,
      folderKeys: [" /b ", "/a", "/a", " "],
      excludeCx2CcGatewayBridge: true,
    } as never;
    const normalizedInput = {
      startTs: 1,
      endTs: 2,
      cliKey: "gemini" as const,
      providerId: 7,
      folderKeys: ["/a", "/b"],
      excludeCx2CcGatewayBridge: true,
    };

    const { result } = renderHook(() => useUsageSummaryV2Query("custom", input), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageSummaryV2).toHaveBeenCalledWith("custom", normalizedInput);
    expect(client.getQueryState(usageKeys.summaryV2("custom", normalizedInput))).toBeTruthy();
    expect(client.getQueryState(usageKeys.summaryV2("custom", input))).toBeUndefined();
  });

  it("rejects invalid usage v2 filters before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(() => useUsageSummaryV2Query("daily", { providerId: 0 } as never), { wrapper })
    ).toThrow("SEC_INVALID_INPUT");
    expect(usageSummaryV2).not.toHaveBeenCalled();
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
          { enabled: false, refetchIntervalMs: 60_000, refetchOnMount: "always" }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageSummaryV2).not.toHaveBeenCalled();
  });

  it("calls usageLeaderboardV2 with tauri runtime and forwards refresh options", async () => {
    setTauriRuntime();

    vi.mocked(usageLeaderboardV2).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 9,
      limit: null,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    };
    const normalizedInput = { ...input, limit: USAGE_LEADERBOARD_V2_DEFAULT_LIMIT };

    renderHook(
      () =>
        useUsageLeaderboardV2Query("provider", "weekly", input, {
          refetchIntervalMs: 60_000,
          refetchOnMount: "always",
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageLeaderboardV2).toHaveBeenCalledWith("provider", "weekly", normalizedInput);
    });

    const query = client
      .getQueryCache()
      .find({ queryKey: usageKeys.leaderboardV2("provider", "weekly", normalizedInput) });
    const options = queryRefreshOptions(query);
    expect(options.refetchInterval).toBe(60_000);
    expect(options.refetchOnMount).toBe("always");
  });

  it("normalizes usage leaderboard v2 limit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(usageLeaderboardV2).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: " claude ",
      providerId: 9,
      limit: 999,
      folderKeys: [" /tmp/project ", "/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    } as never;
    const normalizedInput = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 9,
      limit: USAGE_LEADERBOARD_V2_MAX_LIMIT,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    };

    const { result } = renderHook(() => useUsageLeaderboardV2Query("provider", "weekly", input), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageLeaderboardV2).toHaveBeenCalledWith("provider", "weekly", normalizedInput);
    expect(
      client.getQueryState(usageKeys.leaderboardV2("provider", "weekly", normalizedInput))
    ).toBeTruthy();
    expect(
      client.getQueryState(usageKeys.leaderboardV2("provider", "weekly", input))
    ).toBeUndefined();
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
          { enabled: false, refetchIntervalMs: 60_000, refetchOnMount: "always" }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageLeaderboardV2).not.toHaveBeenCalled();
  });

  it("calls usageDayDetailV1 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageDayDetailV1).mockResolvedValue({
      day: "2026-04-16",
      folders: [],
      hours: Array.from({ length: 24 }, (_, hour) => ({
        hour,
        requests_total: 0,
        total_tokens: 0,
        io_total_tokens: 0,
      })),
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      day: "2026-04-16",
      cliKey: null,
      providerId: null,
      folderLimit: 8,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    };

    renderHook(() => useUsageDayDetailV1Query(input), { wrapper });

    await waitFor(() => {
      expect(usageDayDetailV1).toHaveBeenCalledWith(input);
    });

    const query = client.getQueryCache().find({ queryKey: usageKeys.dayDetailV1(input) });
    const options = queryRefreshOptions(query);
    expect(options.refetchInterval).toBe(false);
  });

  it("normalizes usage day detail folderLimit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(usageDayDetailV1).mockResolvedValue({
      day: "2026-04-16",
      folders: [],
      hours: [],
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      day: " 2026-04-16 ",
      cliKey: " codex ",
      providerId: 7,
      folderLimit: 999,
      folderKeys: [" /tmp/project ", "/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    } as never;
    const normalizedInput = {
      day: "2026-04-16",
      cliKey: "codex" as const,
      providerId: 7,
      folderLimit: USAGE_DAY_DETAIL_FOLDER_MAX_LIMIT,
      folderKeys: ["/tmp/project"],
      excludeCx2CcGatewayBridge: true,
    };

    const { result } = renderHook(() => useUsageDayDetailV1Query(input), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageDayDetailV1).toHaveBeenCalledWith(normalizedInput);
    expect(client.getQueryState(usageKeys.dayDetailV1(normalizedInput))).toBeTruthy();
    expect(client.getQueryState(usageKeys.dayDetailV1(input))).toBeUndefined();
  });

  it("does not call usageDayDetailV1 when disabled", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageDayDetailV1Query(
          {
            day: "2026-04-16",
            cliKey: "claude",
            providerId: 9,
            folderLimit: 8,
            folderKeys: ["/tmp/project"],
            excludeCx2CcGatewayBridge: true,
          },
          { enabled: false }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageDayDetailV1).not.toHaveBeenCalled();
  });

  it("calls usageFolderOptionsV1 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(usageFolderOptionsV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 11,
      excludeCx2CcGatewayBridge: true,
    };
    const normalizedInput = { ...input, folderKeys: null };

    renderHook(() => useUsageFolderOptionsV1Query("daily", input), { wrapper });

    await waitFor(() => {
      expect(usageFolderOptionsV1).toHaveBeenCalledWith("daily", normalizedInput);
    });

    const query = client
      .getQueryCache()
      .find({ queryKey: usageKeys.folderOptionsV1("daily", normalizedInput) });
    const options = queryRefreshOptions(query);
    expect(options.refetchInterval).toBe(false);
  });

  it("normalizes usage folder options filters before cache key and service call", async () => {
    setTauriRuntime();

    vi.mocked(usageFolderOptionsV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: " claude ",
      providerId: 11,
      excludeCx2CcGatewayBridge: true,
    } as never;
    const normalizedInput = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 11,
      folderKeys: null,
      excludeCx2CcGatewayBridge: true,
    };

    const { result } = renderHook(() => useUsageFolderOptionsV1Query("daily", input), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageFolderOptionsV1).toHaveBeenCalledWith("daily", normalizedInput);
    expect(client.getQueryState(usageKeys.folderOptionsV1("daily", normalizedInput))).toBeTruthy();
    expect(client.getQueryState(usageKeys.folderOptionsV1("daily", input))).toBeUndefined();
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
          excludeCx2CcGatewayBridge: true,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(usageProviderCacheRateTrendV1).toHaveBeenCalledWith("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 11,
        folderKeys: null,
        limit: 20,
        excludeCx2CcGatewayBridge: true,
      });
    });
  });

  it("normalizes usage provider cache trend limit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(usageProviderCacheRateTrendV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      startTs: 1,
      endTs: 2,
      cliKey: " claude ",
      providerId: 11,
      limit: 999,
      excludeCx2CcGatewayBridge: true,
    } as never;
    const normalizedInput = {
      startTs: 1,
      endTs: 2,
      cliKey: "claude" as const,
      providerId: 11,
      folderKeys: null,
      limit: USAGE_PROVIDER_CACHE_RATE_TREND_MAX_LIMIT,
      excludeCx2CcGatewayBridge: true,
    };

    const { result } = renderHook(() => useUsageProviderCacheRateTrendV1Query("daily", input), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(usageProviderCacheRateTrendV1).toHaveBeenCalledWith("daily", normalizedInput);
    expect(
      client.getQueryState(usageKeys.providerCacheRateTrendV1("daily", normalizedInput))
    ).toBeTruthy();
    expect(
      client.getQueryState(usageKeys.providerCacheRateTrendV1("daily", input))
    ).toBeUndefined();
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
