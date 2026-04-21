import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getNotificationSoundEnabled,
  playNotificationSound,
  setNotificationSoundEnabled,
} from "../notificationSound";

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return { ...actual, logToConsole: vi.fn() };
});

afterEach(() => {
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

  it("playNotificationSound handles Audio errors gracefully", () => {
    // JSDOM doesn't implement Audio — stub it to throw
    const origAudio = globalThis.Audio;
    globalThis.Audio = class {
      play() {
        return Promise.reject(new Error("play rejected"));
      }
      set currentTime(_v: number) {}
    } as unknown as typeof Audio;

    expect(() => playNotificationSound()).not.toThrow();

    globalThis.Audio = origAudio;
  });

  it("playNotificationSound handles constructor errors gracefully", () => {
    const origAudio = globalThis.Audio;
    globalThis.Audio = class {
      constructor() {
        throw new Error("Audio unavailable");
      }
    } as unknown as typeof Audio;

    expect(() => playNotificationSound()).not.toThrow();

    globalThis.Audio = origAudio;
  });
});
