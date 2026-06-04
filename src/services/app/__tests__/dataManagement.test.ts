import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  appDataDirGet,
  appDataReset,
  appExit,
  appRestart,
  dbDiskUsageGet,
  isClearRequestLogsResult,
  requestLogsClearAll,
} from "../dataManagement";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      dbDiskUsageGet: vi.fn(),
      requestLogsClearAll: vi.fn(),
      appDataReset: vi.fn(),
      appDataDirGet: vi.fn(),
      appExit: vi.fn(),
      appRestart: vi.fn(),
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

describe("services/app/dataManagement", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.dbDiskUsageGet).mockRejectedValueOnce(new Error("data management boom"));

    await expect(dbDiskUsageGet()).rejects.toThrow("data management boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取数据库磁盘用量失败",
      expect.objectContaining({
        cmd: "db_disk_usage_get",
        error: expect.stringContaining("data management boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.dbDiskUsageGet).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(dbDiskUsageGet()).rejects.toThrow("IPC_NULL_RESULT: db_disk_usage_get");
  });

  it("invokes data management commands with expected parameters", async () => {
    vi.mocked(commands.dbDiskUsageGet).mockResolvedValueOnce({
      status: "ok",
      data: { db_bytes: 1, wal_bytes: 2, shm_bytes: 3, total_bytes: 6 } as any,
    });
    vi.mocked(commands.requestLogsClearAll).mockResolvedValueOnce({
      status: "ok",
      data: { request_logs_deleted: 0, request_attempt_logs_deleted: 0 } as any,
    });
    vi.mocked(commands.appDataReset).mockResolvedValueOnce({ status: "ok", data: true });
    vi.mocked(commands.appDataDirGet).mockResolvedValueOnce({ status: "ok", data: "/tmp" as any });
    vi.mocked(commands.appExit).mockResolvedValueOnce({ status: "ok", data: true });
    vi.mocked(commands.appRestart).mockResolvedValueOnce({ status: "ok", data: true });

    await dbDiskUsageGet();
    expect(commands.dbDiskUsageGet).toHaveBeenCalledWith();

    await requestLogsClearAll();
    expect(commands.requestLogsClearAll).toHaveBeenCalledWith();

    await appDataReset();
    expect(commands.appDataReset).toHaveBeenCalledWith(
      expect.objectContaining({
        confirm: expect.objectContaining({
          action: "app_data_reset",
          resource: "app_data",
          nonce: expect.any(String),
        }),
      })
    );

    await appDataDirGet();
    expect(commands.appDataDirGet).toHaveBeenCalledWith();

    await appExit();
    expect(commands.appExit).toHaveBeenCalledWith();

    await appRestart();
    expect(commands.appRestart).toHaveBeenCalledWith();
  });

  it("validates generated disk usage and clear-count payloads", async () => {
    vi.mocked(commands.dbDiskUsageGet).mockResolvedValueOnce({
      status: "ok",
      data: { db_bytes: 1, wal_bytes: 2, shm_bytes: 3, total_bytes: 7 } as any,
    });

    await expect(dbDiskUsageGet()).rejects.toThrow("IPC_INVALID_RESULT");

    vi.mocked(commands.requestLogsClearAll).mockResolvedValueOnce({
      status: "ok",
      data: { request_logs_deleted: -1, request_attempt_logs_deleted: 2 } as any,
    });

    await expect(requestLogsClearAll()).rejects.toThrow("IPC_INVALID_RESULT");

    expect(isClearRequestLogsResult(null)).toBe(false);
    expect(
      isClearRequestLogsResult({ request_logs_deleted: 1, request_attempt_logs_deleted: 2 })
    ).toBe(true);
    expect(
      isClearRequestLogsResult({
        request_logs_deleted: 1.5,
        request_attempt_logs_deleted: 2,
      } as any)
    ).toBe(false);
  });
});
