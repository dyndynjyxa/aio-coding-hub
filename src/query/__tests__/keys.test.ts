import { describe, expect, it } from "vitest";
import {
  appAboutKeys,
  cliManagerKeys,
  cliProxyKeys,
  costKeys,
  dataManagementKeys,
  gatewayKeys,
  mcpKeys,
  modelPricesKeys,
  promptsKeys,
  providersKeys,
  requestLogsKeys,
  sortModesKeys,
  usageKeys,
  wslKeys,
  workspacesKeys,
  skillsKeys,
  settingsKeys,
  updaterKeys,
} from "../keys";

describe("query/keys", () => {
  it("builds providers keys", () => {
    expect(providersKeys.all).toEqual(["providers"]);
    expect(providersKeys.lists()).toEqual(["providers", "list"]);
    expect(providersKeys.list("claude")).toEqual(["providers", "list", "claude"]);
  });

  it("builds gateway keys", () => {
    expect(gatewayKeys.all).toEqual(["gateway"]);
    expect(gatewayKeys.status()).toEqual(["gateway", "status"]);
    expect(gatewayKeys.sessions()).toEqual(["gateway", "sessions"]);
    expect(gatewayKeys.sessionsList(10)).toEqual(["gateway", "sessions", 10]);
    expect(gatewayKeys.circuits()).toEqual(["gateway", "circuitStatus"]);
    expect(gatewayKeys.circuitStatus("claude")).toEqual(["gateway", "circuitStatus", "claude"]);
  });

  it("builds request log keys", () => {
    expect(requestLogsKeys.all).toEqual(["requestLogs"]);
    expect(requestLogsKeys.lists()).toEqual(["requestLogs", "list"]);
    expect(requestLogsKeys.listAll(10)).toEqual(["requestLogs", "list", "all", 10]);
    expect(requestLogsKeys.activeSnapshot()).toEqual(["requestLogs", "activeSnapshot"]);
    expect(requestLogsKeys.detail(1)).toEqual(["requestLogs", "detail", 1]);
    expect(requestLogsKeys.attemptsByTrace("trace-1", 10)).toEqual([
      "requestLogs",
      "attempts",
      "trace-1",
      10,
    ]);
  });

  it("builds sort mode keys", () => {
    expect(sortModesKeys.all).toEqual(["sortModes"]);
    expect(sortModesKeys.list()).toEqual(["sortModes", "list"]);
    expect(sortModesKeys.activeList()).toEqual(["sortModes", "activeList"]);
  });

  it("builds usage keys", () => {
    expect(usageKeys.all).toEqual(["usage"]);
    expect(usageKeys.hourlySeries(7)).toEqual(["usage", "hourlySeries", 7]);
    expect(
      usageKeys.summaryV2("daily", { startTs: 1, endTs: 2, cliKey: "claude", providerId: 3 })
    ).toEqual(["usage", "summaryV2", "daily", 1, 2, "claude", 3, [], null, null]);
    expect(
      usageKeys.summaryV2("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        folderKeys: [" /tmp/b ", "", "/tmp/a", "/tmp/a"],
      })
    ).toEqual(["usage", "summaryV2", "daily", 1, 2, "claude", 3, ["/tmp/a", "/tmp/b"], null, null]);
    expect(
      usageKeys.leaderboardV2("provider", "weekly", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        limit: null,
      })
    ).toEqual([
      "usage",
      "leaderboardV2",
      "provider",
      "weekly",
      1,
      2,
      "claude",
      3,
      null,
      [],
      null,
      null,
    ]);
    expect(
      usageKeys.providerCacheRateTrendV1("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        limit: 20,
        excludeCx2CcGatewayBridge: true,
      })
    ).toEqual(["usage", "providerCacheRateTrendV1", "daily", 1, 2, "claude", 3, 20, true]);
  });

  it("builds cost keys", () => {
    expect(costKeys.all).toEqual(["cost"]);
    expect(
      costKeys.analyticsV1("daily", {
        startTs: 1,
        endTs: 2,
        cliKey: "claude",
        providerId: 3,
        model: "gpt-4.1",
      })
    ).toEqual(["cost", "analyticsV1", "daily", 1, 2, "claude", 3, "gpt-4.1"]);
  });

  it("builds workspaces keys", () => {
    expect(workspacesKeys.all).toEqual(["workspaces"]);
    expect(workspacesKeys.lists()).toEqual(["workspaces", "list"]);
    expect(workspacesKeys.list("claude")).toEqual(["workspaces", "list", "claude"]);
    expect(workspacesKeys.preview(1)).toEqual(["workspaces", "preview", 1]);
  });

  it("builds prompts keys", () => {
    expect(promptsKeys.all).toEqual(["prompts"]);
    expect(promptsKeys.lists()).toEqual(["prompts", "list"]);
    expect(promptsKeys.list(1)).toEqual(["prompts", "list", 1]);
  });

  it("builds mcp keys", () => {
    expect(mcpKeys.all).toEqual(["mcp"]);
    expect(mcpKeys.serversList(1)).toEqual(["mcp", "servers", 1]);
  });

  it("builds skills keys", () => {
    expect(skillsKeys.all).toEqual(["skills"]);
    expect(skillsKeys.reposList()).toEqual(["skills", "repos"]);
    expect(skillsKeys.discoverAvailable(false)).toEqual(["skills", "discoverAvailable", false]);
    expect(skillsKeys.installedList(1)).toEqual(["skills", "installed", 1]);
    expect(skillsKeys.localList(1)).toEqual(["skills", "local", 1]);
    expect(skillsKeys.paths("claude")).toEqual(["skills", "paths", "claude"]);
  });

  it("builds settings keys", () => {
    expect(settingsKeys.all).toEqual(["settings"]);
    expect(settingsKeys.get()).toEqual(["settings", "get"]);
  });

  it("builds cliManager keys", () => {
    expect(cliManagerKeys.all).toEqual(["cliManager"]);
    expect(cliManagerKeys.claudeInfo()).toEqual(["cliManager", "claude", "info"]);
    expect(cliManagerKeys.claudeSettings()).toEqual(["cliManager", "claude", "settings"]);
    expect(cliManagerKeys.codexInfo()).toEqual(["cliManager", "codex", "info"]);
    expect(cliManagerKeys.codexConfig()).toEqual(["cliManager", "codex", "config"]);
    expect(cliManagerKeys.geminiInfo()).toEqual(["cliManager", "gemini", "info"]);
  });

  it("builds modelPrices keys", () => {
    expect(modelPricesKeys.all).toEqual(["modelPrices"]);
    expect(modelPricesKeys.lists()).toEqual(["modelPrices", "list"]);
    expect(modelPricesKeys.list("claude")).toEqual(["modelPrices", "list", "claude"]);
    expect(modelPricesKeys.aliases()).toEqual(["modelPrices", "aliases"]);
  });

  it("builds dataManagement keys", () => {
    expect(dataManagementKeys.all).toEqual(["dataManagement"]);
    expect(dataManagementKeys.dbDiskUsage()).toEqual(["dataManagement", "dbDiskUsage"]);
  });

  it("builds cliProxy keys", () => {
    expect(cliProxyKeys.all).toEqual(["cliProxy"]);
    expect(cliProxyKeys.statusAll()).toEqual(["cliProxy", "statusAll"]);
  });

  it("builds appAbout keys", () => {
    expect(appAboutKeys.all).toEqual(["appAbout"]);
    expect(appAboutKeys.get()).toEqual(["appAbout", "get"]);
  });

  it("builds updater keys", () => {
    expect(updaterKeys.all).toEqual(["updater"]);
    expect(updaterKeys.check()).toEqual(["updater", "check"]);
  });

  it("builds wsl keys", () => {
    expect(wslKeys.all).toEqual(["wsl"]);
    expect(wslKeys.detection()).toEqual(["wsl", "detection"]);
    expect(wslKeys.hostAddress()).toEqual(["wsl", "hostAddress"]);
    expect(wslKeys.configStatus([])).toEqual(["wsl", "configStatus"]);
    expect(wslKeys.configStatus(["Ubuntu"])).toEqual(["wsl", "configStatus", "Ubuntu"]);
    expect(wslKeys.configStatus(["Ubuntu", "Debian"])).toEqual([
      "wsl",
      "configStatus",
      "Debian",
      "Ubuntu",
    ]);
    expect(wslKeys.configStatus([" Ubuntu ", "Ubuntu", ""])).toEqual([
      "wsl",
      "configStatus",
      "Ubuntu",
    ]);
    expect(wslKeys.overview()).toEqual(["wsl", "overview"]);
  });
});
