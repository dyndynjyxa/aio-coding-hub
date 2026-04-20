import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogGetByTraceId,
  requestLogsList,
  requestLogsListAfterId,
  requestLogsListAfterIdAll,
  requestLogsListAll,
} from "../requestLogs";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      requestLogsList: vi.fn(),
      requestLogsListAll: vi.fn(),
      requestLogsListAfterId: vi.fn(),
      requestLogsListAfterIdAll: vi.fn(),
      requestLogGet: vi.fn(),
      requestLogGetByTraceId: vi.fn(),
      requestAttemptLogsByTraceId: vi.fn(),
    },
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
    vi.mocked(commands.requestLogsList).mockRejectedValueOnce(new Error("request logs boom"));

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
    vi.mocked(commands.requestLogsList).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(requestLogsList("claude", 10)).rejects.toThrow(
      "IPC_NULL_RESULT: request_logs_list"
    );
  });

  it("passes request logs command args with stable contract fields", async () => {
    vi.mocked(commands.requestLogsList).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.requestLogsListAll).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.requestLogsListAfterId).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.requestLogsListAfterIdAll).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.requestLogGet).mockResolvedValueOnce({
      status: "ok",
      data: { id: 1 } as any,
    });
    vi.mocked(commands.requestLogGetByTraceId).mockResolvedValueOnce({
      status: "ok",
      data: null as any,
    });
    vi.mocked(commands.requestAttemptLogsByTraceId).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });

    await requestLogsList("claude", 10);
    await requestLogsListAll(20);
    await requestLogsListAfterId("codex", 5, 30);
    await requestLogsListAfterIdAll(6, 40);
    await requestLogGet(1);
    await requestLogGetByTraceId("t1");
    await requestAttemptLogsByTraceId("t1", 99);

    expect(commands.requestLogsList).toHaveBeenCalledWith("claude", 10);
    expect(commands.requestLogsListAll).toHaveBeenCalledWith(20);
    expect(commands.requestLogsListAfterId).toHaveBeenCalledWith("codex", 5, 30);
    expect(commands.requestLogsListAfterIdAll).toHaveBeenCalledWith(6, 40);
    expect(commands.requestLogGet).toHaveBeenCalledWith(1);
    expect(commands.requestLogGetByTraceId).toHaveBeenCalledWith("t1");
    expect(commands.requestAttemptLogsByTraceId).toHaveBeenCalledWith("t1", 99);
  });
});
