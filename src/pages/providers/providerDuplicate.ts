import type { ClaudeModels, ProviderSummary } from "../../services/providers/providers";

const DUPLICATE_SUFFIX = " 副本";

export type ProviderEditorInitialValues = {
  name: string;
  api_key: string;
  auth_mode: "api_key" | "oauth";
  base_urls: string[];
  base_url_mode: "order" | "ping";
  claude_models: ClaudeModels;
  enabled: boolean;
  cost_multiplier: number;
  limit_5h_usd: number | null;
  limit_daily_usd: number | null;
  daily_reset_mode: "fixed" | "rolling";
  daily_reset_time: string;
  limit_weekly_usd: number | null;
  limit_monthly_usd: number | null;
  limit_total_usd: number | null;
  tags: string[];
  note: string;
  source_provider_id: number | null;
  bridge_type: string | null;
  stream_idle_timeout_seconds: number | null;
};

function normalizeProviderName(name: string) {
  return name.trim().toLowerCase();
}

export function buildDuplicatedProviderName(
  sourceName: string,
  existingProviders: ProviderSummary[]
) {
  const baseName = `${sourceName.trim() || "Provider"}${DUPLICATE_SUFFIX}`;
  const usedNames = new Set(
    existingProviders.map((provider) => normalizeProviderName(provider.name))
  );

  if (!usedNames.has(normalizeProviderName(baseName))) {
    return baseName;
  }

  let index = 2;
  let candidate = `${baseName} ${index}`;
  while (usedNames.has(normalizeProviderName(candidate))) {
    index += 1;
    candidate = `${baseName} ${index}`;
  }

  return candidate;
}

export function buildDuplicatedProviderInitialValues(
  provider: ProviderSummary,
  existingProviders: ProviderSummary[],
  apiKey: string | null
): ProviderEditorInitialValues {
  const isBridge = provider.source_provider_id != null;
  return {
    name: buildDuplicatedProviderName(provider.name, existingProviders),
    api_key: !isBridge && provider.auth_mode === "api_key" ? (apiKey ?? "") : "",
    auth_mode: provider.auth_mode,
    base_urls: [...provider.base_urls],
    base_url_mode: provider.base_url_mode,
    claude_models: { ...(provider.claude_models ?? {}) } as ClaudeModels,
    enabled: provider.enabled,
    cost_multiplier: provider.cost_multiplier,
    limit_5h_usd: provider.limit_5h_usd,
    limit_daily_usd: provider.limit_daily_usd,
    daily_reset_mode: provider.daily_reset_mode,
    daily_reset_time: provider.daily_reset_time,
    limit_weekly_usd: provider.limit_weekly_usd,
    limit_monthly_usd: provider.limit_monthly_usd,
    limit_total_usd: provider.limit_total_usd,
    tags: [...(provider.tags ?? [])],
    note: provider.note ?? "",
    source_provider_id: provider.source_provider_id ?? null,
    bridge_type: provider.bridge_type ?? null,
    stream_idle_timeout_seconds: provider.stream_idle_timeout_seconds ?? null,
  };
}
