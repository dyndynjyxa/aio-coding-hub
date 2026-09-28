// 契约：本模块只负责持久化终态的分类（completed / interrupted），不判定"进行中"。
// "进行中"判定的单一真值来源是 requestActivityProjection（activeRequests 注册表成员身份）。

export type RequestLogProgressInput = {
  status: number | null;
  error_code?: string | null;
  // Backend-owned interrupted flag (RequestLogSummary.is_interrupted).
  is_interrupted: boolean;
  created_at?: number;
  created_at_ms?: number | null;
};

const PENDING_IDLE_NOTICE_MS = 10 * 60 * 1000;

export type PersistedRequestLogActivityState = "completed" | "interrupted";
export type ActiveRequestActivityState = "in_progress_active" | "in_progress_idle";
export type RequestLogActivityState = PersistedRequestLogActivityState | ActiveRequestActivityState;

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

export function isPersistedRequestLogIncomplete(log: RequestLogProgressInput) {
  return log.is_interrupted;
}

export function isRequestLogActivityInProgress(activityState: RequestLogActivityState) {
  return activityState === "in_progress_active" || activityState === "in_progress_idle";
}

export function requestLogLastActivityMs(
  log: Pick<RequestLogProgressInput, "created_at" | "created_at_ms"> & {
    last_activity_ms?: number | null;
  }
) {
  const ms = log.last_activity_ms ?? 0;
  if (Number.isFinite(ms) && ms > 0) return ms;
  return requestLogCreatedAtMs(log);
}

export function requestLogActiveActivityState(
  lastActivityMs: number | null | undefined,
  nowMs: number
): ActiveRequestActivityState {
  const idleForMs = Math.max(0, nowMs - (lastActivityMs ?? 0));
  return idleForMs >= PENDING_IDLE_NOTICE_MS ? "in_progress_idle" : "in_progress_active";
}

export function requestLogActivityState(
  log: RequestLogProgressInput
): PersistedRequestLogActivityState {
  return isPersistedRequestLogIncomplete(log) ? "interrupted" : "completed";
}

export function isRequestSignalComplete(signal: RequestSignalLike | null | undefined) {
  return signal?.phase === "complete";
}
