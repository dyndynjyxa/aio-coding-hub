import { afterEach, describe, expect, it, vi } from "vitest";
import {
  HOME_OVERVIEW_TABS,
  HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY,
  normalizeHomeOverviewTabOrder,
  readHomeOverviewTabOrderFromStorage,
  writeHomeOverviewTabOrderToStorage,
} from "../homeOverviewTabOrder";

const defaultOrder = HOME_OVERVIEW_TABS.map((item) => item.key);

describe("services/home/homeOverviewTabOrder", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("normalizes invalid, duplicated, and missing tab keys", () => {
    expect(normalizeHomeOverviewTabOrder("bad")).toEqual(defaultOrder);

    expect(
      normalizeHomeOverviewTabOrder([
        "sessions",
        "sessions",
        "workspaceConfig",
        "bad-key",
        "providerLimit",
      ])
    ).toEqual(["sessions", "workspaceConfig", "providerLimit", "circuit"]);
  });

  it("reads tab order from storage and falls back for empty or malformed values", () => {
    expect(readHomeOverviewTabOrderFromStorage()).toEqual(defaultOrder);

    window.localStorage.setItem(
      HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY,
      JSON.stringify(["providerLimit", "workspaceConfig", "providerLimit"])
    );
    expect(readHomeOverviewTabOrderFromStorage()).toEqual([
      "providerLimit",
      "workspaceConfig",
      "circuit",
      "sessions",
    ]);

    window.localStorage.setItem(HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY, "{bad json");
    expect(readHomeOverviewTabOrderFromStorage()).toEqual(defaultOrder);
  });

  it("returns defaults when window is unavailable", () => {
    const originalWindow = globalThis.window;

    Object.defineProperty(globalThis, "window", {
      value: undefined,
      configurable: true,
      writable: true,
    });

    try {
      expect(readHomeOverviewTabOrderFromStorage()).toEqual(defaultOrder);
      expect(() => writeHomeOverviewTabOrderToStorage(["sessions"])).not.toThrow();
    } finally {
      Object.defineProperty(globalThis, "window", {
        value: originalWindow,
        configurable: true,
        writable: true,
      });
    }
  });

  it("writes normalized order to storage and swallows storage errors", () => {
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem");

    writeHomeOverviewTabOrderToStorage([
      "sessions",
      "providerLimit",
      "sessions",
      "workspaceConfig",
    ]);

    expect(setItemSpy).toHaveBeenCalledWith(
      HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY,
      JSON.stringify(["sessions", "providerLimit", "workspaceConfig", "circuit"])
    );

    setItemSpy.mockImplementation(() => {
      throw new Error("blocked");
    });

    expect(() => writeHomeOverviewTabOrderToStorage(["circuit", "sessions"])).not.toThrow();
  });
});
