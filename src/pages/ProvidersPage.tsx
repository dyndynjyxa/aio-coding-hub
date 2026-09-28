// Usage: Main page for managing providers and route orders. Backend commands: `providers_*`, `sort_modes_*`.

import { useState } from "react";
import { cliKeysWith } from "../constants/clis";
import type { CliKey } from "../services/providers/providers";
import { useSettingsQuery } from "../query/settings";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cli/cliPriorityOrder";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { ProvidersView } from "./providers/ProvidersView";

export function ProvidersPage() {
  const settingsQuery = useSettingsQuery();
  const providerCliKeys = cliKeysWith("provider");
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order, providerCliKeys);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ??
    providerCliKeys[0];
  const [activeCli, setActiveCli] = useState<CliKey | null>(null);
  const effectiveCli = activeCli ?? defaultCli;
  const viewTabs: Array<{ key: CliKey; label: string }> = orderedCliTabs.map((cli) => ({
    key: cli.key,
    label: cli.name,
  }));

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <PageHeader
        title="供应商"
        actions={
          <TabList
            ariaLabel="CLI 切换"
            items={viewTabs}
            value={effectiveCli}
            onChange={setActiveCli}
          />
        }
      />

      <ProvidersView activeCli={effectiveCli} setActiveCli={setActiveCli} />
    </div>
  );
}
