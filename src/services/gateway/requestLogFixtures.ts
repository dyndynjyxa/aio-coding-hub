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
  // Mirror the backend derivation (row never resolved => interrupted) so tests
  // building status-null rows get realistic backend-shaped data.
  const status = "status" in overrides ? (overrides.status ?? null) : 200;
  const errorCode = overrides.error_code ?? null;
  const isInterrupted = overrides.is_interrupted ?? (status == null && errorCode == null);
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
    effective_input_tokens: null,
    cost_usd: null,
    provider_chain_json: null,
    error_details_json: null,
    cost_multiplier: 1,
    last_activity_ms: null,
    activity_details_json: null,
    ...overrides,
    cli_key: cliKey,
    is_interrupted: isInterrupted,
    created_at_ms: createdAtMs,
    created_at: createdAt,
    route: (overrides.route ?? []).map((routeItem) => createRequestLogRouteHop(routeItem)),
  };
}

export function createRequestLogDetail(
  overrides: Partial<RequestLogDetail> = {}
): RequestLogDetail {
  const { createdAtMs, createdAt } = resolveCreatedAt(overrides);
  // Mirror the backend derivation (row never resolved => interrupted).
  const status = "status" in overrides ? (overrides.status ?? null) : 200;
  const isInterrupted =
    overrides.is_interrupted ?? (status == null && (overrides.error_code ?? null) == null);
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
    effective_input_tokens: null,
    usage_json: null,
    requested_model: "claude-3-7-sonnet",
    final_provider_id: 1,
    final_provider_name: "Provider A",
    final_provider_source_id: null,
    final_provider_source_name: null,
    cost_usd: null,
    provider_chain_json: null,
    error_details_json: null,
    cost_multiplier: 1,
    created_at_ms: createdAtMs,
    last_activity_ms: null,
    activity_details_json: null,
    created_at: createdAt,
    ...overrides,
    is_interrupted: isInterrupted,
  };
}
