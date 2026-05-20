import { afterEach, describe, expect, it, vi } from "vitest";
import {
  HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY,
  readHomeOverviewLogsPrimaryLayoutFromStorage,
  subscribeHomeOverviewLogsPrimaryLayout,
  writeHomeOverviewLogsPrimaryLayoutToStorage,
} from "../homeOverviewLayout";

const cleanups: Array<() => void> = [];

function subscribe(listener: () => void) {
  const cleanup = subscribeHomeOverviewLogsPrimaryLayout(listener);
  cleanups.push(cleanup);
  return cleanup;
}

describe("services/home/homeOverviewLayout", () => {
  afterEach(() => {
    while (cleanups.length > 0) {
      cleanups.pop()?.();
    }
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("reads and writes the logs-primary layout preference", () => {
    expect(readHomeOverviewLogsPrimaryLayoutFromStorage()).toBe(false);

    writeHomeOverviewLogsPrimaryLayoutToStorage(true);
    expect(window.localStorage.getItem(HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY)).toBe("true");
    expect(readHomeOverviewLogsPrimaryLayoutFromStorage()).toBe(true);

    writeHomeOverviewLogsPrimaryLayoutToStorage(false);
    expect(readHomeOverviewLogsPrimaryLayoutFromStorage()).toBe(false);
  });

  it("keeps subscribers isolated when one listener throws", () => {
    const failingListener = vi.fn(() => {
      throw new Error("boom");
    });
    const healthyListener = vi.fn();

    subscribe(failingListener);
    subscribe(healthyListener);

    expect(() => writeHomeOverviewLogsPrimaryLayoutToStorage(true)).not.toThrow();
    expect(failingListener).toHaveBeenCalledTimes(1);
    expect(healthyListener).toHaveBeenCalledTimes(1);
  });

  it("syncs keyed and cleared storage events only while subscribed", () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    const removeSpy = vi.spyOn(window, "removeEventListener");
    const listener = vi.fn();

    const cleanup = subscribe(listener);
    expect(addSpy).toHaveBeenCalledWith("storage", expect.any(Function));

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: "unrelated",
        storageArea: window.localStorage,
      })
    );
    expect(listener).not.toHaveBeenCalled();

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: null,
        storageArea: window.sessionStorage,
      })
    );
    expect(listener).not.toHaveBeenCalled();

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(1);

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: null,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(2);

    cleanup();
    expect(removeSpy).toHaveBeenCalledWith("storage", expect.any(Function));

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(2);
  });
});
