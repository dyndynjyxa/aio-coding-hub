import {
  GatewayErrorCodes,
  GatewayErrorDescriptions,
  type GatewayErrorDescription,
} from "../../constants/gatewayErrorCodes";
import { parseAttemptsJson, type AttemptJsonEntry } from "../../services/gateway/attemptsJson";
import type { RequestLogDetail } from "../../services/gateway/requestLogs";

type ParsedReasonFields = {
  reason: string | null;
  upstreamBodyPreview: string | null;
  matchedRule: string | null;
};

type ParsedErrorDetailsJson = {
  attemptDurationMs: number | null;
  circuitFailureCount: number | null;
  circuitFailureThreshold: number | null;
  circuitStateAfter: string | null;
  circuitStateBefore: string | null;
  decision: string | null;
  errorCategory: string | null;
  errorCode: string | null;
  gatewayErrorCode: string | null;
  matchedRule: string | null;
  outcome: string | null;
  providerId: number | null;
  providerIndex: number | null;
  providerName: string | null;
  rawDetailsText: string | null;
  reason: string | null;
  reasonCode: string | null;
  retryIndex: number | null;
  selectionMethod: string | null;
  upstreamBodyPreview: string | null;
  upstreamStatus: number | null;
};

export type AttemptFailureGroup = {
  errorCode: string;
  count: number;
  providerNames: string[];
  // Max structured timeout_secs within the group; non-null only for the
  // GW_UPSTREAM_TIMEOUT group (never derived from outcome strings).
  timeoutSecs: number | null;
  // Circuit attribution from gate-skip attempts: the error code that tripped
  // the breaker (first seen) and the latest recovery point in the group.
  // Both degrade to null for logs without attribution (e.g. after restart).
  circuitTriggerErrorCode: string | null;
  circuitRecoverAtUnix: number | null;
};

export type RequestLogErrorObservation = {
  attemptDurationMs: number | null;
  attemptFailureSummary: AttemptFailureGroup[] | null;
  circuitFailureCount: number | null;
  circuitFailureThreshold: number | null;
  circuitStateAfter: string | null;
  circuitStateBefore: string | null;
  decision: string | null;
  displayErrorCode: string | null;
  errorCategory: string | null;
  gatewayErrorCode: string | null;
  gwDescription: GatewayErrorDescription | null;
  matchedRule: string | null;
  outcome: string | null;
  providerId: number | null;
  providerIndex: number | null;
  providerName: string | null;
  rawDetailsText: string | null;
  reason: string | null;
  reasonCode: string | null;
  retryIndex: number | null;
  selectionMethod: string | null;
  source: "error_details_json" | "summary";
  upstreamBodyPreview: string | null;
  upstreamStatus: number | null;
};

function lookupGatewayErrorDescription(code: string | null): GatewayErrorDescription | null {
  if (!code) return null;
  return GatewayErrorDescriptions[code as keyof typeof GatewayErrorDescriptions] ?? null;
}

function asFiniteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function asOptionalString(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function parseReasonFields(reason: string | null | undefined): ParsedReasonFields {
  const normalizedReason = asOptionalString(reason);
  if (!normalizedReason) {
    return {
      matchedRule: null,
      reason: null,
      upstreamBodyPreview: null,
    };
  }

  const upstreamMarker = "upstream_body=";
  const upstreamMarkerIndex = normalizedReason.indexOf(upstreamMarker);
  const upstreamBodyPreview =
    upstreamMarkerIndex >= 0
      ? asOptionalString(normalizedReason.slice(upstreamMarkerIndex + upstreamMarker.length))
      : null;
  const baseReason =
    upstreamMarkerIndex >= 0
      ? normalizedReason.slice(0, upstreamMarkerIndex).replace(/,\s*$/, "")
      : normalizedReason;
  const matchedRule = baseReason.match(/(?:^|,\s)rule=([^,]+)/)?.[1]?.trim() || null;

  return {
    matchedRule,
    reason: asOptionalString(baseReason),
    upstreamBodyPreview,
  };
}

export function buildAttemptFailureSummary(
  attempts: AttemptJsonEntry[] | null
): AttemptFailureGroup[] | null {
  if (!attempts || attempts.length === 0) return null;

  const groups = new Map<string, AttemptFailureGroup>();
  for (const attempt of attempts) {
    const errorCode = asOptionalString(attempt.error_code);
    if (!errorCode) continue;

    const group = groups.get(errorCode) ?? {
      errorCode,
      count: 0,
      providerNames: [],
      timeoutSecs: null,
      circuitTriggerErrorCode: null,
      circuitRecoverAtUnix: null,
    };
    group.count += 1;

    const providerName = asOptionalString(attempt.provider_name);
    if (providerName && !group.providerNames.includes(providerName)) {
      group.providerNames.push(providerName);
    }

    const timeoutSecs = asFiniteNumber(attempt.timeout_secs);
    if (errorCode === GatewayErrorCodes.UPSTREAM_TIMEOUT && timeoutSecs != null) {
      group.timeoutSecs =
        group.timeoutSecs == null ? timeoutSecs : Math.max(group.timeoutSecs, timeoutSecs);
    }

    const triggerErrorCode = asOptionalString(attempt.circuit_trigger_error_code);
    if (triggerErrorCode && group.circuitTriggerErrorCode == null) {
      group.circuitTriggerErrorCode = triggerErrorCode;
    }
    const recoverAtUnix = asFiniteNumber(attempt.circuit_recover_at_unix);
    if (recoverAtUnix != null) {
      group.circuitRecoverAtUnix =
        group.circuitRecoverAtUnix == null
          ? recoverAtUnix
          : Math.max(group.circuitRecoverAtUnix, recoverAtUnix);
    }

    groups.set(errorCode, group);
  }

  if (groups.size === 0) return null;
  return Array.from(groups.values()).sort((a, b) => b.count - a.count);
}

function parseErrorDetailsJson(
  errorDetailsJson: string | null | undefined
): ParsedErrorDetailsJson {
  if (!errorDetailsJson) {
    return {
      attemptDurationMs: null,
      circuitFailureCount: null,
      circuitFailureThreshold: null,
      circuitStateAfter: null,
      circuitStateBefore: null,
      decision: null,
      errorCategory: null,
      errorCode: null,
      gatewayErrorCode: null,
      matchedRule: null,
      outcome: null,
      providerId: null,
      providerIndex: null,
      providerName: null,
      rawDetailsText: null,
      reason: null,
      reasonCode: null,
      retryIndex: null,
      selectionMethod: null,
      upstreamBodyPreview: null,
      upstreamStatus: null,
    };
  }

  try {
    const parsed = JSON.parse(errorDetailsJson) as unknown;
    if (typeof parsed !== "object" || parsed == null || Array.isArray(parsed)) {
      return {
        attemptDurationMs: null,
        circuitFailureCount: null,
        circuitFailureThreshold: null,
        circuitStateAfter: null,
        circuitStateBefore: null,
        decision: null,
        errorCategory: null,
        errorCode: null,
        gatewayErrorCode: null,
        matchedRule: null,
        outcome: null,
        providerId: null,
        providerIndex: null,
        providerName: null,
        rawDetailsText: errorDetailsJson,
        reason: null,
        reasonCode: null,
        retryIndex: null,
        selectionMethod: null,
        upstreamBodyPreview: null,
        upstreamStatus: null,
      };
    }

    const obj = parsed as Record<string, unknown>;
    const reasonFields = parseReasonFields(asOptionalString(obj.reason));
    return {
      attemptDurationMs: asFiniteNumber(obj.attempt_duration_ms),
      circuitFailureCount: asFiniteNumber(obj.circuit_failure_count),
      circuitFailureThreshold: asFiniteNumber(obj.circuit_failure_threshold),
      circuitStateAfter: asOptionalString(obj.circuit_state_after),
      circuitStateBefore: asOptionalString(obj.circuit_state_before),
      decision: asOptionalString(obj.decision),
      errorCategory: asOptionalString(obj.error_category),
      errorCode: asOptionalString(obj.error_code),
      gatewayErrorCode:
        asOptionalString(obj.gateway_error_code) ?? asOptionalString(obj.request_error_code),
      matchedRule: asOptionalString(obj.matched_rule) ?? reasonFields.matchedRule,
      outcome: asOptionalString(obj.outcome),
      providerId: asFiniteNumber(obj.provider_id),
      providerIndex: asFiniteNumber(obj.provider_index),
      providerName: asOptionalString(obj.provider_name),
      rawDetailsText: Object.keys(obj).length === 0 ? errorDetailsJson : null,
      reason: asOptionalString(obj.reason) ?? reasonFields.reason,
      reasonCode: asOptionalString(obj.reason_code),
      retryIndex: asFiniteNumber(obj.retry_index),
      selectionMethod: asOptionalString(obj.selection_method),
      upstreamBodyPreview:
        asOptionalString(obj.upstream_body_preview) ?? reasonFields.upstreamBodyPreview,
      upstreamStatus: asFiniteNumber(obj.upstream_status),
    };
  } catch {
    return {
      attemptDurationMs: null,
      circuitFailureCount: null,
      circuitFailureThreshold: null,
      circuitStateAfter: null,
      circuitStateBefore: null,
      decision: null,
      errorCategory: null,
      errorCode: null,
      gatewayErrorCode: null,
      matchedRule: null,
      outcome: null,
      providerId: null,
      providerIndex: null,
      providerName: null,
      rawDetailsText: errorDetailsJson,
      reason: null,
      reasonCode: null,
      retryIndex: null,
      selectionMethod: null,
      upstreamBodyPreview: null,
      upstreamStatus: null,
    };
  }
}

function hasObservationSignal(input: RequestLogErrorObservation) {
  return (
    input.displayErrorCode != null ||
    input.gatewayErrorCode != null ||
    input.errorCategory != null ||
    input.outcome != null ||
    input.decision != null ||
    input.reasonCode != null ||
    input.selectionMethod != null ||
    input.matchedRule != null ||
    input.attemptDurationMs != null ||
    input.circuitStateBefore != null ||
    input.circuitStateAfter != null ||
    input.circuitFailureCount != null ||
    input.circuitFailureThreshold != null ||
    input.upstreamStatus != null ||
    input.reason != null ||
    input.upstreamBodyPreview != null ||
    input.providerId != null ||
    input.providerName != null ||
    input.rawDetailsText != null
  );
}

export function resolveRequestLogErrorObservation(
  selectedLog: RequestLogDetail | null | undefined
): RequestLogErrorObservation | null {
  if (!selectedLog) return null;

  const parsedJson = parseErrorDetailsJson(selectedLog.error_details_json);
  const gatewayErrorCode = parsedJson.gatewayErrorCode ?? selectedLog.error_code ?? null;
  const displayErrorCode = parsedJson.errorCode ?? gatewayErrorCode;
  const observation: RequestLogErrorObservation = {
    attemptDurationMs: parsedJson.attemptDurationMs,
    attemptFailureSummary: buildAttemptFailureSummary(parseAttemptsJson(selectedLog.attempts_json)),
    circuitFailureCount: parsedJson.circuitFailureCount,
    circuitFailureThreshold: parsedJson.circuitFailureThreshold,
    circuitStateAfter: parsedJson.circuitStateAfter,
    circuitStateBefore: parsedJson.circuitStateBefore,
    decision: parsedJson.decision,
    displayErrorCode,
    errorCategory: parsedJson.errorCategory,
    gatewayErrorCode,
    gwDescription: lookupGatewayErrorDescription(displayErrorCode),
    matchedRule: parsedJson.matchedRule,
    outcome: parsedJson.outcome,
    providerId: parsedJson.providerId,
    providerIndex: parsedJson.providerIndex,
    providerName: parsedJson.providerName,
    rawDetailsText: parsedJson.rawDetailsText,
    reason: parsedJson.reason,
    reasonCode: parsedJson.reasonCode,
    retryIndex: parsedJson.retryIndex,
    selectionMethod: parsedJson.selectionMethod,
    source: selectedLog.error_details_json != null ? "error_details_json" : "summary",
    upstreamBodyPreview: parsedJson.upstreamBodyPreview,
    upstreamStatus:
      parsedJson.upstreamStatus ??
      (selectedLog.status != null && selectedLog.status >= 400 ? selectedLog.status : null),
  };

  if (hasObservationSignal(observation)) {
    return observation;
  }

  if (selectedLog.error_code != null || (selectedLog.status != null && selectedLog.status >= 400)) {
    return {
      ...observation,
      displayErrorCode: selectedLog.error_code ?? null,
      gatewayErrorCode: selectedLog.error_code ?? null,
      gwDescription: lookupGatewayErrorDescription(selectedLog.error_code ?? null),
      source: "summary",
      upstreamStatus:
        selectedLog.status != null && selectedLog.status >= 400 ? selectedLog.status : null,
    };
  }

  return null;
}
