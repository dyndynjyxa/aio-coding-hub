import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  MAX_WORKSPACE_NAME_CHARS,
  normalizeWorkspaceName,
  workspaceApply,
  workspaceCreate,
  workspaceDelete,
  workspacePreview,
  workspaceRename,
  workspacesList,
  validateWorkspaceCliKey,
  validateWorkspaceId,
  type WorkspaceSummary,
} from "../workspaces";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      workspacesList: vi.fn(),
      workspaceCreate: vi.fn(),
      workspaceRename: vi.fn(),
      workspaceDelete: vi.fn(),
      workspacePreview: vi.fn(),
      workspaceApply: vi.fn(),
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

describe("services/workspace/workspaces", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  function createWorkspaceSummary(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
    return {
      id: 1,
      cli_key: "claude",
      name: "W1",
      created_at: 0,
      updated_at: 0,
      ...overrides,
    };
  }

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.workspacesList).mockRejectedValueOnce(new Error("workspaces boom"));

    await expect(workspacesList("claude")).rejects.toThrow("workspaces boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取工作区列表失败",
      expect.objectContaining({
        cmd: "workspaces_list",
        error: expect.stringContaining("workspaces boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.workspacesList).mockResolvedValueOnce(null as never);

    await expect(workspacesList("claude")).rejects.toThrow("IPC_NULL_RESULT: workspaces_list");
  });

  it("invokes generated commands with normalized args", async () => {
    vi.mocked(commands.workspacesList).mockResolvedValue({
      status: "ok",
      data: { active_id: null, items: [createWorkspaceSummary()] },
    });
    vi.mocked(commands.workspaceCreate).mockResolvedValue({
      status: "ok",
      data: createWorkspaceSummary(),
    });
    vi.mocked(commands.workspaceRename).mockResolvedValue({
      status: "ok",
      data: createWorkspaceSummary({ name: "W9" }),
    });
    vi.mocked(commands.workspaceDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.workspacePreview).mockResolvedValue({
      status: "ok",
      data: {
        cli_key: "claude",
        from_workspace_id: null,
        to_workspace_id: 9,
        prompts: { from_enabled: null, to_enabled: null, will_change: false },
        mcp: { from_enabled: [], to_enabled: [], added: [], removed: [] },
        skills: { from_enabled: [], to_enabled: [], added: [], removed: [] },
      },
    });
    vi.mocked(commands.workspaceApply).mockResolvedValue({
      status: "ok",
      data: {
        cli_key: "claude",
        from_workspace_id: null,
        to_workspace_id: 9,
        applied_at: 1,
      },
    });

    await workspacesList(" claude " as never);
    expect(commands.workspacesList).toHaveBeenCalledWith("claude");

    await workspaceCreate({
      cliKey: " claude " as never,
      name: " W1 ",
      cloneFromActive: true,
    });
    expect(commands.workspaceCreate).toHaveBeenCalledWith("claude", "W1", true);

    await workspaceRename({ workspaceId: 9, name: " W9 " });
    expect(commands.workspaceRename).toHaveBeenCalledWith(9, "W9");

    await workspaceDelete(9);
    expect(commands.workspaceDelete).toHaveBeenCalledWith(9);

    await workspacePreview(9);
    expect(commands.workspacePreview).toHaveBeenCalledWith(9);

    await workspaceApply(9);
    expect(commands.workspaceApply).toHaveBeenCalledWith(9);
  });

  it("rejects invalid cli keys, ids, and blank names before generated commands", async () => {
    expect(validateWorkspaceCliKey(" claude ")).toBe("claude");
    expect(validateWorkspaceId(2)).toBe(2);
    expect(normalizeWorkspaceName("  Work  ")).toBe("Work");
    expect(() => validateWorkspaceCliKey("unknown")).toThrow("SEC_INVALID_INPUT");
    expect(() => validateWorkspaceId(Number.NaN)).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeWorkspaceName("x".repeat(MAX_WORKSPACE_NAME_CHARS + 1))).toThrow(
      "workspace name is too long"
    );
    expect(() => normalizeWorkspaceName("bad\nname")).toThrow(
      "workspace name contains control characters"
    );

    await expect(workspacesList("unknown" as never)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      workspaceCreate({ cliKey: "claude", name: "   ", cloneFromActive: false })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      workspaceCreate({ cliKey: "unknown" as never, name: "W1", cloneFromActive: false })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(workspaceRename({ workspaceId: 0, name: "W9" })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(workspaceRename({ workspaceId: 9, name: "   " })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(
      workspaceCreate({
        cliKey: "claude",
        name: "x".repeat(MAX_WORKSPACE_NAME_CHARS + 1),
        cloneFromActive: false,
      })
    ).rejects.toThrow("workspace name is too long");
    await expect(workspaceRename({ workspaceId: 9, name: "bad\tname" })).rejects.toThrow(
      "workspace name contains control characters"
    );
    await expect(workspaceDelete(-1)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(workspacePreview(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(workspaceApply(1.5)).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.workspacesList).not.toHaveBeenCalled();
    expect(commands.workspaceCreate).not.toHaveBeenCalled();
    expect(commands.workspaceRename).not.toHaveBeenCalled();
    expect(commands.workspaceDelete).not.toHaveBeenCalled();
    expect(commands.workspacePreview).not.toHaveBeenCalled();
    expect(commands.workspaceApply).not.toHaveBeenCalled();
  });
});
