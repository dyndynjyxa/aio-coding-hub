import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSidebarState } from "../useSidebarState";

describe("hooks/useSidebarState", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("falls back safely when localStorage throws", () => {
    const getStorageSpy = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const setStorageSpy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });

    const getSpy = vi.spyOn(window.localStorage, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const setSpy = vi.spyOn(window.localStorage, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });

    const { result } = renderHook(() => useSidebarState());

    expect(result.current.isOpen).toBe(true);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isOpen).toBe(false);

    expect(getStorageSpy).toHaveBeenCalled();
    expect(setStorageSpy).toHaveBeenCalled();

    getSpy.mockRestore();
    setSpy.mockRestore();
  });
});
