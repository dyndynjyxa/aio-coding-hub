/**
 * Notification Sound module - custom notification sound control
 *
 * Usage:
 * - `setNotificationSoundEnabled(true/false)` to toggle
 * - `useNotificationSoundEnabled()` for React state
 * - `playNotificationSound()` to play ding.mp3
 */

import { useSyncExternalStore } from "react";

import { logToConsole } from "../consoleLog";

let enabled = true;
const listeners = new Set<() => void>();

function emitChange() {
  for (const listener of listeners) {
    listener();
  }
}

export function setNotificationSoundEnabled(value: boolean) {
  if (enabled === value) return;
  enabled = value;
  emitChange();
}

export function getNotificationSoundEnabled(): boolean {
  return enabled;
}

export function useNotificationSoundEnabled(): boolean {
  return useSyncExternalStore(
    (callback) => {
      listeners.add(callback);
      return () => {
        listeners.delete(callback);
      };
    },
    () => enabled
  );
}

export function playNotificationSound(): void {
  try {
    // Always create a fresh Audio instance and discard after playback.
    // Do NOT cache the HTMLAudioElement — on macOS, a live Audio element causes
    // the app to register as a "Now Playing" media session, which hijacks the
    // system media keys (F7/F8/F9) away from the user's music player.
    const audio = new Audio("/ding.mp3");
    audio.currentTime = 0;
    const playPromise = audio.play();
    playPromise
      ?.then(() => {
        // Release the audio element after playback ends so macOS drops the media session.
        audio.addEventListener(
          "ended",
          () => {
            audio.src = "";
            audio.load();
          },
          { once: true }
        );
      })
      .catch((err) => {
        logToConsole("warn", "通知音效播放失败", { error: String(err) });
      });
  } catch (err) {
    logToConsole("warn", "通知音效创建失败", { error: String(err) });
  }
}
