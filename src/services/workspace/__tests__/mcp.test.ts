import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  MCP_IMPORT_MAX_SERVERS,
  MCP_PARSE_JSON_MAX_CHARS,
  type McpImportReport,
  type McpImportServer,
  type McpParseResult,
  type McpServerSummary,
  mcpImportFromWorkspaceCli,
  mcpImportServers,
  mcpParseJson,
  mcpServerDelete,
  mcpServerSetEnabled,
  mcpServerUpsert,
  mcpServersList,
  normalizeMcpImportServers,
  validateMcpServerId,
  validateMcpWorkspaceId,
} from "../mcp";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      mcpServersList: vi.fn(),
      mcpServerUpsert: vi.fn(),
      mcpServerSetEnabled: vi.fn(),
      mcpServerDelete: vi.fn(),
      mcpParseJson: vi.fn(),
      mcpImportServers: vi.fn(),
      mcpImportFromWorkspaceCli: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

function makeMcpServerSummary(overrides: Partial<McpServerSummary> = {}): McpServerSummary {
  return {
    id: 1,
    server_key: "fetch",
    name: "Fetch",
    transport: "stdio",
    command: null,
    args: [],
    env_keys: [],
    cwd: null,
    url: null,
    header_keys: [],
    enabled: true,
    created_at: 1,
    updated_at: 1,
    ...overrides,
  };
}

function makeMcpImportServer(overrides: Partial<McpImportServer> = {}): McpImportServer {
  return {
    server_key: "fetch",
    name: "Fetch",
    transport: "http",
    command: null,
    args: [],
    env: {},
    cwd: null,
    url: "http://127.0.0.1:3000",
    headers: { Authorization: "x" },
    enabled: true,
    ...overrides,
  };
}

function makeMcpParseResult(overrides: Partial<McpParseResult> = {}): McpParseResult {
  return {
    servers: [makeMcpImportServer()],
    ...overrides,
  };
}

function makeMcpImportReport(overrides: Partial<McpImportReport> = {}): McpImportReport {
  return {
    inserted: 0,
    updated: 0,
    skipped: [],
    ...overrides,
  };
}

describe("services/workspace/mcp", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.mcpServersList).mockRejectedValueOnce(new Error("mcp boom"));

    await expect(mcpServersList(1)).rejects.toThrow("mcp boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取 MCP 服务列表失败",
      expect.objectContaining({
        cmd: "mcp_servers_list",
        error: expect.stringContaining("mcp boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.mcpServersList).mockResolvedValueOnce(null as never);

    await expect(mcpServersList(1)).rejects.toThrow("IPC_NULL_RESULT: mcp_servers_list");
  });

  it("invokes generated commands with normalized args and typed payloads", async () => {
    vi.mocked(commands.mcpServersList).mockResolvedValueOnce({
      status: "ok",
      data: [makeMcpServerSummary()],
    });
    vi.mocked(commands.mcpServerUpsert).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpServerSummary(),
    });
    vi.mocked(commands.mcpServerSetEnabled).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpServerSummary({ id: 2, enabled: false, updated_at: 2 }),
    });
    vi.mocked(commands.mcpServerDelete).mockResolvedValueOnce({ status: "ok", data: true });
    vi.mocked(commands.mcpParseJson).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpParseResult(),
    });
    vi.mocked(commands.mcpImportFromWorkspaceCli).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpImportReport(),
    });
    vi.mocked(commands.mcpImportServers).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpImportReport({ updated: 1 }),
    });

    const listRows = await mcpServersList(7);
    expect(commands.mcpServersList).toHaveBeenNthCalledWith(1, { workspaceId: 7 });
    expect(listRows[0]?.transport).toBe("stdio");

    const created = await mcpServerUpsert({
      serverKey: "fetch",
      name: "Fetch",
      transport: "stdio",
      command: "npx",
      args: ["-y"],
    });
    expect(commands.mcpServerUpsert).toHaveBeenNthCalledWith(1, {
      serverId: null,
      serverKey: "fetch",
      name: "Fetch",
      transport: "stdio",
      command: "npx",
      args: ["-y"],
      env: {
        preserveKeys: [],
        replace: {},
      },
      cwd: null,
      url: null,
      headers: {
        preserveKeys: [],
        replace: {},
      },
    });
    expect(created.transport).toBe("stdio");

    const updated = await mcpServerSetEnabled({ workspaceId: 9, serverId: 2, enabled: false });
    expect(commands.mcpServerSetEnabled).toHaveBeenCalledWith({
      workspaceId: 9,
      serverId: 2,
      enabled: false,
    });
    expect(updated.enabled).toBe(false);

    await mcpServerDelete(123);
    expect(commands.mcpServerDelete).toHaveBeenCalledWith({ serverId: 123 });

    const parsed = await mcpParseJson('{"mcpServers":[]}');
    expect(commands.mcpParseJson).toHaveBeenCalledWith({
      jsonText: '{"mcpServers":[]}',
    });
    expect(parsed.servers[0]?.transport).toBe("http");

    const imported = await mcpImportFromWorkspaceCli(3);
    expect(commands.mcpImportFromWorkspaceCli).toHaveBeenCalledWith({
      workspaceId: 3,
    });
    expect(imported.inserted).toBe(0);

    const report = await mcpImportServers({
      workspaceId: 1,
      servers: [makeMcpImportServer()],
    });
    expect(commands.mcpImportServers).toHaveBeenCalledWith({
      workspaceId: 1,
      servers: [
        expect.objectContaining({
          server_key: "fetch",
          transport: "http",
          url: "http://127.0.0.1:3000",
        }),
      ],
    });
    expect(report.updated).toBe(1);
  });

  it("normalizes mcp import servers before IPC", () => {
    expect(
      normalizeMcpImportServers([
        makeMcpImportServer({
          server_key: " fetch ",
          name: " Fetch ",
          transport: "stdio",
          command: " npx ",
          args: [" -y ", " @modelcontextprotocol/server-fetch "],
          env: { " TOKEN ": " secret " },
          cwd: " /tmp ",
          url: " http://ignored.example ",
          headers: { Authorization: "ignored" },
        }),
      ])
    ).toEqual([
      {
        server_key: "fetch",
        name: "Fetch",
        transport: "stdio",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-fetch"],
        env: { TOKEN: " secret " },
        cwd: "/tmp",
        url: null,
        headers: {},
        enabled: true,
      },
    ]);

    expect(
      normalizeMcpImportServers([
        makeMcpImportServer({
          server_key: " remote ",
          name: " Remote ",
          transport: "sse",
          command: "ignored",
          args: ["ignored"],
          env: { IGNORED: "1" },
          cwd: "/ignored",
          url: " https://example.com/mcp/sse ",
          headers: { " Authorization ": "Bearer token" },
        }),
      ])
    ).toEqual([
      {
        server_key: "remote",
        name: "Remote",
        transport: "sse",
        command: null,
        args: [],
        env: {},
        cwd: null,
        url: "https://example.com/mcp/sse",
        headers: { Authorization: "Bearer token" },
        enabled: true,
      },
    ]);
  });

  it("rejects invalid ids and oversized import batches before invoking generated commands", async () => {
    expect(validateMcpWorkspaceId(7)).toBe(7);
    expect(validateMcpServerId(9)).toBe(9);
    expect(() => validateMcpWorkspaceId(0)).toThrow("SEC_INVALID_INPUT");
    expect(() => validateMcpServerId(Number.NaN)).toThrow("SEC_INVALID_INPUT");

    await expect(mcpServersList(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      mcpServerSetEnabled({ workspaceId: 1, serverId: 0, enabled: true })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(mcpServerDelete(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(mcpImportFromWorkspaceCli(-1)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(mcpParseJson("   ")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(mcpParseJson("x".repeat(MCP_PARSE_JSON_MAX_CHARS + 1))).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(
      mcpServerUpsert({
        serverId: 0,
        serverKey: "fetch",
        name: "Fetch",
        transport: "stdio",
        command: "npx",
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    const tooManyServers = Array.from({ length: MCP_IMPORT_MAX_SERVERS + 1 }, (_, index) =>
      makeMcpImportServer({ server_key: `fetch-${index}`, name: `Fetch ${index}` })
    );
    await expect(mcpImportServers({ workspaceId: 1, servers: tooManyServers })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );

    expect(commands.mcpServersList).not.toHaveBeenCalledWith({ workspaceId: 0 });
    expect(commands.mcpServerSetEnabled).not.toHaveBeenCalledWith({
      workspaceId: 1,
      serverId: 0,
      enabled: true,
    });
    expect(commands.mcpServerDelete).not.toHaveBeenCalledWith({ serverId: Number.NaN });
    expect(commands.mcpImportFromWorkspaceCli).not.toHaveBeenCalledWith({ workspaceId: -1 });
    expect(commands.mcpParseJson).not.toHaveBeenCalledWith({ jsonText: "" });
    expect(commands.mcpImportServers).not.toHaveBeenCalledWith(
      expect.objectContaining({ servers: tooManyServers })
    );
  });

  it("preserves sse transport when upserting remote mcp servers", async () => {
    vi.mocked(commands.mcpServerUpsert).mockResolvedValueOnce({
      status: "ok",
      data: makeMcpServerSummary({
        id: 8,
        server_key: "remote",
        name: "Remote",
        transport: "sse",
        url: "https://example.com/mcp/sse",
        header_keys: ["Authorization"],
      }),
    });

    const saved = await mcpServerUpsert({
      serverKey: " remote ",
      name: " Remote ",
      transport: "sse",
      command: "ignored",
      args: ["ignored"],
      env: { IGNORED: "1" },
      cwd: "/ignored",
      url: " https://example.com/mcp/sse ",
      headers: { " Authorization ": "Bearer token" },
    });

    expect(commands.mcpServerUpsert).toHaveBeenLastCalledWith({
      serverId: null,
      serverKey: "remote",
      name: "Remote",
      transport: "sse",
      command: null,
      args: [],
      env: {
        preserveKeys: [],
        replace: {},
      },
      cwd: null,
      url: "https://example.com/mcp/sse",
      headers: {
        preserveKeys: [],
        replace: { Authorization: "Bearer token" },
      },
    });
    expect(saved.transport).toBe("sse");
  });
});
