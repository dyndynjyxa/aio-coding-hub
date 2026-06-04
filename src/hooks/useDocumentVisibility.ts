import { useSyncExternalStore } from "react";
import { emitListenerSnapshot } from "../utils/listeners";

function isDocumentVisible() {
  return typeof document === "undefined" ? true : document.visibilityState === "visible";
}

type Listener = () => void;

const listeners = new Set<Listener>();

function emit() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

export function subscribeDocumentVisibility(listener: Listener) {
  if (typeof document === "undefined") return () => {};

  const wasEmpty = listeners.size === 0;
  listeners.add(listener);
  if (wasEmpty) {
    document.addEventListener("visibilitychange", emit);
  }

  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      document.removeEventListener("visibilitychange", emit);
    }
  };
}

export function useDocumentVisibility() {
  return useSyncExternalStore(subscribeDocumentVisibility, isDocumentVisible, () => true);
}
