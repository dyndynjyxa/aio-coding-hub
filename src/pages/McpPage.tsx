// Usage: Manage MCP servers for the active workspace of a CLI (renders sub-view under `src/pages/mcp/*`).

import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { CLIS, cliLongLabel } from "../constants/clis";
import { useSettingsQuery } from "../query/settings";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cliPriorityOrder";
import type { CliKey } from "../services/providers";
import { Button } from "../ui/Button";
import { McpServersView } from "./mcp/McpServersView";
import { useWorkspacesListQuery } from "../query/workspaces";

export function McpPage() {
  const navigate = useNavigate();
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;
  const [activeCli, setActiveCli] = useState<CliKey | null>(null);
  const effectiveCli = activeCli ?? defaultCli;

  const workspacesQuery = useWorkspacesListQuery(effectiveCli);
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;
  const loading = workspacesQuery.isFetching;

  const cliLabel = useMemo(() => cliLongLabel(effectiveCli), [effectiveCli]);

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader
        title="MCP"
        actions={
          <TabList
            ariaLabel="目标 CLI"
            items={orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name }))}
            value={effectiveCli}
            onChange={setActiveCli}
          />
        }
      />

      <div className="shrink-0 rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-300">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>这是高级入口：默认操作当前 workspace。推荐在「Workspaces」配置中心统一管理。</div>
          <Button variant="secondary" onClick={() => navigate("/workspaces")}>
            打开 Workspaces
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto scrollbar-overlay">
        {loading ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
        ) : !activeWorkspaceId ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">
            未找到 {cliLabel} 的当前工作区（workspace）。请先在 Workspaces 页面创建并设为当前。
          </div>
        ) : (
          <McpServersView workspaceId={activeWorkspaceId} />
        )}
      </div>
    </div>
  );
}
