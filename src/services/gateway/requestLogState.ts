export type RequestLogProgressInput = {
  status: number | null;
  error_code?: string | null;
  created_at?: number;
  created_at_ms?: number | null;
};

export type RequestSignalLike = {
  phase?: string | null;
};

export function requestLogCreatedAtMs(
  log: Pick<RequestLogProgressInput, "created_at" | "created_at_ms">
) {
  const ms = log.created_at_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return (log.created_at ?? 0) * 1000;
}

export function isPersistedRequestLogInProgress(log: RequestLogProgressInput) {
  if (log.status != null || (log.error_code ?? null) != null) return false;
  return true;
}

export function isRequestSignalComplete(signal: RequestSignalLike | null | undefined) {
  return signal?.phase === "complete";
}
