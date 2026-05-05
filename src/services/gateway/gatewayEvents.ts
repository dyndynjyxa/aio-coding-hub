import { GatewayErrorCodes } from "../../constants/gatewayErrorCodes";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import { computeOutputTokensPerSecond as computeOutputTokensPerSecondRaw } from "../../utils/formatters";
import { logToConsole, shouldLogToConsole } from "../consoleLog";
import { subscribeGatewayEvent } from "./gatewayEventBus";
import { ingestTraceAttempt, ingestTraceRequest, ingestTraceStart } from "./traceStore";
import { ingestCacheAnomalyRequest, ingestCacheAnomalyRequestStart } from "./cacheAnomalyMonitor";
import type { ClaudeModelMapping } from "./claudeModelMapping";

export type { ClaudeModelMapping } from "./claudeModelMapping";

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
  session_id?: string | null;
  method: string;
  path: string;
  query: string | null;
  requested_model?: string | null;
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
  cache_creation_1h_input_tokens?: number | null;
  claude_model_mapping?: ClaudeModelMapping | null;
};

export type GatewayRequestStartEvent = {
  trace_id: string;
  cli_key: string;
  session_id?: string | null;
  method: string;
  path: string;
  query: string | null;
  requested_model?: string | null;
  ts: number;
};

export type GatewayRequestSignalEvent = {
  trace_id: string;
  cli_key: string;
  session_id?: string | null;
  requested_model?: string | null;
  phase: "start" | "complete";
  ts: number;
};

export type GatewayAttemptEvent = {
  trace_id: string;
  cli_key: string;
  session_id?: string | null;
  method: string;
  path: string;
  query: string | null;
  requested_model?: string | null;
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
  claude_model_mapping?: ClaudeModelMapping | null;
};

export type GatewayLogEvent = {
  level: string;
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

function normalizeLogLevel(level: unknown): "debug" | "info" | "warn" | "error" {
  if (level === "debug" || level === "info" || level === "warn" || level === "error") return level;
  return "info";
}

function normalizeCircuitState(state: string | null | undefined) {
  if (!state) return null;
  if (state === "OPEN" || state === "CLOSED" || state === "HALF_OPEN") return state;
  return null;
}

function circuitStateText(state: string | null | undefined) {
  const normalized = normalizeCircuitState(state);
  if (normalized === "OPEN") return "熔断";
  if (normalized === "HALF_OPEN") return "半开";
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
      return "熔断到期，进入半开试探";
    case "HALF_OPEN_SUCCESS":
      return "半开试探成功，恢复正常";
    case "HALF_OPEN_FAILURE":
      return "半开试探失败，重新熔断";
    case "SKIP_OPEN":
      return "熔断中已跳过";
    case "SKIP_COOLDOWN":
      return "冷却中已跳过";
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
  return computeOutputTokensPerSecondRaw(
    payload.output_tokens,
    payload.duration_ms,
    payload.ttfb_ms ?? null
  );
}

type GatewayEventGuard<TPayload> = (payload: unknown) => payload is TPayload;

function isRecord(payload: unknown): payload is Record<string, unknown> {
  return typeof payload === "object" && payload !== null && !Array.isArray(payload);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

function isNullish(value: unknown): value is null | undefined {
  return value == null;
}

function isNullableString(value: unknown): value is string | null | undefined {
  return isNullish(value) || isString(value);
}

function isNullableNumber(value: unknown): value is number | null | undefined {
  return isNullish(value) || isNumber(value);
}

function isNullableBoolean(value: unknown): value is boolean | null | undefined {
  return isNullish(value) || isBoolean(value);
}

function isClaudeModelMapping(value: unknown): value is ClaudeModelMapping {
  if (!isRecord(value)) return false;
  return (
    isString(value.requestedModel) &&
    isString(value.effectiveModel) &&
    isString(value.mappingKind) &&
    isNumber(value.providerId) &&
    isString(value.providerName) &&
    isBoolean(value.applied)
  );
}

function isNullableClaudeModelMapping(
  value: unknown
): value is ClaudeModelMapping | null | undefined {
  return isNullish(value) || isClaudeModelMapping(value);
}

function isGatewayAttempt(payload: unknown): payload is GatewayAttempt {
  if (!isRecord(payload)) return false;
  return (
    isNumber(payload.provider_id) &&
    isString(payload.provider_name) &&
    isString(payload.base_url) &&
    isString(payload.outcome) &&
    isNullableNumber(payload.status)
  );
}

export function isGatewayRequestStartEvent(payload: unknown): payload is GatewayRequestStartEvent {
  if (!isRecord(payload)) return false;
  return (
    isString(payload.trace_id) &&
    isString(payload.cli_key) &&
    isNullableString(payload.session_id) &&
    isString(payload.method) &&
    isString(payload.path) &&
    isNullableString(payload.query) &&
    isNullableString(payload.requested_model) &&
    isNumber(payload.ts)
  );
}

export function isGatewayRequestSignalEvent(
  payload: unknown
): payload is GatewayRequestSignalEvent {
  if (!isRecord(payload)) return false;
  return (
    isString(payload.trace_id) &&
    isString(payload.cli_key) &&
    isNullableString(payload.session_id) &&
    isNullableString(payload.requested_model) &&
    (payload.phase === "start" || payload.phase === "complete") &&
    isNumber(payload.ts)
  );
}

function isGatewayAttemptEvent(payload: unknown): payload is GatewayAttemptEvent {
  if (!isRecord(payload)) return false;
  return (
    isString(payload.trace_id) &&
    isString(payload.cli_key) &&
    isNullableString(payload.session_id) &&
    isString(payload.method) &&
    isString(payload.path) &&
    isNullableString(payload.query) &&
    isNullableString(payload.requested_model) &&
    isNumber(payload.attempt_index) &&
    isNumber(payload.provider_id) &&
    isNullableBoolean(payload.session_reuse) &&
    isString(payload.provider_name) &&
    isString(payload.base_url) &&
    isString(payload.outcome) &&
    isNullableNumber(payload.status) &&
    isNumber(payload.attempt_started_ms) &&
    isNumber(payload.attempt_duration_ms) &&
    isNullableString(payload.circuit_state_before) &&
    isNullableString(payload.circuit_state_after) &&
    isNullableNumber(payload.circuit_failure_count) &&
    isNullableNumber(payload.circuit_failure_threshold) &&
    isNullableClaudeModelMapping(payload.claude_model_mapping)
  );
}

function isGatewayRequestEvent(payload: unknown): payload is GatewayRequestEvent {
  if (!isRecord(payload)) return false;
  const attempts = payload.attempts;
  return (
    isString(payload.trace_id) &&
    isString(payload.cli_key) &&
    isNullableString(payload.session_id) &&
    isString(payload.method) &&
    isString(payload.path) &&
    isNullableString(payload.query) &&
    isNullableString(payload.requested_model) &&
    isNullableNumber(payload.status) &&
    isNullableString(payload.error_category) &&
    isNullableString(payload.error_code) &&
    isNumber(payload.duration_ms) &&
    isNullableNumber(payload.ttfb_ms) &&
    Array.isArray(attempts) &&
    attempts.every(isGatewayAttempt) &&
    isNullableNumber(payload.input_tokens) &&
    isNullableNumber(payload.output_tokens) &&
    isNullableNumber(payload.total_tokens) &&
    isNullableNumber(payload.cache_read_input_tokens) &&
    isNullableNumber(payload.cache_creation_input_tokens) &&
    isNullableNumber(payload.cache_creation_5m_input_tokens) &&
    isNullableNumber(payload.cache_creation_1h_input_tokens) &&
    isNullableClaudeModelMapping(payload.claude_model_mapping)
  );
}

function isGatewayLogEvent(payload: unknown): payload is GatewayLogEvent {
  if (!isRecord(payload)) return false;
  return (
    isString(payload.level) &&
    isString(payload.error_code) &&
    isString(payload.message) &&
    isNumber(payload.requested_port) &&
    isNumber(payload.bound_port) &&
    isString(payload.base_url)
  );
}

function isGatewayCircuitEvent(payload: unknown): payload is GatewayCircuitEvent {
  if (!isRecord(payload)) return false;
  return (
    isString(payload.trace_id) &&
    isString(payload.cli_key) &&
    isNumber(payload.provider_id) &&
    isString(payload.provider_name) &&
    isString(payload.base_url) &&
    isString(payload.prev_state) &&
    isString(payload.next_state) &&
    isNumber(payload.failure_count) &&
    isNumber(payload.failure_threshold) &&
    isNullableNumber(payload.open_until) &&
    isNullableNumber(payload.cooldown_until) &&
    isString(payload.reason) &&
    isNumber(payload.ts)
  );
}

function readGatewayPayload<TPayload>(
  event: string,
  payload: unknown,
  guard: GatewayEventGuard<TPayload>
): TPayload | null {
  if (guard(payload)) return payload;

  logToConsole(
    "warn",
    "网关事件 payload 无效，已丢弃",
    { event, payload_type: typeof payload },
    "gateway:event_guard"
  );
  return null;
}

export async function listenGatewayEvents(): Promise<() => void> {
  const circuitNonTransitionDedup = new Map<string, number>();
  const CIRCUIT_NON_TRANSITION_DEDUP_WINDOW_MS = 3000;

  const requestStartSub = subscribeGatewayEvent(gatewayEventNames.requestStart, (rawPayload) => {
    const payload = readGatewayPayload(
      gatewayEventNames.requestStart,
      rawPayload,
      isGatewayRequestStartEvent
    );
    if (!payload) return;

    ingestTraceStart(payload);
    ingestCacheAnomalyRequestStart(payload);

    if (!shouldLogToConsole("debug")) return;

    const method = payload.method ?? "未知";
    const path = payload.path ?? "/";
    logToConsole(
      "debug",
      `网关请求开始：${method} ${path}`,
      {
        trace_id: payload.trace_id,
        cli: payload.cli_key,
        method,
        path,
      },
      gatewayEventNames.requestStart
    );
  });

  const attemptSub = subscribeGatewayEvent(gatewayEventNames.attempt, (rawPayload) => {
    const payload = readGatewayPayload(
      gatewayEventNames.attempt,
      rawPayload,
      isGatewayAttemptEvent
    );
    if (!payload) return;

    ingestTraceAttempt(payload);

    // "started" events are high-frequency and intended for realtime UI routing updates.
    // Keep console noise low by only logging completion/failure events.
    if (payload.outcome === "started") return;

    if (!shouldLogToConsole("debug")) return;

    logToConsole(
      "debug",
      attemptTitle(payload),
      {
        trace_id: payload.trace_id,
        cli: payload.cli_key,
        attempt_index: payload.attempt_index,
        provider_id: payload.provider_id,
        provider_name: payload.provider_name,
        status: payload.status,
        outcome: payload.outcome,
        attempt_started_ms: payload.attempt_started_ms,
        attempt_duration_ms: payload.attempt_duration_ms,
        circuit_state_before: circuitStateText(payload.circuit_state_before),
        circuit_state_after: circuitStateText(payload.circuit_state_after),
        circuit_failure_count: payload.circuit_failure_count ?? null,
        circuit_failure_threshold: payload.circuit_failure_threshold ?? null,
      },
      gatewayEventNames.attempt
    );
  });

  const requestSub = subscribeGatewayEvent(gatewayEventNames.request, (rawPayload) => {
    const payload = readGatewayPayload(
      gatewayEventNames.request,
      rawPayload,
      isGatewayRequestEvent
    );
    if (!payload) return;

    ingestTraceRequest(payload);
    ingestCacheAnomalyRequest(payload);

    const hasError = !!payload.error_code;
    const level = hasError ? "warn" : "debug";
    if (!shouldLogToConsole(level)) return;

    const attempts = payload.attempts ?? [];

    const method = payload.method ?? "未知";
    const path = payload.path ?? "/";
    const title = hasError ? `网关请求失败：${method} ${path}` : `网关请求：${method} ${path}`;

    const outputTokensPerSecond = computeOutputTokensPerSecond(payload);

    logToConsole(
      level,
      title,
      {
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
        cache_creation_1h_input_tokens: payload.cache_creation_1h_input_tokens ?? null,
        attempts,
      },
      gatewayEventNames.request
    );
  });

  const logSub = subscribeGatewayEvent(gatewayEventNames.log, (rawPayload) => {
    const payload = readGatewayPayload(gatewayEventNames.log, rawPayload, isGatewayLogEvent);
    if (!payload) return;
    const level = normalizeLogLevel(payload.level);
    if (!shouldLogToConsole(level)) return;

    const title =
      payload.error_code === GatewayErrorCodes.PORT_IN_USE
        ? `端口被占用，已自动切换（${GatewayErrorCodes.PORT_IN_USE}）`
        : `网关日志：${payload.error_code}`;

    logToConsole(
      level,
      title,
      {
        error_code: payload.error_code,
        message: payload.message,
        requested_port: payload.requested_port,
        bound_port: payload.bound_port,
      },
      gatewayEventNames.log
    );
  });

  const circuitSub = subscribeGatewayEvent(gatewayEventNames.circuit, (rawPayload) => {
    const payload = readGatewayPayload(
      gatewayEventNames.circuit,
      rawPayload,
      isGatewayCircuitEvent
    );
    if (!payload) return;

    const prevNormalized = normalizeCircuitState(payload.prev_state);
    const nextNormalized = normalizeCircuitState(payload.next_state);
    const from = circuitStateText(prevNormalized);
    const to = circuitStateText(nextNormalized);
    const provider = payload.provider_name || "未知";
    const reason = circuitReasonText(payload.reason);

    const isTransition =
      prevNormalized != null && nextNormalized != null && prevNormalized !== nextNormalized;

    if (isTransition) {
      const title = `熔断状态变更：${provider} ${from} → ${to}`;
      const level = to === "熔断" ? "warn" : "info";
      logToConsole(
        level,
        title,
        {
          trace_id: payload.trace_id,
          cli: payload.cli_key,
          provider_id: payload.provider_id,
          provider_name: payload.provider_name,
          prev_state: from,
          next_state: to,
          failure_count: payload.failure_count,
          failure_threshold: payload.failure_threshold,
          open_until: payload.open_until,
          cooldown_until: payload.cooldown_until ?? null,
          reason,
          ts: payload.ts,
        },
        gatewayEventNames.circuit
      );
      return;
    }

    const dedupKey = [
      payload.cli_key,
      payload.provider_id,
      payload.reason ?? "",
      prevNormalized ?? payload.prev_state ?? "",
      nextNormalized ?? payload.next_state ?? "",
    ].join(":");

    const now = Date.now();
    const last = circuitNonTransitionDedup.get(dedupKey);
    if (last != null && now - last < CIRCUIT_NON_TRANSITION_DEDUP_WINDOW_MS) return;
    circuitNonTransitionDedup.set(dedupKey, now);
    if (circuitNonTransitionDedup.size > 500) circuitNonTransitionDedup.clear();

    const title = `Provider 跳过：${provider}（${reason}）`;
    logToConsole(
      "debug",
      title,
      {
        trace_id: payload.trace_id,
        cli: payload.cli_key,
        provider_id: payload.provider_id,
        provider_name: payload.provider_name,
        prev_state: from,
        next_state: to,
        failure_count: payload.failure_count,
        failure_threshold: payload.failure_threshold,
        open_until: payload.open_until,
        cooldown_until: payload.cooldown_until ?? null,
        reason,
        ts: payload.ts,
      },
      gatewayEventNames.circuit
    );
  });

  const readyResults = await Promise.allSettled([
    requestStartSub.ready,
    attemptSub.ready,
    requestSub.ready,
    logSub.ready,
    circuitSub.ready,
  ]);

  const subscribeFailed = readyResults.some((result) => result.status === "rejected");
  if (subscribeFailed) {
    requestStartSub.unsubscribe();
    attemptSub.unsubscribe();
    requestSub.unsubscribe();
    logSub.unsubscribe();
    circuitSub.unsubscribe();

    const failedResult = readyResults.find((result) => result.status === "rejected");
    throw failedResult?.reason ?? new Error("gateway event subscriptions failed");
  }

  return () => {
    requestStartSub.unsubscribe();
    attemptSub.unsubscribe();
    requestSub.unsubscribe();
    logSub.unsubscribe();
    circuitSub.unsubscribe();
  };
}
