import { describe, expect, it, vi } from "vitest";
import { appEventNames } from "../../../constants/appEvents";
import { emitTauriEvent, tauriInvoke } from "../../../test/mocks/tauri";
import * as consoleLog from "../../consoleLog";
import {
  appStartupRetry,
  appStartupStatusGet,
  listenAppStartupStatusEvents,
  normalizeAppStartupStatus,
} from "../startupStatus";

const validStatus = {
  running: false,
  currentStage: "idle",
  failedStage: null,
  errorMessage: null,
  canRetry: false,
};

describe("services/app/startupStatus", () => {
  it("normalizes app startup status command results", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce({
      running: false,
      currentStage: "failed",
      failedStage: "starting_gateway",
      errorMessage: "  gateway failed  ",
      canRetry: true,
    });

    await expect(appStartupStatusGet()).resolves.toEqual({
      running: false,
      currentStage: "failed",
      failedStage: "starting_gateway",
      errorMessage: "gateway failed",
      canRetry: true,
    });
    expect(tauriInvoke).toHaveBeenCalledWith("app_startup_status_get");
  });

  it("rejects invalid app startup status command results", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce({
      ...validStatus,
      currentStage: "booting",
    });

    await expect(appStartupRetry()).rejects.toThrow("invalid startup.currentStage=booting");
    expect(tauriInvoke).toHaveBeenCalledWith("app_startup_retry");
  });

  it("truncates oversized startup error messages", () => {
    expect(
      normalizeAppStartupStatus({
        ...validStatus,
        errorMessage: ` ${"x".repeat(4097)} `,
      }).errorMessage
    ).toHaveLength(4096);
  });

  it("normalizes startup status events before notifying subscribers", async () => {
    const onStatus = vi.fn();
    const unlisten = await listenAppStartupStatusEvents(onStatus);

    emitTauriEvent(appEventNames.startupStatus, {
      ...validStatus,
      currentStage: "ready",
    });
    unlisten();

    expect(onStatus).toHaveBeenCalledWith({
      ...validStatus,
      currentStage: "ready",
    });
  });

  it("drops malformed startup status events through the desktop event guard", async () => {
    const logSpy = vi.spyOn(consoleLog, "logToConsole").mockImplementation(() => undefined);
    const onStatus = vi.fn();
    const unlisten = await listenAppStartupStatusEvents(onStatus);

    emitTauriEvent(appEventNames.startupStatus, {
      ...validStatus,
      currentStage: "booting",
    });
    unlisten();

    expect(onStatus).not.toHaveBeenCalled();
    expect(logSpy).toHaveBeenCalledWith(
      "warn",
      "桌面事件处理失败",
      {
        event: appEventNames.startupStatus,
        error: "Error: IPC_INVALID_RESULT: invalid startup.currentStage=booting",
      },
      "desktop:event"
    );
  });
});
