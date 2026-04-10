import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import {
  cliCheckLatestVersion,
  cliUpdateCli,
  type CliVersionCheck,
} from "../../services/cli/cliUpdate";
import { cn } from "../../utils/cn";

type CliVersionBadgeProps = {
  cliKey: string;
  installedVersion: string | null;
  refreshToken?: number;
  onUpdateComplete?: () => void;
};

export function CliVersionBadge({
  cliKey,
  installedVersion,
  refreshToken = 0,
  onUpdateComplete,
}: CliVersionBadgeProps) {
  const [checking, setChecking] = useState(true);
  const [updating, setUpdating] = useState(false);
  const [result, setResult] = useState<CliVersionCheck | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function run() {
      setChecking(true);
      try {
        const next = await cliCheckLatestVersion(cliKey);
        if (!cancelled) {
          setResult(next);
        }
      } catch (error) {
        if (!cancelled) {
          setResult({
            cliKey,
            npmPackage: "",
            installedVersion,
            latestVersion: null,
            updateAvailable: false,
            error: error instanceof Error ? error.message : String(error),
          });
        }
      } finally {
        if (!cancelled) {
          setChecking(false);
        }
      }
    }

    void run();
    return () => {
      cancelled = true;
    };
  }, [cliKey, installedVersion, refreshToken]);

  if (checking) {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 dark:bg-blue-900/30 px-2.5 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400 ring-1 ring-inset ring-blue-600/20">
        <RefreshCw className="h-3 w-3 animate-spin" />
        检查版本…
      </span>
    );
  }

  if (result?.error) {
    return <span className="text-xs text-slate-500 dark:text-slate-400">{result.error}</span>;
  }

  if (!result) {
    return null;
  }

  if (!result.updateAvailable) {
    return <span className="text-xs font-medium text-green-700 dark:text-green-400">已是最新</span>;
  }

  async function handleUpdate() {
    if (!result) return;
    const ok = window.confirm(
      `确认更新 ${cliKey} CLI 到最新版本 ${result.latestVersion ? `v${result.latestVersion}` : ""} 吗？`
    );
    if (!ok) return;

    setUpdating(true);
    try {
      const updateResult = await cliUpdateCli(cliKey);
      if (!updateResult || !updateResult.success) {
        toast.error(updateResult?.error ?? "更新失败");
        return;
      }
      toast.success("更新完成");
      setChecking(true);
      const next = await cliCheckLatestVersion(cliKey);
      setResult(next);
      onUpdateComplete?.();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "更新失败");
    } finally {
      setUpdating(false);
      setChecking(false);
    }
  }

  return (
    <div className="flex items-center gap-2">
      <span className="inline-flex items-center rounded-full bg-amber-50 dark:bg-amber-900/30 px-2.5 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-400 ring-1 ring-inset ring-amber-600/20">
        最新: v{result.latestVersion ?? "—"}
      </span>
      <Button
        type="button"
        size="sm"
        variant="secondary"
        onClick={() => void handleUpdate()}
        disabled={updating}
        className={cn("h-6 px-2 text-xs", updating && "opacity-80")}
      >
        {updating ? "更新中…" : "更新"}
      </Button>
    </div>
  );
}
