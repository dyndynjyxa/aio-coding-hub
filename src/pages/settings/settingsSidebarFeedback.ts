import { toast } from "sonner";
import type { ConfigImportResult } from "../../services/app/configMigrate";
import type { ClearRequestLogsResult } from "../../services/app/dataManagement";
import { logToConsole } from "../../services/consoleLog";
import type { ModelPricesSyncReport } from "../../services/usage/modelPrices";
import {
  buildConfigImportSuccessMessage,
  buildModelPricesSyncMessage,
  buildRequestLogsClearedMessage,
} from "./settingsSidebarModel";

type SidebarFailureInput = {
  logTitle: string;
  toastMessage: string;
  error: unknown;
  meta?: Record<string, unknown>;
};

export function presentSettingsSidebarFailure(input: SidebarFailureInput) {
  const { logTitle, toastMessage, error, meta } = input;
  logToConsole("error", logTitle, {
    error: String(error),
    ...(meta ?? {}),
  });
  toast(toastMessage);
}

export function presentRequestLogsCleared(result: ClearRequestLogsResult) {
  logToConsole("info", "清理请求日志", result);
  toast(buildRequestLogsClearedMessage(result));
}

export function presentResetAllSuccess() {
  logToConsole("info", "清理全部信息", { ok: true });
  toast("已清理全部信息：应用即将退出，请重新打开");
}

export function presentConfigExported() {
  toast("配置已导出");
}

export function presentConfigImported(result: ConfigImportResult) {
  toast(buildConfigImportSuccessMessage(result));
}

export function presentModelPricesSynced(report: ModelPricesSyncReport) {
  toast(buildModelPricesSyncMessage(report));
}
