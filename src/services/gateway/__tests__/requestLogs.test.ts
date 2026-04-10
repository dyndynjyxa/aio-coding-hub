import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogGetByTraceId,
  requestLogsList,
  requestLogsListAfterId,
  requestLogsListAfterIdAll,
  requestLogsListAll,
} from "../requestLogs";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/gateway/requestLogs", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("request logs boom"));

    await expect(requestLogsList("claude", 10)).rejects.toThrow("request logs boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取请求日志失败",
      expect.objectContaining({
        cmd: "request_logs_list",
        error: expect.stringContaining("request logs boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(requestLogsList("claude", 10)).rejects.toThrow(
      "IPC_NULL_RESULT: request_logs_list"
    );
  });

  it("passes request logs command args with stable contract fields", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue([] as any);

    await requestLogsList("claude", 10);
    await requestLogsListAll(20);
    await requestLogsListAfterId("codex", 5, 30);
    await requestLogsListAfterIdAll(6, 40);
    await requestLogGet(1);
    await requestLogGetByTraceId("t1");
    await requestAttemptLogsByTraceId("t1", 99);

    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_logs_list", {
      cliKey: "claude",
      limit: 10,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_logs_list_all", { limit: 20 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_logs_list_after_id", {
      cliKey: "codex",
      afterId: 5,
      limit: 30,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_logs_list_after_id_all", {
      afterId: 6,
      limit: 40,
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_log_get", { logId: 1 });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_log_get_by_trace_id", {
      traceId: "t1",
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_attempt_logs_by_trace_id", {
      traceId: "t1",
      limit: 99,
    });
  });
});
