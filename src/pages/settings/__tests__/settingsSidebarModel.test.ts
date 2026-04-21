import { describe, expect, it } from "vitest";
import {
  buildConfigImportSuccessMessage,
  buildModelPricesSyncMessage,
  buildRequestLogsClearedMessage,
  resolveAvailableStatus,
} from "../settingsSidebarModel";

describe("pages/settings/settingsSidebarModel", () => {
  it("resolves loading and availability states", () => {
    expect(resolveAvailableStatus(null, true)).toBe("checking");
    expect(resolveAvailableStatus(0, false)).toBe("available");
    expect(resolveAvailableStatus(null, false)).toBe("unavailable");
  });

  it("builds request logs cleared message", () => {
    expect(
      buildRequestLogsClearedMessage({
        request_logs_deleted: 3,
        request_attempt_logs_deleted: 4,
      })
    ).toBe("已清理请求日志：request_logs 3 条；legacy request_attempt_logs 4 条");
  });

  it("builds config import success message", () => {
    expect(
      buildConfigImportSuccessMessage({
        providers_imported: 1,
        sort_modes_imported: 2,
        workspaces_imported: 3,
        prompts_imported: 4,
        mcp_servers_imported: 5,
        skill_repos_imported: 6,
        installed_skills_imported: 7,
        local_skills_imported: 8,
      })
    ).toBe(
      "配置导入完成：供应商 1，排序模式 2，工作区 3，提示词 4，MCP 5，技能仓库 6，通用技能 7，本机技能 8"
    );
  });

  it("builds model prices sync messages", () => {
    expect(
      buildModelPricesSyncMessage({
        status: "not_modified",
        inserted: 0,
        updated: 0,
        skipped: 0,
        total: 0,
      })
    ).toBe("模型定价已是最新（无变更）");

    expect(
      buildModelPricesSyncMessage({
        status: "updated",
        inserted: 1,
        updated: 2,
        skipped: 3,
        total: 6,
      })
    ).toBe("同步完成：新增 1，更新 2，跳过 3");
  });
});
