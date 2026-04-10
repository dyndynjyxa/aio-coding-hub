import type { CliKey } from "../../services/providers/providers";

export type HomeWorkspaceConfigItemType = "prompts" | "mcp" | "skills";

export type HomeWorkspaceConfigItem = {
  id: string;
  type: HomeWorkspaceConfigItemType;
  label: string;
  name: string;
};

export type HomeCliWorkspaceConfig = {
  cliKey: CliKey;
  cliLabel: string;
  workspaceId: number | null;
  workspaceName: string | null;
  loading: boolean;
  items: HomeWorkspaceConfigItem[];
};
