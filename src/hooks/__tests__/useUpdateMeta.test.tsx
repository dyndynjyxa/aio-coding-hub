import { act, renderHook } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { tauriInvoke } from "../../test/mocks/tauri";
import { clearTauriRuntime, setTauriRuntime } from "../../test/utils/tauriRuntime";
import { createDeferred } from "../../test/utils/deferred";
import { updaterKeys } from "../../query/keys";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

describe("hooks/useUpdateMeta", () => {
  beforeEach(() => {
    localStorage.removeItem("devPreview.enabled");
  });

  it("uses the shared dev preview toggle to return a mock update and block install", async () => {
    vi.resetModules();
    clearTauriRuntime();
    localStorage.setItem("devPreview.enabled", "1");

    const { queryClient } = await import("../../query/queryClient");
    queryClient.clear();

    const mod = await import("../useUpdateMeta");
    const { updateCheckNow, updateDownloadAndInstall, updateDialogSetOpen, useUpdateMeta } = mod;

    const update = await updateCheckNow({ silent: true, openDialogIfUpdate: true });
    expect(update?.rid).toBe(9_999_001);
    expect(update?.body).toContain("Dev 预览更新日志");

    const wrapper = ({ children }: any) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useUpdateMeta(), { wrapper });
    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.dialogOpen).toBe(true);
    expect(result.current.updateCandidate?.rid).toBe(9_999_001);
    expect(result.current.updateCandidate?.body).toContain("不会参与真实安装");

    queryClient.setQueryData(updaterKeys.check(), update);
    updateDialogSetOpen(true);
    await expect(updateDownloadAndInstall()).resolves.toBe(false);
    expect(toast).toHaveBeenCalledWith("Dev 预览更新仅用于展示，不能安装");

    localStorage.removeItem("devPreview.enabled");
  });

  it("covers update check, dialog state, and download/install flows", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-02-01T00:00:00Z"));

    vi.resetModules();
    clearTauriRuntime();

    const { queryClient } = await import("../../query/queryClient");
    queryClient.clear();

    const mod = await import("../useUpdateMeta");
    const { updateCheckNow, updateDownloadAndInstall, updateDialogSetOpen, useUpdateMeta } = mod;

    // no runtime -> null and no toast
    expect(await updateCheckNow({ silent: false, openDialogIfUpdate: true })).toBeNull();

    setTauriRuntime();

    const checkResults: any[] = [false, { rid: 1, version: "v1", currentVersion: "v0" }];

    const installDeferred = createDeferred<unknown>();

    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "plugin:updater|check") {
        return checkResults.shift() ?? false;
      }
      if (cmd === "plugin:updater|download_and_install") {
        args?.onEvent?.__emit?.({ event: "started", data: { contentLength: 100 } });
        args?.onEvent?.__emit?.({ event: "progress", data: { chunkLength: 10 } });
        args?.onEvent?.__emit?.({ event: "progress", data: { chunkLength: 5 } });
        args?.onEvent?.__emit?.({ event: "finished", data: { ok: true } });
        return installDeferred.promise as any;
      }
      return null as any;
    });

    // latest -> toast
    expect(await updateCheckNow({ silent: false, openDialogIfUpdate: false })).toBeNull();
    expect(toast).toHaveBeenCalledWith("已是最新版本");

    // update available -> open dialog
    const update = await updateCheckNow({ silent: true, openDialogIfUpdate: true });
    expect(update?.rid).toBe(1);

    const wrapper = ({ children }: any) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useUpdateMeta(), { wrapper });
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.dialogOpen).toBe(true);

    // prepare candidate in cache (updateDownloadAndInstall reads queryClient)
    queryClient.setQueryData(updaterKeys.check(), update);

    // cannot close while installing
    const installingPromise = updateDownloadAndInstall();
    await act(async () => {
      await Promise.resolve();
    });
    updateDialogSetOpen(false);
    expect(result.current.dialogOpen).toBe(true);

    installDeferred.resolve(true);
    expect(await installingPromise).toBe(true);

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.installTotalBytes).toBe(100);
    expect(result.current.installDownloadedBytes).toBe(15);

    // close resets install progress
    updateDialogSetOpen(false);
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.dialogOpen).toBe(false);
    expect(result.current.installDownloadedBytes).toBe(0);
    expect(result.current.installTotalBytes).toBeNull();

    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("sets installError when download/install throws", async () => {
    vi.useFakeTimers();
    vi.resetModules();
    setTauriRuntime();

    const { queryClient } = await import("../../query/queryClient");
    queryClient.clear();

    const mod = await import("../useUpdateMeta");
    const { updateDownloadAndInstall, updateDialogSetOpen, useUpdateMeta } = mod;

    queryClient.setQueryData(updaterKeys.check(), { rid: 2 } as any);

    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string) => {
      if (cmd === "plugin:updater|download_and_install") throw new Error("boom");
      return null as any;
    });

    const wrapper = ({ children }: any) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useUpdateMeta(), { wrapper });

    updateDialogSetOpen(true);
    await act(async () => {
      await Promise.resolve();
    });

    expect(await updateDownloadAndInstall()).toBe(false);
    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.installError).toBe("Error: boom");
    expect(toast).toHaveBeenCalledWith("安装更新失败：请稍后重试");

    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it("logs and toasts when update check fails even if localStorage write also throws", async () => {
    vi.resetModules();
    setTauriRuntime();

    const setItemSpy = vi.spyOn(Storage.prototype, "setItem");
    setItemSpy.mockImplementation(() => {
      throw new Error("quota");
    });

    const { queryClient } = await import("../../query/queryClient");
    queryClient.clear();

    const { updateCheckNow } = await import("../useUpdateMeta");

    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string) => {
      if (cmd === "plugin:updater|check") throw new Error("check boom");
      return null as any;
    });

    expect(await updateCheckNow({ silent: false, openDialogIfUpdate: false })).toBeNull();
    expect(toast).toHaveBeenCalledWith("检查更新失败：Error: check boom");

    setItemSpy.mockRestore();
  });

  it("returns null without candidate and reuses the in-flight install promise", async () => {
    vi.resetModules();
    setTauriRuntime();

    const { queryClient } = await import("../../query/queryClient");
    queryClient.clear();

    const mod = await import("../useUpdateMeta");
    const { updateDownloadAndInstall, useUpdateMeta } = mod;

    const wrapper = ({ children }: any) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useUpdateMeta(), { wrapper });

    expect(await updateDownloadAndInstall()).toBeNull();

    const installDeferred = createDeferred<boolean>();
    let downloadCalls = 0;
    queryClient.setQueryData(updaterKeys.check(), { rid: 3 } as any);

    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "plugin:updater|download_and_install") {
        downloadCalls += 1;
        args?.onEvent?.__emit?.({ event: "started", data: {} });
        args?.onEvent?.__emit?.({ event: "progress", data: { chunkLength: 0 } });
        return installDeferred.promise as any;
      }
      return null as any;
    });

    const firstPromise = updateDownloadAndInstall();
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.installTotalBytes).toBeNull();

    const secondPromise = updateDownloadAndInstall();
    expect(downloadCalls).toBe(1);

    installDeferred.resolve(true);
    await expect(firstPromise).resolves.toBe(true);
    await expect(secondPromise).resolves.toBe(true);
  });
});
