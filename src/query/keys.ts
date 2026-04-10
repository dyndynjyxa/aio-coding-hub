import type { CliKey } from "../services/providers/providers";
import type { CostPeriod } from "../services/usage/cost";
import type { UsagePeriod, UsageRange, UsageScope } from "../services/usage/usage";
import type { CliSessionsSource } from "../services/cli/cliSessions";

function normalizeKeyParts(values: readonly string[]): string[] {
  const unique: string[] = [];
  const seen = new Set<string>();
  for (const raw of values) {
    const value = raw.trim();
    if (!value) continue;
    if (seen.has(value)) continue;
    seen.add(value);
    unique.push(value);
  }
  unique.sort((a, b) => a.localeCompare(b));
  return unique;
}

const providersAllKey = ["providers"] as const;
export const providersKeys = {
  all: providersAllKey,
  lists: () => [...providersAllKey, "list"] as const,
  list: (cliKey: CliKey) => [...providersAllKey, "list", cliKey] as const,
};

const gatewayAllKey = ["gateway"] as const;
export const gatewayKeys = {
  all: gatewayAllKey,
  status: () => [...gatewayAllKey, "status"] as const,
  sessions: () => [...gatewayAllKey, "sessions"] as const,
  sessionsList: (limit: number | null) => [...gatewayAllKey, "sessions", limit] as const,
  circuits: () => [...gatewayAllKey, "circuitStatus"] as const,
  circuitStatus: (cliKey: CliKey) => [...gatewayAllKey, "circuitStatus", cliKey] as const,
};

const requestLogsAllKey = ["requestLogs"] as const;
export const requestLogsKeys = {
  all: requestLogsAllKey,
  lists: () => [...requestLogsAllKey, "list"] as const,
  listAll: (limit: number | null) => [...requestLogsAllKey, "list", "all", limit] as const,
  detail: (logId: number | null) => [...requestLogsAllKey, "detail", logId] as const,
  attemptsByTrace: (traceId: string | null, limit: number | null) =>
    [...requestLogsAllKey, "attempts", traceId, limit] as const,
  pollAfterIdAll: (limit: number | null) =>
    [...requestLogsAllKey, "pollAfterIdAll", limit] as const,
};

const sortModesAllKey = ["sortModes"] as const;
export const sortModesKeys = {
  all: sortModesAllKey,
  list: () => [...sortModesAllKey, "list"] as const,
  activeList: () => [...sortModesAllKey, "activeList"] as const,
};

const usageAllKey = ["usage"] as const;
export const usageKeys = {
  all: usageAllKey,
  summary: (range: UsageRange, input: { cliKey: CliKey | null }) =>
    [...usageAllKey, "summary", range, input.cliKey] as const,
  hourlySeries: (days: number) => [...usageAllKey, "hourlySeries", days] as const,
  summaryV2: (
    period: UsagePeriod,
    input: {
      startTs: number | null;
      endTs: number | null;
      cliKey: CliKey | null;
      providerId: number | null;
    }
  ) =>
    [
      ...usageAllKey,
      "summaryV2",
      period,
      input.startTs,
      input.endTs,
      input.cliKey,
      input.providerId,
    ] as const,
  leaderboardV2: (
    scope: UsageScope,
    period: UsagePeriod,
    input: {
      startTs: number | null;
      endTs: number | null;
      cliKey: CliKey | null;
      providerId: number | null;
      limit: number | null;
    }
  ) =>
    [
      ...usageAllKey,
      "leaderboardV2",
      scope,
      period,
      input.startTs,
      input.endTs,
      input.cliKey,
      input.providerId,
      input.limit,
    ] as const,
  providerCacheRateTrendV1: (
    period: UsagePeriod,
    input: {
      startTs: number | null;
      endTs: number | null;
      cliKey: CliKey | null;
      providerId: number | null;
      limit: number | null;
    }
  ) =>
    [
      ...usageAllKey,
      "providerCacheRateTrendV1",
      period,
      input.startTs,
      input.endTs,
      input.cliKey,
      input.providerId,
      input.limit,
    ] as const,
};

const costAllKey = ["cost"] as const;
export const costKeys = {
  all: costAllKey,
  analyticsV1: (
    period: CostPeriod,
    input: {
      startTs: number | null;
      endTs: number | null;
      cliKey: CliKey | null;
      providerId: number | null;
      model: string | null;
    }
  ) =>
    [
      ...costAllKey,
      "analyticsV1",
      period,
      input.startTs,
      input.endTs,
      input.cliKey,
      input.providerId,
      input.model,
    ] as const,
};

const workspacesAllKey = ["workspaces"] as const;
export const workspacesKeys = {
  all: workspacesAllKey,
  lists: () => [...workspacesAllKey, "list"] as const,
  list: (cliKey: CliKey) => [...workspacesAllKey, "list", cliKey] as const,
  preview: (workspaceId: number | null) => [...workspacesAllKey, "preview", workspaceId] as const,
};

const promptsAllKey = ["prompts"] as const;
export const promptsKeys = {
  all: promptsAllKey,
  lists: () => [...promptsAllKey, "list"] as const,
  list: (workspaceId: number | null) => [...promptsAllKey, "list", workspaceId] as const,
};

const mcpAllKey = ["mcp"] as const;
export const mcpKeys = {
  all: mcpAllKey,
  serversList: (workspaceId: number | null) => [...mcpAllKey, "servers", workspaceId] as const,
};

const skillsAllKey = ["skills"] as const;
export const skillsKeys = {
  all: skillsAllKey,
  reposList: () => [...skillsAllKey, "repos"] as const,
  discoverAvailable: (refresh: boolean) => [...skillsAllKey, "discoverAvailable", refresh] as const,
  installedList: (workspaceId: number | null) =>
    [...skillsAllKey, "installed", workspaceId] as const,
  localList: (workspaceId: number | null) => [...skillsAllKey, "local", workspaceId] as const,
  paths: (cliKey: CliKey | null) => [...skillsAllKey, "paths", cliKey] as const,
};

const settingsAllKey = ["settings"] as const;
export const settingsKeys = {
  all: settingsAllKey,
  get: () => [...settingsAllKey, "get"] as const,
};

const cliManagerAllKey = ["cliManager"] as const;
export const cliManagerKeys = {
  all: cliManagerAllKey,
  claudeInfo: () => [...cliManagerAllKey, "claude", "info"] as const,
  claudeSettings: () => [...cliManagerAllKey, "claude", "settings"] as const,
  codexInfo: () => [...cliManagerAllKey, "codex", "info"] as const,
  codexConfig: () => [...cliManagerAllKey, "codex", "config"] as const,
  codexConfigToml: () => [...cliManagerAllKey, "codex", "configToml"] as const,
  geminiInfo: () => [...cliManagerAllKey, "gemini", "info"] as const,
  geminiConfig: () => [...cliManagerAllKey, "gemini", "config"] as const,
};

const modelPricesAllKey = ["modelPrices"] as const;
export const modelPricesKeys = {
  all: modelPricesAllKey,
  lists: () => [...modelPricesAllKey, "list"] as const,
  list: (cliKey: CliKey) => [...modelPricesAllKey, "list", cliKey] as const,
  aliases: () => [...modelPricesAllKey, "aliases"] as const,
};

const dataManagementAllKey = ["dataManagement"] as const;
export const dataManagementKeys = {
  all: dataManagementAllKey,
  dbDiskUsage: () => [...dataManagementAllKey, "dbDiskUsage"] as const,
};

const cliProxyAllKey = ["cliProxy"] as const;
export const cliProxyKeys = {
  all: cliProxyAllKey,
  statusAll: () => [...cliProxyAllKey, "statusAll"] as const,
};

const appAboutAllKey = ["appAbout"] as const;
export const appAboutKeys = {
  all: appAboutAllKey,
  get: () => [...appAboutAllKey, "get"] as const,
};

const updaterAllKey = ["updater"] as const;
export const updaterKeys = {
  all: updaterAllKey,
  check: () => [...updaterAllKey, "check"] as const,
};

const wslAllKey = ["wsl"] as const;
export const wslKeys = {
  all: wslAllKey,
  detection: () => [...wslAllKey, "detection"] as const,
  hostAddress: () => [...wslAllKey, "hostAddress"] as const,
  configStatus: (distros: string[]) =>
    [...wslAllKey, "configStatus", ...normalizeKeyParts(distros)] as const,
  overview: () => [...wslAllKey, "overview"] as const,
};

const providerLimitUsageAllKey = ["providerLimitUsage"] as const;
export const providerLimitUsageKeys = {
  all: providerLimitUsageAllKey,
  list: (cliKey: CliKey | null) => [...providerLimitUsageAllKey, "list", cliKey] as const,
};

const cliSessionsAllKey = ["cliSessions"] as const;
export const cliSessionsKeys = {
  all: cliSessionsAllKey,
  projectsList: (source: CliSessionsSource, wslDistro?: string) =>
    [...cliSessionsAllKey, "projects", source, wslDistro ?? null] as const,
  sessionsList: (source: CliSessionsSource, projectId: string, wslDistro?: string) =>
    [...cliSessionsAllKey, "sessions", source, projectId, wslDistro ?? null] as const,
  folderLookup: (keys: string[], wslDistro?: string) =>
    [...cliSessionsAllKey, "folderLookup", ...normalizeKeyParts(keys), wslDistro ?? null] as const,
  messages: (source: CliSessionsSource, filePath: string, fromEnd = true, wslDistro?: string) =>
    [...cliSessionsAllKey, "messages", source, filePath, fromEnd, wslDistro ?? null] as const,
};
