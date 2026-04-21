import {
  commands,
  type McpImportReport as GeneratedMcpImportReport,
  type McpImportServer as GeneratedMcpImportServer,
  type McpParseResult as GeneratedMcpParseResult,
  type McpServerSummaryView,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type McpTransport = "stdio" | "http" | "sse";

export type McpServerSummary = Omit<McpServerSummaryView, "transport"> & {
  transport: McpTransport;
};

export type McpSecretPatchInput =
  | {
      preserve_keys?: string[];
      replace?: Record<string, string>;
    }
  | Record<string, string>;

export type McpServerUpsertInput = {
  server_id?: number | null;
  server_key: string;
  name: string;
  transport: McpTransport;
  command?: string | null;
  args?: string[];
  env?: McpSecretPatchInput;
  cwd?: string | null;
  url?: string | null;
  headers?: McpSecretPatchInput;
};

export type McpImportServer = Omit<GeneratedMcpImportServer, "transport"> & {
  transport: McpTransport;
};

export type McpParseResult = {
  servers: McpImportServer[];
};

export type McpImportReport = GeneratedMcpImportReport;

type NormalizedMcpSecretPatch = {
  preserveKeys: string[];
  replace: Record<string, string>;
};

function normalizeSecretPatchInput(input: McpSecretPatchInput | undefined): NormalizedMcpSecretPatch {
  if (!input) {
    return {
      preserveKeys: [],
      replace: {},
    };
  }

  const maybePatch = input as {
    preserve_keys?: unknown;
    replace?: unknown;
  };
  const hasPatchShape =
    Array.isArray(maybePatch.preserve_keys) ||
    (maybePatch.replace != null &&
      typeof maybePatch.replace === "object" &&
      !Array.isArray(maybePatch.replace));

  if (hasPatchShape) {
    const patch = input as {
      preserve_keys?: string[];
      replace?: Record<string, string>;
    };
    return {
      preserveKeys: patch.preserve_keys ?? [],
      replace: patch.replace ?? {},
    };
  }

  return {
    preserveKeys: [],
    replace: input as Record<string, string>,
  };
}

function buildSafeSecretPatchLog(patch: NormalizedMcpSecretPatch) {
  return {
    preserveKeys: patch.preserveKeys,
    replaceKeys: Object.keys(patch.replace),
  };
}

function asMcpServerSummary(value: McpServerSummaryView): McpServerSummary {
  return value as McpServerSummary;
}

function asMcpParseResult(value: GeneratedMcpParseResult): McpParseResult {
  return value as McpParseResult;
}

export async function mcpServersList(workspaceId: number) {
  return invokeGeneratedIpc<McpServerSummary[]>({
    title: "读取 MCP 服务列表失败",
    cmd: "mcp_servers_list",
    args: { input: { workspaceId } },
    invoke: async () => {
      const result = await commands.mcpServersList({ workspaceId });
      if (result == null) {
        return result as any;
      }
      if (result.status === "ok") {
        return {
          status: "ok" as const,
          data: result.data.map(asMcpServerSummary),
        };
      }
      return result;
    },
  });
}

export async function mcpServerUpsert(input: McpServerUpsertInput) {
  const normalizedEnv = normalizeSecretPatchInput(input.env);
  const normalizedHeaders = normalizeSecretPatchInput(input.headers);
  const payload = {
    serverId: input.server_id ?? null,
    serverKey: input.server_key,
    name: input.name,
    transport: input.transport,
    command: input.command ?? null,
    args: input.args ?? [],
    env: normalizedEnv,
    cwd: input.cwd ?? null,
    url: input.url ?? null,
    headers: normalizedHeaders,
  };

  return invokeGeneratedIpc<McpServerSummary>({
    title: "保存 MCP 服务失败",
    cmd: "mcp_server_upsert",
    args: {
      input: {
        serverId: payload.serverId,
        serverKey: payload.serverKey,
        name: payload.name,
        transport: payload.transport,
        command: payload.command,
        args: payload.args,
        cwd: payload.cwd,
        url: payload.url,
        env: buildSafeSecretPatchLog(normalizedEnv),
        headers: buildSafeSecretPatchLog(normalizedHeaders),
      },
    },
    invoke: async () => {
      const result = await commands.mcpServerUpsert(payload);
      if (result == null) {
        return result as any;
      }
      if (result.status === "ok") {
        return {
          status: "ok" as const,
          data: asMcpServerSummary(result.data),
        };
      }
      return result;
    },
  });
}

export async function mcpServerSetEnabled(input: {
  workspace_id: number;
  server_id: number;
  enabled: boolean;
}) {
  const payload = {
    workspaceId: input.workspace_id,
    serverId: input.server_id,
    enabled: input.enabled,
  };

  return invokeGeneratedIpc<McpServerSummary>({
    title: "更新 MCP 服务启用状态失败",
    cmd: "mcp_server_set_enabled",
    args: { input: payload },
    invoke: async () => {
      const result = await commands.mcpServerSetEnabled(payload);
      if (result == null) {
        return result as any;
      }
      if (result.status === "ok") {
        return {
          status: "ok" as const,
          data: asMcpServerSummary(result.data),
        };
      }
      return result;
    },
  });
}

export async function mcpServerDelete(serverId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "删除 MCP 服务失败",
    cmd: "mcp_server_delete",
    args: { input: { serverId } },
    invoke: () =>
      commands.mcpServerDelete({ serverId }) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function mcpParseJson(jsonText: string) {
  return invokeGeneratedIpc<McpParseResult>({
    title: "解析 MCP JSON 失败",
    cmd: "mcp_parse_json",
    args: { input: { jsonText } },
    invoke: async () => {
      const result = await commands.mcpParseJson({ jsonText });
      if (result == null) {
        return result as any;
      }
      if (result.status === "ok") {
        return {
          status: "ok" as const,
          data: asMcpParseResult(result.data),
        };
      }
      return result;
    },
  });
}

export async function mcpImportServers(input: {
  workspace_id: number;
  servers: McpImportServer[];
}) {
  const payload = {
    workspaceId: input.workspace_id,
    servers: input.servers,
  };

  return invokeGeneratedIpc<McpImportReport>({
    title: "导入 MCP 服务失败",
    cmd: "mcp_import_servers",
    args: { input: payload },
    invoke: () =>
      commands.mcpImportServers(payload) as Promise<GeneratedCommandResult<McpImportReport>>,
  });
}

export async function mcpImportFromWorkspaceCli(workspace_id: number) {
  return invokeGeneratedIpc<McpImportReport>({
    title: "从工作区 CLI 导入 MCP 服务失败",
    cmd: "mcp_import_from_workspace_cli",
    args: { input: { workspaceId: workspace_id } },
    invoke: () =>
      commands.mcpImportFromWorkspaceCli({
        workspaceId: workspace_id,
      }) as Promise<GeneratedCommandResult<McpImportReport>>,
  });
}
