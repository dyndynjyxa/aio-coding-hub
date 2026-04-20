import type { ConfigImportResult } from "../../services/app/configMigrate";
import type { ClearRequestLogsResult } from "../../services/app/dataManagement";
import type { ModelPricesSyncReport } from "../../services/usage/modelPrices";

export type AvailableStatus = "checking" | "available" | "unavailable";

export function resolveAvailableStatus<TValue>(
  value: TValue | null | undefined,
  isLoading: boolean
): AvailableStatus {
  if (isLoading) {
    return "checking";
  }

  return value != null ? "available" : "unavailable";
}

export function buildRequestLogsClearedMessage(result: ClearRequestLogsResult) {
  return `已清理请求日志：request_logs ${result.request_logs_deleted} 条；legacy request_attempt_logs ${result.request_attempt_logs_deleted} 条`;
}

export function buildConfigImportSuccessMessage(result: ConfigImportResult) {
  return `配置导入完成：供应商 ${result.providers_imported}，排序模式 ${result.sort_modes_imported}，工作区 ${result.workspaces_imported}，提示词 ${result.prompts_imported}，MCP ${result.mcp_servers_imported}，技能仓库 ${result.skill_repos_imported}，通用技能 ${result.installed_skills_imported}，本机技能 ${result.local_skills_imported}`;
}

export function buildModelPricesSyncMessage(report: ModelPricesSyncReport) {
  if (report.status === "not_modified") {
    return "模型定价已是最新（无变更）";
  }

  return `同步完成：新增 ${report.inserted}，更新 ${report.updated}，跳过 ${report.skipped}`;
}
