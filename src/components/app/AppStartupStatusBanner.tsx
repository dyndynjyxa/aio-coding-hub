import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { useAppStartupStatus, setAppStartupStatusSnapshot } from "../../app/startupStatusStore";
import { logToConsole } from "../../services/consoleLog";
import { appStartupRetry, type AppStartupStage } from "../../services/app/startupStatus";
import { Button } from "../../ui/Button";

function startupStageLabel(stage: AppStartupStage | null): string {
  switch (stage) {
    case "initializing_db":
      return "数据库初始化";
    case "reading_settings":
      return "设置加载";
    case "starting_gateway":
      return "网关启动";
    case "syncing_cli_proxy":
      return "CLI 代理同步";
    case "finalizing_wsl":
      return "WSL 启动收尾";
    default:
      return "应用启动";
  }
}

export function AppStartupStatusBanner() {
  const navigate = useNavigate();
  const status = useAppStartupStatus();
  const [retrying, setRetrying] = useState(false);

  if (!status || status.currentStage !== "failed") {
    return null;
  }

  const failedStageLabel = startupStageLabel(status.failedStage);
  const detail = status.errorMessage ?? `${failedStageLabel}失败`;

  async function handleRetry() {
    if (!status.canRetry || retrying) {
      return;
    }

    setRetrying(true);
    try {
      const next = await appStartupRetry();
      setAppStartupStatusSnapshot(next);
    } catch (error) {
      logToConsole("error", "重试启动任务失败", {
        error: String(error),
        failed_stage: status.failedStage,
      });
      toast("重试启动失败：请查看 Console 日志");
    } finally {
      setRetrying(false);
    }
  }

  return (
    <div
      role="alert"
      className="mb-4 rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-900/30 dark:text-amber-200"
    >
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0">
          <div className="font-semibold">启动没有完成，当前功能处于降级状态</div>
          <div className="mt-1 break-words text-amber-800 dark:text-amber-300">
            {failedStageLabel}失败：{detail}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Button size="sm" variant="secondary" onClick={handleRetry} disabled={!status.canRetry || retrying}>
            {retrying ? "重试中..." : "重试启动"}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => navigate("/settings")}>
            打开设置
          </Button>
        </div>
      </div>
    </div>
  );
}
