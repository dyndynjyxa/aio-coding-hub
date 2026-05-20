/**
 * Notification Sound module - custom notification sound control
 *
 * Usage:
 * - `setNotificationSoundEnabled(true/false)` to toggle
 * - `useNotificationSoundEnabled()` for React state
 * - `playNotificationSound()` to play the bundled native notification sound
 */

import { useSyncExternalStore } from "react";

import { logToConsole } from "../consoleLog";
import { desktopNotificationPlaySound } from "../desktop/notification";

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
  void desktopNotificationPlaySound().catch((err) => {
    logToConsole("warn", "通知音效播放失败", { error: String(err) });
  });
}
