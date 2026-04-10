import { invokeService } from "../invokeServiceCommand";

export type ConfigBundlePrompt = {
  name: string;
  content: string;
  enabled: boolean;
};

export type ConfigBundleWorkspace = {
  cli_key?: string;
  name?: string;
  is_active?: boolean;
  prompts?: ConfigBundlePrompt[];
  prompt?: ConfigBundlePrompt | null;
};

export type ConfigBundle = {
  schema_version: number;
  exported_at: string;
  app_version: string;
  settings: string;
  providers: unknown[];
  sort_modes: unknown[];
  sort_mode_active: Record<string, string>;
  workspaces: ConfigBundleWorkspace[];
  mcp_servers: unknown[];
  skill_repos: unknown[];
  installed_skills?: unknown[];
  local_skills?: unknown[];
};

export type ConfigImportResult = {
  providers_imported: number;
  sort_modes_imported: number;
  workspaces_imported: number;
  prompts_imported: number;
  mcp_servers_imported: number;
  skill_repos_imported: number;
  installed_skills_imported: number;
  local_skills_imported: number;
};

export async function configExport() {
  return invokeService<ConfigBundle>("导出配置失败", "config_export");
}

export async function configImport(bundle: object) {
  return invokeService<ConfigImportResult>("导入配置失败", "config_import", { bundle });
}
