import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  mcpImportFromWorkspaceCli,
  mcpImportServers,
  mcpParseJson,
  mcpServerDelete,
  mcpServerSetEnabled,
  mcpServerUpsert,
  mcpServersList,
} from "../mcp";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
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
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("mcp boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(mcpServersList(1)).rejects.toThrow("IPC_NULL_RESULT: mcp_servers_list");
  });

  it("invokes tauri commands with normalized args", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({ inserted: 0, updated: 1, skipped: [] } as any);

    await mcpServersList(7);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_servers_list", { workspaceId: 7 });

    await mcpServerUpsert({
      server_key: "fetch",
      name: "Fetch",
      transport: "stdio",
    });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_server_upsert", {
      serverId: null,
      serverKey: "fetch",
      name: "Fetch",
      transport: "stdio",
      command: null,
      args: [],
      env: {},
      cwd: null,
      url: null,
      headers: {},
    });

    await mcpServerSetEnabled({ workspace_id: 9, server_id: 2, enabled: false });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_server_set_enabled", {
      workspaceId: 9,
      serverId: 2,
      enabled: false,
    });

    await mcpServerDelete(123);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_server_delete", { serverId: 123 });

    await mcpParseJson('{"mcpServers":[]}');
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_parse_json", {
      jsonText: '{"mcpServers":[]}',
    });

    await mcpImportFromWorkspaceCli(3);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_import_from_workspace_cli", {
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
    expect(invokeTauriOrNull).toHaveBeenCalledWith("mcp_import_servers", {
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
