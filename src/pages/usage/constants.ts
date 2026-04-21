import type { UsageScope } from "../../services/usage/usage";
import type { TabListItem } from "../../ui/TabList";
import type { UsageTableTab } from "./types";

type ScopeItem = { key: UsageScope; label: string };

/** Sentinel value representing "all providers" in the provider filter select. */
export const PROVIDER_FILTER_ALL = "all" as const;

export const SCOPE_ITEMS: ScopeItem[] = [
  { key: "provider", label: "供应商" },
  { key: "cli", label: "CLI" },
  { key: "model", label: "模型" },
];

export const USAGE_TABLE_TAB_ITEMS = [
  { key: "usage", label: "用量" },
  { key: "cacheTrend", label: "缓存走势图" },
] satisfies Array<TabListItem<UsageTableTab>>;
