// Usage:
// - Shared parser for `request_logs.attempts_json` (serialized gateway FailoverAttempt list).
// - Single contract for the provider chain view and the error-card failure summary.
// - `timeout_secs` is the structured first-byte timeout; never parse it out of `outcome`.

export type AttemptJsonEntry = {
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  provider_index?: number | null;
  retry_index?: number | null;
  session_reuse?: boolean | null;
  error_category?: string | null;
  error_code?: string | null;
  decision?: string | null;
  reason?: string | null;
  selection_method?: string | null;
  reason_code?: string | null;
  attempt_started_ms?: number | null;
  attempt_duration_ms?: number | null;
  circuit_state_before?: string | null;
  circuit_state_after?: string | null;
  circuit_failure_count?: number | null;
  circuit_failure_threshold?: number | null;
  // Circuit attribution for gate-skip attempts; the backend omits both keys
  // entirely on success and non-circuit paths (space constraint).
  circuit_recover_at_unix?: number | null;
  circuit_trigger_error_code?: string | null;
  timeout_secs?: number | null;
};

export function parseAttemptsJson(raw: string | null | undefined): AttemptJsonEntry[] | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? (parsed as AttemptJsonEntry[]) : null;
  } catch {
    return null;
  }
}
