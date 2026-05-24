export function normalizeBooleanSetting(value: unknown, label: string): boolean {
  if (typeof value === "boolean") return value;
  throw new Error(`SEC_INVALID_INPUT: ${label} must be a boolean`);
}

export function normalizePositiveSafeIntegerSetting(
  value: unknown,
  label: string,
  max: number
): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1 || value > max) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${String(value)}`);
  }
  return value;
}
