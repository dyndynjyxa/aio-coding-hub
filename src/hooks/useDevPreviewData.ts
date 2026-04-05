import { useMemo, useSyncExternalStore } from "react";

const STORAGE_KEY_DEV_PREVIEW_ENABLED = "devPreview.enabled";

type Listener = () => void;

const listeners = new Set<Listener>();

function emit() {
  for (const listener of listeners) listener();
}

function canUseDevPreview() {
  return import.meta.env.DEV;
}

function readDevPreviewEnabled() {
  if (!canUseDevPreview()) return false;

  try {
    return localStorage.getItem(STORAGE_KEY_DEV_PREVIEW_ENABLED) === "1";
  } catch {
    return false;
  }
}

function writeDevPreviewEnabled(enabled: boolean) {
  if (!canUseDevPreview()) return;

  try {
    localStorage.setItem(STORAGE_KEY_DEV_PREVIEW_ENABLED, enabled ? "1" : "0");
  } catch {}
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
  const enabled = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    readDevPreviewEnabled,
    () => false
  );

  return useMemo(
    () => ({
      enabled,
      setEnabled: setDevPreviewEnabled,
      toggle: toggleDevPreviewEnabled,
    }),
    [enabled]
  );
}
