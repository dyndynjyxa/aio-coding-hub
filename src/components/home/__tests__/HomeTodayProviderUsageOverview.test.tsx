import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { UseQueryResult } from "@tanstack/react-query";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HomeTodayProviderUsageOverview } from "../HomeTodayProviderUsageOverview";
import { useHomeTokenCostDataModel } from "../useHomeTokenCostDataModel";
import { useUsageLeaderboardV2Query } from "../../../query/usage";
import type { RequestLogSummary } from "../../../services/gateway/requestLogs";
import type { TraceSession } from "../../../services/gateway/traceStore";
import type { UsageLeaderboardRow } from "../../../services/usage/usage";
import { HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY } from "../../../services/home/homeUsageDevelopmentTime";

vi.mock("../useHomeTokenCostDataModel", () => ({
  useHomeTokenCostDataModel: vi.fn(),
}));

vi.mock("../../../query/usage", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/usage")>("../../../query/usage");
  return {
    ...actual,
    useUsageLeaderboardV2Query: vi.fn(),
  };
});

function createActiveSession(
  providerName: string,
  options?: { providerId?: number; cliKey?: string }
) {
  return {
    cli_key: options?.cliKey ?? "claude",
    session_id: `session-${providerName}`,
    session_suffix: "abcd",
    provider_id: options?.providerId ?? 1,
    provider_name: providerName,
    expires_at: Date.now(),
    request_count: 1,
    total_input_tokens: 100,
    total_output_tokens: 50,
    total_cost_usd: 0.1,
    total_duration_ms: 1000,
  };
}

function createRunningTrace(
  providerName: string,
  options?: { providerId?: number; cliKey?: string; traceId?: string }
): TraceSession {
  const cliKey = options?.cliKey ?? "claude";
  const providerId = options?.providerId ?? 1;
  const now = Date.now();

  return {
    trace_id: options?.traceId ?? `trace-${providerName}`,
    cli_key: cliKey,
    session_id: `session-${providerName}`,
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: null,
    first_seen_ms: now - 5_000,
    last_seen_ms: now,
    attempts: [
      {
        trace_id: options?.traceId ?? `trace-${providerName}`,
        cli_key: cliKey,
        session_id: `session-${providerName}`,
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: null,
        attempt_index: 1,
        provider_id: providerId,
        session_reuse: null,
        provider_name: providerName,
        base_url: "https://example.com",
        outcome: "started",
        status: null,
        attempt_started_ms: now - 3_000,
        attempt_duration_ms: 3_000,
        circuit_state_before: null,
        circuit_state_after: null,
        circuit_failure_count: null,
        circuit_failure_threshold: null,
        claude_model_mapping: null,
      },
    ],
  };
}

function createActiveRequestFromTrace(trace: TraceSession) {
  return {
    trace_id: trace.trace_id,
    cli_key: trace.cli_key,
    session_id: trace.session_id ?? null,
    method: trace.method,
    path: trace.path,
    query: trace.query,
    requested_model: trace.requested_model ?? null,
    created_at_ms: trace.first_seen_ms,
    last_activity_ms: trace.last_seen_ms,
    current_attempt: null,
  };
}

function createLeaderboardRow(
  overrides: Pick<UsageLeaderboardRow, "key" | "name"> &
    Partial<Omit<UsageLeaderboardRow, "key" | "name" | "hourly_estimated_development_time_ms">> & {
      hourly_estimated_development_time_ms?: number[] | null;
    }
): UsageLeaderboardRow {
  const {
    key,
    name,
    folder_path = null,
    hourly_estimated_development_time_ms = null,
    ...rest
  } = overrides;
  return {
    key,
    name,
    folder_path,
    requests_total: 1,
    requests_success: 1,
    requests_failed: 0,
    total_tokens: 1_200,
    io_total_tokens: 1_000,
    input_tokens: 700,
    output_tokens: 300,
    cache_creation_input_tokens: 100,
    cache_read_input_tokens: 100,
    total_duration_ms: 900,
    first_request_created_at_ms: null,
    last_request_created_at_ms: null,
    last_request_completed_at_ms: null,
    estimated_development_time_ms: null,
    hourly_estimated_development_time_ms,
    avg_duration_ms: 900,
    avg_ttfb_ms: 200,
    avg_output_tokens_per_second: 90,
    cost_usd: 0.1,
    ...rest,
  };
}

function createRequestLog(overrides: Partial<RequestLogSummary> = {}): RequestLogSummary {
  return {
    id: 1,
    trace_id: "trace-Claude Main",
    cli_key: "claude",
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    query: null,
    status: null,
    error_code: null,
    duration_ms: 0,
    ttfb_ms: null,
    attempts_json: "[]",
    input_tokens: null,
    output_tokens: null,
    total_tokens: null,
    cache_read_input_tokens: null,
    cache_creation_input_tokens: null,
    cache_creation_5m_input_tokens: null,
    cache_creation_1h_input_tokens: null,
    usage_json: null,
    requested_model: "claude-sonnet",
    cost_usd: null,
    cost_multiplier: 1,
    special_settings_json: null,
    provider_chain_json: null,
    error_details_json: null,
    final_provider_id: null,
    created_at_ms: Date.now(),
    created_at: Math.floor(Date.now() / 1000),
    ...overrides,
  } as RequestLogSummary;
}

function mockDataModel(overrides: Partial<ReturnType<typeof useHomeTokenCostDataModel>> = {}) {
  vi.mocked(useHomeTokenCostDataModel).mockReturnValue({
    summary: {
      requests_total: 20,
      requests_with_usage: 20,
      requests_success: 18,
      requests_failed: 2,
      cost_covered_success: 18,
      total_duration_ms: 425_800,
      avg_duration_ms: 1100,
      avg_ttfb_ms: 260,
      avg_output_tokens_per_second: 95.2,
      input_tokens: 12_000,
      output_tokens: 8_000,
      io_total_tokens: 20_000,
      total_tokens: 25_000,
      cache_read_input_tokens: 3_000,
      cache_creation_input_tokens: 1_000,
      cache_creation_5m_input_tokens: 0,
      cache_creation_1h_input_tokens: 0,
    },
    rows: [
      {
        key: "provider-2",
        name: "Claude Main",
        requests_total: 5,
        requests_success: 5,
        requests_failed: 0,
        total_tokens: 6_200,
        io_total_tokens: 5_000,
        input_tokens: 3_000,
        output_tokens: 2_000,
        cache_creation_input_tokens: 500,
        cache_read_input_tokens: 700,
        total_duration_ms: 62_000,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 900,
        avg_ttfb_ms: 220,
        avg_output_tokens_per_second: 90,
        cost_usd: 0.5,
      },
      {
        key: "provider-4",
        name: "Gemini Mirror",
        requests_total: 7,
        requests_success: 6,
        requests_failed: 1,
        total_tokens: 10_200,
        io_total_tokens: 8_000,
        input_tokens: 4_500,
        output_tokens: 3_500,
        cache_creation_input_tokens: 800,
        cache_read_input_tokens: 1_400,
        total_duration_ms: 308_000,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 1200,
        avg_ttfb_ms: 320,
        avg_output_tokens_per_second: 86,
        cost_usd: 0.9,
      },
      {
        key: "provider-1",
        name: "OpenAI Primary",
        requests_total: 3,
        requests_success: 3,
        requests_failed: 0,
        total_tokens: 5_800,
        io_total_tokens: 4_000,
        input_tokens: 2_000,
        output_tokens: 2_000,
        cache_creation_input_tokens: 600,
        cache_read_input_tokens: 1_200,
        total_duration_ms: 42_000,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 110,
        cost_usd: 0.7,
      },
      {
        key: "provider-5",
        name: "DeepSeek Relay",
        requests_total: 2,
        requests_success: 2,
        requests_failed: 0,
        total_tokens: 3_500,
        io_total_tokens: 2_000,
        input_tokens: 1_400,
        output_tokens: 600,
        cache_creation_input_tokens: 700,
        cache_read_input_tokens: 800,
        total_duration_ms: 12_000,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 760,
        avg_ttfb_ms: 180,
        avg_output_tokens_per_second: 120,
        cost_usd: null,
      },
      {
        key: "provider-3",
        name: "Mistral Edge",
        requests_total: 2,
        requests_success: 1,
        requests_failed: 1,
        total_tokens: 1_600,
        io_total_tokens: 800,
        input_tokens: 500,
        output_tokens: 300,
        cache_creation_input_tokens: 200,
        cache_read_input_tokens: 600,
        total_duration_ms: 1_200,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 1500,
        avg_ttfb_ms: 400,
        avg_output_tokens_per_second: 40,
        cost_usd: 0.1,
      },
      {
        key: "provider-6",
        name: "Local Sandbox",
        requests_total: 1,
        requests_success: 1,
        requests_failed: 0,
        total_tokens: 300,
        io_total_tokens: 200,
        input_tokens: 120,
        output_tokens: 80,
        cache_creation_input_tokens: 20,
        cache_read_input_tokens: 80,
        total_duration_ms: 600,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 600,
        avg_ttfb_ms: 150,
        avg_output_tokens_per_second: 60,
        cost_usd: 0.01,
      },
    ],
    totalCostUsd: 2.21,
    loading: false,
    fetching: false,
    errorText: null,
    previewActive: false,
    refresh: vi.fn(),
    ...overrides,
  } as ReturnType<typeof useHomeTokenCostDataModel>);
}

function rowCellTexts(row: HTMLElement) {
  return within(row)
    .getAllByRole("cell")
    .map((cell) => cell.textContent?.trim() ?? "");
}

function mockDayLeaderboardRows(
  rows: Array<Pick<UsageLeaderboardRow, "key"> & Partial<UsageLeaderboardRow>>
) {
  const data = rows.map(({ key, name, ...row }) =>
    createLeaderboardRow({ key, name: name ?? key, ...row })
  );
  const result = {
    data,
    isLoading: false,
    isError: false,
    isSuccess: true,
    isFetching: false,
    isPending: false,
    isPaused: false,
    isEnabled: true,
    isLoadingError: false,
    isRefetchError: false,
    isRefetching: false,
    isStale: false,
    isFetched: true,
    isFetchedAfterMount: true,
    isPlaceholderData: false,
    isInitialLoading: false,
    status: "success",
    fetchStatus: "idle",
    failureCount: 0,
    failureReason: null,
    errorUpdateCount: 0,
    dataUpdatedAt: Date.now(),
    errorUpdatedAt: 0,
    error: null,
    refetch: vi.fn().mockResolvedValue({ data }),
    promise: Promise.resolve(data),
  } as UseQueryResult<UsageLeaderboardRow[]>;
  vi.mocked(useUsageLeaderboardV2Query).mockReturnValue(result);
}

describe("components/home/HomeTodayProviderUsageOverview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.removeItem("homeUsageDayStartHour");
    window.localStorage.removeItem(HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY);
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    mockDayLeaderboardRows([
      {
        key: "2026-04-16",
        first_request_created_at_ms: new Date(2026, 3, 16, 8, 15).getTime(),
        last_request_completed_at_ms: new Date(2026, 3, 16, 23, 34).getTime(),
        estimated_development_time_ms: 12_600_000,
        hourly_estimated_development_time_ms: [
          0, 0, 0, 0, 0, 0, 0, 0, 3_600_000, 1_800_000, 1_200_000, 0, 0, 0, 0, 0, 0, 0, 3_000_000,
          0, 0, 0, 0, 3_000_000,
        ],
      },
    ]);
  });

  it("uses the fixed today provider query config and renders summary plus top providers", async () => {
    const user = userEvent.setup();
    mockDataModel();

    render(<HomeTodayProviderUsageOverview devPreviewEnabled={true} activeSessions={[]} />);

    expect(vi.mocked(useHomeTokenCostDataModel)).toHaveBeenCalledWith({
      scope: "provider",
      queryConfig: {
        period: "daily",
        input: {
          startTs: null,
          endTs: null,
          cliKey: null,
          providerId: null,
          dayStartHour: 0,
          fullIdleGapMinutes: 15,
          sessionBreakGapMinutes: 30,
          excludeCx2CcGatewayBridge: true,
        },
        previewFactor: 1,
      },
      devPreviewEnabled: true,
      queryRefreshConfig: {
        summary: {
          refetchIntervalMs: 60_000,
          refetchOnMount: "always",
        },
        leaderboard: {
          refetchIntervalMs: 60_000,
          refetchOnMount: "always",
        },
      },
    });
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenCalledWith(
      "day",
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        dayStartHour: 0,
        fullIdleGapMinutes: 15,
        sessionBreakGapMinutes: 30,
        limit: null,
        excludeCx2CcGatewayBridge: true,
      }),
      expect.objectContaining({
        refetchIntervalMs: 60_000,
        refetchOnMount: "always",
      })
    );

    const totalWithCacheCard = screen.getByText("含缓存总 Token").parentElement;
    const inputOutputCacheCard = screen.getByText("输入+出/缓存率").parentElement;
    const activityRangeCard = screen.getByText("活动范围").parentElement;
    expect(totalWithCacheCard).toBeTruthy();
    expect(inputOutputCacheCard).toBeTruthy();
    expect(activityRangeCard).toBeTruthy();
    expect(within(totalWithCacheCard as HTMLElement).getByText("25.0K")).toBeInTheDocument();
    expect(
      within(inputOutputCacheCard as HTMLElement).getByText("20.0K/18.8%")
    ).toBeInTheDocument();
    expect(within(activityRangeCard as HTMLElement).getByText("08:15–23:34")).toBeInTheDocument();
    fireEvent.click(activityRangeCard as HTMLElement);
    expect(within(activityRangeCard as HTMLElement).queryByText("活动范围")).toBeNull();
    expect(within(activityRangeCard as HTMLElement).getByLabelText("逐小时活动趋势")).toHaveClass(
      "h-5"
    );
    fireEvent.keyDown(activityRangeCard as HTMLElement, { key: "Enter" });
    expect(within(activityRangeCard as HTMLElement).getByText("活动范围")).toBeInTheDocument();
    const estimatedDevelopmentTimeCard = screen.getByText("预估开发时间").closest(".relative");
    expect(estimatedDevelopmentTimeCard).toBeTruthy();
    expect(screen.getByText("预估开发时间").parentElement).toHaveClass("flex", "h-4", "leading-4");
    expect(screen.getByText("含缓存总 Token")).toHaveClass("flex", "h-4", "leading-4");
    expect(
      within(estimatedDevelopmentTimeCard as HTMLElement).getByText("3h30m")
    ).toBeInTheDocument();
    expect(screen.queryByText("总请求数")).not.toBeInTheDocument();
    const totalCostCard = screen.getAllByText("总花费")[0]?.parentElement;
    expect(totalCostCard).toBeTruthy();
    const summaryCards = totalWithCacheCard?.parentElement;
    expect(summaryCards).toBeTruthy();
    expect(
      Array.from((summaryCards as HTMLElement).children).map((card) => card.textContent)
    ).toEqual([
      "含缓存总 Token25.0K",
      "输入+出/缓存率20.0K/18.8%",
      "活动范围08:15–23:34",
      "预估开发时间3h30m",
      "总花费$2.21",
    ]);
    expect(within(summaryCards as HTMLElement).queryByText("输入+输出 Token")).toBeNull();
    expect(within(summaryCards as HTMLElement).queryByText("缓存命中率")).toBeNull();
    expect(
      (estimatedDevelopmentTimeCard as HTMLElement).compareDocumentPosition(
        totalCostCard as HTMLElement
      ) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    await user.hover(screen.getByText("预估开发时间").parentElement as HTMLElement);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("15–30 分钟逐步减少");
    fireEvent.click(estimatedDevelopmentTimeCard as HTMLElement);
    expect(
      within(estimatedDevelopmentTimeCard as HTMLElement).getByText("请求总耗时")
    ).toBeInTheDocument();
    expect(
      within(estimatedDevelopmentTimeCard as HTMLElement).getByText("7m6s")
    ).toBeInTheDocument();
    expect(estimatedDevelopmentTimeCard).toHaveAttribute("aria-pressed", "true");
    fireEvent.keyDown(estimatedDevelopmentTimeCard as HTMLElement, { key: "Enter" });
    expect(
      within(estimatedDevelopmentTimeCard as HTMLElement).getByText("预估开发时间")
    ).toBeInTheDocument();
    expect(estimatedDevelopmentTimeCard).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("$2.21")).toBeInTheDocument();
    const providerHeader = screen.getByText("供应商").closest("th");
    const usageTable = screen.getByRole("table", { name: "今日供应商用量" });
    const headerTexts = within(usageTable)
      .getAllByRole("columnheader")
      .map((header) => header.textContent?.trim() ?? "");
    const totalTokenHeader = screen.getByRole("columnheader", { name: "总Token" });
    const cacheHitRateHeader = screen.getByRole("columnheader", { name: "缓存命中率" });
    const inputOutputTokenHeader = screen.getByRole("columnheader", {
      name: "输入+输出Token",
    });
    expect(providerHeader).toBeTruthy();
    expect(headerTexts).toEqual([
      "供应商（前 3 个）",
      "总Token",
      "输入+输出Token",
      "缓存命中率",
      "成功率",
      "总耗时",
      "总花费",
    ]);
    expect(totalTokenHeader).toBeTruthy();
    expect(cacheHitRateHeader).toBeTruthy();
    expect(inputOutputTokenHeader).toBeTruthy();
    expect(within(providerHeader as HTMLElement).getByText("（前 3 个）")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "成功率" })).toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: /缓存情况/ })).not.toBeInTheDocument();
    expect(screen.queryByText("Token 占比")).not.toBeInTheDocument();

    const geminiRow = screen.getByText("Gemini Mirror").closest("tr");
    const claudeRow = screen.getByText("Claude Main").closest("tr");
    const openaiRow = screen.getByText("OpenAI Primary").closest("tr");
    expect(geminiRow).toBeTruthy();
    expect(claudeRow).toBeTruthy();
    expect(openaiRow).toBeTruthy();
    expect(screen.queryByText("DeepSeek Relay")).not.toBeInTheDocument();
    expect(screen.queryByText("Mistral Edge")).not.toBeInTheDocument();
    expect(screen.queryByText("Local Sandbox")).not.toBeInTheDocument();

    expect(rowCellTexts(geminiRow as HTMLElement)).toEqual([
      "Gemini Mirror",
      "10.2K",
      "8.0K",
      "20.9%",
      "85.7%",
      "5m8s",
      "$0.90",
    ]);
    expect(within(geminiRow as HTMLElement).getByText("$0.90")).toBeInTheDocument();
    expect(within(geminiRow as HTMLElement).getByText("85.7%")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).getByText("100.0%")).toBeInTheDocument();
    expect(rowCellTexts(claudeRow as HTMLElement)).toEqual([
      "Claude Main",
      "6.2K",
      "5.0K",
      "16.7%",
      "100.0%",
      "1m2s",
      "$0.50",
    ]);
    expect(rowCellTexts(openaiRow as HTMLElement)).toEqual([
      "OpenAI Primary",
      "5.8K",
      "4.0K",
      "31.6%",
      "100.0%",
      "42s",
      "$0.70",
    ]);
  });

  it("uses the shared stored statistics day start hour for today overview queries", () => {
    window.localStorage.setItem("homeUsageDayStartHour", "7");
    mockDataModel();
    mockDayLeaderboardRows([
      {
        key: "2026-04-16",
        first_request_created_at_ms: new Date(2026, 3, 16, 20, 0).getTime(),
        last_request_completed_at_ms: new Date(2026, 3, 17, 2, 5).getTime(),
        estimated_development_time_ms: 12_600_000,
        hourly_estimated_development_time_ms: [
          3_000_000, 0, 4_200_000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3_600_000,
          1_800_000, 0, 0,
        ],
      },
    ]);

    render(<HomeTodayProviderUsageOverview activeSessions={[]} />);

    expect(vi.mocked(useHomeTokenCostDataModel)).toHaveBeenCalledWith(
      expect.objectContaining({
        queryConfig: expect.objectContaining({
          period: "daily",
          input: expect.objectContaining({
            startTs: null,
            endTs: null,
            cliKey: null,
            providerId: null,
            dayStartHour: 7,
            excludeCx2CcGatewayBridge: true,
          }),
        }),
      })
    );
    expect(screen.getByText("20:00–次日02:05")).toBeInTheDocument();
  });

  it("uses shared development time thresholds for the top card query and tooltip", async () => {
    const user = userEvent.setup();
    window.localStorage.setItem(
      HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY,
      JSON.stringify({ fullIdleGapMinutes: 10, sessionBreakGapMinutes: 45 })
    );
    mockDataModel();

    render(<HomeTodayProviderUsageOverview activeSessions={[]} />);

    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenCalledWith(
      "day",
      "daily",
      expect.objectContaining({
        fullIdleGapMinutes: 10,
        sessionBreakGapMinutes: 45,
      }),
      expect.anything()
    );

    await user.hover(screen.getByText("预估开发时间").parentElement as HTMLElement);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("10–45 分钟逐步减少");
  });

  it("disables polling while the page is hidden", () => {
    mockDataModel();
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });

    render(<HomeTodayProviderUsageOverview />);

    expect(vi.mocked(useHomeTokenCostDataModel)).toHaveBeenCalledWith({
      scope: "provider",
      queryConfig: {
        period: "daily",
        input: {
          startTs: null,
          endTs: null,
          cliKey: null,
          providerId: null,
          dayStartHour: 0,
          fullIdleGapMinutes: 15,
          sessionBreakGapMinutes: 30,
          excludeCx2CcGatewayBridge: true,
        },
        previewFactor: 1,
      },
      devPreviewEnabled: false,
      queryRefreshConfig: {
        summary: {
          refetchIntervalMs: false,
          refetchOnMount: "always",
        },
        leaderboard: {
          refetchIntervalMs: false,
          refetchOnMount: "always",
        },
      },
    });
  });

  it("refreshes once when the window returns to the foreground", () => {
    const refresh = vi.fn();
    mockDataModel({ refresh });

    render(<HomeTodayProviderUsageOverview />);

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it("marks running providers that are already inside the default top three", () => {
    mockDataModel();

    render(
      <HomeTodayProviderUsageOverview activeSessions={[createActiveSession("Claude Main")]} />
    );

    const claudeRow = screen.getByText("Claude Main").closest("tr");
    expect(claudeRow).toBeTruthy();
    expect(within(claudeRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(screen.getByText("OpenAI Primary")).toBeInTheDocument();
  });

  it("replaces lower-ranked rows when a running provider is outside the default top three", () => {
    mockDataModel();

    render(
      <HomeTodayProviderUsageOverview activeSessions={[createActiveSession("DeepSeek Relay")]} />
    );

    const deepseekRow = screen.getByText("DeepSeek Relay").closest("tr");
    expect(deepseekRow).toBeTruthy();
    expect(screen.queryByText("OpenAI Primary")).not.toBeInTheDocument();
    expect(within(deepseekRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(rowCellTexts(deepseekRow as HTMLElement)).toEqual([
      "DeepSeek Relay",
      "3.5K",
      "2.0K",
      "27.6%",
      "100.0%",
      "12s",
      "—",
    ]);
  });

  it("keeps provider rows sorted by requests and name when token totals are tied", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "provider-alpha",
          name: "Alpha Relay",
          requests_total: 2,
          requests_success: 2,
          io_total_tokens: 5_000,
          total_tokens: 6_000,
        }),
        createLeaderboardRow({
          key: "provider-gamma",
          name: "Gamma Relay",
          requests_total: 5,
          requests_success: 5,
          io_total_tokens: 5_000,
          total_tokens: 6_500,
        }),
        createLeaderboardRow({
          key: "provider-beta",
          name: "Beta Relay",
          requests_total: 5,
          requests_success: 5,
          io_total_tokens: 5_000,
          total_tokens: 7_000,
        }),
      ],
    });

    render(<HomeTodayProviderUsageOverview />);

    const usageTable = screen.getByRole("table", { name: "今日供应商用量" });
    const providerNames = within(usageTable)
      .getAllByRole("row")
      .slice(1)
      .map((row) => within(row).getAllByRole("cell")[0]?.textContent?.trim());
    expect(providerNames).toEqual(["Beta Relay", "Gamma Relay", "Alpha Relay"]);
  });

  it("renders a synthetic running row when the provider has no usage row today", () => {
    mockDataModel();

    render(
      <HomeTodayProviderUsageOverview activeSessions={[createActiveSession("Runtime Fresh")]} />
    );

    const runtimeRow = screen.getByText("claude/Runtime Fresh").closest("tr");
    expect(runtimeRow).toBeTruthy();
    expect(screen.queryByText("OpenAI Primary")).not.toBeInTheDocument();
    expect(within(runtimeRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(rowCellTexts(runtimeRow as HTMLElement)).toEqual([
      "claude/Runtime Fresh",
      "—",
      "—",
      "—",
      "—",
      "—",
      "—",
    ]);
  });

  it("skips unnamed active sessions and keeps already-prefixed provider names", () => {
    mockDataModel({ rows: [] });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[
          createActiveSession("", { providerId: 2, cliKey: "claude" }),
          createActiveSession("claude/Runtime Fresh", { providerId: 3, cliKey: "claude" }),
          createActiveSession("claude/Runtime Fresh", { providerId: 3, cliKey: "claude" }),
        ]}
      />
    );

    expect(screen.getByText("claude/Runtime Fresh")).toBeInTheDocument();
    expect(screen.queryByText("未知")).not.toBeInTheDocument();
    expect(screen.queryByText("claude/claude/Runtime Fresh")).not.toBeInTheDocument();
    expect(screen.getAllByLabelText("进行中")).toHaveLength(1);
  });

  it("keeps synthetic provider names unprefixed while preview data is active", () => {
    mockDataModel({ rows: [], previewActive: true });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[createActiveSession("Runtime Fresh", { providerId: 5, cliKey: "claude" })]}
      />
    );

    expect(screen.getByText("Runtime Fresh")).toBeInTheDocument();
    expect(screen.queryByText("claude/Runtime Fresh")).not.toBeInTheDocument();
    expect(screen.getByLabelText("进行中")).toBeInTheDocument();
  });

  it("limits synthetic active session rows to the first three providers", () => {
    mockDataModel({ rows: [] });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[
          createActiveSession("Runtime One", { providerId: 11, cliKey: "claude" }),
          createActiveSession("Runtime Two", { providerId: 12, cliKey: "claude" }),
          createActiveSession("Runtime Three", { providerId: 13, cliKey: "claude" }),
          createActiveSession("Runtime Four", { providerId: 14, cliKey: "claude" }),
        ]}
      />
    );

    expect(screen.getByText("claude/Runtime One")).toBeInTheDocument();
    expect(screen.getByText("claude/Runtime Two")).toBeInTheDocument();
    expect(screen.getByText("claude/Runtime Three")).toBeInTheDocument();
    expect(screen.queryByText("claude/Runtime Four")).not.toBeInTheDocument();
    expect(screen.getAllByLabelText("进行中")).toHaveLength(3);
  });

  it("keeps backend active sessions as the primary running-provider source when traces are empty", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "claude:5",
          name: "claude/Backend Active",
        }),
      ],
    });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[
          createActiveSession("Backend Active", { providerId: 5, cliKey: "claude" }),
        ]}
        requestLogs={[]}
        traces={[]}
      />
    );

    const providerRow = screen.getByText("claude/Backend Active").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
  });

  it("matches active sessions by scoped or unscoped names when provider ids are absent", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "claude:0",
          name: "claude/Zero Id",
          total_tokens: 8_000,
          io_total_tokens: 7_000,
        }),
        createLeaderboardRow({
          key: "provider-name-only",
          name: "Name Only",
          total_tokens: 6_000,
          io_total_tokens: 5_000,
        }),
      ],
    });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[
          createActiveSession("Zero Id", { providerId: 0, cliKey: "claude" }),
          createActiveSession("Name Only", { providerId: 0, cliKey: " " }),
          createActiveSession("New Name Only", { providerId: 0, cliKey: " " }),
        ]}
      />
    );

    expect(screen.getByText("claude/Zero Id").closest("tr")).toHaveTextContent("8.0K");
    expect(screen.getByText("Name Only").closest("tr")).toHaveTextContent("6.0K");
    expect(screen.getByText("New Name Only")).toBeInTheDocument();
    expect(screen.getAllByLabelText("进行中")).toHaveLength(3);
  });

  it("keeps registry-backed traces while ignoring completed, unnamed, unknown, and duplicates", () => {
    mockDataModel({ rows: [] });
    const now = Date.now();
    const completedTrace = {
      ...createRunningTrace("Completed Trace", { providerId: 21, traceId: "completed" }),
      summary: {} as NonNullable<TraceSession["summary"]>,
    };
    const oldTrace = {
      ...createRunningTrace("Old Trace", { providerId: 22, traceId: "old" }),
      first_seen_ms: now - 16 * 60 * 1000,
      last_seen_ms: now,
    };
    const staleTrace = {
      ...createRunningTrace("Stale Trace", { providerId: 23, traceId: "stale" }),
      first_seen_ms: now - 60 * 1000,
      last_seen_ms: now - 6 * 60 * 1000,
    };
    const unnamedTrace = {
      ...createRunningTrace("Unnamed Trace", { providerId: 24, traceId: "unnamed" }),
      attempts: [],
    };
    const missingAttemptsTrace = {
      ...createRunningTrace("Missing Attempts Trace", {
        providerId: 25,
        traceId: "missing-attempts",
      }),
      attempts: undefined,
    } as unknown as TraceSession;
    const unknownTrace = createRunningTrace("Unknown", {
      providerId: 26,
      traceId: "unknown",
    });
    const emptyScopedNameTrace = createRunningTrace("claude/", {
      providerId: 27,
      traceId: "empty-scoped-name",
    });
    const liveTrace = createRunningTrace("Trace Fresh", {
      providerId: 28,
      traceId: "live",
    });
    const duplicateTrace = createRunningTrace("Trace Fresh", {
      providerId: 28,
      traceId: "duplicate",
    });
    const activeTraces = [
      oldTrace,
      staleTrace,
      unnamedTrace,
      missingAttemptsTrace,
      unknownTrace,
      emptyScopedNameTrace,
      liveTrace,
      duplicateTrace,
    ];

    render(
      <HomeTodayProviderUsageOverview
        traces={[completedTrace, ...activeTraces]}
        activeRequests={activeTraces.map(createActiveRequestFromTrace)}
      />
    );

    expect(screen.getByText("claude/Trace Fresh")).toBeInTheDocument();
    expect(screen.queryByText("claude/Completed Trace")).not.toBeInTheDocument();
    expect(screen.getByText("claude/Old Trace")).toBeInTheDocument();
    expect(screen.getByText("claude/Stale Trace")).toBeInTheDocument();
    expect(screen.queryByText("claude/Unknown")).not.toBeInTheDocument();
    expect(screen.queryByText("claude/Missing Attempts Trace")).not.toBeInTheDocument();
    expect(screen.getAllByLabelText("进行中")).toHaveLength(3);
  });

  it("matches active sessions by scoped name when provider id is invalid", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "claude:44",
          name: "claude/Runtime Fresh",
          total_tokens: 9_200,
          io_total_tokens: 8_000,
          input_tokens: 5_000,
          output_tokens: 3_000,
        }),
        createLeaderboardRow({
          key: "provider-other",
          name: "Other Relay",
          total_tokens: 7_000,
          io_total_tokens: 6_000,
        }),
      ],
    });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[createActiveSession("Runtime Fresh", { providerId: 0, cliKey: "claude" })]}
      />
    );

    const providerRow = screen.getByText("claude/Runtime Fresh").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(rowCellTexts(providerRow as HTMLElement)).toEqual([
      "claude/Runtime Fresh",
      "9.2K",
      "8.0K",
      "1.9%",
      "100.0%",
      "<1s",
      "$0.10",
    ]);
  });

  it("shows dashes for provider rates when a row has no requests or cache denominator", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "provider-zero",
          name: "Zero Request Relay",
          requests_total: 0,
          requests_success: 0,
          requests_failed: 0,
          total_tokens: 1_200,
          io_total_tokens: 1_000,
          input_tokens: 0,
          output_tokens: 1_000,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          total_duration_ms: 0,
          cost_usd: null,
        }),
      ],
    });

    render(<HomeTodayProviderUsageOverview />);

    const providerRow = screen.getByText("Zero Request Relay").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(rowCellTexts(providerRow as HTMLElement)).toEqual([
      "Zero Request Relay",
      "1.2K",
      "1.0K",
      "—",
      "—",
      "0s",
      "—",
    ]);
  });

  it("matches a running provider to the prefixed usage row by provider id", () => {
    mockDataModel({
      rows: [
        {
          key: "codex:88",
          name: "codex/鹿森",
          folder_path: null,
          requests_total: 9,
          requests_success: 9,
          requests_failed: 0,
          total_tokens: 12_000,
          io_total_tokens: 10_000,
          input_tokens: 6_000,
          output_tokens: 4_000,
          cache_creation_input_tokens: 700,
          cache_read_input_tokens: 1_300,
          total_duration_ms: 7_380,
          first_request_created_at_ms: null,
          last_request_created_at_ms: null,
          last_request_completed_at_ms: null,
          estimated_development_time_ms: null,
          hourly_estimated_development_time_ms: null,
          avg_duration_ms: 820,
          avg_ttfb_ms: 210,
          avg_output_tokens_per_second: 108,
          cost_usd: 0.88,
        },
      ],
    });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[createActiveSession("鹿森", { providerId: 88, cliKey: "codex" })]}
      />
    );

    const providerRow = screen.getByText("codex/鹿森").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(screen.queryByText("codex/codex/鹿森")).not.toBeInTheDocument();
  });

  it("does not mark same-name providers across CLI when the live trace provider id is missing", () => {
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "claude:21",
          name: "claude/Shared Relay",
          total_tokens: 10_000,
          io_total_tokens: 9_000,
          input_tokens: 5_000,
          output_tokens: 4_000,
        }),
        createLeaderboardRow({
          key: "codex:22",
          name: "codex/Shared Relay",
          total_tokens: 9_000,
          io_total_tokens: 8_000,
          input_tokens: 4_500,
          output_tokens: 3_500,
        }),
        createLeaderboardRow({
          key: "gemini:23",
          name: "gemini/Other Relay",
          total_tokens: 2_000,
          io_total_tokens: 1_500,
          input_tokens: 1_000,
          output_tokens: 500,
        }),
      ],
    });

    const runningTrace = createRunningTrace("Shared Relay", { providerId: 0, cliKey: "codex" });
    render(
      <HomeTodayProviderUsageOverview
        traces={[runningTrace]}
        activeRequests={[createActiveRequestFromTrace(runningTrace)]}
      />
    );

    const claudeRow = screen.getByText("claude/Shared Relay").closest("tr");
    const codexRow = screen.getByText("codex/Shared Relay").closest("tr");
    expect(claudeRow).toBeTruthy();
    expect(codexRow).toBeTruthy();
    expect(within(codexRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).queryByLabelText("进行中")).not.toBeInTheDocument();
  });

  it("does not keep showing a trace-derived running badge when traces are gone", () => {
    mockDataModel({
      rows: [
        {
          key: "claude:1",
          name: "Claude Main",
          folder_path: null,
          requests_total: 5,
          requests_success: 5,
          requests_failed: 0,
          total_tokens: 6_200,
          io_total_tokens: 5_000,
          input_tokens: 3_000,
          output_tokens: 2_000,
          cache_creation_input_tokens: 500,
          cache_read_input_tokens: 700,
          total_duration_ms: 4_500,
          first_request_created_at_ms: null,
          last_request_created_at_ms: null,
          last_request_completed_at_ms: null,
          estimated_development_time_ms: null,
          hourly_estimated_development_time_ms: null,
          avg_duration_ms: 900,
          avg_ttfb_ms: 220,
          avg_output_tokens_per_second: 90,
          cost_usd: 0.5,
        },
      ],
    });

    render(<HomeTodayProviderUsageOverview traces={[]} />);

    const providerRow = screen.getByText("Claude Main").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).queryByLabelText("进行中")).not.toBeInTheDocument();
  });

  it("marks running providers from live traces when traces are present", () => {
    mockDataModel({
      rows: [
        {
          key: "claude:1",
          name: "Claude Main",
          folder_path: null,
          requests_total: 5,
          requests_success: 5,
          requests_failed: 0,
          total_tokens: 6_200,
          io_total_tokens: 5_000,
          input_tokens: 3_000,
          output_tokens: 2_000,
          cache_creation_input_tokens: 500,
          cache_read_input_tokens: 700,
          total_duration_ms: 4_500,
          first_request_created_at_ms: null,
          last_request_created_at_ms: null,
          last_request_completed_at_ms: null,
          estimated_development_time_ms: null,
          hourly_estimated_development_time_ms: null,
          avg_duration_ms: 900,
          avg_ttfb_ms: 220,
          avg_output_tokens_per_second: 90,
          cost_usd: 0.5,
        },
      ],
    });

    const runningTrace = createRunningTrace("Claude Main");
    render(
      <HomeTodayProviderUsageOverview
        traces={[runningTrace]}
        activeRequests={[createActiveRequestFromTrace(runningTrace)]}
      />
    );

    const providerRow = screen.getByText("Claude Main").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
  });

  it("does not start a periodic clock for registry-backed provider hints", () => {
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    mockDataModel({ rows: [] });
    const runningTrace = createRunningTrace("Clockless Provider");

    render(
      <HomeTodayProviderUsageOverview
        traces={[runningTrace]}
        activeRequests={[createActiveRequestFromTrace(runningTrace)]}
      />
    );

    expect(screen.getByText("claude/Clockless Provider")).toBeInTheDocument();
    expect(setIntervalSpy).not.toHaveBeenCalled();
    setIntervalSpy.mockRestore();
  });

  it("keeps running provider hints for long pending traces backed by the active registry", () => {
    vi.useFakeTimers();
    const baseTime = 1_700_000_000_000;
    vi.setSystemTime(baseTime);
    mockDataModel({
      rows: [
        createLeaderboardRow({
          key: "claude:1",
          name: "Claude Main",
        }),
      ],
    });

    const longTrace = createRunningTrace("Claude Main", { providerId: 1 });
    longTrace.first_seen_ms = baseTime - 11 * 60 * 1000;
    longTrace.last_seen_ms = baseTime - 6 * 60 * 1000;

    render(
      <HomeTodayProviderUsageOverview
        traces={[longTrace]}
        requestLogs={[
          createRequestLog({
            trace_id: longTrace.trace_id,
            created_at_ms: baseTime - 11 * 60 * 1000,
            created_at: Math.floor((baseTime - 11 * 60 * 1000) / 1000),
          }),
        ]}
        activeRequests={[
          {
            trace_id: longTrace.trace_id,
            cli_key: "claude",
            session_id: null,
            method: "POST",
            path: "/v1/messages",
            query: null,
            requested_model: "claude-3-opus",
            created_at_ms: baseTime - 11 * 60 * 1000,
            last_activity_ms: baseTime - 6 * 60 * 1000,
            current_attempt: null,
          },
        ]}
      />
    );

    const providerRow = screen.getByText("Claude Main").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();

    vi.useRealTimers();
  });

  it("shows a dash when the activity range has no request timestamps", () => {
    mockDataModel({
      summary: {
        requests_total: 0,
        requests_with_usage: 0,
        requests_success: 0,
        requests_failed: 0,
        cost_covered_success: 0,
        total_duration_ms: 0,
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
      },
      rows: [],
    });
    mockDayLeaderboardRows([
      {
        key: "2026-04-16",
        first_request_created_at_ms: null,
        last_request_completed_at_ms: null,
        estimated_development_time_ms: 0,
      },
    ]);

    render(<HomeTodayProviderUsageOverview />);

    const activityRangeCard = screen.getByText("活动范围").parentElement;
    expect(activityRangeCard).toBeTruthy();
    expect(within(activityRangeCard as HTMLElement).getByText("—")).toBeInTheDocument();
    expect(screen.getByText("总花费")).toBeInTheDocument();
  });

  it("renders the error card and retries refresh when loading failed", () => {
    const refresh = vi.fn();
    mockDataModel({
      summary: null,
      rows: [],
      errorText: "boom",
      refresh,
    });

    render(<HomeTodayProviderUsageOverview />);

    expect(screen.getByText("加载失败")).toBeInTheDocument();
    expect(
      screen.getByText("读取今日供应商用量失败，请重试；必要时查看 Console 日志。")
    ).toBeInTheDocument();
    expect(screen.getByText("今日暂无供应商用量数据。")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it("renders loading skeletons before data arrives", () => {
    mockDataModel({
      summary: null,
      rows: [],
      loading: true,
    });

    const { container } = render(<HomeTodayProviderUsageOverview />);

    expect(container.querySelectorAll(".animate-pulse").length).toBeGreaterThan(0);
    expect(screen.queryByText("今日暂无供应商用量数据。")).not.toBeInTheDocument();
  });
});
