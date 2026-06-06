import type { RequestLogDetail, RequestLogRouteHop, RequestLogSummary } from "./requestLogs";

function resolveCreatedAt(overrides: {
  created_at_ms?: number | null;
  created_at?: number | null;
}): { createdAtMs: number; createdAt: number } {
  const createdAtMs =
    overrides.created_at_ms != null
      ? overrides.created_at_ms
      : overrides.created_at != null
        ? overrides.created_at * 1000
        : Date.now();
  const createdAt = overrides.created_at ?? Math.floor(createdAtMs / 1000);
  return {
    createdAtMs,
    createdAt,
  };
}

export function createRequestLogRouteHop(
  overrides: Partial<RequestLogRouteHop> = {}
): RequestLogRouteHop {
  return {
    provider_id: 1,
    provider_name: "Provider A",
    ok: true,
    attempts: 1,
    skipped: false,
    status: 200,
    error_code: null,
    decision: null,
    reason: null,
    ...overrides,
  };
}

export function createRequestLogSummary(
  overrides: Omit<
    Partial<RequestLogSummary>,
    "cli_key" | "route" | "created_at_ms" | "created_at"
  > & {
    cli_key?: string;
    created_at_ms?: number | null;
    created_at?: number | null;
    route?: Array<Partial<RequestLogRouteHop>>;
  } = {}
): RequestLogSummary {
  const { createdAtMs, createdAt } = resolveCreatedAt(overrides);
  const cliKey = (overrides.cli_key ?? "claude") as RequestLogSummary["cli_key"];
  return {
    id: 1,
    trace_id: "trace-1",
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    excluded_from_stats: false,
    special_settings_json: null,
    requested_model: "claude-3-7-sonnet",
    status: 200,
    error_code: null,
    duration_ms: 100,
    ttfb_ms: 50,
    attempt_count: 1,
    has_failover: false,
    start_provider_id: 1,
    start_provider_name: "Provider A",
    final_provider_id: 1,
    final_provider_name: "Provider A",
    final_provider_source_id: null,
    final_provider_source_name: null,
    session_reuse: false,
    input_tokens: null,
    output_tokens: null,
    total_tokens: null,
    cache_read_input_tokens: null,
    cache_creation_input_tokens: null,
    cache_creation_5m_input_tokens: null,
    cache_creation_1h_input_tokens: null,
    cost_usd: null,
    provider_chain_json: null,
    error_details_json: null,
    association_audit_json: null,
    cost_multiplier: 1,
    ...overrides,
    cli_key: cliKey,
    created_at_ms: createdAtMs,
    created_at: createdAt,
    route: (overrides.route ?? []).map((routeItem) => createRequestLogRouteHop(routeItem)),
  };
}

export function createRequestLogDetail(
  overrides: Partial<RequestLogDetail> = {}
): RequestLogDetail {
  const { createdAtMs, createdAt } = resolveCreatedAt(overrides);
  return {
    id: 1,
    trace_id: "trace-1",
    cli_key: "claude",
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    query: "hello",
    excluded_from_stats: false,
    special_settings_json: null,
    status: 200,
    error_code: null,
    duration_ms: 100,
    ttfb_ms: 50,
    attempts_json: "[]",
    input_tokens: null,
    output_tokens: null,
    total_tokens: null,
    cache_read_input_tokens: null,
    cache_creation_input_tokens: null,
    cache_creation_5m_input_tokens: null,
    cache_creation_1h_input_tokens: null,
    usage_json: null,
    requested_model: "claude-3-7-sonnet",
    final_provider_id: 1,
    final_provider_name: "Provider A",
    final_provider_source_id: null,
    final_provider_source_name: null,
    cost_usd: null,
    provider_chain_json: null,
    error_details_json: null,
    association_audit_json: null,
    cost_multiplier: 1,
    created_at_ms: createdAtMs,
    created_at: createdAt,
    ...overrides,
  };
}
