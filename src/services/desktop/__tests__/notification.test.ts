import { describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import {
  desktopNotificationIsPermissionGranted,
  desktopNotificationNotify,
  desktopNotificationPlaySound,
  desktopNotificationRequestPermission,
} from "../notification";

describe("services/desktop/notification", () => {
  it("maps permission check results to a strict boolean", async () => {
    vi.mocked(tauriInvoke)
      .mockResolvedValueOnce(true as any)
      .mockResolvedValueOnce(null);

    await expect(desktopNotificationIsPermissionGranted()).resolves.toBe(true);
    await expect(desktopNotificationIsPermissionGranted()).resolves.toBe(false);

    expect(tauriInvoke).toHaveBeenNthCalledWith(1, "desktop_notification_is_permission_granted");
    expect(tauriInvoke).toHaveBeenNthCalledWith(2, "desktop_notification_is_permission_granted");
  });

  it("falls back to denied when permission request returns null", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce(null);

    await expect(desktopNotificationRequestPermission()).resolves.toBe("denied");
    expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_request_permission");
  });

  it("keeps the backend permission status when the request succeeds", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce("prompt-with-rationale");

    await expect(desktopNotificationRequestPermission()).resolves.toBe("prompt-with-rationale");
    expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_request_permission");
  });

  it("rejects invalid backend permission states", async () => {
    vi.mocked(tauriInvoke).mockResolvedValueOnce("unknown-state");

    await expect(desktopNotificationRequestPermission()).rejects.toThrow(
      "invalid desktop notification permission=unknown-state"
    );
  });

  it("sends notification payloads with optional sound only when provided", async () => {
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    await desktopNotificationNotify({
      title: "Build finished",
      body: "All checks passed",
      sound: "Glass",
    });
    await desktopNotificationNotify({
      title: "Build finished",
      body: "All checks passed",
    });

    expect(tauriInvoke).toHaveBeenNthCalledWith(1, "desktop_notification_notify", {
      options: {
        title: "Build finished",
        body: "All checks passed",
        sound: "Glass",
      },
    });
    expect(tauriInvoke).toHaveBeenNthCalledWith(2, "desktop_notification_notify", {
      options: {
        title: "Build finished",
        body: "All checks passed",
        sound: null,
      },
    });
  });

  it("normalizes notification payloads before invoking the backend", async () => {
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    await desktopNotificationNotify({
      title: "  Build finished  ",
      body: "  All checks passed  ",
      sound: "  default  ",
    });
    await desktopNotificationNotify({
      title: "  Build finished  ",
      body: "  All checks passed  ",
      sound: "   ",
    });

    expect(tauriInvoke).toHaveBeenNthCalledWith(1, "desktop_notification_notify", {
      options: {
        title: "Build finished",
        body: "All checks passed",
        sound: "default",
      },
    });
    expect(tauriInvoke).toHaveBeenNthCalledWith(2, "desktop_notification_notify", {
      options: {
        title: "Build finished",
        body: "All checks passed",
        sound: null,
      },
    });
  });

  it("rejects invalid notification payloads before invoking the backend", async () => {
    await expect(
      desktopNotificationNotify({ title: "   ", body: "All checks passed" })
    ).rejects.toThrow("title is required");
    await expect(
      desktopNotificationNotify({ title: "Build finished", body: "x".repeat(4097) })
    ).rejects.toThrow("body is too long");
    await expect(
      desktopNotificationNotify({
        title: "Build finished",
        body: "All checks passed",
        sound: "x".repeat(129),
      })
    ).rejects.toThrow("sound is too long");

    expect(tauriInvoke).not.toHaveBeenCalled();
  });

  it("delegates custom notification sound playback to the backend", async () => {
    vi.mocked(tauriInvoke).mockResolvedValue(true as any);

    await expect(desktopNotificationPlaySound()).resolves.toBeUndefined();

    expect(tauriInvoke).toHaveBeenCalledWith("desktop_notification_play_sound");
  });
});
