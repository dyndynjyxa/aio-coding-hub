import { useMemo } from "react";
import { CLIS } from "../../../constants/clis";
import { useMcpServersListQuery } from "../../../query/mcp";
import { usePromptsListQuery } from "../../../query/prompts";
import { useSkillsInstalledListQuery } from "../../../query/skills";
import { pickWorkspaceById, useWorkspacesListQuery } from "../../../query/workspaces";
import type {
  HomeCliWorkspaceConfig,
  HomeWorkspaceConfigItem,
} from "../../../components/home/homeWorkspaceConfigTypes";
import type { CliKey } from "../../../services/providers/providers";

function buildWorkspaceConfigItems(input: {
  prompts: Array<{ id: number; name: string }>;
  mcp: Array<{ id: number; name: string; enabled: boolean }>;
  skills: Array<{ id: number; name: string; enabled: boolean }>;
}) {
  const items: HomeWorkspaceConfigItem[] = [];

  const prompts = [...input.prompts].sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const enabledMcp = input.mcp
    .filter((row) => row.enabled)
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const enabledSkills = input.skills
    .filter((row) => row.enabled)
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));

  for (const row of prompts) {
    items.push({
      id: `prompt:${row.id}`,
      type: "prompts",
      label: "Prompt",
      name: row.name,
    });
  }

  for (const row of enabledMcp) {
    items.push({
      id: `mcp:${row.id}`,
      type: "mcp",
      label: "MCP",
      name: row.name,
    });
  }

  for (const row of enabledSkills) {
    items.push({
      id: `skill:${row.id}`,
      type: "skills",
      label: "Skill",
      name: row.name,
    });
  }

  return items;
}

function buildCliWorkspaceConfig(input: {
  cliKey: CliKey;
  enabled: boolean;
  workspacesQuery: ReturnType<typeof useWorkspacesListQuery>;
  promptsQuery: ReturnType<typeof usePromptsListQuery>;
  mcpQuery: ReturnType<typeof useMcpServersListQuery>;
  skillsQuery: ReturnType<typeof useSkillsInstalledListQuery>;
}): HomeCliWorkspaceConfig {
  const { cliKey, enabled, workspacesQuery, promptsQuery, mcpQuery, skillsQuery } = input;
  const cliLabel = CLIS.find((cli) => cli.key === cliKey)?.name ?? cliKey;
  const activeWorkspaceId = workspacesQuery.data?.active_id ?? null;
  const activeWorkspace = pickWorkspaceById(workspacesQuery.data?.items ?? [], activeWorkspaceId);

  return {
    cliKey,
    cliLabel,
    workspaceId: activeWorkspaceId,
    workspaceName: activeWorkspace?.name ?? null,
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
    }),
  };
}

export function useHomeWorkspaceConfigs(options?: { enabled?: boolean }) {
  const enabled = options?.enabled ?? true;

  const claudeWorkspacesQuery = useWorkspacesListQuery("claude", { enabled });
  const codexWorkspacesQuery = useWorkspacesListQuery("codex", { enabled });
  const geminiWorkspacesQuery = useWorkspacesListQuery("gemini", { enabled });

  const claudeWorkspaceId = claudeWorkspacesQuery.data?.active_id ?? null;
  const codexWorkspaceId = codexWorkspacesQuery.data?.active_id ?? null;
  const geminiWorkspaceId = geminiWorkspacesQuery.data?.active_id ?? null;

  const claudePromptsQuery = usePromptsListQuery(claudeWorkspaceId, { enabled });
  const codexPromptsQuery = usePromptsListQuery(codexWorkspaceId, { enabled });
  const geminiPromptsQuery = usePromptsListQuery(geminiWorkspaceId, { enabled });

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
      }),
      buildCliWorkspaceConfig({
        cliKey: "codex",
        enabled,
        workspacesQuery: codexWorkspacesQuery,
        promptsQuery: codexPromptsQuery,
        mcpQuery: codexMcpQuery,
        skillsQuery: codexSkillsQuery,
      }),
      buildCliWorkspaceConfig({
        cliKey: "gemini",
        enabled,
        workspacesQuery: geminiWorkspacesQuery,
        promptsQuery: geminiPromptsQuery,
        mcpQuery: geminiMcpQuery,
        skillsQuery: geminiSkillsQuery,
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
    ]
  );
}
