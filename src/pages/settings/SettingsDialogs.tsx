import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { ModelPriceAliasesDialog } from "../../components/settings/ModelPriceAliasesDialog";
import type { ConfigBundle, ConfigBundleWorkspace } from "../../services/app/configMigrate";

function countWorkspacePrompts(workspaces: ConfigBundleWorkspace[]) {
  return workspaces.reduce((total, workspace) => {
    if (Array.isArray(workspace.prompts) && workspace.prompts.length > 0) {
      return total + workspace.prompts.length;
    }
    return total + (workspace.prompt ? 1 : 0);
  }, 0);
}

export function SettingsDialogs({
  modelPriceAliasesDialogOpen,
  setModelPriceAliasesDialogOpen,

  clearRequestLogsDialogOpen,
  setClearRequestLogsDialogOpen,
  clearingRequestLogs,
  setClearingRequestLogs,
  clearRequestLogs,

  resetAllDialogOpen,
  setResetAllDialogOpen,
  resettingAll,
  setResettingAll,
  resetAllData,

  configImportDialogOpen,
  setConfigImportDialogOpen,
  importingConfig,
  setImportingConfig,
  pendingConfigBundle,
  confirmConfigImport,
}: {
  modelPriceAliasesDialogOpen: boolean;
  setModelPriceAliasesDialogOpen: (open: boolean) => void;

  clearRequestLogsDialogOpen: boolean;
  setClearRequestLogsDialogOpen: (open: boolean) => void;
  clearingRequestLogs: boolean;
  setClearingRequestLogs: (next: boolean) => void;
  clearRequestLogs: () => Promise<void>;

  resetAllDialogOpen: boolean;
  setResetAllDialogOpen: (open: boolean) => void;
  resettingAll: boolean;
  setResettingAll: (next: boolean) => void;
  resetAllData: () => Promise<void>;

  configImportDialogOpen: boolean;
  setConfigImportDialogOpen: (open: boolean) => void;
  importingConfig: boolean;
  setImportingConfig: (next: boolean) => void;
  pendingConfigBundle: ConfigBundle | null;
  confirmConfigImport: () => Promise<void>;
}) {
  const providersCount = pendingConfigBundle?.providers.length ?? 0;
  const sortModesCount = pendingConfigBundle?.sort_modes.length ?? 0;
  const workspacesCount = pendingConfigBundle?.workspaces.length ?? 0;
  const promptsCount = pendingConfigBundle
    ? countWorkspacePrompts(pendingConfigBundle.workspaces)
    : 0;
  const mcpServersCount = pendingConfigBundle?.mcp_servers.length ?? 0;
  const skillReposCount = pendingConfigBundle?.skill_repos.length ?? 0;
  const installedSkillsCount = pendingConfigBundle?.installed_skills?.length ?? 0;
  const localSkillsCount = pendingConfigBundle?.local_skills?.length ?? 0;

  return (
    <>
      <ModelPriceAliasesDialog
        open={modelPriceAliasesDialogOpen}
        onOpenChange={setModelPriceAliasesDialogOpen}
      />

      <Dialog
        open={clearRequestLogsDialogOpen}
        onOpenChange={(open) => {
          if (!open && clearingRequestLogs) return;
          setClearRequestLogsDialogOpen(open);
          if (!open) setClearingRequestLogs(false);
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
              onClick={() => setClearRequestLogsDialogOpen(false)}
              variant="secondary"
              disabled={clearingRequestLogs}
            >
              取消
            </Button>
            <Button
              onClick={() => void clearRequestLogs()}
              variant="warning"
              disabled={clearingRequestLogs}
            >
              {clearingRequestLogs ? "清理中…" : "确认清理"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={resetAllDialogOpen}
        onOpenChange={(open) => {
          if (!open && resettingAll) return;
          setResetAllDialogOpen(open);
          if (!open) setResettingAll(false);
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
              onClick={() => setResetAllDialogOpen(false)}
              variant="secondary"
              disabled={resettingAll}
            >
              取消
            </Button>
            <Button onClick={() => void resetAllData()} variant="danger" disabled={resettingAll}>
              {resettingAll ? "清理中…" : "确认清理并退出"}
            </Button>
          </div>
        </div>
      </Dialog>

      <Dialog
        open={configImportDialogOpen}
        onOpenChange={(open) => {
          if (!open && importingConfig) return;
          setConfigImportDialogOpen(open);
          if (!open) setImportingConfig(false);
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
            <div className="font-medium text-slate-900 dark:text-slate-100">导入内容摘要</div>
            <div className="mt-2 space-y-1">
              <div>{`Providers：${providersCount}`}</div>
              <div>{`Sort Modes：${sortModesCount}`}</div>
              <div>{`Workspaces：${workspacesCount}`}</div>
              <div>{`Prompts：${promptsCount}`}</div>
              <div>{`MCP Servers：${mcpServersCount}`}</div>
              <div>{`Skill Repos：${skillReposCount}`}</div>
              <div>{`Installed Skills：${installedSkillsCount}`}</div>
              <div>{`Local Skills：${localSkillsCount}`}</div>
            </div>
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2 border-t border-slate-100 pt-3 dark:border-slate-700">
            <Button
              onClick={() => setConfigImportDialogOpen(false)}
              variant="secondary"
              disabled={importingConfig}
            >
              取消
            </Button>
            <Button
              onClick={() => void confirmConfigImport()}
              variant="danger"
              disabled={importingConfig || !pendingConfigBundle}
            >
              {importingConfig ? "导入中…" : "确认导入"}
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
