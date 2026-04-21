import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { ModelPriceAliasesDialog } from "../../components/settings/ModelPriceAliasesDialog";

type DialogController = {
  open: boolean;
  setOpen: (open: boolean) => void;
};

type PendingDialogController = DialogController & {
  pending: boolean;
  confirm: () => Promise<void>;
};

type ConfigImportDialogController = PendingDialogController & {
  pendingFilePath: string | null;
};

export function SettingsDialogs({
  modelPriceAliases,
  clearRequestLogs,
  resetAll,
  configImport,
}: {
  modelPriceAliases: DialogController;
  clearRequestLogs: PendingDialogController;
  resetAll: PendingDialogController;
  configImport: ConfigImportDialogController;
}) {
  return (
    <>
      <ModelPriceAliasesDialog
        open={modelPriceAliases.open}
        onOpenChange={modelPriceAliases.setOpen}
      />

      <Dialog
        open={clearRequestLogs.open}
        onOpenChange={(open) => {
          if (!open && clearRequestLogs.pending) return;
          clearRequestLogs.setOpen(open);
        }}
        title="确认清理请求日志"
        description="将清空 request_logs（兼容旧版本时也会清理 request_attempt_logs）。此操作不可撤销。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="text-sm text-slate-700 dark:text-slate-300">
            说明：仅影响请求日志与明细，不会影响 Providers、Prompts、MCP 等配置。
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button
              onClick={() => clearRequestLogs.setOpen(false)}
              variant="secondary"
              disabled={clearRequestLogs.pending}
            >
              取消
            </Button>
            <Button
              onClick={() => void clearRequestLogs.confirm()}
              variant="warning"
              disabled={clearRequestLogs.pending}
            >
              {clearRequestLogs.pending ? "清理中…" : "确认清理"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={resetAll.open}
        onOpenChange={(open) => {
          if (!open && resetAll.pending) return;
          resetAll.setOpen(open);
        }}
        title="确认清理全部信息"
        description="将删除本地数据库与 settings.json，并在完成后退出应用。下次启动会以默认配置重新初始化。此操作不可撤销。"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800">
            注意：此操作会清空所有本地数据与配置。完成后应用会自动退出，需要手动重新打开。
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 dark:border-slate-700 pt-3">
            <Button
              onClick={() => resetAll.setOpen(false)}
              variant="secondary"
              disabled={resetAll.pending}
            >
              取消
            </Button>
            <Button onClick={() => void resetAll.confirm()} variant="danger" disabled={resetAll.pending}>
              {resetAll.pending ? "清理中…" : "确认清理并退出"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={configImport.open}
        onOpenChange={(open) => {
          if (!open && configImport.pending) return;
          configImport.setOpen(open);
        }}
        title="确认导入配置"
        className="max-w-lg"
      >
        <div className="space-y-4">
          <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
            ⚠️ 导入文件中包含 API Key 等敏感信息，请确认文件来源可信。
          </div>
          <div className="rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800">
            ⚠️ 导入将覆盖当前所有配置（供应商、工作区、提示词、MCP 服务器等），此操作不可撤销。
          </div>
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700 dark:border-slate-700 dark:bg-slate-800/60 dark:text-slate-200">
            <div className="font-medium text-slate-900 dark:text-slate-100">导入文件</div>
            <div className="mt-2 break-all font-mono text-xs">
              {configImport.pendingFilePath ?? "未选择文件"}
            </div>
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 pt-3 dark:border-slate-700">
            <Button
              onClick={() => configImport.setOpen(false)}
              variant="secondary"
              disabled={configImport.pending}
            >
              取消
            </Button>
            <Button
              onClick={() => void configImport.confirm()}
              variant="danger"
              disabled={configImport.pending || !configImport.pendingFilePath}
            >
              {configImport.pending ? "导入中…" : "确认导入"}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
