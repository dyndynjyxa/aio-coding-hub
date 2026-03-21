// Usage: Main page for managing providers and sort modes (renders sub-views under `src/pages/providers/*`). Backend commands: `providers_*`, `sort_modes_*`.

import { useState } from "react";
import { CLIS } from "../constants/clis";
import type { CliKey, ProviderSummary } from "../services/providers";
import { useProvidersListQuery } from "../query/providers";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { ProvidersView } from "./providers/ProvidersView";
import { SortModesView } from "./providers/SortModesView";

type ViewKey = CliKey | "sortModes";

const VIEW_TABS: Array<{ key: ViewKey; label: string }> = [
  ...CLIS.map((cli) => ({ key: cli.key, label: cli.name })),
  { key: "sortModes", label: "排序模板" },
];

export function ProvidersPage() {
  const [view, setView] = useState<ViewKey>("claude");
  const [activeCli, setActiveCli] = useState<CliKey>("claude");
  const providersQuery = useProvidersListQuery(activeCli);
  const providers: ProviderSummary[] = providersQuery.data ?? [];
  const providersLoading = providersQuery.isFetching;

  function handleViewChange(next: ViewKey) {
    setView(next);
    if (next !== "sortModes") {
      setActiveCli(next);
    }
  }

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title={view === "sortModes" ? "排序模板" : "供应商"}
        actions={
          <TabList
            ariaLabel="视图切换"
            items={VIEW_TABS}
            value={view}
            onChange={handleViewChange}
          />
        }
      />

      {view !== "sortModes" ? (
        <ProvidersView activeCli={activeCli} setActiveCli={setActiveCli} />
      ) : (
        <SortModesView
          activeCli={activeCli}
          setActiveCli={setActiveCli}
          providers={providers}
          providersLoading={providersLoading}
        />
      )}
    </div>
  );
}
