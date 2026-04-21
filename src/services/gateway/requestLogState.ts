export type RequestLogProgressInput = {
  status: number | null;
  error_code?: string | null;
  created_at?: number;
  created_at_ms?: number | null;
};

export type RequestSignalLike = {
  phase?: string | null;
};

const STALE_IN_PROGRESS_THRESHOLD_MS = 10 * 60 * 1000;

export function requestLogCreatedAtMs(
  log: Pick<RequestLogProgressInput, "created_at" | "created_at_ms">
) {
  const ms = log.created_at_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return (log.created_at ?? 0) * 1000;
}

export function isPersistedRequestLogInProgress(log: RequestLogProgressInput) {
  if (log.status != null || (log.error_code ?? null) != null) return false;

  const createdAtMs = requestLogCreatedAtMs(log);
  if (createdAtMs > 0 && Date.now() - createdAtMs > STALE_IN_PROGRESS_THRESHOLD_MS) {
    return false;
  }

  return true;
}

export function isRequestSignalComplete(signal: RequestSignalLike | null | undefined) {
  return signal?.phase === "complete";
}
