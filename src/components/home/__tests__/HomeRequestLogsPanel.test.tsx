import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import type { RequestLogSummary } from "../../../services/requestLogs";
import type { TraceSession } from "../../../services/traceStore";
import { HomeRequestLogsPanel } from "../HomeRequestLogsPanel";

describe("components/home/HomeRequestLogsPanel", () => {
  it("renders traces + logs and supports refresh/select", () => {
    const traces: TraceSession[] = [
      {
        trace_id: "t-live",
        cli_key: "claude",
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

    expect(screen.getByText("使用记录（最近 50 条）")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /claude-3-opus.*P1/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "最近使用记录简洁模式" }));
    expect(screen.getByRole("button", { name: /claude-3-opus.*P1/ })).toBeInTheDocument();
    expect(screen.getByText("$0.123456")).toBeInTheDocument();
    expect(screen.getByText("$0.123456").closest("div")?.getAttribute("title")).toBe("$0.123456");

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    expect(onRefreshRequestLogs).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /claude-3-opus/ }));
    expect(onSelectLogId).toHaveBeenCalledWith(1);
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
    expect(screen.getByText("链路[降级*2]")).toBeInTheDocument();
    expect(screen.getByText("会话复用")).toBeInTheDocument();
    expect(screen.getByText("x1.50")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "日志" }));
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
    expect(screen.getByRole("button", { name: "日志" })).toBeDisabled();
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
    expect(screen.getByText("链路")).toBeInTheDocument();
    expect(screen.queryByText(/链路\[降级\*/)).not.toBeInTheDocument();
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

  it("supports local preview rows in dev-like empty state", async () => {
    const user = userEvent.setup();

    render(
      <MemoryRouter>
        <HomeRequestLogsPanel
          showCustomTooltip={false}
          requestLogsPreviewEnabled={true}
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

    expect(screen.getByText("当前没有最近使用记录")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "预览记录样式" }));

    expect(screen.getAllByText("gpt-5.4").length).toBeGreaterThan(0);
    expect(screen.getAllByText("免费").length).toBeGreaterThan(0);
    expect(screen.getByText("进行中")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭预览" })).toBeInTheDocument();
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

    // 标签文本应包含降级计数
    expect(screen.getByText("链路[降级*4]")).toBeInTheDocument();

    // 鼠标悬停触发 tooltip 显示富文本内容
    const routeLabel = screen.getByText("链路[降级*4]");
    await user.hover(routeLabel);

    // tooltip 路径概览中应显示 provider 名称
    // ProvA 出现在 tooltip 路径概览 + tooltip 详情行（卡片中 final_provider 是 ProvB）
    await waitFor(() => expect(screen.getAllByText("ProvA").length).toBeGreaterThanOrEqual(2));
    // ProvB 同时出现在卡片 provider 区域和 tooltip 中
    await waitFor(() => expect(screen.getAllByText("ProvB").length).toBeGreaterThanOrEqual(2));
    // 失败3次的标签
    await waitFor(() => expect(screen.getAllByText("失败3次").length).toBeGreaterThan(0));
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

    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
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
