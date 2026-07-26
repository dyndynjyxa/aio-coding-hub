// Usage:
// - special_settings_json 的展示层解析助手（Claude 模型映射文案、Codex 优先服务层判定）。
// - 底层解析逻辑位于 services/gateway/requestLogSpecialSettings，此处仅做 Home 展示相关派生。

import {
  normalizeClaudeModelMapping,
  type ClaudeModelMapping,
} from "../../services/gateway/claudeModelMapping";

export {
  hasClaudeModelMappingSpecialSetting,
  resolveClaudeModelMappingFromSpecialSettings,
} from "../../services/gateway/requestLogSpecialSettings";

export function formatClaudeModelMappingText(
  requestedModel: string | null | undefined,
  mapping: ClaudeModelMapping | null | undefined
) {
  const normalized = normalizeClaudeModelMapping(mapping);
  if (normalized) {
    return `${normalized.requestedModel} → ${normalized.effectiveModel}`;
  }

  const fallback = requestedModel?.trim();
  return fallback || "未知";
}

type CodexServiceTierResultSetting = {
  type: "codex_service_tier_result";
  requestedServiceTier?: string | null;
  actualServiceTier?: string | null;
  billingSourcePreference?: string | null;
  resolvedFrom?: string | null;
  effectivePriority?: boolean;
};

function isCodexServiceTierResultSetting(value: unknown): value is CodexServiceTierResultSetting {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as Record<string, unknown>).type === "codex_service_tier_result"
  );
}

/**
 * Check if the request has priority service tier applied (Codex fast mode).
 */
export function hasPriorityServiceTierSpecialSetting(
  specialSettingsJson: string | null | undefined
): boolean {
  if (!specialSettingsJson) return false;

  try {
    const settings = JSON.parse(specialSettingsJson) as unknown;
    if (!Array.isArray(settings)) return false;

    const codexTierSetting = [...settings].reverse().find(isCodexServiceTierResultSetting);
    if (!codexTierSetting) return false;

    // Legacy compatibility: if no billingSourcePreference, check actualServiceTier
    if (
      codexTierSetting.billingSourcePreference == null &&
      codexTierSetting.resolvedFrom == null &&
      codexTierSetting.actualServiceTier != null
    ) {
      return codexTierSetting.actualServiceTier === "priority";
    }

    return codexTierSetting.effectivePriority === true;
  } catch {
    return false;
  }
}
