import { useMemo } from "react";
import type { CliKey } from "../../services/providers";
import { EmptyState } from "../../ui/EmptyState";
import { cn } from "../../utils/cn";
import { CliBrandIcon } from "./CliBrandIcon";
import type { HomeCliWorkspaceConfig } from "./homeWorkspaceConfigTypes";

export type HomeWorkspaceConfigPanelProps = {
  configs: HomeCliWorkspaceConfig[];
  selectedCliKey: CliKey;
  onSelectCliKey: (cliKey: CliKey) => void;
  routeStrategyByCli: Record<CliKey, string>;
};

export function HomeWorkspaceConfigPanel({
  configs,
  selectedCliKey,
  onSelectCliKey,
  routeStrategyByCli,
}: HomeWorkspaceConfigPanelProps) {
  const selectedConfig = useMemo(() => {
    return configs.find((row) => row.cliKey === selectedCliKey) ?? configs[0] ?? null;
  }, [configs, selectedCliKey]);

  if (!selectedConfig) {
    return <EmptyState title="暂无工作区配置信息" />;
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap gap-2">
        {configs.map((config) => {
          const active = config.cliKey === selectedConfig.cliKey;

          return (
            <button
              key={config.cliKey}
              type="button"
              aria-pressed={active}
              onClick={() => onSelectCliKey(config.cliKey)}
              className={cn(
                "inline-flex items-center rounded-full border px-3 py-1.5 text-xs font-medium transition-colors",
                active
                  ? "border-indigo-200 bg-indigo-50 text-indigo-700 dark:border-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300"
                  : "border-slate-200 bg-white text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700"
              )}
            >
              <CliBrandIcon
                cliKey={config.cliKey}
                className="mr-1.5 h-3.5 w-3.5 shrink-0 rounded-[4px] object-contain"
              />
              {config.cliLabel}
            </button>
          );
        })}
      </div>

      <div className="grid gap-2 sm:grid-cols-2">
        <div className="rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-2.5 dark:border-slate-700 dark:bg-slate-800/50">
          <div className="text-[11px] font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
            工作区
          </div>
          <div className="mt-1 text-sm font-medium text-slate-700 dark:text-slate-200">
            {selectedConfig.workspaceName?.trim() || "未设置"}
          </div>
        </div>
        <div className="rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-2.5 dark:border-slate-700 dark:bg-slate-800/50">
          <div className="text-[11px] font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
            路由策略模板
          </div>
          <div className="mt-1 text-sm font-medium text-slate-700 dark:text-slate-200">
            {routeStrategyByCli[selectedConfig.cliKey]}
          </div>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {selectedConfig.loading ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
        ) : selectedConfig.items.length === 0 ? (
          <EmptyState title="当前工作区暂无配置信息" />
        ) : (
          <div className="space-y-2">
            {selectedConfig.items.map((item) => (
              <div
                key={item.id}
                className="flex items-center gap-3 rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-2 dark:border-slate-700 dark:bg-slate-800/50"
              >
                <span className="shrink-0 rounded-full bg-white px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:bg-slate-700 dark:text-slate-300">
                  {item.label}
                </span>
                <div
                  className="min-w-0 truncate text-sm text-slate-700 dark:text-slate-200"
                  title={item.name}
                >
                  {item.name}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
