import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

export type RequestLogSummary = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
  session_id?: string | null;
  method: string;
  path: string;
  excluded_from_stats?: boolean;
  special_settings_json?: string | null;
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
  final_provider_source_id?: number | null;
  final_provider_source_name?: string | null;
  route: RequestLogRouteHop[];
  session_reuse: boolean;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  cache_read_input_tokens: number | null;
  cache_creation_input_tokens: number | null;
  cache_creation_5m_input_tokens: number | null;
  cache_creation_1h_input_tokens: number | null;
  cost_usd: number | null;
  cost_multiplier: number;
  created_at_ms: number | null;
  created_at: number;
};

export type RequestLogRouteHop = {
  provider_id: number;
  provider_name: string;
  ok: boolean;
  attempts?: number;
  /** 该 provider 是否被跳过（熔断/限流等，请求未实际发送） */
  skipped?: boolean;
  status?: number | null;
  error_code?: string | null;
  decision?: string | null;
  reason?: string | null;
};

export type RequestLogDetail = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
  session_id?: string | null;
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
  cache_creation_1h_input_tokens: number | null;
  usage_json: string | null;
  requested_model: string | null;
  final_provider_id?: number | null;
  final_provider_name?: string | null;
  final_provider_source_id?: number | null;
  final_provider_source_name?: string | null;
  cost_usd: number | null;
  error_details_json?: string | null;
  cost_multiplier: number;
  created_at_ms: number | null;
  created_at: number;
};

export type RequestAttemptLog = {
  id: number;
  trace_id: string;
  cli_key: CliKey;
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
  return invokeService<RequestLogSummary[]>("读取请求日志失败", "request_logs_list", {
    cliKey,
    limit,
  });
}

export async function requestLogsListAll(limit?: number) {
  return invokeService<RequestLogSummary[]>("读取全局请求日志失败", "request_logs_list_all", {
    limit,
  });
}

export async function requestLogsListAfterId(cliKey: CliKey, afterId: number, limit?: number) {
  return invokeService<RequestLogSummary[]>("读取增量请求日志失败", "request_logs_list_after_id", {
    cliKey,
    afterId,
    limit,
  });
}

export async function requestLogsListAfterIdAll(afterId: number, limit?: number) {
  return invokeService<RequestLogSummary[]>(
    "读取全局增量请求日志失败",
    "request_logs_list_after_id_all",
    {
      afterId,
      limit,
    }
  );
}

export async function requestLogGet(logId: number) {
  return invokeService<RequestLogDetail>("读取请求日志详情失败", "request_log_get", { logId });
}

export async function requestLogGetByTraceId(traceId: string) {
  return invokeService<RequestLogDetail | null>(
    "按追踪 ID 读取请求日志失败",
    "request_log_get_by_trace_id",
    {
      traceId,
    }
  );
}

export async function requestAttemptLogsByTraceId(traceId: string, limit?: number) {
  return invokeService<RequestAttemptLog[]>(
    "读取请求尝试日志失败",
    "request_attempt_logs_by_trace_id",
    {
      traceId,
      limit,
    }
  );
}
