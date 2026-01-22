// Usage: Main page for managing providers and sort modes (renders sub-views under `src/pages/providers/*`). Backend commands: `providers_*`, `sort_modes_*`.

import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../services/consoleLog";
import { providersList, type CliKey, type ProviderSummary } from "../services/providers";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { ProvidersView } from "./providers/ProvidersView";
import { SortModesView } from "./providers/SortModesView";

type ViewKey = "providers" | "sortModes";

const VIEW_TABS: Array<{ key: ViewKey; label: string }> = [
  { key: "providers", label: "供应商" },
  { key: "sortModes", label: "排序模板" },
];

export function ProvidersPage() {
  const [view, setView] = useState<ViewKey>("providers");

  const [activeCli, setActiveCli] = useState<CliKey>("claude");
  const activeCliRef = useRef(activeCli);
  useEffect(() => {
    activeCliRef.current = activeCli;
  }, [activeCli]);

  const [providers, setProviders] = useState<ProviderSummary[]>([]);
  const [providersLoading, setProvidersLoading] = useState(false);

  async function refreshProviders(cliKey: CliKey) {
    setProvidersLoading(true);
    try {
      const items = await providersList(cliKey);
      if (activeCliRef.current !== cliKey) return;
      if (!items) {
        setProviders([]);
        return;
      }
      setProviders(items);
    } catch (err) {
      if (activeCliRef.current !== cliKey) return;
      logToConsole("error", "读取供应商失败", {
        cli: cliKey,
        error: String(err),
      });
      toast("读取供应商失败：请查看控制台日志");
    } finally {
      if (activeCliRef.current === cliKey) {
        setProvidersLoading(false);
      }
    }
  }

  useEffect(() => {
    void refreshProviders(activeCli);
  }, [activeCli]);

  return (
    <div className="flex flex-col gap-6 lg:h-[calc(100vh-40px)] lg:overflow-hidden">
      <PageHeader
        title={view === "providers" ? "供应商" : "排序模板"}
        actions={
          <TabList ariaLabel="视图切换" items={VIEW_TABS} value={view} onChange={setView} />
        }
      />

      {view === "providers" ? (
        <ProvidersView
          activeCli={activeCli}
          setActiveCli={setActiveCli}
          providers={providers}
          setProviders={setProviders}
          providersLoading={providersLoading}
          refreshProviders={refreshProviders}
        />
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
