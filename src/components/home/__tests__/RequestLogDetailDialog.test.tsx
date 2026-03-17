import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { copyText } from "../../../services/clipboard";
import { logToConsole } from "../../../services/consoleLog";
import type { RequestLogDetail } from "../../../services/requestLogs";
import { RequestLogDetailDialog } from "../RequestLogDetailDialog";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/clipboard", () => ({ copyText: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

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

describe("home/RequestLogDetailDialog", () => {
  it("renders loading state and closes via dialog close button", async () => {
    const onSelectLogId = vi.fn();

    render(
      <RequestLogDetailDialog
        selectedLogId={1}
        onSelectLogId={onSelectLogId}
        selectedLog={null}
        selectedLogLoading={true}
        attemptLogs={[]}
        attemptLogsLoading={false}
      />
    );

    expect(screen.getByText("加载中…")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("关闭"));
    await waitFor(() => {
      expect(onSelectLogId).toHaveBeenCalledWith(null);
    });
  });

  it("renders detail view and handles clipboard copy success + failure for trace_id and usage_json", async () => {
    const onSelectLogId = vi.fn();

    vi.mocked(copyText).mockResolvedValue(undefined);

    render(
      <RequestLogDetailDialog
        selectedLogId={1}
        onSelectLogId={onSelectLogId}
        selectedLog={createSelectedLog()}
        selectedLogLoading={false}
        attemptLogs={[]}
        attemptLogsLoading={false}
      />
    );

    expect(screen.getByText("/v1/messages")).toBeInTheDocument();
    expect(screen.getByText("GW_STREAM_ABORTED")).toBeInTheDocument();
    expect(screen.getByText(/成本 \$0.12/)).toBeInTheDocument();
    expect(screen.getByText("Provider Claude Bridge")).toBeInTheDocument();
    expect(screen.getByText("source: OpenAI Primary")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "复制 trace_id" }));
    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith("trace-1");
    });
    expect(toast).toHaveBeenCalledWith("已复制 trace_id");

    fireEvent.click(screen.getByRole("button", { name: "复制 usage_json" }));
    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith(
        expect.stringContaining("cache_creation_1h_input_tokens")
      );
    });
    expect(toast).toHaveBeenCalledWith("已复制 usage_json");

    // pretty-printed JSON should now include cache_creation_1h_input_tokens
    expect(screen.getByText(/\"input_tokens\"/)).toBeInTheDocument();
    expect(screen.getAllByText(/cache_creation_1h_input_tokens/).length).toBeGreaterThan(0);

    vi.mocked(copyText).mockRejectedValueOnce(new Error("nope"));
    fireEvent.click(screen.getByRole("button", { name: "复制 trace_id" }));
    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("error", "复制 trace_id 失败", {
        error: "Error: nope",
      });
    });
    expect(toast).toHaveBeenCalledWith("复制失败：当前环境不支持剪贴板");
  });

  it("falls back to raw usage_json when JSON parsing fails", () => {
    vi.mocked(copyText).mockResolvedValue(undefined);

    render(
      <RequestLogDetailDialog
        selectedLogId={1}
        onSelectLogId={vi.fn()}
        selectedLog={createSelectedLog({ usage_json: "not-json" })}
        selectedLogLoading={false}
        attemptLogs={[]}
        attemptLogsLoading={false}
      />
    );

    expect(screen.getByText("not-json")).toBeInTheDocument();
  });
});
