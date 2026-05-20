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
type NotificationSoundListener = () => void;

const listeners = new Set<NotificationSoundListener>();

function emitChange() {
  for (const listener of Array.from(listeners)) {
    if (!listeners.has(listener)) continue;
    try {
      listener();
    } catch (err) {
      logToConsole("warn", "通知音效状态订阅处理失败", { error: String(err) });
    }
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

export function subscribeNotificationSoundEnabled(listener: NotificationSoundListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useNotificationSoundEnabled(): boolean {
  return useSyncExternalStore(subscribeNotificationSoundEnabled, () => enabled);
}

export function playNotificationSound(): void {
  void desktopNotificationPlaySound().catch((err) => {
    logToConsole("warn", "通知音效播放失败", { error: String(err) });
  });
}
