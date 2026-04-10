import { memo } from "react";
import { Command, Edit2, Globe, Link, Terminal, Trash2 } from "lucide-react";
import type { McpServerSummary } from "../../../services/workspace/mcp";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Switch } from "../../../ui/Switch";

export type McpServerCardProps = {
  server: McpServerSummary;
  toggling: boolean;
  onToggleEnabled: (server: McpServerSummary) => void;
  onEdit: (server: McpServerSummary) => void;
  onDelete: (server: McpServerSummary) => void;
};

function describeServer(server: Pick<McpServerSummary, "transport" | "command" | "url">) {
  if (server.transport === "http") return server.url || "（未填写 url）";
  return server.command || "（未填写 command）";
}

export const McpServerCard = memo(function McpServerCard({
  server,
  toggling,
  onToggleEnabled,
  onEdit,
  onDelete,
}: McpServerCardProps) {
  const serverDescription = describeServer(server);

  return (
    <Card padding="md">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-4 min-w-0">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-slate-100 dark:bg-slate-700 text-slate-500 dark:text-slate-400 ring-1 ring-slate-200 dark:ring-slate-600">
            {server.transport === "http" ? (
              <Globe className="h-6 w-6" />
            ) : (
              <Terminal className="h-6 w-6" />
            )}
          </div>

          <div className="min-w-0 space-y-1">
            <div className="flex items-center gap-2">
              <div className="truncate text-base font-semibold text-slate-900 dark:text-slate-100 leading-tight">
                {server.name}
              </div>
              <span className="inline-flex items-center gap-1 rounded-md bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 text-[10px] font-medium text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-600 uppercase tracking-wider">
                {server.transport}
              </span>
            </div>

            <div className="flex items-center gap-3 text-xs text-slate-500 dark:text-slate-400">
              <div
                className="flex items-center gap-1 truncate max-w-[200px] sm:max-w-xs"
                title={serverDescription}
              >
                {server.transport === "http" ? (
                  <Link className="h-3 w-3 shrink-0" />
                ) : (
                  <Command className="h-3 w-3 shrink-0" />
                )}
                <span className="truncate">{serverDescription}</span>
              </div>
            </div>
          </div>
        </div>

        <div className="flex items-center justify-between gap-4 sm:justify-end">
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-2">
              <Switch
                checked={server.enabled}
                disabled={toggling}
                onCheckedChange={() => onToggleEnabled(server)}
                className="scale-90"
              />
              <span className="text-xs font-medium text-slate-500 dark:text-slate-400">
                {server.enabled ? "已启用" : "未启用"}
              </span>
            </div>
          </div>

          <div className="h-8 w-px bg-slate-200 dark:bg-slate-600" />

          <div className="flex items-center gap-1">
            <Button
              onClick={() => onEdit(server)}
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 text-slate-500 hover:text-indigo-600 hover:bg-indigo-50 dark:text-slate-400 dark:hover:text-indigo-400 dark:hover:bg-indigo-900/30"
              title="编辑"
            >
              <Edit2 className="h-4 w-4" />
            </Button>
            <Button
              onClick={() => onDelete(server)}
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:text-rose-400 dark:hover:bg-rose-900/30"
              title="删除"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
});
