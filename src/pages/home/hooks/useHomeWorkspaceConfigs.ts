import { useMemo } from "react";
import { CLIS } from "../../../constants/clis";
import { useMcpServersListQuery } from "../../../query/mcp";
import { usePromptsListSummaryQuery } from "../../../query/prompts";
import { useSkillsInstalledListQuery } from "../../../query/skills";
import { pickWorkspaceById, useWorkspacesListQuery } from "../../../query/workspaces";
import type {
  HomeCliWorkspaceConfig,
  HomeWorkspaceConfigItem,
} from "../../../components/home/homeWorkspaceConfigTypes";
import type { CliKey } from "../../../services/providers/providers";

function buildWorkspaceConfigItems(input: {
  prompts: Array<{ id: number; name: string; enabled: boolean }>;
  mcp: Array<{ id: number; name: string; enabled: boolean }>;
  skills: Array<{ id: number; name: string; enabled: boolean }>;
  showAllItems: boolean;
}) {
  const items: HomeWorkspaceConfigItem[] = [];

  const prompts = input.prompts
    .filter((row) => input.showAllItems || row.enabled)
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const mcp = input.mcp
    .filter((row) => input.showAllItems || row.enabled)
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const skills = input.skills
    .filter((row) => input.showAllItems || row.enabled)
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));

  for (const row of prompts) {
    items.push({
      id: `prompt:${row.id}`,
      resourceId: row.id,
      type: "prompts",
      label: "Prompt",
      name: row.name,
      enabled: row.enabled,
    });
  }

  for (const row of skills) {
    items.push({
      id: `skill:${row.id}`,
      resourceId: row.id,
      type: "skills",
      label: "Skill",
      name: row.name,
      enabled: row.enabled,
    });
  }

  for (const row of mcp) {
    items.push({
      id: `mcp:${row.id}`,
      resourceId: row.id,
      type: "mcp",
      label: "MCP",
      name: row.name,
      enabled: row.enabled,
    });
  }

  return items;
}

function buildCliWorkspaceConfig(input: {
  cliKey: CliKey;
  enabled: boolean;
  workspacesQuery: ReturnType<typeof useWorkspacesListQuery>;
  promptsQuery: ReturnType<typeof usePromptsListSummaryQuery>;
  mcpQuery: ReturnType<typeof useMcpServersListQuery>;
  skillsQuery: ReturnType<typeof useSkillsInstalledListQuery>;
  showAllItems: boolean;
}): HomeCliWorkspaceConfig {
  const { cliKey, enabled, workspacesQuery, promptsQuery, mcpQuery, skillsQuery, showAllItems } =
    input;
  const cliLabel = CLIS.find((cli) => cli.key === cliKey)?.name ?? cliKey;
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;
  const activeWorkspace = pickWorkspaceById(workspacesQuery.data?.items ?? [], activeWorkspaceId);

  return {
    cliKey,
    cliLabel,
    workspaceId: activeWorkspaceId,
    workspaceName: activeWorkspace?.name ?? null,
    workspaces: (workspacesQuery.data?.items ?? []).map((workspace) => ({
      id: workspace.id,
      name: workspace.name,
      isActive: activeWorkspaceId === workspace.id,
    })),
    loading:
      enabled &&
      (workspacesQuery.isLoading ||
        promptsQuery.isLoading ||
        mcpQuery.isLoading ||
        skillsQuery.isLoading),
    items: buildWorkspaceConfigItems({
      prompts: promptsQuery.data ?? [],
      mcp: mcpQuery.data ?? [],
      skills: skillsQuery.data ?? [],
      showAllItems,
    }),
  };
}

export function useHomeWorkspaceConfigs(options?: { enabled?: boolean; showAllItems?: boolean }) {
  const enabled = options?.enabled ?? true;
  const showAllItems = options?.showAllItems ?? false;

  const claudeWorkspacesQuery = useWorkspacesListQuery("claude", { enabled });
  const codexWorkspacesQuery = useWorkspacesListQuery("codex", { enabled });
  const geminiWorkspacesQuery = useWorkspacesListQuery("gemini", { enabled });

  const claudeWorkspaceId = claudeWorkspacesQuery.data?.active_id ?? null;
  const codexWorkspaceId = codexWorkspacesQuery.data?.active_id ?? null;
  const geminiWorkspaceId = geminiWorkspacesQuery.data?.active_id ?? null;

  const claudePromptsQuery = usePromptsListSummaryQuery(claudeWorkspaceId, { enabled });
  const codexPromptsQuery = usePromptsListSummaryQuery(codexWorkspaceId, { enabled });
  const geminiPromptsQuery = usePromptsListSummaryQuery(geminiWorkspaceId, { enabled });

  const claudeMcpQuery = useMcpServersListQuery(claudeWorkspaceId, { enabled });
  const codexMcpQuery = useMcpServersListQuery(codexWorkspaceId, { enabled });
  const geminiMcpQuery = useMcpServersListQuery(geminiWorkspaceId, { enabled });

  const claudeSkillsQuery = useSkillsInstalledListQuery(claudeWorkspaceId, { enabled });
  const codexSkillsQuery = useSkillsInstalledListQuery(codexWorkspaceId, { enabled });
  const geminiSkillsQuery = useSkillsInstalledListQuery(geminiWorkspaceId, { enabled });

  return useMemo(
    () => [
      buildCliWorkspaceConfig({
        cliKey: "claude",
        enabled,
        workspacesQuery: claudeWorkspacesQuery,
        promptsQuery: claudePromptsQuery,
        mcpQuery: claudeMcpQuery,
        skillsQuery: claudeSkillsQuery,
        showAllItems,
      }),
      buildCliWorkspaceConfig({
        cliKey: "codex",
        enabled,
        workspacesQuery: codexWorkspacesQuery,
        promptsQuery: codexPromptsQuery,
        mcpQuery: codexMcpQuery,
        skillsQuery: codexSkillsQuery,
        showAllItems,
      }),
      buildCliWorkspaceConfig({
        cliKey: "gemini",
        enabled,
        workspacesQuery: geminiWorkspacesQuery,
        promptsQuery: geminiPromptsQuery,
        mcpQuery: geminiMcpQuery,
        skillsQuery: geminiSkillsQuery,
        showAllItems,
      }),
    ],
    [
      claudeMcpQuery,
      claudePromptsQuery,
      claudeSkillsQuery,
      claudeWorkspacesQuery,
      codexMcpQuery,
      codexPromptsQuery,
      codexSkillsQuery,
      codexWorkspacesQuery,
      enabled,
      geminiMcpQuery,
      geminiPromptsQuery,
      geminiSkillsQuery,
      geminiWorkspacesQuery,
      showAllItems,
    ]
  );
}
