import type { AppAboutInfo } from "../../services/appAbout";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";

type SettingsAboutCardProps = {
  about: AppAboutInfo | null;
  checkingUpdate: boolean;
  checkUpdate: () => Promise<void>;
};

export function SettingsAboutCard({ about, checkingUpdate, checkUpdate }: SettingsAboutCardProps) {
  return (
    <Card>
      <div className="mb-4 font-semibold text-slate-900 dark:text-slate-100">关于应用</div>
      {about ? (
        <div className="grid gap-2 text-sm text-slate-700 dark:text-slate-300">
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">版本</span>
            <span className="font-mono">{about.app_version}</span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">构建</span>
            <span className="font-mono">{about.profile}</span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">平台</span>
            <span className="font-mono">
              {about.os}/{about.arch}
            </span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">Bundle</span>
            <span className="font-mono">{about.bundle_type ?? "—"}</span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">运行模式</span>
            <span className="font-mono">{about.run_mode}</span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="text-slate-500 dark:text-slate-400">
              {about.run_mode === "portable" ? "获取新版本" : "检查更新"}
            </span>
            <Button
              onClick={() => void checkUpdate()}
              variant="secondary"
              size="sm"
              disabled={checkingUpdate}
            >
              {checkingUpdate ? "检查中…" : about.run_mode === "portable" ? "打开" : "检查"}
            </Button>
          </div>
        </div>
      ) : (
        <div className="text-sm text-slate-600 dark:text-slate-400">加载中…</div>
      )}
    </Card>
  );
}
