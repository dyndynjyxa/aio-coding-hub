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
      })
    ).toBe("已清理请求日志：request_logs 3 条");
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
        warnings: [],
      })
    ).toBe(
      "配置导入完成：供应商 1，排序模式 2，工作区 3，提示词 4，MCP 5，技能仓库 6，通用技能 7，本机技能 8"
    );
  });

  it("reports committed import with client sync warnings", () => {
    const message = buildConfigImportSuccessMessage({
      providers_imported: 1,
      sort_modes_imported: 0,
      workspaces_imported: 0,
      prompts_imported: 0,
      mcp_servers_imported: 0,
      skill_repos_imported: 0,
      installed_skills_imported: 0,
      local_skills_imported: 0,
      warnings: ["本机客户端配置同步失败，请在 CLI 管理中重试"],
    });
    expect(message).toContain("配置导入完成");
    expect(message).toContain("本机客户端配置同步失败，请在 CLI 管理中重试");
  });

  it("builds model prices sync messages", () => {
    expect(
      buildModelPricesSyncMessage({
        status: "not_modified",
        inserted: 0,
        updated: 0,
        unchanged: 0,
        total: 0,
        error: null,
      })
    ).toBe("定价已是最新，无变更");

    expect(
      buildModelPricesSyncMessage({
        status: "updated",
        inserted: 1,
        updated: 2,
        unchanged: 3,
        total: 6,
        error: null,
      })
    ).toBe("定价同步完成：新增 1 · 更新 2 · 共 6 条");

    expect(
      buildModelPricesSyncMessage({
        status: "failed",
        inserted: 0,
        updated: 0,
        unchanged: 0,
        total: 0,
        error: "BaseLLM returned HTTP 500",
      })
    ).toBe("定价同步失败：BaseLLM returned HTTP 500");
  });
});
