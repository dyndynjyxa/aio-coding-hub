import { useRef, useState } from "react";
import type { UsageScope } from "../../services/usage/usage";
import type { UsageTableTab } from "./types";

export function useUsagePageTableState() {
  const [tableTab, setTableTab] = useState<UsageTableTab>("usage");
  const [scope, setScope] = useState<UsageScope>("provider");
  const scopeBeforeCacheTrendRef = useRef<UsageScope>("provider");

  function onChangeTableTab(next: UsageTableTab) {
    if (next === tableTab) return;
    if (next === "cacheTrend") {
      scopeBeforeCacheTrendRef.current = scope;
      if (scope !== "provider") setScope("provider");
    } else {
      const prev = scopeBeforeCacheTrendRef.current;
      if (prev && prev !== scope) setScope(prev);
    }
    setTableTab(next);
  }

  return { tableTab, scope, setScope, onChangeTableTab };
}
