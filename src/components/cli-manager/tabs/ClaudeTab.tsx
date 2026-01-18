import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { toast } from "sonner";
import type { ClaudeCliInfo } from "../../../services/cliManager";
import { cn } from "../../../utils/cn";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { Switch } from "../../../ui/Switch";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  ExternalLink,
  FileJson,
  FolderOpen,
  RefreshCw,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerClaudeTabProps = {
  claudeAvailable: CliManagerAvailability;
  claudeLoading: boolean;
  claudeSaving: boolean;
  claudeInfo: ClaudeCliInfo | null;
  claudeMcpTimeoutMsText: string;
  setClaudeMcpTimeoutMsText: (value: string) => void;
  refreshClaudeInfo: () => Promise<void> | void;
  openClaudeConfigDir: () => Promise<void> | void;
  persistClaudeEnv: (input: {
    mcp_timeout_ms: number | null;
    disable_error_reporting: boolean;
  }) => Promise<void> | void;
  normalizeClaudeMcpTimeoutMsOrNull: (raw: string) => number | null;
  blurOnEnter: (e: ReactKeyboardEvent<HTMLInputElement>) => void;
  maxMcpTimeoutMs: number;
};

export function CliManagerClaudeTab({
  claudeAvailable,
  claudeLoading,
  claudeSaving,
  claudeInfo,
  claudeMcpTimeoutMsText,
  setClaudeMcpTimeoutMsText,
  refreshClaudeInfo,
  openClaudeConfigDir,
  persistClaudeEnv,
  normalizeClaudeMcpTimeoutMsOrNull,
  blurOnEnter,
  maxMcpTimeoutMs,
}: CliManagerClaudeTabProps) {
  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 border-b border-slate-100 pb-6 mb-6">
          <div className="flex items-center gap-4">
            <div className="h-14 w-14 rounded-xl bg-[#D97757]/10 flex items-center justify-center text-[#D97757]">
              <Bot className="h-8 w-8" />
            </div>
            <div>
              <h2 className="text-xl font-bold text-slate-900">Claude Code</h2>
              <div className="flex items-center gap-2 mt-1">
                {claudeAvailable === "available" && claudeInfo?.found ? (
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 px-2.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                    <CheckCircle2 className="h-3 w-3" />
                    已安装 {claudeInfo.version}
                  </span>
                ) : claudeAvailable === "checking" || claudeLoading ? (
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
            onClick={() => void refreshClaudeInfo()}
            variant="secondary"
            size="sm"
            disabled={claudeLoading}
            className="gap-2"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", claudeLoading && "animate-spin")} />
            刷新状态
          </Button>
        </div>

        {claudeAvailable === "unavailable" ? (
          <div className="text-sm text-slate-600 text-center py-8">仅在 Tauri Desktop 环境可用</div>
        ) : !claudeInfo ? (
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
                    {claudeInfo.executable_path ?? "—"}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">SHELL ($SHELL)</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {claudeInfo.shell ?? "—"}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">解析方式</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {claudeInfo.resolved_via}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">配置目录</div>
                  <div className="flex gap-2">
                    <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all flex-1">
                      {claudeInfo.config_dir}
                    </div>
                    <Button
                      onClick={() => void openClaudeConfigDir()}
                      size="sm"
                      variant="secondary"
                      className="shrink-0 h-auto py-1"
                    >
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
                <div>
                  <div className="text-xs text-slate-500 mb-1">settings.json</div>
                  <div className="font-mono text-xs text-slate-700 bg-slate-50 p-2 rounded border border-slate-100 break-all">
                    {claudeInfo.settings_path}
                  </div>
                </div>
              </div>
            </div>

            <div className="space-y-4">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
                <FileJson className="h-4 w-4 text-slate-400" />
                环境配置 (env)
              </h3>
              <div className="rounded-lg border border-slate-200 bg-white p-4 space-y-4">
                <div>
                  <label className="text-sm font-medium text-slate-700 mb-1 block">
                    MCP_TIMEOUT (ms)
                  </label>
                  <div className="flex gap-2">
                    <Input
                      type="number"
                      value={claudeMcpTimeoutMsText}
                      onChange={(e) => setClaudeMcpTimeoutMsText(e.currentTarget.value)}
                      onBlur={() => {
                        if (!claudeInfo) return;
                        const normalized =
                          normalizeClaudeMcpTimeoutMsOrNull(claudeMcpTimeoutMsText);
                        if (normalized !== null && !Number.isFinite(normalized)) {
                          toast(`MCP_TIMEOUT 必须为 0-${maxMcpTimeoutMs} 毫秒`);
                          setClaudeMcpTimeoutMsText(
                            claudeInfo.mcp_timeout_ms == null
                              ? ""
                              : String(claudeInfo.mcp_timeout_ms)
                          );
                          return;
                        }
                        void persistClaudeEnv({
                          mcp_timeout_ms: normalized,
                          disable_error_reporting: claudeInfo.disable_error_reporting,
                        });
                      }}
                      onKeyDown={blurOnEnter}
                      className="font-mono"
                      min={0}
                      max={maxMcpTimeoutMs}
                      disabled={claudeSaving}
                      placeholder="默认"
                    />
                  </div>
                  <p className="mt-1.5 text-xs text-slate-500">
                    MCP 连接超时时间。留空或 0 表示使用默认值。
                  </p>
                </div>

                <div className="flex items-center justify-between py-2">
                  <div>
                    <div className="text-sm font-medium text-slate-700">
                      DISABLE_ERROR_REPORTING
                    </div>
                    <div className="text-xs text-slate-500">禁用错误上报功能</div>
                  </div>
                  <Switch
                    checked={claudeInfo.disable_error_reporting}
                    onCheckedChange={(checked) => {
                      void persistClaudeEnv({
                        mcp_timeout_ms: claudeInfo.mcp_timeout_ms,
                        disable_error_reporting: checked,
                      });
                    }}
                    disabled={claudeSaving}
                  />
                </div>
              </div>
            </div>
          </div>
        )}

        {claudeInfo?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 p-4 text-sm text-rose-600 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {claudeInfo.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
