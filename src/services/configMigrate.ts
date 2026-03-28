import { invokeService } from "./invokeServiceCommand";

export type ConfigBundle = {
  schema_version: number;
  exported_at: string;
  app_version: string;
  settings: string;
  providers: unknown[];
  sort_modes: unknown[];
  sort_mode_active: Record<string, string>;
  workspaces: unknown[];
  mcp_servers: unknown[];
  skill_repos: unknown[];
};

export type ConfigImportResult = {
  providers_imported: number;
  sort_modes_imported: number;
  workspaces_imported: number;
  mcp_servers_imported: number;
  skill_repos_imported: number;
};

export async function configExport() {
  return invokeService<ConfigBundle>("导出配置失败", "config_export");
}

export async function configImport(bundle: object) {
  return invokeService<ConfigImportResult>("导入配置失败", "config_import", { bundle });
}
