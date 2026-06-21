import type { CliKey } from "../../services/providers/providers";

export type HomeWorkspaceConfigItemType = "prompts" | "mcp" | "skills";

export type HomeWorkspaceConfigItem = {
  id: string;
  resourceId: number;
  type: HomeWorkspaceConfigItemType;
  label: string;
  name: string;
  enabled: boolean;
};

export type HomeWorkspaceOption = {
  id: number;
  name: string;
  isActive: boolean;
};

export type HomeCliWorkspaceConfig = {
  cliKey: CliKey;
  cliLabel: string;
  workspaceId: number | null;
  workspaceName: string | null;
  workspaces: HomeWorkspaceOption[];
  loading: boolean;
  items: HomeWorkspaceConfigItem[];
};
