import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const { mockLogToConsole } = vi.hoisted(() => ({
  mockLogToConsole: vi.fn(),
}));

vi.mock("../../services/consoleLog", () => ({ logToConsole: mockLogToConsole }));

import { useAsyncListener } from "../useAsyncListener";

describe("hooks/useAsyncListener", () => {
  it("subscribes on mount and cleans up on unmount", async () => {
    const unlisten = vi.fn();
    const subscribe = vi.fn().mockResolvedValue(unlisten);

    const { unmount } = renderHook(() => useAsyncListener(subscribe, "test-stage", "test message"));

    expect(subscribe).toHaveBeenCalledOnce();

    // Wait for promise to resolve
    await vi.waitFor(() => {
      expect(unlisten).not.toHaveBeenCalled();
    });

    unmount();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("calls unlisten immediately if unmounted before resolve", async () => {
    let resolveSubscribe!: (fn: () => void) => void;
    const subscribe = vi.fn(
      () =>
        new Promise<() => void>((r) => {
          resolveSubscribe = r;
        })
    );
    const unlisten = vi.fn();

    const { unmount } = renderHook(() => useAsyncListener(subscribe, "stage", "msg"));

    // Unmount before subscribe resolves
    unmount();

    // Now resolve — unlisten should be called immediately (cancelled path)
    resolveSubscribe(unlisten);
    await vi.waitFor(() => {
      expect(unlisten).toHaveBeenCalledOnce();
    });
  });

  it("logs warning when subscribe rejects", async () => {
    const subscribe = vi.fn().mockRejectedValue(new Error("fail"));

    renderHook(() => useAsyncListener(subscribe, "my-stage", "Something failed"));

    await vi.waitFor(() => {
      expect(mockLogToConsole).toHaveBeenCalledWith(
        "warn",
        "Something failed",
        expect.objectContaining({
          stage: "my-stage",
          error: "Error: fail",
        })
      );
    });
  });

  it("logs warning when subscribe throws before returning a promise", async () => {
    const subscribe = vi.fn(() => {
      throw new Error("sync fail");
    });

    renderHook(() => useAsyncListener(subscribe, "sync-stage", "Sync listener failed"));

    await vi.waitFor(() => {
      expect(mockLogToConsole).toHaveBeenCalledWith(
        "warn",
        "Sync listener failed",
        expect.objectContaining({
          stage: "sync-stage",
          error: "Error: sync fail",
        })
      );
    });
  });
});
