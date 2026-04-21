import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RequestAttemptLog, RequestLogDetail } from "../../../services/gateway/requestLogs";
import type { TraceSession } from "../../../services/gateway/traceStore";
import { RequestLogDetailDialog } from "../RequestLogDetailDialog";

const requestLogQueryState = vi.hoisted(() => ({
  selectedLog: null as RequestLogDetail | null,
  selectedLogLoading: false,
  attemptLogs: [] as RequestAttemptLog[],
  attemptLogsLoading: false,
}));

const traceStoreState = vi.hoisted(() => ({
  traces: [] as TraceSession[],
}));

vi.mock("../../../query/requestLogs", () => ({
  useRequestLogDetailQuery: () => ({
    data: requestLogQueryState.selectedLog,
    isFetching: requestLogQueryState.selectedLogLoading,
  }),
  useRequestAttemptLogsByTraceIdQuery: () => ({
    data: requestLogQueryState.attemptLogs,
    isFetching: requestLogQueryState.attemptLogsLoading,
  }),
}));

vi.mock("../../../services/gateway/traceStore", () => ({
  useTraceStore: () => ({
    traces: traceStoreState.traces,
  }),
}));

function createSelectedLog(overrides: Partial<RequestLogDetail> = {}): RequestLogDetail {
  return {
    id: 1,
    trace_id: "trace-1",
    cli_key: "claude",
    method: "post",
    path: "/v1/messages",
    query: "hello",
    excluded_from_stats: false,
    special_settings_json: null,
    status: 499,
    error_code: "GW_STREAM_ABORTED",
    duration_ms: 1234,
    ttfb_ms: 100,
    attempts_json: "[]",
    input_tokens: 10,
    output_tokens: 20,
    total_tokens: 30,
    cache_read_input_tokens: 5,
    cache_creation_input_tokens: 2,
    cache_creation_5m_input_tokens: 1,
    cache_creation_1h_input_tokens: null,
    usage_json: JSON.stringify({ input_tokens: 10, cache_creation_1h_input_tokens: 999 }),
    requested_model: "claude-3",
    final_provider_id: 12,
    final_provider_name: "Claude Bridge",
    final_provider_source_id: 7,
    final_provider_source_name: "OpenAI Primary",
    cost_usd: 0.12,
    cost_multiplier: 1.25,
    created_at_ms: null,
    created_at: 1000,
    ...overrides,
  };
}

function setRequestLogQueryState(overrides: Partial<typeof requestLogQueryState> = {}) {
  requestLogQueryState.selectedLog = overrides.selectedLog ?? null;
  requestLogQueryState.selectedLogLoading = overrides.selectedLogLoading ?? false;
  requestLogQueryState.attemptLogs = overrides.attemptLogs ?? [];
  requestLogQueryState.attemptLogsLoading = overrides.attemptLogsLoading ?? false;
}

function setTraceStoreState(overrides: Partial<typeof traceStoreState> = {}) {
  traceStoreState.traces = overrides.traces ?? [];
}

function expectMetricValue(label: string, value: string) {
  const labelNode = screen.getByText(label);
  const card = labelNode.parentElement as HTMLElement | null;
  expect(card).not.toBeNull();
  expect(within(card as HTMLElement).getByText(value)).toBeInTheDocument();
}

function switchToTab(label: string) {
  fireEvent.click(screen.getByRole("tab", { name: label }));
}

describe("home/RequestLogDetailDialog", () => {
  afterEach(() => {
    setRequestLogQueryState();
    setTraceStoreState();
    vi.useRealTimers();
  });

  it("renders loading state and closes via dialog close button", async () => {
    const onSelectLogId = vi.fn();
    setRequestLogQueryState({ selectedLogLoading: true });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={onSelectLogId} />);

    expect(screen.getByText("加载中…")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("关闭"));
    await waitFor(() => {
      expect(onSelectLogId).toHaveBeenCalledWith(null);
    });
  });

  it("renders metrics on the summary tab", () => {
    setRequestLogQueryState({ selectedLog: createSelectedLog() });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("代理记录详情")).toBeInTheDocument();
    // Summary tab should be active by default
    expect(screen.getByText("关键指标")).toBeInTheDocument();
    expect(screen.getByText("输入 Token")).toBeInTheDocument();
    expect(screen.getByText("输出 Token")).toBeInTheDocument();
    expect(screen.getByText("缓存创建")).toBeInTheDocument();
    expect(screen.getByText("缓存读取")).toBeInTheDocument();
    expect(screen.getByText("总耗时")).toBeInTheDocument();
    expect(screen.getByText("TTFB")).toBeInTheDocument();
    expect(screen.getByText("速率")).toBeInTheDocument();
    expect(screen.getByText("花费")).toBeInTheDocument();

    // Raw data not visible on summary tab
    expect(screen.queryByText(/usage_json/)).not.toBeInTheDocument();
  });

  it("falls back to raw usage_json when JSON parsing fails without rendering raw json section", () => {
    setRequestLogQueryState({ selectedLog: createSelectedLog({ usage_json: "not-json" }) });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.queryByText("not-json")).not.toBeInTheDocument();
    expect(screen.getByText("关键指标")).toBeInTheDocument();
  });

  it("shows audit semantics for excluded warmup-style records", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        excluded_from_stats: true,
        special_settings_json: JSON.stringify({ type: "warmup_intercept" }),
        final_provider_id: 0,
        final_provider_name: "Unknown",
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("审计语义")).toBeInTheDocument();
    expect(screen.getByText("Warmup")).toBeInTheDocument();
    expect(screen.getByText("不计统计")).toBeInTheDocument();
    expect(
      screen.getByText("Warmup 命中后由网关直接应答，仅保留审计记录，不进入统计。")
    ).toBeInTheDocument();
  });

  it("renders not-found state when the selected log detail is unavailable", () => {
    setRequestLogQueryState({ selectedLog: null, selectedLogLoading: false });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("未找到记录详情（可能已过期被留存策略清理）。")).toBeInTheDocument();
  });

  it("hides metrics when no token or timing fields exist", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        duration_ms: undefined,
        ttfb_ms: null,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        final_provider_id: null,
        final_provider_name: null,
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.queryByText("关键指标")).not.toBeInTheDocument();

    // Switch to chain tab to check provider fallback
    switchToTab("决策链");
    expect(screen.getByText("最终供应商：未知")).toBeInTheDocument();
  });

  it("shows failover success and prefers the 1h cache creation metric when present", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: 200,
        error_code: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: 8,
      }),
      attemptLogs: [
        {
          id: 1,
          trace_id: "trace-1",
          cli_key: "claude",
          attempt_index: 0,
          provider_id: 11,
          provider_name: "Alpha",
          base_url: "https://alpha.example.com",
          outcome: "failed",
          status: 502,
          attempt_started_ms: 100,
          attempt_duration_ms: 50,
          created_at: 1000,
        },
        {
          id: 2,
          trace_id: "trace-1",
          cli_key: "claude",
          attempt_index: 1,
          provider_id: 12,
          provider_name: "Beta",
          base_url: "https://beta.example.com",
          outcome: "succeeded",
          status: 200,
          attempt_started_ms: 200,
          attempt_duration_ms: 80,
          created_at: 1001,
        },
      ],
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("200 切换后成功")).toBeInTheDocument();
    expectMetricValue("缓存创建", "8 (1h)");
  });

  it("hides error observation for 200 success even when error_details_json exists", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: 200,
        error_code: null,
        error_details_json: JSON.stringify({
          error_code: "GW_UPSTREAM_5XX",
          error_category: "PROVIDER_ERROR",
          upstream_status: 502,
          decision: "switch",
        }),
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    // For 200 success, error observation should not produce a visible card
    // because resolveRequestLogErrorObservation returns null when status is OK and no error_code
    expect(screen.queryByText("上游服务返回服务端错误")).not.toBeInTheDocument();
  });

  it("renders error observation card on summary tab for failed requests", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: 502,
        error_code: "GW_UPSTREAM_ALL_FAILED",
        error_details_json: JSON.stringify({
          gateway_error_code: "GW_UPSTREAM_ALL_FAILED",
          error_code: "GW_UPSTREAM_5XX",
          error_category: "PROVIDER_ERROR",
          upstream_status: 502,
        }),
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    // Error observation card visible on summary tab with displayErrorCode = GW_UPSTREAM_5XX
    // (parsedJson.errorCode takes priority over gatewayErrorCode)
    expect(screen.getByText("上游服务返回服务端错误 (5xx)")).toBeInTheDocument();
    // The suggestion text should be present
    expect(
      screen.getByText(/Provider 内部错误/)
    ).toBeInTheDocument();
  });

  it("uses live trace provider and elapsed duration for in-progress logs", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        duration_ms: 0,
        final_provider_id: 0,
        final_provider_name: "Unknown",
      }),
    });
    setTraceStoreState({
      traces: [
        {
          trace_id: "trace-1",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
          query: null,
          requested_model: "claude-3",
          first_seen_ms: Date.now() - 6500,
          last_seen_ms: Date.now() - 100,
          attempts: [
            {
              trace_id: "trace-1",
              cli_key: "claude",
              method: "POST",
              path: "/v1/messages",
              query: null,
              requested_model: "claude-3",
              attempt_index: 0,
              provider_id: 42,
              session_reuse: false,
              provider_name: "Provider Live",
              base_url: "https://provider-live.example.com",
              outcome: "started",
              status: null,
              attempt_started_ms: 0,
              attempt_duration_ms: 0,
            },
          ],
        },
      ],
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    // Live duration is on the summary tab
    expectMetricValue("总耗时", "6.50s");

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expectMetricValue("总耗时", "7.50s");

    // Switch to chain tab to see live provider
    switchToTab("决策链");
    expect(screen.getByText("当前供应商：Provider Live")).toBeInTheDocument();
  });

  it("uses base cache creation tokens and falls back to dash for missing timing metrics", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        duration_ms: undefined,
        ttfb_ms: null,
        cache_creation_input_tokens: 2,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expectMetricValue("缓存创建", "2");
    expectMetricValue("TTFB", "—");
    expectMetricValue("速率", "—");
  });

  it("keeps zero-valued cache window metrics visible when they are the only cache source", () => {
    const view = render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: null,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expectMetricValue("缓存创建", "0 (5m)");

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: 0,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expectMetricValue("缓存创建", "0 (1h)");

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expectMetricValue("缓存创建", "—");
  });

  // --- Tab switching tests ---

  it("switches between tabs and shows correct content", () => {
    setRequestLogQueryState({ selectedLog: createSelectedLog() });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    // Summary tab active by default
    expect(screen.getByText("关键指标")).toBeInTheDocument();

    // Switch to chain tab
    switchToTab("决策链");
    expect(screen.queryByText("关键指标")).not.toBeInTheDocument();

    // Switch to raw tab
    switchToTab("原始数据");
    expect(screen.getByText("attempts_json")).toBeInTheDocument();

    // Switch back to summary
    switchToTab("概览");
    expect(screen.getByText("关键指标")).toBeInTheDocument();
  });

  it("shows raw error_details_json on raw tab when available", () => {
    const errorJson = { gateway_error_code: "GW_UPSTREAM_ALL_FAILED", upstream_status: 502 };
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        error_details_json: JSON.stringify(errorJson),
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    switchToTab("原始数据");
    expect(screen.getByText("error_details_json")).toBeInTheDocument();
    expect(screen.getByText(/GW_UPSTREAM_ALL_FAILED/)).toBeInTheDocument();
  });
});
