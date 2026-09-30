import { CLIS, CLI_KEYS, isCliKey, type CliItem } from "../../constants/clis";
import type { CliKey } from "../providers/providers";

export const DEFAULT_CLI_PRIORITY_ORDER: CliKey[] = [...CLI_KEYS];

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

  DEFAULT_CLI_PRIORITY_ORDER.forEach((cliKey, index) => {
    if (seen.has(cliKey)) return;
    // A CLI missing from the saved order goes right after the CLI that precedes it by default.
    const previous = DEFAULT_CLI_PRIORITY_ORDER.slice(0, index)
      .filter((key) => seen.has(key))
      .pop();
    seen.add(cliKey);
    nextOrder.splice(previous ? nextOrder.indexOf(previous) + 1 : 0, 0, cliKey);
  });

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
