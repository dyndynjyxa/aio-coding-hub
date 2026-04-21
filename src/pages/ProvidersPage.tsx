// Usage: Main page for managing providers and sort modes (renders sub-views under `src/pages/providers/*`). Backend commands: `providers_*`, `sort_modes_*`.

import { useState } from "react";
import { CLIS } from "../constants/clis";
import type { CliKey, ProviderSummary } from "../services/providers/providers";
import { useProvidersListQuery } from "../query/providers";
import { useSettingsQuery } from "../query/settings";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cli/cliPriorityOrder";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { ProvidersView } from "./providers/ProvidersView";
import { SortModesView } from "./providers/SortModesView";

type ViewKey = CliKey | "sortModes";

export function ProvidersPage() {
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;
  const [view, setView] = useState<ViewKey | null>(null);
  const [activeCli, setActiveCli] = useState<CliKey | null>(null);
  const effectiveCli = activeCli ?? defaultCli;
  const effectiveView = view ?? defaultCli;
  const providersQuery = useProvidersListQuery(effectiveCli);
  const providers: ProviderSummary[] = providersQuery.data ?? [];
  const providersLoading = providersQuery.isFetching;
  const viewTabs: Array<{ key: ViewKey; label: string }> = [
    ...orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name })),
    { key: "sortModes", label: "排序模板" },
  ];

  function handleViewChange(next: ViewKey) {
    setView(next);
    if (next !== "sortModes") {
      setActiveCli(next);
    }
  }

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title={effectiveView === "sortModes" ? "排序模板" : "供应商"}
        actions={
          <TabList
            ariaLabel="视图切换"
            items={viewTabs}
            value={effectiveView}
            onChange={handleViewChange}
          />
        }
      />

      {effectiveView !== "sortModes" ? (
        <ProvidersView activeCli={effectiveCli} setActiveCli={setActiveCli} />
      ) : (
        <SortModesView
          activeCli={effectiveCli}
          setActiveCli={setActiveCli}
          providers={providers}
          providersLoading={providersLoading}
        />
      )}
    </div>
  );
}
