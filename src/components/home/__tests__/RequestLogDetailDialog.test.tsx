import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RequestAttemptLog, RequestLogDetail } from "../../../services/gateway/requestLogs";
import { createRequestLogDetail } from "../../../services/gateway/requestLogFixtures";
import type { TraceSession } from "../../../services/gateway/traceStore";
import { logToConsole } from "../../../services/consoleLog";
import { usePluginActiveContributionsQuery } from "../../../query/plugins";
import { RequestLogDetailDialog } from "../RequestLogDetailDialog";

const requestLogQueryState = vi.hoisted(() => ({
  selectedLog: null as RequestLogDetail | null,
  selectedLogLoading: false,
  selectedLogRefetch: (() => Promise.resolve(null)) as () => Promise<unknown>,
  attemptLogs: [] as RequestAttemptLog[],
  attemptLogsLoading: false,
  attemptLogsRefetch: (() => Promise.resolve(null)) as () => Promise<unknown>,
}));

const traceStoreState = vi.hoisted(() => ({
  traces: [] as TraceSession[],
}));

const gatewayEventState = vi.hoisted(() => ({
  requestSignalHandler: null as ((payload: unknown) => void) | null,
  unsubscribe: (() => undefined) as () => void,
}));

vi.mock("../../../query/requestLogs", () => ({
  useRequestLogDetailQuery: () => ({
    data: requestLogQueryState.selectedLog,
    isFetching: requestLogQueryState.selectedLogLoading,
    refetch: requestLogQueryState.selectedLogRefetch,
  }),
  useRequestAttemptLogsByTraceIdQuery: () => ({
    data: requestLogQueryState.attemptLogs,
    isFetching: requestLogQueryState.attemptLogsLoading,
    refetch: requestLogQueryState.attemptLogsRefetch,
  }),
}));

vi.mock("../../../services/gateway/gatewayEventBus", () => ({
  subscribeGatewayEvent: vi.fn((event: string, handler: (payload: unknown) => void) => {
    if (event === "gateway:request_signal") {
      gatewayEventState.requestSignalHandler = handler;
    }
    return {
      ready: Promise.resolve(),
      unsubscribe: () => {
        if (gatewayEventState.requestSignalHandler === handler) {
          gatewayEventState.requestSignalHandler = null;
        }
        gatewayEventState.unsubscribe();
      },
    };
  }),
}));

vi.mock("../../../services/gateway/traceStore", () => ({
  useTraceStore: () => ({
    traces: traceStoreState.traces,
  }),
}));

vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../query/plugins", () => ({
  usePluginActiveContributionsQuery: vi.fn(() => ({
    data: { ui: [] },
    isLoading: false,
    error: null,
  })),
}));

function createSelectedLog(overrides: Partial<RequestLogDetail> = {}): RequestLogDetail {
  const hasTimestampOverride = "created_at" in overrides || "created_at_ms" in overrides;
  return createRequestLogDetail({
    method: "post",
    query: "hello",
    status: 499,
    error_code: "GW_STREAM_ABORTED",
    duration_ms: 1234,
    ttfb_ms: 100,
    input_tokens: 10,
    effective_input_tokens: 10,
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
    ...(hasTimestampOverride ? {} : { created_at_ms: 1_000_000, created_at: 1000 }),
    ...overrides,
  });
}

function setRequestLogQueryState(overrides: Partial<typeof requestLogQueryState> = {}) {
  requestLogQueryState.selectedLog = overrides.selectedLog ?? null;
  requestLogQueryState.selectedLogLoading = overrides.selectedLogLoading ?? false;
  requestLogQueryState.selectedLogRefetch =
    overrides.selectedLogRefetch ?? (() => Promise.resolve(null));
  requestLogQueryState.attemptLogs = overrides.attemptLogs ?? [];
  requestLogQueryState.attemptLogsLoading = overrides.attemptLogsLoading ?? false;
  requestLogQueryState.attemptLogsRefetch =
    overrides.attemptLogsRefetch ?? (() => Promise.resolve(null));
}

function setTraceStoreState(overrides: Partial<typeof traceStoreState> = {}) {
  traceStoreState.traces = overrides.traces ?? [];
}

function createLiveTrace(traceId = "trace-1"): TraceSession {
  return {
    trace_id: traceId,
    cli_key: "claude",
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: "claude-3",
    first_seen_ms: Date.now() - 1000,
    last_seen_ms: Date.now(),
    attempts: [],
  };
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
    gatewayEventState.requestSignalHandler = null;
    gatewayEventState.unsubscribe = () => undefined;
    vi.mocked(usePluginActiveContributionsQuery).mockReturnValue({
      data: { ui: [] },
      isLoading: false,
      error: null,
    } as any);
    vi.mocked(logToConsole).mockReset();
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
    expectMetricValue("费用系数", "x1.25");

    // Raw data not visible on summary tab
    expect(screen.queryByText(/usage_json/)).not.toBeInTheDocument();
  });

  it("shows both transport hops, fallback, provider switches and recovery in the existing chain", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cli_key: "codex",
        final_provider_id: 12,
        final_provider_name: "Provider B",
        special_settings_json: JSON.stringify([
          {
            type: "codex_responses_transport",
            client_transport: "responses_ws",
            upstream_transport: "http",
            transport_action: "http_fallback",
            providerId: 7,
            failure_class: "transport",
            reason_code: "ws_connect_timeout",
            output_committed: false,
          },
          {
            type: "codex_responses_transport",
            client_transport: "responses_ws",
            upstream_transport: "http",
            transport_action: "provider_switch",
            providerId: 12,
            failure_class: "provider",
            reason_code: "upstream_429",
            status_source: "responses_event",
            handshake_status: 101,
            event_status: 429,
            output_committed: false,
          },
          {
            type: "codex_responses_transport",
            client_transport: "http",
            upstream_transport: "http",
            transport_action: "full_input_retry",
            providerId: 12,
            recovery_from_trace_id: "trace-original",
          },
        ]),
      }),
    });
    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    switchToTab("决策链");
    expect(screen.getByText("Responses 传输记录")).toBeInTheDocument();
    const fallback = screen.getByText("同供应商降级 HTTP").closest("li");
    expect(fallback).not.toBeNull();
    expect(within(fallback!).getByText("供应商：#7")).toBeInTheDocument();
    expect(within(fallback!).getByText("客户端 WS · 上游 HTTP")).toBeInTheDocument();
    expect(within(fallback!).getByText("传输失败")).toBeInTheDocument();
    expect(within(fallback!).getByText("原因：ws_connect_timeout")).toBeInTheDocument();
    expect(within(fallback!).getByText("尚未开始输出")).toBeInTheDocument();
    expect(screen.getByText("切换供应商")).toBeInTheDocument();
    expect(screen.getByText("WS 握手：101")).toBeInTheDocument();
    expect(screen.getByText("响应事件状态：429")).toBeInTheDocument();
    expect(screen.getAllByText("供应商：Provider B")).toHaveLength(2);
    expect(screen.getByText("恢复上下文并重发完整请求")).toBeInTheDocument();
    expect(screen.getByText("trace-original")).toBeInTheDocument();
    expect(screen.queryByText(/不能拼接新供应商响应/)).not.toBeInTheDocument();
  });

  it("shows committed output interruption without implying a seamless retry", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cli_key: "codex",
        special_settings_json: JSON.stringify({
          type: "codex_responses_transport",
          client_transport: "responses_ws",
          upstream_transport: "responses_ws",
          providerId: 12,
          failure_class: "transport",
          reason_code: "ws_stream_closed",
          output_committed: true,
        }),
      }),
    });
    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    switchToTab("决策链");
    expect(screen.getByText("客户端 WS · 上游 WS")).toBeInTheDocument();
    expect(screen.getByText("已开始输出")).toBeInTheDocument();
    expect(
      screen.getByText("响应中断，已输出内容保留，不能拼接新供应商响应。")
    ).toBeInTheDocument();
    expect(screen.queryByText("切换供应商")).not.toBeInTheDocument();
  });

  it.each([null, "bad-json", JSON.stringify([{ type: "other" }])])(
    "keeps transport details hidden for legacy logs (%s)",
    (specialSettings) => {
      setRequestLogQueryState({
        selectedLog: createSelectedLog({ special_settings_json: specialSettings }),
      });
      render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
      switchToTab("决策链");
      expect(screen.queryByText("Responses 传输记录")).not.toBeInTheDocument();
    }
  );

  it("shows Codex fast mode badge on the summary tab", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cli_key: "codex",
        special_settings_json: JSON.stringify([
          {
            type: "codex_service_tier_result",
            requestedServiceTier: "priority",
            actualServiceTier: "priority",
            billingSourcePreference: "actual",
            resolvedFrom: "actual",
            effectivePriority: true,
          },
        ]),
        cost_multiplier: 1.5,
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("fast")).toBeInTheDocument();
    expectMetricValue("费用系数", "x1.50");
  });

  it("does not promote the Codex system marker into request details", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cli_key: "codex",
        special_settings_json: JSON.stringify([
          { type: "codex_system_request", threadSource: "system" },
        ]),
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.queryByText("Codex 系统请求")).not.toBeInTheDocument();
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
        final_provider_id: 0,
        final_provider_name: "Unknown",
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.queryByText("关键指标")).not.toBeInTheDocument();

    // Switch to chain tab to check provider fallback
    switchToTab("决策链");
    expect(screen.getByText("最终供应商：未知")).toBeInTheDocument();
  });

  it.each([false, true])(
    "does not label same-provider transport fallback as failover (loaded=%s)",
    (loaded) => {
      const attempts: RequestAttemptLog[] = [101, 200].map((status, index) => ({
        id: index + 1,
        trace_id: "trace-1",
        cli_key: "codex",
        attempt_index: index + 1,
        provider_id: 12,
        provider_name: "Provider A",
        base_url: "https://provider.example",
        outcome: index === 0 ? "transport_fallback" : "success",
        status,
        attempt_started_ms: index * 100,
        attempt_duration_ms: 50,
        created_at: 1000,
      }));
      setRequestLogQueryState({
        selectedLog: createSelectedLog({
          cli_key: "codex",
          status: 200,
          error_code: null,
          attempts_json: JSON.stringify(attempts),
        }),
        attemptLogs: loaded ? attempts : [],
      });
      render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
      expect(screen.getByText("200 成功")).toBeInTheDocument();
      expect(screen.queryByText("200 切换后成功")).not.toBeInTheDocument();
    }
  );

  it("shows incomplete terminal status in both summary and transport details", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cli_key: "codex",
        status: 200,
        error_code: null,
        special_settings_json: JSON.stringify([
          {
            type: "codex_responses_transport",
            scope: "stream",
            terminal: "incomplete",
            client_transport: "responses_ws",
            upstream_transport: "http",
            output_committed: true,
          },
        ]),
      }),
    });
    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expect(screen.getByText("200 不完整结束")).toBeInTheDocument();
    expect(screen.queryByText("200 成功")).not.toBeInTheDocument();
    expectMetricValue("输出 Token", "20");
    switchToTab("决策链");
    expect(screen.getByText("终态：不完整结束")).toBeInTheDocument();
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
    expect(screen.getByText(/Provider 内部错误/)).toBeInTheDocument();
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
              session_id: null,
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
              circuit_state_before: null,
              circuit_state_after: null,
              circuit_failure_count: null,
              circuit_failure_threshold: null,
              claude_model_mapping: null,
              model_redirect: null,
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

  it.each([
    {
      label: "fake-200",
      status: 502,
      errorCode: "GW_FAKE_200",
      expectedBadge: "502 失败",
    },
    {
      label: "stream-abort",
      status: 499,
      errorCode: "GW_STREAM_ABORTED",
      expectedBadge: "499 已中断",
    },
  ])(
    "refreshes the selected in-progress detail when a $label terminal signal arrives",
    async ({ status, errorCode, expectedBadge }) => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));
      const traceId = "trace-terminal";
      const terminalLog = createSelectedLog({
        trace_id: traceId,
        status,
        error_code: errorCode,
        duration_ms: 777,
      });
      const selectedLogRefetch = vi.fn(async () => {
        requestLogQueryState.selectedLog = terminalLog;
        return { data: terminalLog };
      });
      const attemptLogsRefetch = vi.fn(async () => ({ data: [] }));

      setRequestLogQueryState({
        selectedLog: createSelectedLog({
          trace_id: traceId,
          status: null,
          error_code: null,
          created_at: Math.floor(Date.now() / 1000),
          created_at_ms: Date.now(),
          duration_ms: 0,
        }),
        selectedLogRefetch,
        attemptLogsRefetch,
      });
      setTraceStoreState({ traces: [createLiveTrace(traceId)] });

      const view = render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
      expect(screen.getByText("进行中")).toBeInTheDocument();

      act(() => {
        gatewayEventState.requestSignalHandler?.({
          trace_id: "other-trace",
          cli_key: "claude",
          phase: "complete",
          ts: Date.now(),
        });
        gatewayEventState.requestSignalHandler?.({
          trace_id: traceId,
          cli_key: "claude",
          phase: "start",
          ts: Date.now(),
        });
        gatewayEventState.requestSignalHandler?.({
          trace_id: traceId,
          cli_key: "claude",
          phase: "complete",
          status,
          error_code: errorCode,
          ts: Date.now(),
        });
        gatewayEventState.requestSignalHandler?.({
          trace_id: traceId,
          cli_key: "claude",
          phase: "complete",
          status,
          error_code: errorCode,
          ts: Date.now() + 1,
        });
      });

      expect(selectedLogRefetch).not.toHaveBeenCalled();
      expect(attemptLogsRefetch).not.toHaveBeenCalled();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(400);
        await Promise.resolve();
      });

      expect(selectedLogRefetch).toHaveBeenCalledTimes(1);
      expect(attemptLogsRefetch).toHaveBeenCalledTimes(1);

      view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
      expect(screen.queryByText("进行中")).not.toBeInTheDocument();
      expect(screen.getByText(expectedBadge)).toBeInTheDocument();
    }
  );

  it("cancels a queued terminal-signal refresh when the selected trace changes", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));
    const traceA = "trace-a";
    const traceB = "trace-b";
    const refetchA = vi.fn(async () => ({ data: requestLogQueryState.selectedLog }));
    const refetchB = vi.fn(async () => ({ data: requestLogQueryState.selectedLog }));

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        id: 1,
        trace_id: traceA,
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        created_at_ms: Date.now(),
      }),
      selectedLogRefetch: refetchA,
      attemptLogsRefetch: vi.fn(async () => ({ data: [] })),
    });
    setTraceStoreState({ traces: [createLiveTrace(traceA), createLiveTrace(traceB)] });

    const view = render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    act(() => {
      gatewayEventState.requestSignalHandler?.({
        trace_id: traceA,
        cli_key: "claude",
        phase: "complete",
        ts: Date.now(),
      });
    });

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        id: 2,
        trace_id: traceB,
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        created_at_ms: Date.now(),
      }),
      selectedLogRefetch: refetchB,
      attemptLogsRefetch: vi.fn(async () => ({ data: [] })),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={2} onSelectLogId={vi.fn()} />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
      await Promise.resolve();
    });

    expect(refetchA).not.toHaveBeenCalled();
    expect(refetchB).not.toHaveBeenCalled();

    act(() => {
      gatewayEventState.requestSignalHandler?.({
        trace_id: traceB,
        cli_key: "claude",
        phase: "complete",
        ts: Date.now(),
      });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
      await Promise.resolve();
    });

    expect(refetchA).not.toHaveBeenCalled();
    expect(refetchB).toHaveBeenCalledTimes(1);
  });

  it("does not subscribe to a placeholder detail trace after selection changes", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));
    const traceA = "trace-placeholder-a";
    const traceB = "trace-placeholder-b";
    const refetchA = vi.fn(async () => ({ data: requestLogQueryState.selectedLog }));
    const refetchB = vi.fn(async () => ({ data: requestLogQueryState.selectedLog }));

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        id: 1,
        trace_id: traceA,
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        created_at_ms: Date.now(),
      }),
      selectedLogRefetch: refetchA,
      attemptLogsRefetch: vi.fn(async () => ({ data: [] })),
    });
    setTraceStoreState({ traces: [createLiveTrace(traceA), createLiveTrace(traceB)] });

    const view = render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expect(gatewayEventState.requestSignalHandler).not.toBeNull();

    setRequestLogQueryState({
      // React Query's keepPreviousData can expose the old detail while the new id is fetching.
      selectedLog: createSelectedLog({
        id: 1,
        trace_id: traceA,
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        created_at_ms: Date.now(),
      }),
      selectedLogLoading: true,
      selectedLogRefetch: refetchB,
      attemptLogsRefetch: vi.fn(async () => ({ data: [] })),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={2} onSelectLogId={vi.fn()} />);

    expect(gatewayEventState.requestSignalHandler).toBeNull();
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
      await Promise.resolve();
    });
    expect(refetchA).not.toHaveBeenCalled();
    expect(refetchB).not.toHaveBeenCalled();

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        id: 2,
        trace_id: traceB,
        status: null,
        error_code: null,
        created_at: Math.floor(Date.now() / 1000),
        created_at_ms: Date.now(),
      }),
      selectedLogRefetch: refetchB,
      attemptLogsRefetch: vi.fn(async () => ({ data: [] })),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={2} onSelectLogId={vi.fn()} />);
    expect(gatewayEventState.requestSignalHandler).not.toBeNull();

    act(() => {
      gatewayEventState.requestSignalHandler?.({
        trace_id: traceB,
        cli_key: "claude",
        phase: "complete",
        ts: Date.now(),
      });
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
      await Promise.resolve();
    });

    expect(refetchA).not.toHaveBeenCalled();
    expect(refetchB).toHaveBeenCalledTimes(1);
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

  it("uses effective input and displays canonical cache buckets", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        input_tokens: 1000,
        effective_input_tokens: 700,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cache_read_input_tokens: 100,
      }),
    });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expectMetricValue("输入 Token", "700");
    expectMetricValue("缓存创建", "200");
    expectMetricValue("缓存读取", "100");
  });

  it("keeps zero-valued cache metrics visible and hides entirely missing metrics", () => {
    const view = render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: null,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expectMetricValue("缓存创建", "0");

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: 0,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expectMetricValue("缓存创建", "0");

    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
      }),
    });
    view.rerender(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);
    expect(screen.queryByText("缓存创建")).not.toBeInTheDocument();
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

  it("renders log detail contribution tabs after built-in tabs and sends command context", () => {
    vi.mocked(usePluginActiveContributionsQuery).mockReturnValue({
      data: {
        ui: [
          {
            pluginId: "acme.debug",
            contributionId: "trace-tools",
            slotId: "logs.detail.tabs",
            title: "调试工具",
            order: 1,
            schema: {
              type: "panel",
              fields: [
                {
                  type: "button",
                  key: "export",
                  label: "导出 Trace",
                  command: "debug.exportTrace",
                },
              ],
            },
          },
        ],
      },
      isLoading: false,
      error: null,
    } as any);
    setRequestLogQueryState({ selectedLog: createSelectedLog({ id: 77, trace_id: "trace-77" }) });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={77} onSelectLogId={vi.fn()} />);

    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "概览",
      "决策链",
      "原始数据",
      "调试工具",
    ]);

    switchToTab("调试工具");
    fireEvent.click(screen.getByRole("button", { name: "导出 Trace" }));

    expect(logToConsole).toHaveBeenCalledWith(
      "info",
      "插件日志详情命令",
      expect.objectContaining({
        command: "debug.exportTrace",
        traceId: "trace-77",
        logId: 77,
        pluginId: "acme.debug",
        contributionId: "trace-tools",
      })
    );
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

  it("uses unavailable terminal state as the final provider label", () => {
    setRequestLogQueryState({
      selectedLog: createSelectedLog({
        status: 503,
        error_code: "GW_ALL_PROVIDERS_UNAVAILABLE",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        attempts_json: JSON.stringify([
          {
            provider_id: 48,
            provider_name: "Burned Provider",
            outcome: "skipped",
            status: null,
            error_code: "GW_PROVIDER_CIRCUIT_OPEN",
            decision: "skip",
            reason: "provider skipped by circuit breaker",
          },
        ]),
      }),
    });
    setTraceStoreState({ traces: [] });

    render(<RequestLogDetailDialog selectedLogId={1} onSelectLogId={vi.fn()} />);

    expect(screen.getByText("全部不可用")).toBeInTheDocument();
    switchToTab("决策链");
    expect(screen.getByText("最终供应商：无可用供应商")).toBeInTheDocument();
    expect(screen.queryByText("最终供应商：Burned Provider")).not.toBeInTheDocument();
  });
});
