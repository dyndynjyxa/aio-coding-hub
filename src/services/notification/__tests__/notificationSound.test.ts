import { afterEach, describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { desktopNotificationPlaySound } from "../../desktop/notification";
import {
  getNotificationSoundEnabled,
  playNotificationSound,
  setNotificationSoundEnabled,
  subscribeNotificationSoundEnabled,
} from "../notificationSound";

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return { ...actual, logToConsole: vi.fn() };
});

vi.mock("../../desktop/notification", () => ({
  desktopNotificationPlaySound: vi.fn(),
}));

afterEach(() => {
  vi.clearAllMocks();
  // reset module-level state to default
  setNotificationSoundEnabled(true);
});

describe("services/notification/notificationSound", () => {
  it("getNotificationSoundEnabled returns current state", () => {
    expect(getNotificationSoundEnabled()).toBe(true);
    setNotificationSoundEnabled(false);
    expect(getNotificationSoundEnabled()).toBe(false);
  });

  it("setNotificationSoundEnabled is idempotent when value unchanged", () => {
    setNotificationSoundEnabled(true);
    setNotificationSoundEnabled(true); // should not emit
    expect(getNotificationSoundEnabled()).toBe(true);
  });

  it("isolates notification sound subscribers when one throws", () => {
    const throwingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeThrowing = subscribeNotificationSoundEnabled(throwingListener);
    const unsubscribeHealthy = subscribeNotificationSoundEnabled(healthyListener);

    try {
      setNotificationSoundEnabled(false);

      expect(throwingListener).toHaveBeenCalledTimes(1);
      expect(healthyListener).toHaveBeenCalledTimes(1);
      expect(logToConsole).toHaveBeenCalledWith("warn", "通知音效状态订阅处理失败", {
        error: "Error: listener boom",
      });
    } finally {
      unsubscribeThrowing();
      unsubscribeHealthy();
    }
  });

  it("playNotificationSound delegates playback to the native desktop service", async () => {
    const origAudio = globalThis.Audio;
    globalThis.Audio = vi.fn(() => {
      throw new Error("browser Audio must not be used");
    }) as unknown as typeof Audio;
    vi.mocked(desktopNotificationPlaySound).mockResolvedValueOnce();

    expect(() => playNotificationSound()).not.toThrow();
    await vi.waitFor(() => {
      expect(desktopNotificationPlaySound).toHaveBeenCalledTimes(1);
    });
    expect(globalThis.Audio).not.toHaveBeenCalled();

    globalThis.Audio = origAudio;
  });

  it("playNotificationSound logs native playback failures without throwing", async () => {
    vi.mocked(desktopNotificationPlaySound).mockRejectedValueOnce(new Error("native unavailable"));

    expect(() => playNotificationSound()).not.toThrow();

    await vi.waitFor(() => {
      expect(logToConsole).toHaveBeenCalledWith("warn", "通知音效播放失败", {
        error: "Error: native unavailable",
      });
    });
  });
});
