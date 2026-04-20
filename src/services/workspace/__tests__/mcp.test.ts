import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  mcpImportFromWorkspaceCli,
  mcpImportServers,
  mcpParseJson,
  mcpServerDelete,
  mcpServerSetEnabled,
  mcpServerUpsert,
  mcpServersList,
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
    vi.mocked(commands.mcpServersList).mockResolvedValueOnce(null as any);

    await expect(mcpServersList(1)).rejects.toThrow("IPC_NULL_RESULT: mcp_servers_list");
  });

  it("invokes generated commands with normalized args", async () => {
    vi.mocked(commands.mcpServersList).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.mcpServerUpsert).mockResolvedValueOnce({
      status: "ok",
      data: {
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
      } as any,
    });
    vi.mocked(commands.mcpServerSetEnabled).mockResolvedValueOnce({
      status: "ok",
      data: {
        id: 2,
        server_key: "fetch",
        name: "Fetch",
        transport: "stdio",
        command: null,
        args: [],
        env_keys: [],
        cwd: null,
        url: null,
        header_keys: [],
        enabled: false,
        created_at: 1,
        updated_at: 2,
      } as any,
    });
    vi.mocked(commands.mcpServerDelete).mockResolvedValueOnce({ status: "ok", data: true });
    vi.mocked(commands.mcpParseJson).mockResolvedValueOnce({
      status: "ok",
      data: { servers: [] } as any,
    });
    vi.mocked(commands.mcpImportFromWorkspaceCli).mockResolvedValueOnce({
      status: "ok",
      data: { inserted: 0, updated: 0, skipped: [] } as any,
    });
    vi.mocked(commands.mcpImportServers).mockResolvedValueOnce({
      status: "ok",
      data: { inserted: 0, updated: 1, skipped: [] } as any,
    });

    await mcpServersList(7);
    expect(commands.mcpServersList).toHaveBeenNthCalledWith(1, { workspaceId: 7 });

    await mcpServerUpsert({
      server_key: "fetch",
      name: "Fetch",
      transport: "stdio",
    });
    expect(commands.mcpServerUpsert).toHaveBeenNthCalledWith(1, {
      serverId: null,
      serverKey: "fetch",
      name: "Fetch",
      transport: "stdio",
      command: null,
      args: [],
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

    await mcpServerSetEnabled({ workspace_id: 9, server_id: 2, enabled: false });
    expect(commands.mcpServerSetEnabled).toHaveBeenCalledWith({
      workspaceId: 9,
      serverId: 2,
      enabled: false,
    });

    await mcpServerDelete(123);
    expect(commands.mcpServerDelete).toHaveBeenCalledWith({ serverId: 123 });

    await mcpParseJson('{"mcpServers":[]}');
    expect(commands.mcpParseJson).toHaveBeenCalledWith({
      jsonText: '{"mcpServers":[]}',
    });

    await mcpImportFromWorkspaceCli(3);
    expect(commands.mcpImportFromWorkspaceCli).toHaveBeenCalledWith({
      workspaceId: 3,
    });

    await mcpImportServers({
      workspace_id: 1,
      servers: [
        {
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
        },
      ],
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
  });
});
