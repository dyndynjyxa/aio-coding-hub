import { describe, expect, it, vi } from "vitest";
import * as consoleLog from "../../consoleLog";
import { emitTauriEvent, tauriUnlisten } from "../../../test/mocks/tauri";
import { listenDesktopEvent } from "../event";

function rejectedThenable(message: string): PromiseLike<void> {
  return {
    then<TResult1 = void, TResult2 = never>(
      onfulfilled?: ((value: void) => TResult1 | PromiseLike<TResult1>) | null,
      onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null
    ): PromiseLike<TResult1 | TResult2> {
      return Promise.reject(new Error(message)).then(onfulfilled, onrejected);
    },
  };
}

describe("services/desktop/event", () => {
  it("subscribes to desktop events and unregisters cleanly", async () => {
    const handler = vi.fn();

    const unlisten = await listenDesktopEvent<{ value: number }>("desktop:test", handler);
    emitTauriEvent("desktop:test", { value: 1 });
    unlisten();
    emitTauriEvent("desktop:test", { value: 2 });

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith({ value: 1 });
    expect(tauriUnlisten).toHaveBeenCalledTimes(1);
  });

  it("logs handler failures without rejecting the desktop event callback", async () => {
    const logSpy = vi.spyOn(consoleLog, "logToConsole").mockImplementation(() => undefined);
    const handler = vi.fn().mockRejectedValue(new Error("boom"));

    const unlisten = await listenDesktopEvent("desktop:test-error", handler);
    emitTauriEvent("desktop:test-error", { value: 1 });
    await Promise.resolve();

    expect(logSpy).toHaveBeenCalledWith(
      "warn",
      "桌面事件处理失败",
      { event: "desktop:test-error", error: "Error: boom" },
      "desktop:event"
    );

    unlisten();
  });

  it("logs thenable handler rejections without rejecting the desktop event callback", async () => {
    const logSpy = vi.spyOn(consoleLog, "logToConsole").mockImplementation(() => undefined);
    const handler = vi.fn(() => rejectedThenable("thenable boom"));

    const unlisten = await listenDesktopEvent("desktop:test-thenable-error", handler);
    expect(() => emitTauriEvent("desktop:test-thenable-error", { value: 1 })).not.toThrow();

    await vi.waitFor(() => {
      expect(logSpy).toHaveBeenCalledWith(
        "warn",
        "桌面事件处理失败",
        { event: "desktop:test-thenable-error", error: "Error: thenable boom" },
        "desktop:event"
      );
    });

    unlisten();
  });
});
