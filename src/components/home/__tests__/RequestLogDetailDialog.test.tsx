import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RequestAttemptLog, RequestLogDetail } from "../../../services/requestLogs";
import type { TraceSession } from "../../../services/traceStore";
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

vi.mock("../../../services/traceStore", () => ({
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

  it("renders metrics first and hides raw trace/query details", () => {
    setRequestLogQueryState({ selectedLog: createSelectedLog() });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("代理记录详情")).toBeInTheDocument();
    expect(screen.getByText("关键指标")).toBeInTheDocument();
    expect(screen.getByText("输入 Token")).toBeInTheDocument();
    expect(screen.getByText("输出 Token")).toBeInTheDocument();
    expect(screen.getByText("缓存创建")).toBeInTheDocument();
    expect(screen.getByText("缓存读取")).toBeInTheDocument();
    expect(screen.getByText("总耗时")).toBeInTheDocument();
    expect(screen.getByText("TTFB")).toBeInTheDocument();
    expect(screen.getByText("速率")).toBeInTheDocument();
    expect(screen.getByText("花费")).toBeInTheDocument();

    expect(screen.queryByText(/请求追踪 ID/)).not.toBeInTheDocument();
    expect(screen.queryByText(/查询参数/)).not.toBeInTheDocument();
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

  it("hides metrics when no token or timing fields exist and falls back to unknown provider", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: null,
        error_code: null,
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
    expect(screen.getByText("当前供应商：未知")).toBeInTheDocument();
    expect(screen.getByText("决策链")).toBeInTheDocument();
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

    expect(screen.queryByText("错误详情")).not.toBeInTheDocument();
  });

  it("renders structured error observation fields from error_details_json", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: 502,
        error_code: "GW_UPSTREAM_ALL_FAILED",
        error_details_json: JSON.stringify({
          gateway_error_code: "GW_UPSTREAM_ALL_FAILED",
          error_code: "GW_UPSTREAM_5XX",
          error_category: "PROVIDER_ERROR",
          upstream_status: 502,
          provider_id: 12,
          provider_name: "Alpha",
          decision: "switch",
          selection_method: "ordered",
          provider_index: 2,
          retry_index: 3,
          reason_code: "retry_failed",
          matched_rule: "bad_gateway",
          reason: "status=502, rule=bad_gateway",
          upstream_body_preview: '{"error":"boom"}',
          attempt_duration_ms: 88,
          circuit_state_before: "closed",
          circuit_state_after: "open",
          circuit_failure_count: 3,
          circuit_failure_threshold: 3,
        }),
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("错误详情")).toBeInTheDocument();
    expect(screen.getByText("尝试错误码")).toBeInTheDocument();
    expect(screen.getByText("网关错误码")).toBeInTheDocument();
    expect(screen.getByText("GW_UPSTREAM_5XX")).toBeInTheDocument();
    expect(screen.getByText("GW_UPSTREAM_ALL_FAILED")).toBeInTheDocument();
    expect(screen.getByText("命中供应商")).toBeInTheDocument();
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("调度决策")).toBeInTheDocument();
    expect(screen.getByText("切换供应商")).toBeInTheDocument();
    expect(screen.getByText("选择方式")).toBeInTheDocument();
    expect(screen.getByText("顺序选择")).toBeInTheDocument();
    expect(screen.getByText("尝试位置")).toBeInTheDocument();
    expect(screen.getByText("供应商 2 / 重试 3")).toBeInTheDocument();
    expect(screen.getByText("原因标签")).toBeInTheDocument();
    expect(screen.getByText("retry_failed")).toBeInTheDocument();
    expect(screen.getByText("匹配规则")).toBeInTheDocument();
    expect(screen.getByText("bad_gateway")).toBeInTheDocument();
    expect(screen.getByText("原因")).toBeInTheDocument();
    expect(screen.getByText("status=502, rule=bad_gateway")).toBeInTheDocument();
    expect(screen.getByText("上游返回")).toBeInTheDocument();
    expect(screen.getByText('{"error":"boom"}')).toBeInTheDocument();
    expect(screen.getByText("熔断状态")).toBeInTheDocument();
    expect(screen.getByText("closed → open")).toBeInTheDocument();
    expect(screen.getByText("失败计数")).toBeInTheDocument();
    expect(screen.getByText("3 / 3")).toBeInTheDocument();
  });

  it("uses live trace provider and elapsed duration for in-progress logs", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: null,
        error_code: null,
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

    expect(screen.getByText("当前供应商：Provider Live")).toBeInTheDocument();
    expectMetricValue("总耗时", "6.50s");

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expectMetricValue("总耗时", "7.50s");
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
});
