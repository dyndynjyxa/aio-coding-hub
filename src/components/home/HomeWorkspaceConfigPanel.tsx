import { useMemo, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { Command, Cpu, Pencil } from "lucide-react";
import type { CliKey } from "../../services/providers/providers";
import { EmptyState } from "../../ui/EmptyState";
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
  headerAddon?: ReactNode;
};

export function HomeWorkspaceConfigPanel({
  configs,
  selectedCliKey,
  onSelectCliKey,
  headerAddon,
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

      <div
        className={cn(
          "grid grid-cols-1 gap-2",
          headerAddon != null && "md:grid-cols-[minmax(0,1fr)_fit-content(240px)]"
        )}
      >
        <div className="flex min-w-0 items-center gap-2 rounded-lg border border-border bg-secondary/70 px-3 py-1.5 text-sm dark:border-border dark:bg-secondary/50">
          <span className="shrink-0 font-medium text-muted-foreground">工作区：</span>
          <span className="min-w-0 truncate font-medium text-secondary-foreground dark:text-foreground">
            {selectedConfig.workspaceName?.trim() || "默认"}
          </span>
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
                    className="min-w-0 truncate text-sm text-secondary-foreground dark:text-foreground"
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
