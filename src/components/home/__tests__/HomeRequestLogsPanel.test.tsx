import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import type { CliSessionsFolderLookupEntry } from "../../../services/cliSessions";
import type { RequestLogSummary } from "../../../services/requestLogs";
import type { TraceSession } from "../../../services/traceStore";
import { HomeRequestLogsPanel } from "../HomeRequestLogsPanel";

const { useCliSessionsFolderLookupByIdsQueryMock } = vi.hoisted(() => ({
  useCliSessionsFolderLookupByIdsQueryMock: vi.fn<
    () => { data: CliSessionsFolderLookupEntry[]; isLoading: boolean }
  >(() => ({ data: [], isLoading: false })),
}));

vi.mock("../../../query/cliSessions", () => ({
  useCliSessionsFolderLookupByIdsQuery: useCliSessionsFolderLookupByIdsQueryMock,
}));

describe("components/home/HomeRequestLogsPanel", () => {
  afterEach(() => {
    localStorage.removeItem("home_request_logs_compact");
    vi.useRealTimers();
    useCliSessionsFolderLookupByIdsQueryMock.mockReset();
    useCliSessionsFolderLookupByIdsQueryMock.mockReturnValue({ data: [], isLoading: false });
  });
  it("renders traces + logs and supports refresh/select", () => {
    useCliSessionsFolderLookupByIdsQueryMock.mockReturnValue({
      data: [
        {
          source: "claude",
          session_id: "claude-live-session",
          folder_name: "workspace-live",
          folder_path: "/Users/demo/workspace-live",
        },
        {
          source: "claude",
          session_id: "claude-log-session",
          folder_name: "workspace-log",
          folder_path: "/Users/demo/workspace-log",
        },
      ],
      isLoading: false,
    });

    const traces: TraceSession[] = [
      {
        trace_id: "t-live",
        cli_key: "claude",
        session_id: "claude-live-session",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-3-opus",
        first_seen_ms: Date.now() - 1000,
        last_seen_ms: Date.now() - 200,
        attempts: [
          {
            trace_id: "t-live",
            cli_key: "claude",
            method: "POST",
            path: "/v1/messages",
            query: null,
            attempt_index: 1,
            provider_id: 1,
            provider_name: "P1",
            base_url: "https://p1",
            outcome: "started",
            status: null,
            attempt_started_ms: 0,
            attempt_duration_ms: 0,
            session_reuse: false,
          } as any,
        ],
      },
    ];

    const requestLogs: RequestLogSummary[] = [
      {
        id: 1,
        trace_id: "t1",
        cli_key: "claude",
        session_id: "claude-log-session",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: 200,
        error_code: null,
        duration_ms: 1234,
        ttfb_ms: 120,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 1,
        final_provider_name: "P1",
        final_provider_source_id: 7,
        final_provider_source_name: "OpenAI Primary",
        route: [
          {
            provider_id: 1,
            provider_name: "P1",
            ok: true,
            status: 200,
          },
        ],
        session_reuse: false,
        input_tokens: 10,
        output_tokens: 20,
        total_tokens: 30,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 0.123456,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    const onRefreshRequestLogs = vi.fn();
    const onSelectLogId = vi.fn();

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          compactModeOverride={false}
          traces={traces}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={onRefreshRequestLogs}
          selectedLogId={null}
          onSelectLogId={onSelectLogId}
        />
      </MemoryRouter>
    );

    expect(screen.getByText("最近代理记录")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /claude-3-opus.*P1/ })).toBeInTheDocument();
    expect(screen.getByText("workspace-live")).toBeInTheDocument();
    expect(screen.getByText("workspace-log")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));
    expect(screen.getByRole("button", { name: /claude-3-opus.*P1/ })).toBeInTheDocument();
    expect(screen.getByText("$0.123456")).toBeInTheDocument();
    expect(screen.getByText("$0.123456").closest("div")?.getAttribute("title")).toBe("$0.123456");
    expect(screen.queryByText("source: OpenAI Primary")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(onRefreshRequestLogs).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /claude-3-opus/ }));
    expect(onSelectLogId).toHaveBeenCalledWith(1);
  });

  it("keeps Claude realtime traces visible when the persisted request log already exists", () => {
    const traces: TraceSession[] = [
      {
        trace_id: "t-log-claude",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-3-opus",
        first_seen_ms: Date.now() - 1000,
        last_seen_ms: Date.now() - 200,
        attempts: [],
      },
      {
        trace_id: "t-live-codex",
        cli_key: "codex",
        method: "POST",
        path: "/v1/responses",
        query: null,
        requested_model: "gpt-5",
        first_seen_ms: Date.now() - 1000,
        last_seen_ms: Date.now() - 200,
        attempts: [],
      },
    ];

    const requestLogs: RequestLogSummary[] = [
      {
        id: 1,
        trace_id: "t-log-claude",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: null,
        error_code: null,
        duration_ms: 0,
        ttfb_ms: null,
        attempt_count: 0,
        has_failover: false,
        start_provider_id: 0,
        start_provider_name: "Unknown",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          compactModeOverride={false}
          traces={traces}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("claude-3-opus")).toHaveLength(1);
    expect(screen.getByText("gpt-5")).toBeInTheDocument();
    expect(screen.getAllByText("进行中")).toHaveLength(2);
  });

  it("hides folder labels for unsupported cli keys and missing session ids", () => {
    useCliSessionsFolderLookupByIdsQueryMock.mockReturnValue({
      data: [
        {
          source: "codex",
          session_id: "codex-session-1",
          folder_name: "platform-core",
          folder_path: "/Users/demo/platform-core",
        },
      ],
      isLoading: false,
    });

    const requestLogs: RequestLogSummary[] = [
      {
        id: 21,
        trace_id: "t-folder-codex",
        cli_key: "codex",
        session_id: "codex-session-1",
        method: "POST",
        path: "/v1/responses",
        requested_model: "gpt-5.4",
        status: 200,
        error_code: null,
        duration_ms: 800,
        ttfb_ms: 100,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 1,
        final_provider_name: "P1",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
      {
        id: 22,
        trace_id: "t-folder-gemini",
        cli_key: "gemini",
        session_id: "gemini-session-1",
        method: "POST",
        path: "/v1/chat/completions",
        requested_model: "gemini-2.5-pro",
        status: 200,
        error_code: null,
        duration_ms: 900,
        ttfb_ms: 120,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P2",
        final_provider_id: 1,
        final_provider_name: "P2",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          traces={[]}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getByText("platform-core")).toBeInTheDocument();
    expect(screen.queryByText("gemini-session-1")).not.toBeInTheDocument();
  });

  it("promotes persisted in-progress request logs into realtime cards when live events are missing", () => {
    useCliSessionsFolderLookupByIdsQueryMock.mockReturnValue({
      data: [
        {
          source: "claude",
          session_id: "claude-session-missing-live-event",
          folder_name: "workspace-live-fallback",
          folder_path: "/Users/demo/workspace-live-fallback",
        },
      ],
      isLoading: false,
    });

    const requestLogs: RequestLogSummary[] = [
      {
        id: 11,
        trace_id: "t-pending-claude",
        cli_key: "claude",
        session_id: "claude-session-missing-live-event",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: null,
        error_code: null,
        duration_ms: 0,
        ttfb_ms: null,
        attempt_count: 0,
        has_failover: false,
        start_provider_id: 0,
        start_provider_name: "Unknown",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          traces={[]}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("进行中").length).toBeGreaterThan(0);
    expect(screen.getByText("当前阶段")).toBeInTheDocument();
    expect(screen.getByText("等待首个尝试")).toBeInTheDocument();
    expect(screen.getAllByText("workspace-live-fallback")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: /claude-3-opus/ })).not.toBeInTheDocument();
  });

  it("keeps in-progress request logs at the top while preserving time order for the rest", () => {
    const nowMs = Date.now();
    const requestLogs: RequestLogSummary[] = [
      {
        id: 21,
        trace_id: "t-completed-newer",
        cli_key: "codex",
        method: "POST",
        path: "/v1/responses",
        requested_model: "done-newer-model",
        status: 200,
        error_code: null,
        duration_ms: 1200,
        ttfb_ms: 200,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 1,
        final_provider_name: "P1",
        route: [],
        session_reuse: false,
        input_tokens: 10,
        output_tokens: 20,
        total_tokens: 30,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 0.01,
        cost_multiplier: 1,
        created_at_ms: nowMs,
        created_at: Math.floor(nowMs / 1000),
      },
      {
        id: 22,
        trace_id: "t-pending-older",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "pending-model",
        status: null,
        error_code: null,
        duration_ms: 0,
        ttfb_ms: null,
        attempt_count: 0,
        has_failover: false,
        start_provider_id: 0,
        start_provider_name: "Unknown",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: nowMs - 10_000,
        created_at: Math.floor((nowMs - 10_000) / 1000),
      },
      {
        id: 23,
        trace_id: "t-completed-older",
        cli_key: "gemini",
        method: "POST",
        path: "/v1/chat/completions",
        requested_model: "done-older-model",
        status: 500,
        error_code: "GW_UPSTREAM_5XX",
        duration_ms: 2200,
        ttfb_ms: 400,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 2,
        start_provider_name: "P2",
        final_provider_id: 2,
        final_provider_name: "P2",
        route: [],
        session_reuse: false,
        input_tokens: 5,
        output_tokens: 6,
        total_tokens: 11,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 0.02,
        cost_multiplier: 1,
        created_at_ms: nowMs - 20_000,
        created_at: Math.floor((nowMs - 20_000) / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          traces={[]}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getByText("pending-model")).toBeInTheDocument();
    expect(screen.getByText("当前阶段")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /pending-model/ })).not.toBeInTheDocument();
    const completedNewerButton = screen.getByRole("button", { name: /done-newer-model/ });
    const completedOlderButton = screen.getByRole("button", { name: /done-older-model/ });

    expect(completedNewerButton.compareDocumentPosition(completedOlderButton)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
  });

  it("uses live trace data to show current provider and elapsed duration for in-progress logs", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-03-29T12:00:00.000Z"));

    const traces: TraceSession[] = [
      {
        trace_id: "t-live-provider",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-3-opus",
        first_seen_ms: Date.now() - 6500,
        last_seen_ms: Date.now() - 100,
        attempts: [
          {
            trace_id: "t-live-provider",
            cli_key: "claude",
            method: "POST",
            path: "/v1/messages",
            query: null,
            requested_model: "claude-3-opus",
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
    ];

    const requestLogs: RequestLogSummary[] = [
      {
        id: 12,
        trace_id: "t-live-provider",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: null,
        error_code: null,
        duration_ms: 0,
        ttfb_ms: null,
        attempt_count: 0,
        has_failover: false,
        start_provider_id: 0,
        start_provider_name: "Unknown",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        route: [],
        session_reuse: false,
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cache_read_input_tokens: null,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: null,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          compactModeOverride={false}
          traces={traces}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("Provider Live").length).toBeGreaterThan(0);
    expect(screen.getByText("6.50s")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(screen.getByText("7.50s")).toBeInTheDocument();
  });

  it("covers status text branches + logs page navigation + rich log row variants", () => {
    const nowMs = Date.now();
    const traces: TraceSession[] = [
      {
        trace_id: "t-old",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "old",
        first_seen_ms: nowMs - 16 * 60 * 1000,
        last_seen_ms: nowMs - 16 * 60 * 1000,
        attempts: [],
      } as any,
      {
        trace_id: "t-live",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        requested_model: "claude-3-opus",
        first_seen_ms: nowMs - 1000,
        last_seen_ms: nowMs - 200,
        attempts: [],
      } as any,
    ];

    const requestLogs: RequestLogSummary[] = [
      {
        id: 1,
        trace_id: "t1",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: 500,
        error_code: "GW_STREAM_ABORTED",
        duration_ms: 1000,
        ttfb_ms: 9000,
        attempt_count: 2,
        has_failover: true,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 0,
        final_provider_name: "Unknown",
        route: [
          { provider_id: 1, provider_name: "P1", ok: true, status: 200 },
          {
            provider_id: 2,
            provider_name: "Unknown",
            ok: false,
            status: null,
            error_code: "GW_UPSTREAM_TIMEOUT",
          },
        ],
        session_reuse: true,
        input_tokens: 123,
        output_tokens: 1000,
        total_tokens: 1123,
        cache_read_input_tokens: 50,
        cache_creation_input_tokens: null,
        cache_creation_5m_input_tokens: 10,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 9.99,
        cost_multiplier: 1.5,
        created_at_ms: null,
        created_at: Math.floor(nowMs / 1000),
      },
      {
        id: 2,
        trace_id: "t2",
        cli_key: "codex",
        method: "POST",
        path: "/v1/responses",
        requested_model: " ",
        status: 200,
        error_code: null,
        duration_ms: 500,
        ttfb_ms: 100,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 2,
        final_provider_name: "P2",
        route: [],
        session_reuse: false,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 30,
        cache_creation_5m_input_tokens: null,
        cache_creation_1h_input_tokens: null,
        cost_usd: 0,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(nowMs / 1000),
      },
    ];

    const onRefreshRequestLogs = vi.fn();
    const onSelectLogId = vi.fn();

    render(
      <MemoryRouter initialEntries={["/"]}>
        <Routes>
          <Route
            path="/"
            element={
              <HomeRequestLogsPanel
                showCustomTooltip={true}
                traces={traces}
                requestLogs={requestLogs}
                requestLogsLoading={false}
                requestLogsRefreshing={false}
                requestLogsAvailable={true}
                onRefreshRequestLogs={onRefreshRequestLogs}
                selectedLogId={1}
                onSelectLogId={onSelectLogId}
              />
            }
          />
          <Route path="/logs" element={<div>LOGS_PAGE</div>} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText("共 2 条")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(onRefreshRequestLogs).toHaveBeenCalled();

    // selection click hits the row onClick handler
    fireEvent.click(screen.getByRole("button", { name: /claude-3-opus/ }));
    expect(onSelectLogId).toHaveBeenCalledWith(1);

    // spot-check some conditional text rendering paths
    expect(screen.getAllByText("未知").length).toBeGreaterThan(0);
    expect(screen.getByText("切换 2 次")).toBeInTheDocument();
    expect(screen.getByText("会话复用")).toBeInTheDocument();
    expect(screen.getByText("x1.50")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /500 已中断/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "代理记录" }));
    expect(screen.getByText("LOGS_PAGE")).toBeInTheDocument();
  });

  it("shows free when cost multiplier is zero", () => {
    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          traces={[]}
          requestLogs={[
            {
              id: 9,
              trace_id: "t-free",
              cli_key: "gemini",
              method: "POST",
              path: "/v1/chat/completions",
              requested_model: "gemini-2.5-pro",
              status: 200,
              error_code: null,
              duration_ms: 800,
              ttfb_ms: 200,
              attempt_count: 1,
              has_failover: false,
              start_provider_id: 1,
              start_provider_name: "P1",
              final_provider_id: 1,
              final_provider_name: "P1",
              route: [],
              session_reuse: false,
              input_tokens: 10,
              output_tokens: 20,
              total_tokens: 30,
              cache_read_input_tokens: 0,
              cache_creation_input_tokens: 0,
              cache_creation_5m_input_tokens: 0,
              cache_creation_1h_input_tokens: 0,
              cost_usd: 0,
              cost_multiplier: 0,
              created_at_ms: null,
              created_at: Math.floor(Date.now() / 1000),
            },
          ]}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("免费").length).toBeGreaterThan(0);
  });

  it("handles requestLogsAvailable=false (tauri-only) states", () => {
    const onRefreshRequestLogs = vi.fn();
    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          traces={[]}
          requestLogs={[]}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={false}
          onRefreshRequestLogs={onRefreshRequestLogs}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("数据不可用").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "代理记录" })).toBeDisabled();
  });

  it("shows plain 链路 when route exists without failover", () => {
    const onRefreshRequestLogs = vi.fn();
    const requestLogs: RequestLogSummary[] = [
      {
        id: 11,
        trace_id: "t11",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-5-sonnet",
        status: 200,
        error_code: null,
        duration_ms: 123,
        ttfb_ms: 12,
        attempt_count: 1,
        has_failover: false,
        start_provider_id: 1,
        start_provider_name: "P1",
        final_provider_id: 1,
        final_provider_name: "P1",
        route: [{ provider_id: 1, provider_name: "P1", ok: true, status: 200 }],
        session_reuse: false,
        input_tokens: 1,
        output_tokens: 2,
        total_tokens: 3,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 0.01,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(Date.now() / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          traces={[]}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={onRefreshRequestLogs}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));
    expect(screen.getByText("直连完成")).toBeInTheDocument();
    expect(screen.queryByText(/切换 \d+ 次/)).not.toBeInTheDocument();
  });

  it("renders loading/refreshing empty state variants", () => {
    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          traces={[]}
          requestLogs={[]}
          requestLogsLoading={true}
          requestLogsRefreshing={true}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByText("加载中…").length).toBeGreaterThan(0);
  });

  it("supports page-specific summary and empty state copy", () => {
    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          title="代理记录列表"
          summaryTextOverride="共 0 / 3 条"
          emptyStateTitle="没有符合筛选条件的代理记录"
          traces={[]}
          requestLogs={[]}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getByText("代理记录列表")).toBeInTheDocument();
    expect(screen.getByText("共 0 / 3 条")).toBeInTheDocument();
    expect(screen.getByText("没有符合筛选条件的代理记录")).toBeInTheDocument();
  });

  it("renders preview rows when dev preview is enabled in empty state", () => {
    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          devPreviewEnabled={true}
          traces={[]}
          requestLogs={[]}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getAllByTitle("Codex / gpt-5.4").length).toBeGreaterThan(0);
    expect(screen.getAllByTitle("Claude Code / claude-sonnet-4").length).toBeGreaterThan(0);
    expect(screen.getAllByTitle("Gemini / gemini-2.5-pro").length).toBeGreaterThan(0);
    expect(screen.getAllByText("免费").length).toBeGreaterThan(0);
    expect(screen.getAllByText("进行中").length).toBeGreaterThan(0);
    expect(screen.getByText("切换处理中")).toBeInTheDocument();
    expect(screen.getByText("等待首个尝试")).toBeInTheDocument();
    expect(screen.getByText("Claude Main → Claude Backup")).toBeInTheDocument();
    expect(screen.queryByText("当前没有最近使用记录")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "关闭预览" })).not.toBeInTheDocument();
  });

  it("renders rich tooltip with attempt counts for failover routes", async () => {
    const user = userEvent.setup();
    const nowMs = Date.now();
    const requestLogs: RequestLogSummary[] = [
      {
        id: 20,
        trace_id: "t20",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        requested_model: "claude-3-opus",
        status: 200,
        error_code: null,
        duration_ms: 3000,
        ttfb_ms: 200,
        attempt_count: 4,
        has_failover: true,
        start_provider_id: 1,
        start_provider_name: "ProvA",
        final_provider_id: 2,
        final_provider_name: "ProvB",
        route: [
          {
            provider_id: 1,
            provider_name: "ProvA",
            ok: false,
            attempts: 3,
            status: 500,
            error_code: "GW_UPSTREAM_5XX",
            decision: "failover",
            reason: "status=500",
          },
          {
            provider_id: 2,
            provider_name: "ProvB",
            ok: true,
            attempts: 1,
            status: 200,
          },
        ],
        session_reuse: false,
        input_tokens: 100,
        output_tokens: 200,
        total_tokens: 300,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
        cache_creation_1h_input_tokens: 0,
        cost_usd: 0.05,
        cost_multiplier: 1,
        created_at_ms: null,
        created_at: Math.floor(nowMs / 1000),
      },
    ];

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={true}
          traces={[]}
          requestLogs={requestLogs}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));

    // 标签文本应包含切换摘要
    expect(screen.getByText("切换 4 次")).toBeInTheDocument();

    // 鼠标悬停触发 tooltip 显示富文本内容
    const routeLabel = screen.getByText("切换 4 次");
    await user.hover(routeLabel);

    // tooltip 路径概览中应显示 provider 名称
    // ProvA 出现在 tooltip 路径概览 + tooltip 详情行（卡片中 final_provider 是 ProvB）
    await waitFor(() => expect(screen.getAllByText("ProvA").length).toBeGreaterThanOrEqual(2));
    // ProvB 同时出现在卡片 provider 区域和 tooltip 中
    await waitFor(() => expect(screen.getAllByText("ProvB").length).toBeGreaterThanOrEqual(2));
    // 失败3次的标签
    await waitFor(() => expect(screen.getAllByText("失败 3 次").length).toBeGreaterThan(0));
    // 成功的标签
    await waitFor(() => expect(screen.getAllByText("成功").length).toBeGreaterThan(0));
  });

  it("supports compact mode to show only the first-row fields", async () => {
    const user = userEvent.setup();

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          traces={[]}
          requestLogs={[
            {
              id: 31,
              trace_id: "t31",
              cli_key: "codex",
              method: "POST",
              path: "/v1/responses",
              requested_model: "gpt-5.4",
              status: 200,
              error_code: "GW_STREAM_ABORTED",
              duration_ms: 3200,
              ttfb_ms: 600,
              attempt_count: 1,
              has_failover: false,
              start_provider_id: 1,
              start_provider_name: "P1",
              final_provider_id: 1,
              final_provider_name: "P1",
              route: [{ provider_id: 1, provider_name: "P1", ok: true, status: 200 }],
              session_reuse: true,
              input_tokens: 100,
              output_tokens: 200,
              total_tokens: 300,
              cache_read_input_tokens: 50,
              cache_creation_input_tokens: 25,
              cache_creation_5m_input_tokens: 0,
              cache_creation_1h_input_tokens: 0,
              cost_usd: 0.01,
              cost_multiplier: 1.5,
              created_at_ms: null,
              created_at: Math.floor(Date.now() / 1000),
            },
          ]}
          requestLogsLoading={false}
          requestLogsRefreshing={false}
          requestLogsAvailable={true}
          onRefreshRequestLogs={vi.fn()}
          selectedLogId={null}
          onSelectLogId={vi.fn()}
        />
      </MemoryRouter>
    );

    expect(screen.getByTitle("Codex / gpt-5.4")).toBeInTheDocument();
    expect(screen.getAllByText("P1").length).toBeGreaterThan(0);
    expect(screen.getByText("流中断")).toBeInTheDocument();
    expect(screen.queryByText("3.20s")).not.toBeInTheDocument();
    expect(screen.queryByText("输入")).not.toBeInTheDocument();
    expect(screen.getByText("会话复用")).toBeInTheDocument();

    await user.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));

    expect(screen.getByText("输入")).toBeInTheDocument();
    expect(screen.getAllByText("P1").length).toBeGreaterThan(0);
  });
});
