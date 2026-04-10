// Usage:
// - Render in `HomeOverviewPanel` left column to show each CLI's proxy state.

import { CLIS } from "../../constants/clis";
import type { CliKey } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { CliBrandIcon } from "./CliBrandIcon";

export type HomeWorkStatusCardProps = {
  layout?: "vertical" | "horizontal";
  chrome?: "card" | "plain";
  cliProxyLoading: boolean;
  cliProxyAvailable: boolean | null;

  cliProxyEnabled: Record<CliKey, boolean>;
  cliProxyAppliedToCurrentGateway: Record<CliKey, boolean | null>;
  cliProxyToggling: Record<CliKey, boolean>;
  onSetCliProxyEnabled: (cliKey: CliKey, enabled: boolean) => void;
};

export function HomeWorkStatusCard({
  layout = "vertical",
  chrome = "card",
  cliProxyLoading,
  cliProxyAvailable,
  cliProxyEnabled,
  cliProxyAppliedToCurrentGateway,
  cliProxyToggling,
  onSetCliProxyEnabled,
}: HomeWorkStatusCardProps) {
  const horizontal = layout === "horizontal";
  const plain = chrome === "plain";

  const content = (
    <>
      <div className="flex items-center justify-between gap-2">
        <div className="text-sm font-semibold">代理状态</div>
      </div>

      {cliProxyLoading ? (
        <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">加载中…</div>
      ) : cliProxyAvailable === false ? (
        <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">数据不可用</div>
      ) : (
        <div
          className={
            horizontal ? "mt-3 grid grid-cols-1 gap-2.5 md:grid-cols-3" : "mt-3 space-y-2.5"
          }
        >
          {CLIS.map((cli) => {
            const cliKey = cli.key as CliKey;
            const drifted =
              cliProxyEnabled[cliKey] && cliProxyAppliedToCurrentGateway[cliKey] === false;

            return (
              <div
                key={cli.key}
                className="rounded-lg border border-slate-200 bg-white px-3 py-2.5 shadow-sm transition-all duration-200 hover:bg-slate-50 hover:border-indigo-200 hover:shadow-md dark:border-slate-700 dark:bg-slate-800 dark:shadow-none dark:hover:bg-slate-700 dark:hover:border-indigo-700"
              >
                <div className="flex items-center justify-between gap-3">
                  <div className={cn("min-w-0", !horizontal && "flex-1")}>
                    <div className="flex items-center gap-2 text-left text-xs font-medium text-slate-700 dark:text-slate-300">
                      <CliBrandIcon
                        cliKey={cliKey}
                        className="h-4 w-4 shrink-0 rounded-[4px] object-contain"
                      />
                      <span className="truncate">{cli.name}</span>
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-col items-end justify-center gap-1 text-right">
                    {drifted ? (
                      <>
                        <div className="flex items-center justify-end gap-2">
                          <Button
                            variant="danger"
                            size="sm"
                            className="h-6 px-2 py-0 text-[11px]"
                            disabled={cliProxyToggling[cliKey]}
                            onClick={() => onSetCliProxyEnabled(cliKey, true)}
                            aria-label={`修复 ${cli.name} 代理`}
                          >
                            修复
                          </Button>
                          <Switch
                            checked={cliProxyEnabled[cliKey]}
                            disabled={cliProxyToggling[cliKey]}
                            onCheckedChange={(next) => onSetCliProxyEnabled(cliKey, next)}
                            size="sm"
                            aria-label={`${cli.name} 代理开关`}
                          />
                        </div>
                        <span className="text-[11px] font-medium leading-none text-rose-600 dark:text-rose-400">
                          当前未指向本网关
                        </span>
                      </>
                    ) : (
                      <Switch
                        checked={cliProxyEnabled[cliKey]}
                        disabled={cliProxyToggling[cliKey]}
                        onCheckedChange={(next) => onSetCliProxyEnabled(cliKey, next)}
                        size="sm"
                        aria-label={`${cli.name} 代理开关`}
                      />
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </>
  );

  if (plain) {
    return <div className="flex h-full flex-1 flex-col">{content}</div>;
  }

  return (
    <Card padding="sm" className="flex h-full flex-1 flex-col">
      {content}
    </Card>
  );
}
