// 类型以生成 bindings 为唯一基准（Rust ClaudeModelMapping 改字段 → 前端 typecheck 翻红）。
import type { ClaudeModelMapping } from "../../generated/bindings";

export type { ClaudeModelMapping } from "../../generated/bindings";

export function normalizeClaudeModelMapping(
  mapping: ClaudeModelMapping | null | undefined
): ClaudeModelMapping | null {
  if (!mapping?.applied) return null;

  const requestedModel = mapping.requestedModel.trim();
  const effectiveModel = mapping.effectiveModel.trim();
  const mappingKind = mapping.mappingKind.trim();
  const providerName = mapping.providerName.trim();
  if (!requestedModel || !effectiveModel || requestedModel === effectiveModel) return null;
  if (!mappingKind || !providerName || !Number.isFinite(mapping.providerId)) return null;

  return {
    requestedModel,
    effectiveModel,
    mappingKind,
    providerId: mapping.providerId,
    providerName,
    applied: true,
  };
}
