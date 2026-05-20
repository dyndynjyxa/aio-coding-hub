import { useCallback, useEffect, useRef } from "react";

type UseCoalescedAsyncRefreshOptions<TSource, TResult> = {
  enabled: boolean;
  delayMs: number;
  task: (source: TSource) => Promise<TResult>;
  onError?: (error: unknown, source: TSource) => TResult | Promise<TResult>;
};

export function useCoalescedAsyncRefresh<TSource, TResult = unknown>({
  enabled,
  delayMs,
  task,
  onError,
}: UseCoalescedAsyncRefreshOptions<TSource, TResult>) {
  const timerRef = useRef<number | null>(null);
  const queuedRef = useRef(false);
  const queuedSourceRef = useRef<TSource | undefined>(undefined);
  const inFlightRef = useRef(false);
  const enabledRef = useRef(enabled);
  const taskRef = useRef(task);
  const onErrorRef = useRef(onError);
  const flushRef = useRef<((source: TSource) => Promise<TResult | null> | null) | null>(null);

  useEffect(() => {
    taskRef.current = task;
  }, [task]);

  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  const clearQueued = useCallback(() => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    queuedRef.current = false;
    queuedSourceRef.current = undefined;
  }, []);

  const runTask = useCallback(async (source: TSource): Promise<TResult> => {
    try {
      return await taskRef.current(source);
    } catch (error) {
      if (onErrorRef.current) {
        return await onErrorRef.current(error, source);
      }
      throw error;
    }
  }, []);

  const flush = useCallback(
    (source: TSource): Promise<TResult | null> | null => {
      if (!enabledRef.current) {
        clearQueued();
        return null;
      }

      if (inFlightRef.current) {
        queuedRef.current = true;
        queuedSourceRef.current = source;
        return null;
      }

      queuedRef.current = false;
      queuedSourceRef.current = undefined;
      inFlightRef.current = true;

      return runTask(source).finally(() => {
        inFlightRef.current = false;
        if (!queuedRef.current || !enabledRef.current) {
          queuedRef.current = false;
          queuedSourceRef.current = undefined;
          return;
        }

        const nextSource = queuedSourceRef.current as TSource;
        queuedRef.current = false;
        queuedSourceRef.current = undefined;
        void flushRef.current?.(nextSource);
      });
    },
    [clearQueued, runTask]
  );

  flushRef.current = flush;

  const schedule = useCallback(
    (source: TSource) => {
      if (!enabledRef.current) {
        return;
      }

      if (timerRef.current != null) {
        queuedRef.current = true;
        queuedSourceRef.current = source;
        return;
      }

      timerRef.current = window.setTimeout(() => {
        timerRef.current = null;
        void flush(source);
      }, delayMs);
    },
    [delayMs, flush]
  );

  useEffect(() => {
    enabledRef.current = enabled;
    if (!enabled) {
      clearQueued();
    }
  }, [clearQueued, enabled]);

  useEffect(() => {
    return () => {
      enabledRef.current = false;
      inFlightRef.current = false;
      clearQueued();
    };
  }, [clearQueued]);

  return {
    clearQueued,
    flush,
    schedule,
  };
}
