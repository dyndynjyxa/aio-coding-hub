import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const toastMock = vi.hoisted(() => vi.fn());
const logToConsoleMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock("sonner", () => ({ toast: toastMock }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: logToConsoleMock }));
vi.mock("../../../services/providers/providerEvents", () => ({
  listenProviderCodexCatalogEvents: listenMock,
}));

import { useCodexCatalogRefreshFeedback } from "../hooks/useCodexCatalogRefreshFeedback";

describe("useCodexCatalogRefreshFeedback", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows the restart and recovery messages", async () => {
    let onEvent: ((payload: { status: "updated" | "failed" }) => void) | null = null;
    const cleanup = vi.fn();
    listenMock.mockImplementation(async (handler) => {
      onEvent = handler;
      return cleanup;
    });

    const { unmount } = renderHook(() => useCodexCatalogRefreshFeedback());
    await act(async () => undefined);

    act(() => onEvent?.({ status: "updated" }));
    act(() => onEvent?.({ status: "failed" }));
    expect(toastMock).toHaveBeenNthCalledWith(1, "模型映射已更新，重启 Codex 后生效");
    expect(toastMock).toHaveBeenNthCalledWith(
      2,
      "Codex 模型目录更新失败，请检查 Codex CLI、目录文件和权限后重试"
    );

    unmount();
    expect(cleanup).toHaveBeenCalledTimes(1);
  });

  it("cleans up when unmounted before listener initialization completes", async () => {
    let resolveListener: ((cleanup: () => void) => void) | null = null;
    const cleanup = vi.fn();
    listenMock.mockReturnValue(
      new Promise<() => void>((resolve) => {
        resolveListener = resolve;
      })
    );

    const { unmount } = renderHook(() => useCodexCatalogRefreshFeedback());
    unmount();
    await act(async () => resolveListener?.(cleanup));

    expect(cleanup).toHaveBeenCalledTimes(1);
  });

  it("logs listener initialization failures", async () => {
    listenMock.mockRejectedValue(new Error("listen failed"));

    renderHook(() => useCodexCatalogRefreshFeedback());
    await act(async () => undefined);

    expect(logToConsoleMock).toHaveBeenCalledWith("warn", "监听 Codex 模型目录状态失败", {
      error: "Error: listen failed",
    });
  });
});
