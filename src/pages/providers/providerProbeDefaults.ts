// Usage: Derives availability-probe defaults from a provider's model policy.
// Mirrors the backend rule in `src-tauri/src/domain/provider_availability.rs`
// (`probe_model_from_policy`) so the dialog opens on the model the backend would
// have picked anyway. Mapping to the upstream model name stays server-side.

import type { ProviderModelPolicyV1 } from "../../services/providers/providers";

export const DEFAULT_PROBE_PROMPT = "hi";

/** Concrete (wildcard-free) models the user configured for this provider. */
export function probeModelCandidates(policy: ProviderModelPolicyV1 | null): string[] {
  if (!policy) return [];

  // `excluded` patterns are a blocklist, not a menu of models to probe.
  const patterns = policy.mode === "excluded" ? [] : policy.modelPatterns;
  const candidates = [...patterns, ...policy.mappings.map((mapping) => mapping.source)]
    .map((model) => model.trim())
    .filter((model) => model && !model.includes("*"));

  return [...new Set(candidates)];
}

export function defaultProbeModel(policy: ProviderModelPolicyV1 | null): string {
  return probeModelCandidates(policy)[0] ?? "";
}
