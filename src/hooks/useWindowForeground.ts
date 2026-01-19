import { useEffect, useRef } from "react";

export type UseWindowForegroundOptions = {
  enabled: boolean;
  onForeground: () => void;
  throttleMs?: number;
};

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

    function maybeFire() {
      const now = Date.now();
      const throttle = throttleMsRef.current;
      if (Number.isFinite(throttle) && throttle > 0) {
        const elapsed = now - lastFiredAtMsRef.current;
        if (elapsed >= 0 && elapsed < throttle) return;
      }
      lastFiredAtMsRef.current = now;
      onForegroundRef.current();
    }

    function handleFocus() {
      maybeFire();
    }

    function handleVisibilityChange() {
      if (document.visibilityState === "visible") {
        maybeFire();
      }
    }

    window.addEventListener("focus", handleFocus);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.removeEventListener("focus", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [enabled]);
}
