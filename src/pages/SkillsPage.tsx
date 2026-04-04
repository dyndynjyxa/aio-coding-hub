// Usage: Manage installed/local skills for the active workspace of a CLI.

import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { CLIS, cliFromKeyOrDefault, isCliKey } from "../constants/clis";
import { logToConsole } from "../services/consoleLog";
import { getOrderedClis, pickDefaultCliByPriority } from "../services/cliPriorityOrder";
import type { CliKey } from "../services/providers";
import { Button } from "../ui/Button";
import { PageHeader } from "../ui/PageHeader";
import { TabList } from "../ui/TabList";
import { SkillsView } from "./skills/SkillsView";
import { useSettingsQuery } from "../query/settings";
import { useWorkspacesListQuery } from "../query/workspaces";

function readCliFromStorage(): CliKey | null {
  try {
    const raw = localStorage.getItem("skills.activeCli");
    if (isCliKey(raw)) return raw;
  } catch {}
  return null;
}

function writeCliToStorage(cli: CliKey) {
  try {
    localStorage.setItem("skills.activeCli", cli);
  } catch {}
}

export function SkillsPage() {
  const navigate = useNavigate();
  const settingsQuery = useSettingsQuery();
  const orderedCliTabs = getOrderedClis(settingsQuery.data?.cli_priority_order);
  const orderedCliKeys = orderedCliTabs.map((cli) => cli.key);
  const defaultCli =
    pickDefaultCliByPriority(settingsQuery.data?.cli_priority_order, orderedCliKeys) ?? CLIS[0].key;
  const [activeCli, setActiveCli] = useState<CliKey | null>(() => readCliFromStorage());
  const effectiveCli = activeCli ?? defaultCli;
  const currentCli = useMemo(() => cliFromKeyOrDefault(effectiveCli), [effectiveCli]);

  const workspacesQuery = useWorkspacesListQuery(effectiveCli);
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;
  const loading = workspacesQuery.isFetching;

  useEffect(() => {
    writeCliToStorage(effectiveCli);
  }, [effectiveCli]);

  useEffect(() => {
    if (!workspacesQuery.error) return;
    logToConsole("error", "加载工作区失败", {
      error: String(workspacesQuery.error),
      cli: effectiveCli,
    });
    toast("加载失败：请查看控制台日志");
  }, [effectiveCli, workspacesQuery.error]);

  return (
    <div className="flex flex-col gap-6 h-full overflow-hidden">
      <div className="shrink-0">
        <PageHeader
          title="Skill"
          actions={
            <>
              <Button onClick={() => navigate("/skills/market")} variant="primary">
                Skill 市场
              </Button>
              <TabList
                ariaLabel="CLI 选择"
                items={orderedCliTabs.map((cli) => ({ key: cli.key, label: cli.name }))}
                value={effectiveCli}
                onChange={setActiveCli}
              />
            </>
          }
        />
      </div>

      <div className="shrink-0 rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-300">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>这是高级入口：默认操作当前 workspace。推荐在「Workspaces」配置中心统一管理。</div>
          <Button variant="secondary" onClick={() => navigate("/workspaces")}>
            打开 Workspaces
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 lg:overflow-hidden">
        {loading ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
        ) : !activeWorkspaceId ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">
            未找到 {currentCli.name} 的当前工作区（workspace）。请先在 Workspaces
            页面创建并设为当前。
          </div>
        ) : (
          <SkillsView workspaceId={activeWorkspaceId} cliKey={effectiveCli} isActiveWorkspace />
        )}
      </div>
    </div>
  );
}
