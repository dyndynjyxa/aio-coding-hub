import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import { noticeSend } from "../../../services/notification/notice";
import { tauriInvoke } from "../../../test/mocks/tauri";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../../services/notification/notice", async () => {
  const actual = await vi.importActual<typeof import("../../../services/notification/notice")>(
    "../../../services/notification/notice"
  );
  return { ...actual, noticeSend: vi.fn() };
});

function mockInvoke(overrides: {
  isPermissionGranted?: boolean | Error;
  requestPermission?: string;
}) {
  vi.mocked(tauriInvoke).mockImplementation(async (command: string) => {
    if (command === "plugin:notification|is_permission_granted") {
      if (overrides.isPermissionGranted instanceof Error) throw overrides.isPermissionGranted;
      return overrides.isPermissionGranted ?? false;
    }
    if (command === "plugin:notification|request_permission") {
      return overrides.requestPermission ?? "denied";
    }
    return undefined;
  });
}

describe("pages/settings/useSystemNotification", () => {
  it("loads permission status on mount and requests permission", async () => {
    vi.resetModules();
    mockInvoke({ isPermissionGranted: false, requestPermission: "granted" });

    const { useSystemNotification } = await import("../useSystemNotification");

    const { result } = renderHook(() => useSystemNotification());
    await waitFor(() => expect(result.current.noticePermissionStatus).toBe("not_granted"));

    await act(async () => {
      await result.current.requestSystemNotificationPermission();
    });
    expect(result.current.noticePermissionStatus).toBe("granted");
    expect(toast).toHaveBeenCalledWith("系统通知权限已授权");
  });

  it("handles permission check failures", async () => {
    vi.resetModules();
    mockInvoke({ isPermissionGranted: new Error("nope") });

    const { useSystemNotification } = await import("../useSystemNotification");

    const { result } = renderHook(() => useSystemNotification());
    await waitFor(() => expect(result.current.noticePermissionStatus).toBe("unknown"));
    expect(logToConsole).toHaveBeenCalledWith("error", "检查系统通知权限失败", {
      error: "Error: nope",
    });
  });

  it("toasts when sending test without permission", async () => {
    vi.resetModules();
    mockInvoke({ isPermissionGranted: false });

    const { useSystemNotification } = await import("../useSystemNotification");
    const { result } = renderHook(() => useSystemNotification());
    await waitFor(() => expect(result.current.noticePermissionStatus).toBe("not_granted"));

    await act(async () => {
      await result.current.sendSystemNotificationTest();
    });
    expect(toast).toHaveBeenCalledWith("请先在「系统通知」中授权通知权限");
  });

  it("toasts when notice_send unavailable and when sending succeeds", async () => {
    vi.mocked(noticeSend).mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    vi.resetModules();
    mockInvoke({ isPermissionGranted: true });

    const { useSystemNotification } = await import("../useSystemNotification");

    const { result } = renderHook(() => useSystemNotification());
    await waitFor(() => expect(result.current.noticePermissionStatus).toBe("granted"));

    await act(async () => {
      await result.current.sendSystemNotificationTest();
    });

    await act(async () => {
      await result.current.sendSystemNotificationTest();
    });
    expect(toast).toHaveBeenCalledWith("已发送测试通知");
  });
});
