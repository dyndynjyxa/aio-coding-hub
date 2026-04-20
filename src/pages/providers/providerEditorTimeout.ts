export function resolveStreamIdleTimeoutSeconds(raw: string) {
  const trimmed = raw.trim();
  if (!trimmed) return 0;
  const value = Number(trimmed);
  if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0 || value > 3600) {
    return undefined;
  }
  return value;
}
