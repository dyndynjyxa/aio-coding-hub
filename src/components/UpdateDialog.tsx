import Markdown from "react-markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { AIO_RELEASES_URL } from "../constants/urls";
import { logToConsole } from "../services/consoleLog";
import { appRestart } from "../services/app/dataManagement";
import {
  updateDialogSetOpen,
  updateDownloadAndInstall,
  useUpdateMeta,
} from "../hooks/useUpdateMeta";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { formatBytes, formatIsoDateTime } from "../utils/formatters";

export function UpdateDialog() {
  const meta = useUpdateMeta();
  const updateCandidate = meta.updateCandidate;
  const about = meta.about;
  const isPortable = about?.run_mode === "portable";

  async function openReleases() {
    try {
      await openUrl(AIO_RELEASES_URL);
    } catch (err) {
      logToConsole("error", "打开 Releases 失败", { error: String(err), url: AIO_RELEASES_URL });
      try {
        window.open(AIO_RELEASES_URL, "_blank", "noopener,noreferrer");
      } catch {}
      toast("打开下载页失败：请查看控制台日志");
    }
  }

  async function installUpdate() {
    if (!updateCandidate) return;
    if (meta.installingUpdate) return;

    if (isPortable) {
      toast("portable 模式请手动下载");
      await openReleases();
      updateDialogSetOpen(false);
      return;
    }

    const ok = await updateDownloadAndInstall();
    if (ok === false) return;

    updateDialogSetOpen(false);

    const totalSeconds = 3;
    let remaining = totalSeconds;
    const toastId = toast.loading(`准备重启（${remaining}s）`);

    const timer = window.setInterval(() => {
      remaining -= 1;

      if (remaining > 0) {
        toast.loading(`准备重启（${remaining}s）`, { id: toastId });
        return;
      }

      window.clearInterval(timer);
      toast.loading("正在重启…", { id: toastId });

      appRestart()
        .then((restartOk) => {
          if (!restartOk) {
            toast("更新已安装：请手动重启应用以生效", { id: toastId });
          }
        })
        .catch((err) => {
          logToConsole("error", "自动重启失败", { error: String(err) });
          toast("自动重启失败：请手动重启应用以生效", { id: toastId });
        });
    }, 1000);
  }

  return (
    <Dialog
      open={meta.dialogOpen}
      onOpenChange={(open) => updateDialogSetOpen(open)}
      title="发现新版本"
      description="下载并安装需要确认；安装完成后将自动重启生效。"
      className="max-w-xl"
    >
      <div className="space-y-4">
        <div className="grid gap-2 text-sm text-slate-700 dark:text-slate-300">
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">当前版本</span>
            <span className="font-mono">
              {updateCandidate?.currentVersion ?? about?.app_version ?? "—"}
            </span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">最新版本</span>
            <span className="font-mono">{updateCandidate?.version ?? "—"}</span>
          </div>
          {updateCandidate?.date ? (
            <div className="flex items-center justify-between gap-4">
              <span className="text-slate-500 dark:text-slate-400">发布日期</span>
              <span className="font-mono">{formatIsoDateTime(updateCandidate.date)}</span>
            </div>
          ) : null}
        </div>

        {updateCandidate?.body ? (
          <div className="space-y-1">
            <span className="text-xs font-medium text-slate-500 dark:text-slate-400">更新日志</span>
            <div className="prose prose-sm dark:prose-invert max-h-60 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3 text-sm text-slate-700 dark:text-slate-300">
              <Markdown>{updateCandidate.body}</Markdown>
            </div>
          </div>
        ) : null}

        {!updateCandidate ? (
          <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3 text-sm text-slate-700 dark:text-slate-300">
            未发现可安装更新。
          </div>
        ) : null}

        {meta.installingUpdate ? (
          <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-3 text-sm text-slate-700 dark:text-slate-300">
            <div className="font-medium">下载并安装中…</div>
            <div className="mt-1 font-mono text-xs text-slate-500 dark:text-slate-400">
              {formatBytes(meta.installDownloadedBytes)}
              {meta.installTotalBytes != null ? ` / ${formatBytes(meta.installTotalBytes)}` : ""}
            </div>
          </div>
        ) : null}

        {meta.installError ? (
          <div className="rounded-lg border border-rose-200 dark:border-rose-700 bg-rose-50 dark:bg-rose-900/30 p-3 text-xs text-rose-700 dark:text-rose-400">
            安装失败：{meta.installError}
          </div>
        ) : null}

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            type="button"
            variant="secondary"
            onClick={() => updateDialogSetOpen(false)}
            disabled={meta.installingUpdate}
          >
            取消
          </Button>
          {isPortable ? (
            <Button
              type="button"
              variant="primary"
              onClick={openReleases}
              disabled={!updateCandidate}
            >
              打开下载页
            </Button>
          ) : (
            <Button
              type="button"
              variant="primary"
              onClick={installUpdate}
              disabled={!updateCandidate || meta.installingUpdate}
            >
              {meta.installingUpdate ? "安装中…" : "下载并安装"}
            </Button>
          )}
        </div>
      </div>
    </Dialog>
  );
}
