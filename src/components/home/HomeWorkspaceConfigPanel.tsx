import { useMemo } from "react";
import type { LucideIcon } from "lucide-react";
import { Command, Cpu, Pencil } from "lucide-react";
import type { CliKey } from "../../services/providers";
import type { SortModeSummary } from "../../services/sortModes";
import { EmptyState } from "../../ui/EmptyState";
import { Select } from "../../ui/Select";
import { cn } from "../../utils/cn";
import { CliBrandIcon } from "./CliBrandIcon";
import type {
  HomeCliWorkspaceConfig,
  HomeWorkspaceConfigItemType,
} from "./homeWorkspaceConfigTypes";

const ICON_BY_TYPE: Record<HomeWorkspaceConfigItemType, LucideIcon> = {
  prompts: Pencil,
  mcp: Command,
  skills: Cpu,
};
export type HomeWorkspaceConfigPanelProps = {
  configs: HomeCliWorkspaceConfig[];
  selectedCliKey: CliKey | null;
  onSelectCliKey: (cliKey: CliKey) => void;
  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  onSetCliActiveMode: (cliKey: CliKey, modeId: number | null) => void;
};

export function HomeWorkspaceConfigPanel({
  configs,
  selectedCliKey,
  onSelectCliKey,
  sortModes,
  sortModesLoading,
  sortModesAvailable,
  activeModeByCli,
  activeModeToggling,
  onSetCliActiveMode,
}: HomeWorkspaceConfigPanelProps) {
  const selectedConfig = useMemo(() => {
    return configs.find((row) => row.cliKey === selectedCliKey) ?? configs[0] ?? null;
  }, [configs, selectedCliKey]);
  const selectedModeValue = selectedConfig
    ? String(activeModeByCli[selectedConfig.cliKey] ?? "")
    : "";
  const sortModeSelectDisabled =
    selectedConfig == null ||
    sortModesLoading ||
    sortModesAvailable === false ||
    activeModeToggling[selectedConfig.cliKey];

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

      <div className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_fit-content(240px)]">
        <div className="flex min-w-0 items-center gap-2 rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-1.5 text-sm dark:border-slate-700 dark:bg-slate-800/50">
          <span className="shrink-0 font-medium text-slate-500 dark:text-slate-400">工作区：</span>
          <span className="min-w-0 truncate font-medium text-slate-700 dark:text-slate-200">
            {selectedConfig.workspaceName?.trim() || "默认"}
          </span>
        </div>
        <div className="flex min-w-0 items-center justify-between gap-2 rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-1.5 text-sm dark:border-slate-700 dark:bg-slate-800/50">
          <span className="shrink-0 font-medium text-slate-500 dark:text-slate-400">
            路由策略：
          </span>
          <Select
            value={selectedModeValue}
            onChange={(e) => {
              const nextValue = e.currentTarget.value;
              onSetCliActiveMode(
                selectedConfig.cliKey,
                nextValue === "" ? null : Number(nextValue)
              );
            }}
            disabled={sortModeSelectDisabled}
            className="h-7 min-w-[112px] w-auto flex-none border-slate-200 bg-white px-2 pr-7 text-sm dark:border-slate-600 dark:bg-slate-900"
            aria-label={`${selectedConfig.cliLabel} 路由策略`}
          >
            <option value="">Default</option>
            {sortModes.map((mode) => (
              <option key={mode.id} value={String(mode.id)}>
                {mode.name}
              </option>
            ))}
          </Select>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {selectedConfig.loading ? (
          <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
        ) : selectedConfig.items.length === 0 ? (
          <EmptyState title="当前工作区暂无配置信息" />
        ) : (
          <div className="space-y-2">
            {selectedConfig.items.map((item) => {
              const Icon = ICON_BY_TYPE[item.type];

              return (
                <div
                  key={item.id}
                  className="flex items-center gap-3 rounded-lg border border-slate-200 bg-slate-50/70 px-3 py-2 dark:border-slate-700 dark:bg-slate-800/50"
                >
                  <span className="shrink-0 inline-flex items-center gap-1 rounded-full bg-white px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:bg-slate-700 dark:text-slate-300">
                    <Icon className="h-3 w-3" />
                    {item.label}
                  </span>
                  <div
                    className="min-w-0 truncate text-sm text-slate-700 dark:text-slate-200"
                    title={item.name}
                  >
                    {item.name}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
