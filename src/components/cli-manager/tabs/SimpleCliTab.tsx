import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import type { SimpleCliInfo } from "../../../services/cliManager";
import { cn } from "../../../utils/cn";
import {
  AlertTriangle,
  CheckCircle2,
  FileJson,
  FolderOpen,
  RefreshCw,
  type LucideIcon,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type SimpleCliTabProps = {
  title: string;
  Icon: LucideIcon;
  available: CliManagerAvailability;
  loading: boolean;
  info: SimpleCliInfo | null;
  onRefresh: () => Promise<void> | void;
};

export function SimpleCliTab({
  title,
  Icon,
  available,
  loading,
  info,
  onRefresh,
}: SimpleCliTabProps) {
  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 border-b border-slate-100 pb-6 mb-6">
          <div className="flex items-center gap-4">
            <div className="h-14 w-14 rounded-xl bg-slate-900/5 flex items-center justify-center text-slate-700">
              <Icon className="h-8 w-8" />
            </div>
            <div>
              <h2 className="text-xl font-bold text-slate-900">{title}</h2>
              <div className="flex items-center gap-2 mt-1">
                {available === "available" && info?.found ? (
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 px-2.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                    <CheckCircle2 className="h-3 w-3" />
                    已安装 {info.version}
                  </span>
                ) : available === "checking" || loading ? (
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 px-2.5 py-0.5 text-xs font-medium text-blue-700 ring-1 ring-inset ring-blue-600/20">
                    <RefreshCw className="h-3 w-3 animate-spin" />
                    检测中...
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 px-2.5 py-0.5 text-xs font-medium text-slate-600 ring-1 ring-inset ring-slate-500/10">
                    未检测到
                  </span>
                )}
              </div>
            </div>
          </div>
          <Button
            onClick={() => void onRefresh()}
            variant="secondary"
            size="sm"
            disabled={loading}
            className="gap-2"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            刷新状态
          </Button>
        </div>

        {available === "unavailable" ? (
          <div className="text-sm text-slate-600 text-center py-8">仅在 Tauri Desktop 环境可用</div>
        ) : !info ? (
          <div className="text-sm text-slate-500 text-center py-8">暂无信息，请尝试刷新</div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2">
            <div className="space-y-4">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
                <FolderOpen className="h-4 w-4 text-slate-400" />
                路径信息
              </h3>
              <div className="space-y-3">
                <div>
                  <div className="text-xs text-slate-500 mb-1">可执行文件</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {info.executable_path ?? "—"}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">版本</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {info.version ?? "—"}
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-4">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
                <FileJson className="h-4 w-4 text-slate-400" />
                解析环境
              </h3>
              <div className="space-y-3">
                <div>
                  <div className="text-xs text-slate-500 mb-1">$SHELL</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {info.shell ?? "—"}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">解析方式</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {info.resolved_via}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {info?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 p-4 text-sm text-rose-600 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {info.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
