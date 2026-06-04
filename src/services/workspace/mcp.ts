import {
  commands,
  type McpImportReport,
  type McpImportServer as GeneratedMcpImportServer,
  type McpParseResult as GeneratedMcpParseResult,
  type McpSecretPatchInput as GeneratedMcpSecretPatchInput,
  type McpServerSummaryView as GeneratedMcpServerSummaryView,
  type McpServerUpsertInput as GeneratedMcpServerUpsertInput,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import {
  narrowGeneratedStringUnion,
  type OptionalNullableGeneratedFields,
  type Override,
} from "../generatedTypeUtils";

const MCP_TRANSPORT_VALUES = ["stdio", "http", "sse"] as const;

export const MCP_IMPORT_MAX_SERVERS = 512;
export const MCP_PARSE_JSON_MAX_CHARS = 1024 * 1024;

export type McpTransport = (typeof MCP_TRANSPORT_VALUES)[number];

export type McpServerSummary = Override<
  GeneratedMcpServerSummaryView,
  {
    transport: McpTransport;
  }
>;

export type McpSecretPatchInput =
  | OptionalNullableGeneratedFields<GeneratedMcpSecretPatchInput>
  | Record<string, string>;

type McpServerUpsertTransportInput = OptionalNullableGeneratedFields<GeneratedMcpServerUpsertInput>;

export type McpServerUpsertInput = Override<
  McpServerUpsertTransportInput,
  {
    transport: McpTransport;
    env?: McpSecretPatchInput;
    headers?: McpSecretPatchInput;
  }
>;

export type McpImportServer = Override<
  GeneratedMcpImportServer,
  {
    transport: McpTransport;
  }
>;

export type McpParseResult = Override<
  GeneratedMcpParseResult,
  {
    servers: McpImportServer[];
  }
>;

type McpSecretPatchDraft = OptionalNullableGeneratedFields<GeneratedMcpSecretPatchInput>;

function validatePositiveSafeInteger(label: string, value: number): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

export function validateMcpWorkspaceId(workspaceId: number): number {
  return validatePositiveSafeInteger("workspaceId", workspaceId);
}

export function validateMcpServerId(serverId: number): number {
  return validatePositiveSafeInteger("serverId", serverId);
}

function normalizeOptionalMcpServerId(serverId: number | null | undefined): number | null {
  if (serverId == null) return null;
  return validateMcpServerId(serverId);
}

function toMcpTransport(value: string, label: string): McpTransport {
  return narrowGeneratedStringUnion(value, MCP_TRANSPORT_VALUES, label);
}

function normalizeRequiredText(value: string, label: string): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  return normalized;
}

function normalizeMcpJsonText(value: string): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: JSON is required");
  }
  if (normalized.length > MCP_PARSE_JSON_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: JSON must contain at most ${MCP_PARSE_JSON_MAX_CHARS} characters`
    );
  }
  return normalized;
}

function normalizeOptionalText(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized ? normalized : null;
}

function normalizeStringArray(values: readonly string[] | undefined, label: string): string[] {
  if (!values) return [];
  if (!Array.isArray(values)) {
    throw new Error(`SEC_INVALID_INPUT: ${label} must be an array`);
  }
  return values.map((value) => normalizeRequiredText(value, label));
}

function normalizeStringRecord(
  value: Partial<Record<string, string>> | undefined,
  label: string
): Record<string, string> {
  if (!value) return {};
  if (typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`SEC_INVALID_INPUT: ${label} must be an object`);
  }

  const normalized: Record<string, string> = {};
  for (const [rawKey, entryValue] of Object.entries(value)) {
    const key = rawKey.trim();
    if (!key) {
      throw new Error(`SEC_INVALID_INPUT: ${label} key is required`);
    }
    if (typeof entryValue !== "string") {
      throw new Error(`SEC_INVALID_INPUT: ${label}.${key} must be a string`);
    }
    if (!entryValue.trim()) {
      throw new Error(`SEC_INVALID_INPUT: ${label}.${key} is required`);
    }
    normalized[key] = entryValue;
  }
  return normalized;
}

function normalizeSecretPatchInput(
  input: McpSecretPatchInput | undefined
): GeneratedMcpSecretPatchInput {
  if (!input) {
    return {
      preserveKeys: [],
      replace: {},
    };
  }

  const patchInput = input as McpSecretPatchDraft;
  const hasPatchShape =
    Array.isArray(patchInput.preserveKeys) ||
    (patchInput.replace != null &&
      typeof patchInput.replace === "object" &&
      !Array.isArray(patchInput.replace));

  if (hasPatchShape) {
    return {
      preserveKeys: normalizeStringArray(patchInput.preserveKeys ?? [], "preserveKeys"),
      replace: normalizeStringRecord(patchInput.replace ?? {}, "replace"),
    };
  }

  return {
    preserveKeys: [],
    replace: normalizeStringRecord(input, "replace"),
  };
}

function buildSafeSecretPatchLog(patch: GeneratedMcpSecretPatchInput) {
  return {
    preserveKeys: patch.preserveKeys ?? [],
    replaceKeys: Object.keys(patch.replace ?? {}),
  };
}

function toMcpServerSummary(value: GeneratedMcpServerSummaryView): McpServerSummary {
  return {
    ...value,
    transport: toMcpTransport(value.transport, "mcp_server.transport"),
  };
}

function toMcpImportServer(value: GeneratedMcpImportServer): McpImportServer {
  return {
    ...value,
    transport: toMcpTransport(value.transport, "mcp_import_server.transport"),
  };
}

function toMcpParseResult(value: GeneratedMcpParseResult): McpParseResult {
  return {
    ...value,
    servers: value.servers.map(toMcpImportServer),
  };
}

function normalizeMcpImportServer(server: McpImportServer, index: number): McpImportServer {
  const label = `servers[${index}]`;
  const transport = toMcpTransport(server.transport, `${label}.transport`);
  const isStdio = transport === "stdio";
  const command = normalizeOptionalText(server.command);
  const url = normalizeOptionalText(server.url);

  if (isStdio && !command) {
    throw new Error(`SEC_INVALID_INPUT: ${label}.command is required`);
  }
  if (!isStdio && !url) {
    throw new Error(`SEC_INVALID_INPUT: ${label}.url is required`);
  }
  if (typeof server.enabled !== "boolean") {
    throw new Error(`SEC_INVALID_INPUT: ${label}.enabled must be a boolean`);
  }

  return {
    server_key: normalizeOptionalText(server.server_key) ?? "",
    name: normalizeRequiredText(server.name, `${label}.name`),
    transport,
    command: isStdio ? command : null,
    args: isStdio ? normalizeStringArray(server.args, `${label}.args`) : [],
    env: isStdio ? normalizeStringRecord(server.env, `${label}.env`) : {},
    cwd: isStdio ? normalizeOptionalText(server.cwd) : null,
    url: isStdio ? null : url,
    headers: isStdio ? {} : normalizeStringRecord(server.headers, `${label}.headers`),
    enabled: server.enabled,
  };
}

export function normalizeMcpImportServers(servers: readonly McpImportServer[]): McpImportServer[] {
  if (!Array.isArray(servers)) {
    throw new Error("SEC_INVALID_INPUT: servers must be an array");
  }
  if (servers.length === 0) {
    throw new Error("SEC_INVALID_INPUT: servers is required");
  }
  if (servers.length > MCP_IMPORT_MAX_SERVERS) {
    throw new Error(
      `SEC_INVALID_INPUT: servers must contain at most ${MCP_IMPORT_MAX_SERVERS} entries`
    );
  }
  return servers.map(normalizeMcpImportServer);
}

export async function mcpServersList(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);

  return invokeGeneratedIpc<McpServerSummary[]>({
    title: "读取 MCP 服务列表失败",
    cmd: "mcp_servers_list",
    args: { input: { workspaceId: normalizedWorkspaceId } },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.mcpServersList({ workspaceId: normalizedWorkspaceId }),
        (rows) => rows.map(toMcpServerSummary)
      ),
  });
}

export async function mcpServerUpsert(input: McpServerUpsertInput) {
  const transport = toMcpTransport(input.transport, "mcp_server_upsert.transport");
  const isStdio = transport === "stdio";
  const command = normalizeOptionalText(input.command);
  const url = normalizeOptionalText(input.url);

  if (isStdio && !command) {
    throw new Error("SEC_INVALID_INPUT: command is required");
  }
  if (!isStdio && !url) {
    throw new Error("SEC_INVALID_INPUT: url is required");
  }

  const normalizedEnv = normalizeSecretPatchInput(input.env);
  const normalizedHeaders = normalizeSecretPatchInput(input.headers);
  const payload: GeneratedMcpServerUpsertInput = {
    serverId: normalizeOptionalMcpServerId(input.serverId),
    serverKey: normalizeOptionalText(input.serverKey) ?? "",
    name: normalizeRequiredText(input.name, "name"),
    transport,
    command: isStdio ? command : null,
    args: isStdio ? normalizeStringArray(input.args ?? [], "args") : [],
    env: isStdio ? normalizedEnv : { preserveKeys: [], replace: {} },
    cwd: isStdio ? normalizeOptionalText(input.cwd) : null,
    url: isStdio ? null : url,
    headers: isStdio ? { preserveKeys: [], replace: {} } : normalizedHeaders,
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
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.mcpServerUpsert(payload), toMcpServerSummary),
  });
}

export async function mcpServerSetEnabled(input: {
  workspaceId: number;
  serverId: number;
  enabled: boolean;
}) {
  const payload = {
    workspaceId: validateMcpWorkspaceId(input.workspaceId),
    serverId: validateMcpServerId(input.serverId),
    enabled: input.enabled,
  };

  return invokeGeneratedIpc<McpServerSummary>({
    title: "更新 MCP 服务启用状态失败",
    cmd: "mcp_server_set_enabled",
    args: { input: payload },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.mcpServerSetEnabled(payload), toMcpServerSummary),
  });
}

export async function mcpServerDelete(serverId: number) {
  const normalizedServerId = validateMcpServerId(serverId);

  return invokeGeneratedIpc<boolean>({
    title: "删除 MCP 服务失败",
    cmd: "mcp_server_delete",
    args: { input: { serverId: normalizedServerId } },
    invoke: () => commands.mcpServerDelete({ serverId: normalizedServerId }),
  });
}

export async function mcpParseJson(jsonText: string) {
  const normalizedJsonText = normalizeMcpJsonText(jsonText);
  return invokeGeneratedIpc<McpParseResult>({
    title: "解析 MCP JSON 失败",
    cmd: "mcp_parse_json",
    args: { input: { jsonTextChars: normalizedJsonText.length } },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.mcpParseJson({ jsonText: normalizedJsonText }),
        toMcpParseResult
      ),
  });
}

export async function mcpImportServers(input: { workspaceId: number; servers: McpImportServer[] }) {
  const payload = {
    workspaceId: validateMcpWorkspaceId(input.workspaceId),
    servers: normalizeMcpImportServers(input.servers),
  };

  return invokeGeneratedIpc<McpImportReport>({
    title: "导入 MCP 服务失败",
    cmd: "mcp_import_servers",
    args: { input: payload },
    invoke: () => commands.mcpImportServers(payload),
  });
}

export async function mcpImportFromWorkspaceCli(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);

  return invokeGeneratedIpc<McpImportReport>({
    title: "从工作区 CLI 导入 MCP 服务失败",
    cmd: "mcp_import_from_workspace_cli",
    args: { input: { workspaceId: normalizedWorkspaceId } },
    invoke: () => commands.mcpImportFromWorkspaceCli({ workspaceId: normalizedWorkspaceId }),
  });
}

export type { McpImportReport };
