// Usage: UI for configuring local CLI integrations and related app settings. Backend commands: `cli_manager_*`, `settings_*`, `cli_proxy_*`, `gateway_*`.

import { lazy, Suspense } from "react";
import { CliManagerGeneralTab } from "../components/cli-manager/tabs/GeneralTab";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import {
  CLI_MANAGER_TABS,
  useCliManagerPageDataModel,
} from "./cli-manager/useCliManagerPageDataModel";

const LazyClaudeTab = lazy(() =>
  import("../components/cli-manager/tabs/ClaudeTab").then((m) => ({
    default: m.CliManagerClaudeTab,
  }))
);

const LazyCodexTab = lazy(() =>
  import("../components/cli-manager/tabs/CodexTab").then((m) => ({
    default: m.CliManagerCodexTab,
  }))
);

const LazyCx2ccTab = lazy(() =>
  import("../components/cli-manager/tabs/Cx2ccTab").then((m) => ({
    default: m.CliManagerCx2ccTab,
  }))
);

const LazyGeminiTab = lazy(() =>
  import("../components/cli-manager/tabs/GeminiTab").then((m) => ({
    default: m.CliManagerGeminiTab,
  }))
);

const TAB_FALLBACK = <div className="p-6 text-sm text-muted-foreground">加载中…</div>;

export function CliManagerPage() {
  const model = useCliManagerPageDataModel();

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader
        title="CLI 管理"
        actions={
          <TabList
            ariaLabel="CLI 管理视图切换"
            items={CLI_MANAGER_TABS}
            value={model.tab}
            onChange={model.setTab}
          />
        }
      />

      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay">
        {model.tab === "general" ? <CliManagerGeneralTab {...model.generalTabProps} /> : null}

        {model.tab === "claude" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyClaudeTab {...model.claudeTabProps} />
          </Suspense>
        ) : null}

        {model.tab === "codex" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyCodexTab {...model.codexTabProps} />
          </Suspense>
        ) : null}

        {model.tab === "cx2cc" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyCx2ccTab {...model.cx2ccTabProps} />
          </Suspense>
        ) : null}

        {model.tab === "gemini" ? (
          <Suspense fallback={TAB_FALLBACK}>
            <LazyGeminiTab {...model.geminiTabProps} />
          </Suspense>
        ) : null}
      </div>
    </div>
  );
}
