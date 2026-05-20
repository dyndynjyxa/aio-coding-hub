import { beforeEach, describe, expect, it, vi } from "vitest";
import { appEventNames } from "../../../constants/appEvents";
import { tauriInvoke, tauriListen, tauriUnlisten } from "../../../test/mocks/tauri";

const logToConsoleMock = vi.hoisted(() => vi.fn());
const getNotificationSoundEnabledMock = vi.hoisted(() => vi.fn());
const playNotificationSoundMock = vi.hoisted(() => vi.fn());

vi.mock("../../consoleLog", () => ({
  logToConsole: logToConsoleMock,
}));

vi.mock("../notificationSound", () => ({
  getNotificationSoundEnabled: getNotificationSoundEnabledMock,
  playNotificationSound: playNotificationSoundMock,
}));

describe("services/notification/noticeEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("listens and sends notifications when permission is granted", async () => {
    vi.resetModules();

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);
    vi.mocked(tauriInvoke).mockImplementation(async (command: string) => {
      if (command === "desktop_notification_is_permission_granted") return true;
      return undefined;
    });
    getNotificationSoundEnabledMock.mockReturnValue(true);

    const { listenNoticeEvents } = await import("../noticeEvents");
    const unlisten = await listenNoticeEvents();

    expect(tauriListen).toHaveBeenCalledWith(appEventNames.notice, expect.any(Function));

    const handler = vi
      .mocked(tauriListen)
      .mock.calls.find((c) => c[0] === appEventNames.notice)?.[1];
    expect(handler).toBeTypeOf("function");

    handler?.({ payload: { level: "info", title: "T", body: "B" } } as any);
    await vi.waitFor(() => {
      expect(playNotificationSoundMock).toHaveBeenCalledTimes(1);
      expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_is_permission_granted");
      expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_notify", {
        options: { title: "T", body: "B", sound: null },
      });
    });

    unlisten();
    expect(tauriUnlisten).toHaveBeenCalled();
  });

  it("does not send notifications when permission is denied", async () => {
    vi.resetModules();

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);
    vi.mocked(tauriInvoke).mockImplementation(async (command: string) => {
      if (command === "desktop_notification_is_permission_granted") return false;
      return undefined;
    });
    getNotificationSoundEnabledMock.mockReturnValue(true);

    const { listenNoticeEvents } = await import("../noticeEvents");
    await listenNoticeEvents();

    const handler = vi
      .mocked(tauriListen)
      .mock.calls.find((c) => c[0] === appEventNames.notice)?.[1];
    handler?.({ payload: { level: "info", title: "T", body: "B" } } as any);
    await vi.waitFor(() => {
      expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_is_permission_granted");
    });

    expect(playNotificationSoundMock).not.toHaveBeenCalled();
    expect(tauriInvoke).not.toHaveBeenCalledWith("desktop_notification_notify", expect.anything());
  });

  it("logs error when sendNotification throws", async () => {
    vi.resetModules();

    vi.mocked(tauriListen).mockResolvedValue(tauriUnlisten);
    vi.mocked(tauriInvoke).mockImplementation(async (command: string) => {
      if (command === "desktop_notification_is_permission_granted") return true;
      if (command === "desktop_notification_notify") throw new Error("notification failed");
      return undefined;
    });
    getNotificationSoundEnabledMock.mockReturnValue(true);

    const { listenNoticeEvents } = await import("../noticeEvents");
    await listenNoticeEvents();

    const handler = vi
      .mocked(tauriListen)
      .mock.calls.find((c) => c[0] === appEventNames.notice)?.[1];
    expect(handler).toBeTypeOf("function");

    handler?.({ payload: { level: "error", title: "T", body: "B" } } as any);

    await vi.waitFor(() => {
      expect(logToConsoleMock).toHaveBeenCalledWith(
        "error",
        "发送系统通知失败",
        expect.objectContaining({
          error: expect.stringContaining("notification failed"),
          level: "error",
          title: "T",
        })
      );
      expect(playNotificationSoundMock).toHaveBeenCalledTimes(1);
    });
  });
});
