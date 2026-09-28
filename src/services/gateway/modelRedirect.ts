import { GATEWAY_EVENT_TEXT_LIMITS } from "../../constants/gatewayEvents";
import type { ModelRedirect } from "../../generated/bindings";
import { normalizeClaudeModelMapping, type ClaudeModelMapping } from "./claudeModelMapping";

export type { ModelRedirect } from "../../generated/bindings";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function normalizeModelRedirect(value: unknown): ModelRedirect | null {
  if (!isRecord(value)) return null;
  const stage = typeof value.stage === "string" ? value.stage.trim() : "";
  const providerName = typeof value.providerName === "string" ? value.providerName.trim() : "";
  const sourceModel = typeof value.sourceModel === "string" ? value.sourceModel.trim() : "";
  const targetModel = typeof value.targetModel === "string" ? value.targetModel.trim() : "";
  const providerId = value.providerId;
  if (
    !stage ||
    !providerName ||
    !sourceModel ||
    !targetModel ||
    sourceModel === targetModel ||
    typeof providerId !== "number" ||
    !Number.isFinite(providerId) ||
    stage.length > GATEWAY_EVENT_TEXT_LIMITS.STATE_MAX_LENGTH ||
    providerName.length > GATEWAY_EVENT_TEXT_LIMITS.SHORT_TEXT_MAX_LENGTH ||
    sourceModel.length > GATEWAY_EVENT_TEXT_LIMITS.SHORT_TEXT_MAX_LENGTH ||
    targetModel.length > GATEWAY_EVENT_TEXT_LIMITS.SHORT_TEXT_MAX_LENGTH
  ) {
    return null;
  }
  return { stage, providerId, providerName, sourceModel, targetModel };
}

export function modelRedirectFromClaudeModelMapping(
  mapping: ClaudeModelMapping | null | undefined
): ModelRedirect | null {
  const normalized = normalizeClaudeModelMapping(mapping);
  if (!normalized) return null;
  return {
    stage: "legacy",
    providerId: normalized.providerId,
    providerName: normalized.providerName,
    sourceModel: normalized.requestedModel,
    targetModel: normalized.effectiveModel,
  };
}
