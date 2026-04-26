import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HomeTodayProviderUsageOverview } from "../HomeTodayProviderUsageOverview";
import { useHomeTokenCostDataModel } from "../useHomeTokenCostDataModel";
import type { TraceSession } from "../../../services/gateway/traceStore";
import type { UsageLeaderboardRow } from "../../../services/usage/usage";

vi.mock("../useHomeTokenCostDataModel", () => ({
  useHomeTokenCostDataModel: vi.fn(),
}));

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
        provider_name: providerName,
        base_url: "https://example.com",
        outcome: "started",
        status: null,
        attempt_started_ms: now - 3_000,
        attempt_duration_ms: 3_000,
      },
    ],
  };
}

function createLeaderboardRow(
  overrides: Pick<UsageLeaderboardRow, "key" | "name"> &
    Partial<Omit<UsageLeaderboardRow, "key" | "name">>
): UsageLeaderboardRow {
  const { key, name, ...rest } = overrides;
  return {
    key,
    name,
    requests_total: 1,
    requests_success: 1,
    requests_failed: 0,
    total_tokens: 1_200,
    io_total_tokens: 1_000,
    input_tokens: 700,
    output_tokens: 300,
    cache_creation_input_tokens: 100,
    cache_read_input_tokens: 100,
    avg_duration_ms: 900,
    avg_ttfb_ms: 200,
    avg_output_tokens_per_second: 90,
    cost_usd: 0.1,
    ...rest,
  };
}

function mockDataModel(overrides: Partial<ReturnType<typeof useHomeTokenCostDataModel>> = {}) {
  vi.mocked(useHomeTokenCostDataModel).mockReturnValue({
    summary: {
      requests_total: 20,
      requests_with_usage: 20,
      requests_success: 18,
      requests_failed: 2,
      cost_covered_success: 18,
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

describe("components/home/HomeTodayProviderUsageOverview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
  });

  it("uses the fixed today provider query config and renders summary plus top providers", () => {
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

    const totalWithCacheCard = screen.getByText("含缓存总 Token").parentElement;
    const inputOutputTokenCard = screen.getAllByText("输入+输出 Token")[0]?.parentElement;
    const cacheHitRateCard = screen.getByText("缓存命中率").parentElement;
    expect(totalWithCacheCard).toBeTruthy();
    expect(inputOutputTokenCard).toBeTruthy();
    expect(cacheHitRateCard).toBeTruthy();
    expect(within(totalWithCacheCard as HTMLElement).getByText("25.0K")).toBeInTheDocument();
    expect(within(inputOutputTokenCard as HTMLElement).getByText("20.0K")).toBeInTheDocument();
    expect(within(cacheHitRateCard as HTMLElement).getByText("18.8%")).toBeInTheDocument();
    expect(screen.getByText("今日请求数")).toBeInTheDocument();
    expect(screen.getByText("20")).toBeInTheDocument();
    expect(screen.getByText("今日花费")).toBeInTheDocument();
    expect(screen.getByText("$2.21")).toBeInTheDocument();
    const providerHeader = screen.getByText("供应商").closest("th");
    const inputOutputTokenHeader = screen.getByRole("columnheader", {
      name: /输入\+输出 Token/,
    });
    const cacheHeader = screen.getByText("缓存情况").closest("th");
    expect(providerHeader).toBeTruthy();
    expect(inputOutputTokenHeader).toBeTruthy();
    expect(cacheHeader).toBeTruthy();
    expect(within(providerHeader as HTMLElement).getByText("（前 3 个）")).toBeInTheDocument();
    expect(
      within(cacheHeader as HTMLElement).getByText("（含缓存/缓存/命中率）")
    ).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "成功率" })).toBeInTheDocument();
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

    expect(within(geminiRow as HTMLElement).getByLabelText("8.0K/40.0%")).toBeInTheDocument();
    expect(within(geminiRow as HTMLElement).getByLabelText("10.2K/2.2K/20.9%")).toBeInTheDocument();
    expect(within(geminiRow as HTMLElement).getByText("$0.90")).toBeInTheDocument();
    expect(within(geminiRow as HTMLElement).getByText("85.7%")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).getByText("100.0%")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).getByLabelText("5.0K/25.0%")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).getByLabelText("6.2K/1.2K/16.7%")).toBeInTheDocument();
    expect(within(openaiRow as HTMLElement).getByLabelText("4.0K/20.0%")).toBeInTheDocument();
    expect(within(openaiRow as HTMLElement).getByLabelText("5.8K/1.8K/31.6%")).toBeInTheDocument();
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
    expect(
      within(deepseekRow as HTMLElement).getByLabelText("3.5K/1.5K/27.6%")
    ).toBeInTheDocument();
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
    expect(within(runtimeRow as HTMLElement).getByLabelText("—/—")).toBeInTheDocument();
    expect(within(runtimeRow as HTMLElement).getByLabelText("—/—/—")).toBeInTheDocument();
    expect(within(runtimeRow as HTMLElement).getAllByText("—").length).toBeGreaterThanOrEqual(2);
  });

  it("matches a running provider to the prefixed usage row by provider id", () => {
    mockDataModel({
      rows: [
        {
          key: "codex:88",
          name: "codex/鹿森",
          requests_total: 9,
          requests_success: 9,
          requests_failed: 0,
          total_tokens: 12_000,
          io_total_tokens: 10_000,
          input_tokens: 6_000,
          output_tokens: 4_000,
          cache_creation_input_tokens: 700,
          cache_read_input_tokens: 1_300,
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

    render(
      <HomeTodayProviderUsageOverview
        traces={[createRunningTrace("Shared Relay", { providerId: 0, cliKey: "codex" })]}
      />
    );

    const claudeRow = screen.getByText("claude/Shared Relay").closest("tr");
    const codexRow = screen.getByText("codex/Shared Relay").closest("tr");
    expect(claudeRow).toBeTruthy();
    expect(codexRow).toBeTruthy();
    expect(within(codexRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
    expect(within(claudeRow as HTMLElement).queryByLabelText("进行中")).not.toBeInTheDocument();
  });

  it("does not keep showing a running badge when traces are gone", () => {
    mockDataModel({
      rows: [
        {
          key: "claude:1",
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
          avg_duration_ms: 900,
          avg_ttfb_ms: 220,
          avg_output_tokens_per_second: 90,
          cost_usd: 0.5,
        },
      ],
    });

    render(
      <HomeTodayProviderUsageOverview
        activeSessions={[createActiveSession("Claude Main", { providerId: 1, cliKey: "claude" })]}
        traces={[]}
      />
    );

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
          requests_total: 5,
          requests_success: 5,
          requests_failed: 0,
          total_tokens: 6_200,
          io_total_tokens: 5_000,
          input_tokens: 3_000,
          output_tokens: 2_000,
          cache_creation_input_tokens: 500,
          cache_read_input_tokens: 700,
          avg_duration_ms: 900,
          avg_ttfb_ms: 220,
          avg_output_tokens_per_second: 90,
          cost_usd: 0.5,
        },
      ],
    });

    render(<HomeTodayProviderUsageOverview traces={[createRunningTrace("Claude Main")]} />);

    const providerRow = screen.getByText("Claude Main").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByLabelText("进行中")).toBeInTheDocument();
  });

  it("shows a dash for cache hit rate when the summary has no denominator", () => {
    mockDataModel({
      summary: {
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
      },
      rows: [],
    });

    render(<HomeTodayProviderUsageOverview />);

    expect(screen.getByText("缓存命中率")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.getByText("今日花费")).toBeInTheDocument();
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
