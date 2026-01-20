import { logToConsole } from "./consoleLog";
import { hasTauriRuntime } from "./tauriInvoke";
import { ingestTraceAttempt, ingestTraceRequest, ingestTraceStart } from "./traceStore";

export type GatewayAttempt = {
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
};

export type GatewayRequestEvent = {
  trace_id: string;
  cli_key: string;
  method: string;
  path: string;
  query: string | null;
  status: number | null;
  error_category: string | null;
  error_code: string | null;
  duration_ms: number;
  ttfb_ms?: number | null;
  attempts: GatewayAttempt[];
  input_tokens?: number | null;
  output_tokens?: number | null;
  total_tokens?: number | null;
  cache_read_input_tokens?: number | null;
  cache_creation_input_tokens?: number | null;
  cache_creation_5m_input_tokens?: number | null;
};

export type GatewayRequestStartEvent = {
  trace_id: string;
  cli_key: string;
  method: string;
  path: string;
  query: string | null;
  requested_model?: string | null;
  ts: number;
};

export type GatewayAttemptEvent = {
  trace_id: string;
  cli_key: string;
  method: string;
  path: string;
  query: string | null;
  attempt_index: number;
  provider_id: number;
  session_reuse?: boolean | null;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  attempt_started_ms: number;
  attempt_duration_ms: number;
  circuit_state_before?: string | null;
  circuit_state_after?: string | null;
  circuit_failure_count?: number | null;
  circuit_failure_threshold?: number | null;
};

export type GatewayLogEvent = {
  level: "info" | "warn" | "error";
  error_code: string;
  message: string;
  requested_port: number;
  bound_port: number;
  base_url: string;
};

export type GatewayCircuitEvent = {
  trace_id: string;
  cli_key: string;
  provider_id: number;
  provider_name: string;
  base_url: string;
  prev_state: string;
  next_state: string;
  failure_count: number;
  failure_threshold: number;
  open_until: number | null;
  cooldown_until?: number | null;
  reason: string;
  ts: number;
};

function attemptLevel(outcome: string): "info" | "warn" {
  if (outcome === "success") return "info";
  if (outcome === "started") return "info";
  return "warn";
}

function normalizeLogLevel(level: unknown): "info" | "warn" | "error" {
  if (level === "warn" || level === "error" || level === "info") return level;
  return "info";
}

function normalizeCircuitState(state: string | null | undefined) {
  if (!state) return null;
  if (state === "OPEN" || state === "CLOSED") return state;
  return null;
}

function circuitStateText(state: string | null | undefined) {
  const normalized = normalizeCircuitState(state);
  if (normalized === "OPEN") return "熔断";
  if (normalized === "CLOSED") return "正常";
  return "未知";
}

function circuitReasonText(reason: string | null | undefined) {
  const r = reason?.trim();
  if (!r) return "未知";
  switch (r) {
    case "FAILURE_THRESHOLD_REACHED":
      return "失败次数达到阈值";
    case "OPEN_EXPIRED":
      return "熔断到期";
    default:
      return r;
  }
}

function attemptTitle(event: GatewayAttemptEvent) {
  const method = event.method ?? "未知";
  const path = event.path ?? "/";
  const provider = event.provider_name || "未知";
  const statusLabel = event.status == null ? "—" : String(event.status);
  const phase =
    event.outcome === "success" ? "成功" : event.outcome === "started" ? "开始" : "失败";
  return `故障切换尝试${phase}（#${event.attempt_index}）：${method} ${path} · ${provider} · ${statusLabel}`;
}

function computeOutputTokensPerSecond(payload: GatewayRequestEvent) {
  const output = payload.output_tokens;
  const durationMs = payload.duration_ms;
  const ttfbMs = payload.ttfb_ms ?? null;
  if (output == null) return null;
  if (!Number.isFinite(durationMs) || durationMs <= 0) return null;
  if (ttfbMs == null || !Number.isFinite(ttfbMs)) return null;
  const generationMs = durationMs - ttfbMs;
  if (!Number.isFinite(generationMs) || generationMs <= 0) return null;
  return output / (generationMs / 1000);
}

export async function listenGatewayEvents(): Promise<() => void> {
  if (!hasTauriRuntime()) return () => {};

  const { listen } = await import("@tauri-apps/api/event");

  const unlistenRequestStart = await listen<GatewayRequestStartEvent>(
    "gateway:request_start",
    (event) => {
      const payload = event.payload;
      if (!payload) return;

      ingestTraceStart(payload);

      const method = payload.method ?? "未知";
      const path = payload.path ?? "/";
      logToConsole("info", `网关请求开始：${method} ${path}`, {
        trace_id: payload.trace_id,
        cli: payload.cli_key,
        method,
        path,
      });
    }
  );

  const unlistenAttempt = await listen<GatewayAttemptEvent>("gateway:attempt", (event) => {
    const payload = event.payload;
    if (!payload) return;

    ingestTraceAttempt(payload);

    // "started" events are high-frequency and intended for realtime UI routing updates.
    // Keep console noise low by only logging completion/failure events.
    if (payload.outcome !== "started") {
      logToConsole(attemptLevel(payload.outcome), attemptTitle(payload), {
        trace_id: payload.trace_id,
        cli: payload.cli_key,
        attempt_index: payload.attempt_index,
        provider_id: payload.provider_id,
        provider_name: payload.provider_name,
        base_url: payload.base_url,
        status: payload.status,
        outcome: payload.outcome,
        attempt_started_ms: payload.attempt_started_ms,
        attempt_duration_ms: payload.attempt_duration_ms,
        circuit_state_before: circuitStateText(payload.circuit_state_before),
        circuit_state_after: circuitStateText(payload.circuit_state_after),
        circuit_failure_count: payload.circuit_failure_count ?? null,
        circuit_failure_threshold: payload.circuit_failure_threshold ?? null,
      });
    }
  });

  const unlistenRequest = await listen<GatewayRequestEvent>("gateway:request", (event) => {
    const payload = event.payload;
    if (!payload) return;

    ingestTraceRequest(payload);

    const attempts = payload.attempts ?? [];

    const method = payload.method ?? "未知";
    const path = payload.path ?? "/";
    const title = payload.error_code
      ? `网关请求失败：${method} ${path}`
      : `网关请求：${method} ${path}`;

    const outputTokensPerSecond = computeOutputTokensPerSecond(payload);

    logToConsole(payload.error_code ? "error" : "info", title, {
      trace_id: payload.trace_id,
      cli: payload.cli_key,
      status: payload.status,
      error_category: payload.error_category ?? null,
      error_code: payload.error_code,
      duration_ms: payload.duration_ms,
      ttfb_ms: payload.ttfb_ms ?? null,
      output_tokens_per_second: outputTokensPerSecond,
      input_tokens: payload.input_tokens,
      output_tokens: payload.output_tokens,
      total_tokens: payload.total_tokens,
      cache_read_input_tokens: payload.cache_read_input_tokens,
      cache_creation_input_tokens: payload.cache_creation_input_tokens,
      cache_creation_5m_input_tokens: payload.cache_creation_5m_input_tokens,
      attempts,
    });
  });

  const unlistenLog = await listen<GatewayLogEvent>("gateway:log", (event) => {
    const payload = event.payload;
    if (!payload) return;

    const title =
      payload.error_code === "GW_PORT_IN_USE"
        ? "端口被占用，已自动切换（GW_PORT_IN_USE）"
        : `网关日志：${payload.error_code}`;

    logToConsole(normalizeLogLevel(payload.level), title, {
      error_code: payload.error_code,
      message: payload.message,
      requested_port: payload.requested_port,
      bound_port: payload.bound_port,
      base_url: payload.base_url,
    });
  });

  const unlistenCircuit = await listen<GatewayCircuitEvent>("gateway:circuit", (event) => {
    const payload = event.payload;
    if (!payload) return;

    const from = circuitStateText(payload.prev_state);
    const to = circuitStateText(payload.next_state);
    const provider = payload.provider_name || "未知";
    const title = `熔断状态变更：${provider} ${from} → ${to}`;
    const level = to === "熔断" ? "warn" : "info";

    logToConsole(level, title, {
      trace_id: payload.trace_id,
      cli: payload.cli_key,
      provider_id: payload.provider_id,
      provider_name: payload.provider_name,
      base_url: payload.base_url,
      prev_state: from,
      next_state: to,
      failure_count: payload.failure_count,
      failure_threshold: payload.failure_threshold,
      open_until: payload.open_until,
      cooldown_until: payload.cooldown_until ?? null,
      reason: circuitReasonText(payload.reason),
      ts: payload.ts,
    });
  });

  return () => {
    unlistenRequestStart();
    unlistenAttempt();
    unlistenRequest();
    unlistenLog();
    unlistenCircuit();
  };
}
