import type {
  ProviderModelDiscoveryErrorCode,
  ProviderModelDiscoveryUnsupportedReason,
  ProviderModelMapping,
  ProviderModelPolicyV1,
} from "../../services/providers/providers";

export const DEFAULT_PROVIDER_MODEL_POLICY: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  modelPatterns: [],
  mappings: [],
};

export function cloneProviderModelPolicy(policy: ProviderModelPolicyV1): ProviderModelPolicyV1 {
  return {
    version: policy.version,
    mode: policy.mode,
    modelPatterns: [...policy.modelPatterns],
    mappings: policy.mappings.map((mapping) => ({ ...mapping })),
  };
}

export function validateProviderModelPolicy(policy: ProviderModelPolicyV1): string | null {
  if (policy.version !== 1) return "模型策略版本不受支持";

  const patternSources = new Set<string>();
  for (const rawPattern of policy.modelPatterns) {
    const pattern = rawPattern.trim();
    const error = validateModelPattern(pattern, "模型");
    if (error) return error;
    if (patternSources.has(pattern)) return "模型不能重复";
    patternSources.add(pattern);
  }

  const mappingSources = new Set<string>();
  for (const mapping of policy.mappings) {
    const source = mapping.source.trim();
    const target = mapping.target.trim();
    const sourceError = validateModelPattern(source, "请求模型");
    if (sourceError) return sourceError;
    if (!target) return "上游模型不能为空";
    if ((target.match(/\*/g) ?? []).length > 1) return "上游模型最多包含一个 *";
    if (!source.includes("*") && target.includes("*")) {
      return "上游模型使用 * 时，请求模型也必须使用 *";
    }
    if (mappingSources.has(source)) return "请求模型不能重复";
    mappingSources.add(source);
  }

  const uniqueSources = new Set([...patternSources, ...mappingSources]);
  if (policy.mode === "selected" && uniqueSources.size === 0) {
    return "仅这些可用模式至少需要一个模型或映射";
  }
  return null;
}

function validateModelPattern(value: string, label: string) {
  if (!value) return `${label}不能为空`;
  if ((value.match(/\*/g) ?? []).length > 1) return `${label}最多包含一个 *`;
  return null;
}

export function normalizeProviderModelPolicyDraft(policy: ProviderModelPolicyV1) {
  return {
    ...policy,
    modelPatterns: policy.modelPatterns.map((pattern) => pattern.trim()),
    mappings: policy.mappings.map<ProviderModelMapping>((mapping) => ({
      source: mapping.source.trim(),
      target: mapping.target.trim(),
    })),
  };
}

export type ProviderModelDiscoveryUiState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "changed" }
  | {
      status: "ready";
      models: string[];
      origin: string;
      baseUrlIndex: number | null;
    }
  | { status: "empty"; origin: string; baseUrlIndex: number | null }
  | { status: "unsupported"; reason: ProviderModelDiscoveryUnsupportedReason }
  | { status: "oauth_unsaved" }
  | { status: "error"; code: ProviderModelDiscoveryErrorCode; httpStatus: number | null }
  | { status: "unexpected_error" };
