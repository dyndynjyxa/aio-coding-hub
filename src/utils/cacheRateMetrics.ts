type MaybeNumber = number | null | undefined;

function normalizeTokenCount(value: MaybeNumber) {
  if (value == null || !Number.isFinite(value)) return 0;
  return Math.max(0, value);
}

// NOTE: effective input tokens are computed by the BACKEND
// (src-tauri/src/domain/usage_stats/tokens.rs) and shipped on request-log rows
// and gateway:request events — the frontend must not re-derive the formula.

export function computeCacheHitRateDenomTokens(
  effectiveInputTokens: MaybeNumber,
  cacheCreationTokens: MaybeNumber,
  cacheReadTokens: MaybeNumber
) {
  const effectiveInput = normalizeTokenCount(effectiveInputTokens);
  const creation = normalizeTokenCount(cacheCreationTokens);
  const read = normalizeTokenCount(cacheReadTokens);
  return effectiveInput + creation + read;
}

export function computeCacheHitRate(
  effectiveInputTokens: MaybeNumber,
  cacheCreationTokens: MaybeNumber,
  cacheReadTokens: MaybeNumber
) {
  const read = normalizeTokenCount(cacheReadTokens);
  const denom = computeCacheHitRateDenomTokens(
    effectiveInputTokens,
    cacheCreationTokens,
    cacheReadTokens
  );
  if (denom <= 0) return NaN;
  return read / denom;
}
