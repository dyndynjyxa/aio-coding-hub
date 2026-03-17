// Usage:
// - Render in `HomeOverviewPanel` left column to show each CLI's current workspace and proxy state.

import { CLIS } from "../../constants/clis";
import type { CliKey } from "../../services/providers";
import type { SortModeSummary } from "../../services/sortModes";
import { Card } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { CliBrandIcon } from "./CliBrandIcon";

export type HomeWorkStatusCardProps = {
  layout?: "vertical" | "horizontal";
  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;

  cliProxyEnabled: Record<CliKey, boolean>;
  cliProxyToggling: Record<CliKey, boolean>;
  onSetCliProxyEnabled: (cliKey: CliKey, enabled: boolean) => void;
};

export function HomeWorkStatusCard({
  layout = "vertical",
  sortModes,
  sortModesLoading,
  sortModesAvailable,
  activeModeByCli,
  activeModeToggling,
  cliProxyEnabled,
  cliProxyToggling,
  onSetCliProxyEnabled,
}: HomeWorkStatusCardProps) {
  const horizontal = layout === "horizontal";

  return (
    <Card padding="sm" className="flex h-full flex-1 flex-col">
      <div className="flex items-center justify-between gap-2">
        <div className="text-sm font-semibold">代理状态</div>
      </div>

      {sortModesLoading ? (
        <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">加载中…</div>
      ) : sortModesAvailable === false ? (
        <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">数据不可用</div>
      ) : (
        <div
          className={
            horizontal ? "mt-3 grid grid-cols-1 gap-2.5 md:grid-cols-3" : "mt-3 space-y-2.5"
          }
        >
          {CLIS.map((cli) => {
            const cliKey = cli.key as CliKey;
            const activeModeId = activeModeByCli[cliKey] ?? null;
            const workspaceLabel =
              activeModeToggling[cliKey] || sortModesLoading
                ? "加载中…"
                : activeModeId == null
                  ? "Default"
                  : (sortModes.find((mode) => mode.id === activeModeId)?.name ??
                    `#${activeModeId}`);

            return (
              <div
                key={cli.key}
                className="rounded-lg border border-slate-200 bg-white px-3 py-2.5 shadow-sm transition-all duration-200 hover:bg-slate-50 hover:border-indigo-200 hover:shadow-md dark:border-slate-700 dark:bg-slate-800 dark:shadow-none dark:hover:bg-slate-700 dark:hover:border-indigo-700"
              >
                <div className="flex items-center justify-between gap-3">
                  <div
                    className={
                      horizontal
                        ? "flex min-w-0 items-center gap-2 text-left text-xs font-medium text-slate-700 dark:text-slate-300"
                        : "min-w-0 flex flex-1 items-center gap-2 text-left text-xs font-medium text-slate-700 dark:text-slate-300"
                    }
                  >
                    <CliBrandIcon
                      cliKey={cliKey}
                      className="h-4 w-4 shrink-0 rounded-[4px] object-contain"
                    />
                    <span className="truncate">{cli.name}</span>
                  </div>

                  <div
                    className={
                      horizontal
                        ? "flex shrink-0 items-center justify-end gap-2"
                        : "flex shrink-0 items-center justify-end gap-2"
                    }
                  >
                    <span
                      className="max-w-[140px] rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-xs font-medium text-slate-700 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300"
                      title={`当前工作区：${workspaceLabel}`}
                    >
                      <span className="block truncate">{workspaceLabel}</span>
                    </span>
                    <Switch
                      checked={cliProxyEnabled[cliKey]}
                      disabled={cliProxyToggling[cliKey]}
                      onCheckedChange={(next) => onSetCliProxyEnabled(cliKey, next)}
                      size="sm"
                      aria-label={`${cli.name} 代理开关`}
                    />
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}
