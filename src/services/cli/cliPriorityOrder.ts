import { CLIS, isCliKey, type CliItem } from "../../constants/clis";
import type { CliKey } from "../providers/providers";

export const DEFAULT_CLI_PRIORITY_ORDER: CliKey[] = ["claude", "codex", "gemini"];

const CLI_BY_KEY = new Map<CliKey, CliItem>(CLIS.map((cli) => [cli.key, cli]));

export function normalizeCliPriorityOrder(input: readonly unknown[] | null | undefined): CliKey[] {
  const nextOrder: CliKey[] = [];
  const seen = new Set<CliKey>();

  if (Array.isArray(input)) {
    for (const item of input) {
      if (!isCliKey(item) || seen.has(item)) continue;
      seen.add(item);
      nextOrder.push(item);
    }
  }

  for (const cliKey of DEFAULT_CLI_PRIORITY_ORDER) {
    if (seen.has(cliKey)) continue;
    seen.add(cliKey);
    nextOrder.push(cliKey);
  }

  return nextOrder;
}

function getOrderedCliKeys(
  order: readonly unknown[] | null | undefined,
  allowed?: readonly CliKey[]
): CliKey[] {
  const normalized = normalizeCliPriorityOrder(order);
  if (!allowed) return normalized;

  const allowedSet = new Set(allowed);
  return normalized.filter((cliKey) => allowedSet.has(cliKey));
}

export function getOrderedClis(
  order: readonly unknown[] | null | undefined,
  allowed?: readonly CliKey[]
) {
  return getOrderedCliKeys(order, allowed)
    .map((cliKey) => CLI_BY_KEY.get(cliKey))
    .filter((cli): cli is CliItem => cli != null);
}

export function pickDefaultCliByPriority(
  order: readonly unknown[] | null | undefined,
  allowed: readonly CliKey[]
): CliKey | null {
  return getOrderedCliKeys(order, allowed)[0] ?? null;
}
