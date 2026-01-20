import { invokeTauriOrNull } from "./tauriInvoke";
import type { CliKey } from "./providers";

export type RequestLogSummary = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
  method: string;
  path: string;
  requested_model: string | null;
  status: number | null;
  error_code: string | null;
  duration_ms: number;
  ttfb_ms: number | null;
  attempt_count: number;
  has_failover: boolean;
  start_provider_id: number;
  start_provider_name: string;
  final_provider_id: number;
  final_provider_name: string;
  route: RequestLogRouteHop[];
  session_reuse: boolean;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  cache_read_input_tokens: number | null;
  cache_creation_input_tokens: number | null;
  cache_creation_5m_input_tokens: number | null;
  cost_usd: number | null;
  cost_multiplier: number;
  created_at_ms: number | null;
  created_at: number;
};

export type RequestLogRouteHop = {
  provider_id: number;
  provider_name: string;
  ok: boolean;
};

export type RequestLogDetail = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
  method: string;
  path: string;
  query: string | null;
  excluded_from_stats: boolean;
  special_settings_json: string | null;
  status: number | null;
  error_code: string | null;
  duration_ms: number;
  ttfb_ms: number | null;
  attempts_json: string;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  cache_read_input_tokens: number | null;
  cache_creation_input_tokens: number | null;
  cache_creation_5m_input_tokens: number | null;
  usage_json: string | null;
  requested_model: string | null;
  cost_usd: number | null;
  cost_multiplier: number;
  created_at_ms: number | null;
  created_at: number;
};

export type RequestAttemptLog = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
  method: string;
  path: string;
  query: string | null;
  attempt_index: number;
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  attempt_started_ms: number;
  attempt_duration_ms: number;
  created_at: number;
};

export async function requestLogsList(cliKey: CliKey, limit?: number) {
  return invokeTauriOrNull<RequestLogSummary[]>("request_logs_list", {
    cliKey,
    limit,
  });
}

export async function requestLogsListAll(limit?: number) {
  return invokeTauriOrNull<RequestLogSummary[]>("request_logs_list_all", { limit });
}

export async function requestLogsListAfterId(cliKey: CliKey, afterId: number, limit?: number) {
  return invokeTauriOrNull<RequestLogSummary[]>("request_logs_list_after_id", {
    cliKey,
    afterId,
    limit,
  });
}

export async function requestLogsListAfterIdAll(afterId: number, limit?: number) {
  return invokeTauriOrNull<RequestLogSummary[]>("request_logs_list_after_id_all", {
    afterId,
    limit,
  });
}

export async function requestLogGet(logId: number) {
  return invokeTauriOrNull<RequestLogDetail>("request_log_get", { logId });
}

export async function requestLogGetByTraceId(traceId: string) {
  return invokeTauriOrNull<RequestLogDetail | null>("request_log_get_by_trace_id", {
    traceId,
  });
}

export async function requestAttemptLogsByTraceId(traceId: string, limit?: number) {
  return invokeTauriOrNull<RequestAttemptLog[]>("request_attempt_logs_by_trace_id", {
    traceId,
    limit,
  });
}
