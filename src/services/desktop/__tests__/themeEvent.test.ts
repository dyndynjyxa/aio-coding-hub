import { describe, expect, it, vi } from "vitest";
import * as consoleLog from "../../consoleLog";
import { listenThemeChanged } from "../themeEvent";

const windowMocks = vi.hoisted(() => ({
  onThemeChanged: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onThemeChanged: windowMocks.onThemeChanged,
  }),
}));

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

describe("services/desktop/themeEvent", () => {
  it("narrows native theme payloads before invoking the handler", async () => {
    const unlisten = vi.fn();
    let callback: (event: { payload: unknown }) => void = () => undefined;
    windowMocks.onThemeChanged.mockImplementation(async (handler) => {
      callback = handler;
      return unlisten;
    });
    const handler = vi.fn();

    const cleanup = await listenThemeChanged(handler);
    callback({ payload: "dark" });
    callback({ payload: "sepia" });
    cleanup();

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith("dark");
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("logs theme handler failures without rejecting the native callback", async () => {
    const logSpy = vi.spyOn(consoleLog, "logToConsole").mockImplementation(() => undefined);
    let callback: (event: { payload: unknown }) => void = () => undefined;
    windowMocks.onThemeChanged.mockImplementation(async (handler) => {
      callback = handler;
      return vi.fn();
    });

    await listenThemeChanged(() => Promise.reject(new Error("theme boom")));
    callback({ payload: "light" });
    await Promise.resolve();

    expect(logSpy).toHaveBeenCalledWith(
      "warn",
      "系统主题事件处理失败",
      { error: "Error: theme boom" },
      "desktop:theme"
    );
  });

  it("logs thenable theme handler failures without rejecting the native callback", async () => {
    const logSpy = vi.spyOn(consoleLog, "logToConsole").mockImplementation(() => undefined);
    let callback: (event: { payload: unknown }) => void = () => undefined;
    windowMocks.onThemeChanged.mockImplementation(async (handler) => {
      callback = handler;
      return vi.fn();
    });

    await listenThemeChanged(() => rejectedThenable("theme thenable boom"));
    expect(() => callback({ payload: "dark" })).not.toThrow();

    await vi.waitFor(() => {
      expect(logSpy).toHaveBeenCalledWith(
        "warn",
        "系统主题事件处理失败",
        { error: "Error: theme thenable boom" },
        "desktop:theme"
      );
    });
  });
});
