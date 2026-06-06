import {
  commands,
  type RequestAttemptLog as GeneratedRequestAttemptLog,
  type RequestLogDetail as GeneratedRequestLogDetail,
  type RequestLogRouteHop as GeneratedRequestLogRouteHop,
  type RequestLogSummary as GeneratedRequestLogSummary,
} from "../../generated/bindings";
import type { CliKey } from "../providers/providers";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export const REQUEST_LOGS_DEFAULT_LIMIT = 50;
export const REQUEST_LOGS_MIN_LIMIT = 1;
export const REQUEST_LOGS_MAX_LIMIT = 500;
export const REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT = REQUEST_LOGS_DEFAULT_LIMIT;
export const REQUEST_ATTEMPT_LOGS_MAX_LIMIT = 200;
export const REQUEST_LOG_TRACE_ID_MAX_LENGTH = 256;

export type RequestLogRouteHop = GeneratedRequestLogRouteHop;

export type RequestLogSummary = Override<
  GeneratedRequestLogSummary,
  {
    cli_key: CliKey;
  }
>;

export type RequestLogDetail = Override<
  GeneratedRequestLogDetail,
  {
    cli_key: CliKey;
  }
>;

export type RequestAttemptLog = Override<
  GeneratedRequestAttemptLog,
  {
    cli_key: CliKey;
  }
>;

export type AssociationAuditStatus =
  | "pending"
  | "completed"
  | "skipped"
  | "failed"
  | "timeout"
  | "invalid_response"
  | "not_configured"
  | "unsupported";

export type AssociationAuditRisk = "none" | "low" | "medium" | "high" | "critical" | "unknown";

export type AssociationAuditSeverity = "info" | "low" | "medium" | "high" | "critical";

export type AssociationAuditSignal = {
  code: string;
  severity: AssociationAuditSeverity;
  confidence: number | null;
  event_index: number | null;
  event_kind: string | null;
  reason: string;
  evidence: string | null;
};

export type AssociationAuditResult = {
  status: AssociationAuditStatus;
  reason: string | null;
  provider_id: number | null;
  provider_name: string | null;
  provider_cli_key: string | null;
  model: string | null;
  started_at_ms: number | null;
  finished_at_ms: number | null;
  duration_ms: number | null;
  input_chars: number | null;
  output_chars: number | null;
  package_truncated: boolean;
  association_score: number | null;
  overall_risk: AssociationAuditRisk;
  signals: AssociationAuditSignal[];
  insufficient_context: boolean;
  notes: string | null;
};

export type ParsedAssociationAudit =
  | { result: AssociationAuditResult | null; malformed: false }
  | { result: null; malformed: true; raw: string };

const ASSOCIATION_AUDIT_STATUS_VALUES = [
  "pending",
  "completed",
  "skipped",
  "failed",
  "timeout",
  "invalid_response",
  "not_configured",
  "unsupported",
] as const satisfies readonly AssociationAuditStatus[];

const ASSOCIATION_AUDIT_RISK_VALUES = [
  "none",
  "low",
  "medium",
  "high",
  "critical",
  "unknown",
] as const satisfies readonly AssociationAuditRisk[];

const ASSOCIATION_AUDIT_SEVERITY_VALUES = [
  "info",
  "low",
  "medium",
  "high",
  "critical",
] as const satisfies readonly AssociationAuditSeverity[];

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value, CLI_KEY_VALUES, label);
}

function normalizeBoundedLimit(
  label: string,
  limit: number | null | undefined,
  maxLimit: number
): number | null {
  if (limit == null) return null;
  if (!Number.isSafeInteger(limit)) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label} limit=${limit}`);
  }
  return Math.min(Math.max(limit, REQUEST_LOGS_MIN_LIMIT), maxLimit);
}

export function normalizeRequestLogsLimit(limit?: number | null): number | null {
  return normalizeBoundedLimit("request logs", limit, REQUEST_LOGS_MAX_LIMIT);
}

export function normalizeRequestAttemptLogsLimit(limit?: number | null): number | null {
  return normalizeBoundedLimit("request attempt logs", limit, REQUEST_ATTEMPT_LOGS_MAX_LIMIT);
}

export function normalizeRequestLogId(logId: number): number {
  if (!Number.isSafeInteger(logId) || logId <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid logId=${logId}`);
  }
  return logId;
}

export function normalizeRequestLogCursorId(afterId: number): number {
  if (!Number.isSafeInteger(afterId) || afterId < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid afterId=${afterId}`);
  }
  return afterId;
}

export function normalizeRequestLogTraceId(traceId: string): string {
  const normalized = traceId.trim();
  if (
    !normalized ||
    normalized.length > REQUEST_LOG_TRACE_ID_MAX_LENGTH ||
    /[\u0000-\u001f\u007f]/.test(normalized)
  ) {
    throw new Error("SEC_INVALID_INPUT: invalid traceId");
  }
  return normalized;
}

export function normalizeRequestLogTraceIdOrNull(
  traceId: string | null | undefined
): string | null {
  if (traceId == null) return null;
  try {
    return normalizeRequestLogTraceId(traceId);
  } catch {
    return null;
  }
}

function readString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function readFiniteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function readClampedUnitNumber(value: unknown): number | null {
  const number = readFiniteNumber(value);
  if (number == null) return null;
  return Math.min(Math.max(number, 0), 1);
}

function readBoolean(value: unknown): boolean {
  return value === true;
}

function readLiteral<const TAllowed extends readonly string[]>(
  value: unknown,
  allowed: TAllowed,
  fallback: TAllowed[number]
): TAllowed[number] {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as TAllowed[number])
    : fallback;
}

function readRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function normalizeAssociationAuditSignals(value: unknown): AssociationAuditSignal[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item): AssociationAuditSignal | null => {
      const record = readRecord(item);
      if (!record) return null;
      const code = readString(record.code)?.trim() || "unknown";
      const reason = readString(record.reason)?.trim() || "";
      return {
        code,
        severity: readLiteral(record.severity, ASSOCIATION_AUDIT_SEVERITY_VALUES, "info"),
        confidence: readClampedUnitNumber(record.confidence),
        event_index: readFiniteNumber(record.event_index),
        event_kind: readString(record.event_kind),
        reason,
        evidence: readString(record.evidence),
      };
    })
    .filter((item): item is AssociationAuditSignal => item !== null);
}

export function parseAssociationAuditJson(
  raw: string | null | undefined
): ParsedAssociationAudit {
  if (!raw) return { result: null, malformed: false };
  try {
    const parsed = JSON.parse(raw) as unknown;
    const record = readRecord(parsed);
    if (!record) return { result: null, malformed: true, raw };

    const result: AssociationAuditResult = {
      status: readLiteral(record.status, ASSOCIATION_AUDIT_STATUS_VALUES, "failed"),
      reason: readString(record.reason),
      provider_id: readFiniteNumber(record.provider_id),
      provider_name: readString(record.provider_name),
      provider_cli_key: readString(record.provider_cli_key),
      model: readString(record.model),
      started_at_ms: readFiniteNumber(record.started_at_ms),
      finished_at_ms: readFiniteNumber(record.finished_at_ms),
      duration_ms: readFiniteNumber(record.duration_ms),
      input_chars: readFiniteNumber(record.input_chars),
      output_chars: readFiniteNumber(record.output_chars),
      package_truncated: readBoolean(record.package_truncated),
      association_score: readClampedUnitNumber(record.association_score),
      overall_risk: readLiteral(record.overall_risk, ASSOCIATION_AUDIT_RISK_VALUES, "unknown"),
      signals: normalizeAssociationAuditSignals(record.signals),
      insufficient_context: readBoolean(record.insufficient_context),
      notes: readString(record.notes),
    };
    return { result, malformed: false };
  } catch {
    return { result: null, malformed: true, raw };
  }
}

function toRequestLogSummary(value: GeneratedRequestLogSummary): RequestLogSummary {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "request_logs_list.cli_key"),
  };
}

function toRequestLogDetail(value: GeneratedRequestLogDetail): RequestLogDetail {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "request_log_get.cli_key"),
  };
}

function toRequestAttemptLog(value: GeneratedRequestAttemptLog): RequestAttemptLog {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "request_attempt_logs_by_trace_id.cli_key"),
  };
}

export async function requestLogsList(cliKey: CliKey, limit?: number | null) {
  const normalizedLimit = normalizeRequestLogsLimit(limit);

  return invokeGeneratedIpc<RequestLogSummary[]>({
    title: "读取请求日志失败",
    cmd: "request_logs_list",
    args: { cliKey, limit: normalizedLimit },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.requestLogsList(cliKey, normalizedLimit), (rows) =>
        rows.map(toRequestLogSummary)
      ),
  });
}

export async function requestLogsListAll(limit?: number | null) {
  const normalizedLimit = normalizeRequestLogsLimit(limit);

  return invokeGeneratedIpc<RequestLogSummary[]>({
    title: "读取全局请求日志失败",
    cmd: "request_logs_list_all",
    args: { limit: normalizedLimit },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.requestLogsListAll(normalizedLimit), (rows) =>
        rows.map(toRequestLogSummary)
      ),
  });
}

export async function requestLogsListAfterId(
  cliKey: CliKey,
  afterId: number,
  limit?: number | null
) {
  const normalizedLimit = normalizeRequestLogsLimit(limit);
  const normalizedAfterId = normalizeRequestLogCursorId(afterId);

  return invokeGeneratedIpc<RequestLogSummary[]>({
    title: "读取增量请求日志失败",
    cmd: "request_logs_list_after_id",
    args: { cliKey, afterId: normalizedAfterId, limit: normalizedLimit },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.requestLogsListAfterId(cliKey, normalizedAfterId, normalizedLimit),
        (rows) => rows.map(toRequestLogSummary)
      ),
  });
}

export async function requestLogsListAfterIdAll(afterId: number, limit?: number | null) {
  const normalizedLimit = normalizeRequestLogsLimit(limit);
  const normalizedAfterId = normalizeRequestLogCursorId(afterId);

  return invokeGeneratedIpc<RequestLogSummary[]>({
    title: "读取全局增量请求日志失败",
    cmd: "request_logs_list_after_id_all",
    args: { afterId: normalizedAfterId, limit: normalizedLimit },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.requestLogsListAfterIdAll(normalizedAfterId, normalizedLimit),
        (rows) => rows.map(toRequestLogSummary)
      ),
  });
}

export async function requestLogGet(logId: number) {
  const normalizedLogId = normalizeRequestLogId(logId);

  return invokeGeneratedIpc<RequestLogDetail>({
    title: "读取请求日志详情失败",
    cmd: "request_log_get",
    args: { logId: normalizedLogId },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.requestLogGet(normalizedLogId),
        toRequestLogDetail
      ),
  });
}

export async function requestLogGetByTraceId(traceId: string) {
  const normalizedTraceId = normalizeRequestLogTraceId(traceId);

  return invokeGeneratedIpc<RequestLogDetail | null, null>({
    title: "按追踪 ID 读取请求日志失败",
    cmd: "request_log_get_by_trace_id",
    args: { traceId: normalizedTraceId },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.requestLogGetByTraceId(normalizedTraceId),
        (value) => (value == null ? null : toRequestLogDetail(value))
      ),
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
}

export async function requestAttemptLogsByTraceId(traceId: string, limit?: number | null) {
  const normalizedTraceId = normalizeRequestLogTraceId(traceId);
  const normalizedLimit = normalizeRequestAttemptLogsLimit(limit);

  return invokeGeneratedIpc<RequestAttemptLog[]>({
    title: "读取请求尝试日志失败",
    cmd: "request_attempt_logs_by_trace_id",
    args: { traceId: normalizedTraceId, limit: normalizedLimit },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.requestAttemptLogsByTraceId(normalizedTraceId, normalizedLimit),
        (rows) => rows.map(toRequestAttemptLog)
      ),
  });
}
