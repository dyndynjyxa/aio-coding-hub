import { useEffect, useRef } from "react";

export type UseWindowForegroundOptions = {
  enabled: boolean;
  onForeground: () => void;
  throttleMs?: number;
};

type RefBox<T> = {
  current: T;
};

type ForegroundSubscriber = {
  onForegroundRef: RefBox<() => void>;
  throttleMsRef: RefBox<number>;
  lastFiredAtMsRef: RefBox<number>;
};

const foregroundSubscribers = new Set<ForegroundSubscriber>();
let foregroundListenersInstalled = false;

function maybeFireSubscriber(subscriber: ForegroundSubscriber) {
  const now = Date.now();
  const throttle = subscriber.throttleMsRef.current;
  if (Number.isFinite(throttle) && throttle > 0) {
    const elapsed = now - subscriber.lastFiredAtMsRef.current;
    if (elapsed >= 0 && elapsed < throttle) return;
  }
  subscriber.lastFiredAtMsRef.current = now;
  subscriber.onForegroundRef.current();
}

function emitForeground() {
  for (const subscriber of Array.from(foregroundSubscribers)) {
    if (foregroundSubscribers.has(subscriber)) {
      maybeFireSubscriber(subscriber);
    }
  }
}

function handleFocus() {
  emitForeground();
}

function handleVisibilityChange() {
  if (typeof document === "undefined" || document.visibilityState === "visible") {
    emitForeground();
  }
}

function installForegroundListeners() {
  if (
    foregroundListenersInstalled ||
    typeof window === "undefined" ||
    typeof document === "undefined"
  ) {
    return;
  }
  window.addEventListener("focus", handleFocus);
  document.addEventListener("visibilitychange", handleVisibilityChange);
  foregroundListenersInstalled = true;
}

function uninstallForegroundListeners() {
  if (
    !foregroundListenersInstalled ||
    typeof window === "undefined" ||
    typeof document === "undefined"
  ) {
    return;
  }
  window.removeEventListener("focus", handleFocus);
  document.removeEventListener("visibilitychange", handleVisibilityChange);
  foregroundListenersInstalled = false;
}

export function useWindowForeground({
  enabled,
  onForeground,
  throttleMs = 1000,
}: UseWindowForegroundOptions) {
  const onForegroundRef = useRef(onForeground);
  const throttleMsRef = useRef(throttleMs);
  const lastFiredAtMsRef = useRef(0);

  useEffect(() => {
    onForegroundRef.current = onForeground;
  }, [onForeground]);

  useEffect(() => {
    throttleMsRef.current = throttleMs;
  }, [throttleMs]);

  useEffect(() => {
    if (!enabled) return;

    const subscriber: ForegroundSubscriber = {
      onForegroundRef,
      throttleMsRef,
      lastFiredAtMsRef,
    };
    foregroundSubscribers.add(subscriber);
    installForegroundListeners();

    return () => {
      foregroundSubscribers.delete(subscriber);
      if (foregroundSubscribers.size === 0) {
        uninstallForegroundListeners();
      }
    };
  }, [enabled]);
}
