import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RealtimeTraceCards } from "../RealtimeTraceCards";

function traceBase(overrides: Partial<any> = {}) {
  return {
    trace_id: "t-1",
    cli_key: "claude",
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: "gpt-5",
    first_seen_ms: 1_700_000_000_000,
    last_seen_ms: 1_700_000_000_000,
    attempts: [],
    ...overrides,
  };
}

describe("components/home/RealtimeTraceCards", () => {
  it("does not start timer when traces list is empty", () => {
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    render(
      <RealtimeTraceCards
        folderLookupBySessionKey={new Map()}
        traces={[]}
        formatUnixSeconds={(ts) => String(ts)}
        showCustomTooltip={false}
      />
    );
    expect(setIntervalSpy).not.toHaveBeenCalled();
    setIntervalSpy.mockRestore();
  });

  it("renders in-progress and completed traces, including route and cache hints", () => {
    vi.useFakeTimers();
    const baseTime = 1_700_000_000_000;
    vi.setSystemTime(baseTime);

    const inProgress = traceBase({
      trace_id: "t-progress",
      session_id: "claude-session-1",
      requested_model: "   ",
      first_seen_ms: baseTime - 1000,
      last_seen_ms: baseTime - 1000,
      attempts: [],
      summary: undefined,
    });

    const completedError = traceBase({
      trace_id: "t-error",
      cli_key: "claude",
      requested_model: "claude-opus",
      first_seen_ms: baseTime - 5000,
      last_seen_ms: baseTime - 100,
      attempts: [
        { attempt_index: 0, provider_name: "P1", outcome: "started" },
        { attempt_index: 1, provider_name: "P1", outcome: "started" },
        { attempt_index: 2, provider_name: "P1", outcome: "success", session_reuse: true },
        { attempt_index: 3, provider_name: "P2", outcome: "failed" },
        { attempt_index: 4, provider_name: "Unknown", outcome: "failed" },
      ],
      summary: {
        trace_id: "t-error",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        status: 499,
        error_code: "GW_STREAM_ABORTED",
        duration_ms: 100,
        ttfb_ms: 10,
      },
    });

    const completedOk = traceBase({
      trace_id: "t-ok",
      cli_key: "codex",
      requested_model: "gpt-5-codex",
      first_seen_ms: baseTime - 6000,
      last_seen_ms: baseTime - 50,
      attempts: [{ attempt_index: 0, provider_name: "P3", outcome: "success" }],
      summary: {
        trace_id: "t-ok",
        cli_key: "codex",
        method: "POST",
        path: "/v1/responses",
        query: null,
        status: 200,
        error_code: null,
        duration_ms: 1000,
        ttfb_ms: 100,
        input_tokens: 1000,
        output_tokens: 900,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 10,
        cost_usd: 1.23,
      },
    });

    render(
      <RealtimeTraceCards
        folderLookupBySessionKey={
          new Map([
            [
              "claude:claude-session-1",
              {
                source: "claude",
                session_id: "claude-session-1",
                folder_name: "workspace-alpha",
                folder_path: "/Users/demo/workspace-alpha",
              },
            ],
          ])
        }
        traces={[inProgress, completedError, completedOk] as any}
        formatUnixSeconds={(ts) => `ts:${ts}`}
        showCustomTooltip={false}
      />
    );

    expect(screen.getByText("进行中")).toBeInTheDocument();
    expect(screen.getByText("当前阶段")).toBeInTheDocument();
    expect(screen.getByText("等待首个尝试")).toBeInTheDocument();
    expect(screen.getByText("尝试次数")).toBeInTheDocument();
    expect(screen.getByText("workspace-alpha")).toBeInTheDocument();
    expect(screen.getAllByText("未知").length).toBeGreaterThan(0); // model/provider fallback
    expect(screen.getAllByText("P3").length).toBeGreaterThan(0);
    expect(screen.getByText("流中断")).toBeInTheDocument();
    expect(screen.getAllByText("会话复用").length).toBeGreaterThan(0);
    expect(screen.getByTitle("P1 → P2")).toBeInTheDocument();
    expect(screen.getAllByText(/t\/s/).length).toBeGreaterThan(0);
    expect(screen.getByText("$1.230000")).toBeInTheDocument();
    expect(screen.getAllByText("$0.000000").length).toBeGreaterThan(0);

    vi.useRealTimers();
  });
});
