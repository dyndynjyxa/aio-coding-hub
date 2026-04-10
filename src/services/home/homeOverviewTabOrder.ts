export type HomeOverviewTabKey = "workspaceConfig" | "circuit" | "sessions" | "providerLimit";

export const HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY = "aio-home-overview-tab-order";

export const HOME_OVERVIEW_TABS: Array<{ key: HomeOverviewTabKey; label: string }> = [
  { key: "workspaceConfig", label: "配置信息" },
  { key: "circuit", label: "熔断信息" },
  { key: "sessions", label: "活跃 Session" },
  { key: "providerLimit", label: "供应商限额" },
];

export function normalizeHomeOverviewTabOrder(input: unknown): HomeOverviewTabKey[] {
  const defaultKeys = HOME_OVERVIEW_TABS.map((item) => item.key);
  if (!Array.isArray(input)) return defaultKeys;

  const seen = new Set<HomeOverviewTabKey>();
  const normalized: HomeOverviewTabKey[] = [];

  for (const value of input) {
    if (
      typeof value === "string" &&
      defaultKeys.includes(value as HomeOverviewTabKey) &&
      !seen.has(value as HomeOverviewTabKey)
    ) {
      normalized.push(value as HomeOverviewTabKey);
      seen.add(value as HomeOverviewTabKey);
    }
  }

  for (const key of defaultKeys) {
    if (!seen.has(key)) normalized.push(key);
  }

  return normalized;
}

export function readHomeOverviewTabOrderFromStorage(): HomeOverviewTabKey[] {
  if (typeof window === "undefined") return HOME_OVERVIEW_TABS.map((item) => item.key);

  try {
    const raw = window.localStorage.getItem(HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY);
    if (!raw) return HOME_OVERVIEW_TABS.map((item) => item.key);
    return normalizeHomeOverviewTabOrder(JSON.parse(raw));
  } catch {
    return HOME_OVERVIEW_TABS.map((item) => item.key);
  }
}

export function writeHomeOverviewTabOrderToStorage(order: HomeOverviewTabKey[]) {
  if (typeof window === "undefined") return;

  try {
    window.localStorage.setItem(
      HOME_OVERVIEW_TAB_ORDER_STORAGE_KEY,
      JSON.stringify(normalizeHomeOverviewTabOrder(order))
    );
  } catch {}
}
