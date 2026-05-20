import { useMemo, useSyncExternalStore } from "react";
import { emitListenerSnapshot } from "../utils/listeners";

const STORAGE_KEY_DEV_PREVIEW_ENABLED = "devPreview.enabled";

type Listener = () => void;

const listeners = new Set<Listener>();
let storageListenerInstalled = false;

function emit() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

function canUseWindow() {
  return typeof window !== "undefined";
}

function canUseDevPreview() {
  return import.meta.env.DEV;
}

function readDevPreviewEnabled() {
  if (!canUseDevPreview()) return false;

  try {
    return window.localStorage.getItem(STORAGE_KEY_DEV_PREVIEW_ENABLED) === "1";
  } catch {
    return false;
  }
}

function writeDevPreviewEnabled(enabled: boolean) {
  if (!canUseDevPreview()) return;

  try {
    window.localStorage.setItem(STORAGE_KEY_DEV_PREVIEW_ENABLED, enabled ? "1" : "0");
  } catch {}
}

function handleStorageEvent(event: StorageEvent) {
  if (event.key !== null && event.key !== STORAGE_KEY_DEV_PREVIEW_ENABLED) return;
  emit();
}

function ensureStorageListener() {
  if (!canUseWindow() || !canUseDevPreview() || storageListenerInstalled) return;
  window.addEventListener("storage", handleStorageEvent);
  storageListenerInstalled = true;
}

function teardownStorageListener() {
  if (!canUseWindow() || !storageListenerInstalled) return;
  window.removeEventListener("storage", handleStorageEvent);
  storageListenerInstalled = false;
}

export function subscribeDevPreview(listener: Listener) {
  const wasEmpty = listeners.size === 0;
  listeners.add(listener);
  if (wasEmpty && listeners.size > 0) {
    ensureStorageListener();
  }

  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      teardownStorageListener();
    }
  };
}

export function getDevPreviewEnabled() {
  return readDevPreviewEnabled();
}

export function setDevPreviewEnabled(enabled: boolean) {
  writeDevPreviewEnabled(enabled);
  emit();
}

export function toggleDevPreviewEnabled() {
  setDevPreviewEnabled(!readDevPreviewEnabled());
}

export function useDevPreviewData() {
  const enabled = useSyncExternalStore(subscribeDevPreview, readDevPreviewEnabled, () => false);

  return useMemo(
    () => ({
      enabled,
      setEnabled: setDevPreviewEnabled,
      toggle: toggleDevPreviewEnabled,
    }),
    [enabled]
  );
}
