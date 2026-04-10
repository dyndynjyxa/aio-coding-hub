import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  appDataDirGet,
  appDataReset,
  appExit,
  appRestart,
  dbDiskUsageGet,
  requestLogsClearAll,
} from "../dataManagement";

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

describe("services/app/dataManagement", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("data management boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(dbDiskUsageGet()).rejects.toThrow("IPC_NULL_RESULT: db_disk_usage_get");
  });

  it("invokes data management commands with expected parameters", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await dbDiskUsageGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("db_disk_usage_get");

    await requestLogsClearAll();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("request_logs_clear_all");

    await appDataReset();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("app_data_reset");

    await appDataDirGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("app_data_dir_get");

    await appExit();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("app_exit");

    await appRestart();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("app_restart");
  });
});
