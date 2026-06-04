// Usage:
// - Provides a reactive current timestamp in milliseconds.
// - Starts/stops its interval based on `enabled`.
// - Useful for live elapsed-duration UI that should keep ticking while a task is in progress.

import { useEffect, useState } from "react";
import { emitListenerSnapshot } from "../utils/listeners";

type ClockListener = (nowMs: number) => void;

type ClockBucket = {
  listeners: Set<ClockListener>;
  intervalHandle: ReturnType<typeof window.setInterval> | null;
};

const clockBuckets = new Map<number, ClockBucket>();

function getClockBucket(intervalMs: number): ClockBucket {
  let bucket = clockBuckets.get(intervalMs);
  if (!bucket) {
    bucket = {
      listeners: new Set(),
      intervalHandle: null,
    };
    clockBuckets.set(intervalMs, bucket);
  }
  return bucket;
}

function startClockBucket(bucket: ClockBucket, intervalMs: number) {
  if (bucket.intervalHandle != null) return;
  bucket.intervalHandle = window.setInterval(() => {
    const nowMs = Date.now();
    emitListenerSnapshot(bucket.listeners, (listener) => listener(nowMs));
  }, intervalMs);
}

function stopClockBucket(intervalMs: number, bucket: ClockBucket) {
  if (bucket.intervalHandle != null) {
    window.clearInterval(bucket.intervalHandle);
    bucket.intervalHandle = null;
  }
  clockBuckets.delete(intervalMs);
}

export function subscribeNowMs(intervalMs: number, listener: ClockListener) {
  const bucket = getClockBucket(intervalMs);
  bucket.listeners.add(listener);
  startClockBucket(bucket, intervalMs);

  return () => {
    bucket.listeners.delete(listener);
    if (bucket.listeners.size === 0) {
      stopClockBucket(intervalMs, bucket);
    }
  };
}

export function useNowMs(enabled: boolean, intervalMs = 250): number {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    if (!enabled) return;
    setNowMs(Date.now());
    return subscribeNowMs(intervalMs, setNowMs);
  }, [enabled, intervalMs]);

  return nowMs;
}
