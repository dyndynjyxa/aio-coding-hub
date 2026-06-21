import { useMemo, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { Command, Cpu, Pencil } from "lucide-react";
import type { CliKey } from "../../services/providers/providers";
import { EmptyState } from "../../ui/EmptyState";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { CliBrandIcon } from "./CliBrandIcon";
import type {
  HomeCliWorkspaceConfig,
  HomeWorkspaceConfigItem,
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
  headerAddon?: ReactNode;
  showQuickToggle?: boolean;
  togglingItemIds?: Set<string>;
  switchingWorkspaceKey?: string | null;
  onSwitchWorkspace?: (cliKey: CliKey, workspaceId: number) => void;
  onToggleItemEnabled?: (
    workspaceId: number,
    item: HomeWorkspaceConfigItem,
    enabled: boolean
  ) => void;
};

export function HomeWorkspaceConfigPanel({
  configs,
  selectedCliKey,
  onSelectCliKey,
  headerAddon,
  showQuickToggle = false,
  togglingItemIds,
  switchingWorkspaceKey,
  onSwitchWorkspace,
  onToggleItemEnabled,
}: HomeWorkspaceConfigPanelProps) {
  const selectedConfig = useMemo(() => {
    return configs.find((row) => row.cliKey === selectedCliKey) ?? configs[0] ?? null;
  }, [configs, selectedCliKey]);
  const selectedWorkspaceId = selectedConfig?.workspaceId ?? null;
  const selectedWorkspaceValue = selectedWorkspaceId == null ? "" : String(selectedWorkspaceId);
  const workspaceOptions = selectedConfig?.workspaces ?? [];
  const switchingSelectedCli =
    selectedConfig != null && switchingWorkspaceKey?.startsWith(`${selectedConfig.cliKey}:`);
  const workspaceSelectDisabled =
    selectedConfig == null ||
    selectedConfig.loading ||
    selectedWorkspaceId == null ||
    workspaceOptions.length === 0 ||
    switchingSelectedCli ||
    onSwitchWorkspace == null;

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
                  : "border-border bg-white text-muted-foreground hover:bg-secondary dark:border-border dark:bg-secondary dark:text-foreground dark:hover:bg-muted"
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

      <div className="grid grid-cols-1 gap-2">
        <div
          className={cn(
            "flex min-w-0 items-center gap-1.5 rounded-lg border border-border bg-secondary/70 px-2.5 py-1 text-sm dark:border-border dark:bg-secondary/50",
            headerAddon == null && "w-full"
          )}
        >
          <span className="shrink-0 font-medium text-muted-foreground">工作区：</span>
          <Select
            value={selectedWorkspaceValue}
            disabled={workspaceSelectDisabled}
            aria-label={`${selectedConfig.cliLabel} 工作区`}
            title={selectedConfig.workspaceName?.trim() || "默认"}
            onChange={(event) => {
              const nextWorkspaceId = Number(event.currentTarget.value);
              if (!Number.isSafeInteger(nextWorkspaceId) || nextWorkspaceId <= 0) return;
              if (nextWorkspaceId === selectedWorkspaceId) return;
              onSwitchWorkspace?.(selectedConfig.cliKey, nextWorkspaceId);
            }}
            className="h-7 min-w-0 flex-1 border-0 bg-transparent px-0 py-0 text-sm font-semibold text-secondary-foreground shadow-none outline-none focus:border-transparent focus:bg-transparent focus:ring-0 focus:ring-offset-0 disabled:bg-transparent dark:bg-transparent dark:text-foreground dark:disabled:bg-transparent"
          >
            {workspaceOptions.length === 0 ? (
              <option value={selectedWorkspaceValue}>
                {selectedConfig.workspaceName?.trim() || "默认"}
              </option>
            ) : (
              workspaceOptions.map((workspace) => (
                <option key={workspace.id} value={String(workspace.id)}>
                  {workspace.name.trim() || "默认"}
                </option>
              ))
            )}
          </Select>
          {switchingSelectedCli ? (
            <span className="shrink-0 text-xs text-muted-foreground">切换中…</span>
          ) : null}
        </div>
        {headerAddon}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto pr-1 scrollbar-overlay">
        {selectedConfig.loading ? (
          <div className="text-sm text-muted-foreground">加载中…</div>
        ) : selectedConfig.items.length === 0 ? (
          <EmptyState title="当前工作区暂无配置信息" />
        ) : (
          <div className="space-y-2">
            {selectedConfig.items.map((item) => {
              const Icon = ICON_BY_TYPE[item.type];

              return (
                <div
                  key={item.id}
                  className="flex items-center gap-3 rounded-lg border border-border bg-secondary/70 px-3 py-2 dark:border-border dark:bg-secondary/50"
                >
                  <span className="shrink-0 inline-flex items-center gap-1 rounded-full bg-white/80 px-2 py-0.5 text-[11px] font-medium text-muted-foreground dark:bg-white/10 dark:text-foreground">
                    <Icon className="h-3 w-3" />
                    {item.label}
                  </span>
                  <div
                    className="min-w-0 flex-1 truncate text-sm text-secondary-foreground dark:text-foreground"
                    title={item.name}
                  >
                    {item.name}
                  </div>
                  {showQuickToggle ? (
                    <div className="ml-auto flex shrink-0 items-center">
                      <Switch
                        size="sm"
                        checked={item.enabled}
                        disabled={
                          selectedConfig.workspaceId == null ||
                          togglingItemIds?.has(item.id) ||
                          onToggleItemEnabled == null
                        }
                        aria-label={`${item.label} ${item.name} 启用状态`}
                        onCheckedChange={(next) => {
                          if (selectedConfig.workspaceId == null) return;
                          onToggleItemEnabled?.(selectedConfig.workspaceId, item, next);
                        }}
                      />
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
