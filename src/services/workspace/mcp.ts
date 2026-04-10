import { invokeService } from "../invokeServiceCommand";

export type McpTransport = "stdio" | "http";

export type McpServerSummary = {
  id: number;
  server_key: string;
  name: string;
  transport: McpTransport;
  command: string | null;
  args: string[];
  env: Record<string, string>;
  cwd: string | null;
  url: string | null;
  headers: Record<string, string>;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type McpImportServer = {
  server_key: string;
  name: string;
  transport: McpTransport;
  command: string | null;
  args: string[];
  env: Record<string, string>;
  cwd: string | null;
  url: string | null;
  headers: Record<string, string>;
  enabled: boolean;
};

export type McpParseResult = {
  servers: McpImportServer[];
};

export type McpImportReport = {
  inserted: number;
  updated: number;
  skipped?: Array<{
    name: string;
    reason: string;
  }>;
};

export async function mcpServersList(workspaceId: number) {
  return invokeService<McpServerSummary[]>("读取 MCP 服务列表失败", "mcp_servers_list", {
    workspaceId,
  });
}

export async function mcpServerUpsert(input: {
  server_id?: number | null;
  server_key: string;
  name: string;
  transport: McpTransport;
  command?: string | null;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string | null;
  url?: string | null;
  headers?: Record<string, string>;
}) {
  return invokeService<McpServerSummary>("保存 MCP 服务失败", "mcp_server_upsert", {
    serverId: input.server_id ?? null,
    serverKey: input.server_key,
    name: input.name,
    transport: input.transport,
    command: input.command ?? null,
    args: input.args ?? [],
    env: input.env ?? {},
    cwd: input.cwd ?? null,
    url: input.url ?? null,
    headers: input.headers ?? {},
  });
}

export async function mcpServerSetEnabled(input: {
  workspace_id: number;
  server_id: number;
  enabled: boolean;
}) {
  return invokeService<McpServerSummary>("更新 MCP 服务启用状态失败", "mcp_server_set_enabled", {
    workspaceId: input.workspace_id,
    serverId: input.server_id,
    enabled: input.enabled,
  });
}

export async function mcpServerDelete(serverId: number) {
  return invokeService<boolean>("删除 MCP 服务失败", "mcp_server_delete", { serverId });
}

export async function mcpParseJson(jsonText: string) {
  return invokeService<McpParseResult>("解析 MCP JSON 失败", "mcp_parse_json", { jsonText });
}

export async function mcpImportServers(input: {
  workspace_id: number;
  servers: McpImportServer[];
}) {
  return invokeService<McpImportReport>("导入 MCP 服务失败", "mcp_import_servers", {
    workspaceId: input.workspace_id,
    servers: input.servers,
  });
}

export async function mcpImportFromWorkspaceCli(workspace_id: number) {
  return invokeService<McpImportReport>(
    "从工作区 CLI 导入 MCP 服务失败",
    "mcp_import_from_workspace_cli",
    {
      workspaceId: workspace_id,
    }
  );
}
