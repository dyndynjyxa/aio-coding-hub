import { useEffect } from "react";
import { logToConsole } from "../services/consoleLog";

/**
 * Subscribes to an async event listener on mount and cleans up on unmount.
 *
 * Handles the race condition where the component unmounts before the listener
 * promise resolves — in that case the unlisten callback is invoked immediately.
 *
 * @param subscribe - Async function that returns an unlisten callback.
 * @param stage     - Label used for warning logs on failure.
 * @param message   - Human-readable failure description for logs.
 */
export function useAsyncListener(
  subscribe: () => Promise<() => void>,
  stage: string,
  message: string
) {
  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | null = null;

    let subscription: Promise<() => void>;
    try {
      subscription = subscribe();
    } catch (error) {
      subscription = Promise.reject(error);
    }

    subscription
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
          return;
        }
        cleanup = unlisten;
      })
      .catch((error) => {
        logToConsole("warn", message, {
          stage,
          error: String(error),
        });
      });

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [message, stage, subscribe]);
}
