import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  WorkspacesListResult,
  WorkspaceSummary,
  WorkspacePreview,
} from "../../services/workspace/workspaces";
import {
  MAX_WORKSPACE_NAME_CHARS,
  workspaceApply,
  workspaceCreate,
  workspaceDelete,
  workspacePreview,
  workspaceRename,
  workspacesList,
} from "../../services/workspace/workspaces";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { workspacesKeys } from "../keys";
import {
  isWorkspacePreviewReady,
  pickWorkspaceById,
  useWorkspaceApplyMutation,
  useWorkspaceCreateMutation,
  useWorkspaceDeleteMutation,
  useWorkspacePreviewQuery,
  useWorkspaceRenameMutation,
  useWorkspacesListQuery,
} from "../workspaces";

vi.mock("../../services/workspace/workspaces", async () => {
  const actual = await vi.importActual<typeof import("../../services/workspace/workspaces")>(
    "../../services/workspace/workspaces"
  );
  return {
    ...actual,
    workspacesList: vi.fn(),
    workspaceCreate: vi.fn(),
    workspacePreview: vi.fn(),
    workspaceRename: vi.fn(),
    workspaceDelete: vi.fn(),
    workspaceApply: vi.fn(),
  };
});

describe("query/workspaces", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls workspacesList with tauri runtime", async () => {
    setTauriRuntime();

    const res: WorkspacesListResult = { active_id: null, items: [] };
    vi.mocked(workspacesList).mockResolvedValue(res);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWorkspacesListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(workspacesList).toHaveBeenCalledWith("claude");
    });
  });

  it("rejects invalid cli keys or workspace ids before creating workspace query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() => renderHook(() => useWorkspacesListQuery("unknown" as never), { wrapper })).toThrow(
      "SEC_INVALID_INPUT"
    );
    expect(() => renderHook(() => useWorkspacePreviewQuery(0), { wrapper })).toThrow(
      "SEC_INVALID_INPUT"
    );
  });

  it("useWorkspacesListQuery enters error state when workspacesList rejects", async () => {
    setTauriRuntime();

    vi.mocked(workspacesList).mockRejectedValue(new Error("workspaces query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspacesListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useWorkspacePreviewQuery calls workspacePreview when workspaceId is set", async () => {
    setTauriRuntime();

    const preview: WorkspacePreview = {
      cli_key: "claude",
      from_workspace_id: null,
      to_workspace_id: 1,
      prompts: { from_enabled: null, to_enabled: null, will_change: false },
      mcp: { from_enabled: [], to_enabled: [], added: [], removed: [] },
      skills: { from_enabled: [], to_enabled: [], added: [], removed: [] },
    };
    vi.mocked(workspacePreview).mockResolvedValue(preview);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useWorkspacePreviewQuery(1), { wrapper });

    await waitFor(() => {
      expect(workspacePreview).toHaveBeenCalledWith(1);
    });
  });

  it("keeps null workspace preview queries disabled", () => {
    setTauriRuntime();
    vi.mocked(workspacePreview).mockResolvedValue(null as never);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspacePreviewQuery(null), { wrapper });

    expect(result.current.fetchStatus).toBe("idle");
    expect(workspacePreview).not.toHaveBeenCalled();
  });

  it("useWorkspaceCreateMutation inserts into cached list", async () => {
    setTauriRuntime();

    const created: WorkspaceSummary = {
      id: 2,
      cli_key: "claude",
      name: "W2",
      created_at: 0,
      updated_at: 0,
    };
    vi.mocked(workspaceCreate).mockResolvedValue(created);

    const client = createTestQueryClient();
    client.setQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"), {
      active_id: 1,
      items: [
        {
          id: 1,
          cli_key: "claude",
          name: "W1",
          created_at: 0,
          updated_at: 0,
        },
      ],
    });
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspaceCreateMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", name: " W2 " });
    });

    const next = client.getQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"));
    expect(next?.items.map((w) => w.id)).toEqual([2, 1]);
    expect(workspaceCreate).toHaveBeenCalledWith({
      cliKey: "claude",
      name: "W2",
      cloneFromActive: undefined,
    });
  });

  it("useWorkspaceRenameMutation updates cached list row", async () => {
    setTauriRuntime();

    const updated: WorkspaceSummary = {
      id: 1,
      cli_key: "claude",
      name: "Renamed",
      created_at: 0,
      updated_at: 1,
    };
    vi.mocked(workspaceRename).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"), {
      active_id: 1,
      items: [
        { id: 1, cli_key: "claude", name: "W1", created_at: 0, updated_at: 0 },
        { id: 2, cli_key: "claude", name: "W2", created_at: 0, updated_at: 0 },
      ],
    });
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspaceRenameMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", workspaceId: 1, name: "Renamed" });
    });

    const next = client.getQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"));
    expect(next?.items[0]).toEqual(updated);
  });

  it("useWorkspaceDeleteMutation removes row and clears active_id if deleted", async () => {
    setTauriRuntime();

    vi.mocked(workspaceDelete).mockResolvedValue(true);

    const client = createTestQueryClient();
    client.setQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"), {
      active_id: 1,
      items: [
        { id: 1, cli_key: "claude", name: "W1", created_at: 0, updated_at: 0 },
        { id: 2, cli_key: "claude", name: "W2", created_at: 0, updated_at: 0 },
      ],
    });
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspaceDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", workspaceId: 1 });
    });

    const next = client.getQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"));
    expect(next?.active_id).toBeNull();
    expect(next?.items.map((w) => w.id)).toEqual([2]);
  });

  it("useWorkspaceApplyMutation updates active_id and invalidates preview", async () => {
    setTauriRuntime();

    vi.mocked(workspaceApply).mockResolvedValue({
      cli_key: "claude",
      from_workspace_id: null,
      to_workspace_id: 2,
      applied_at: 0,
    });

    const client = createTestQueryClient();
    client.setQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"), {
      active_id: null,
      items: [{ id: 2, cli_key: "claude", name: "W2", created_at: 0, updated_at: 0 }],
    });
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useWorkspaceApplyMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", workspaceId: 2 });
    });

    const next = client.getQueryData<WorkspacesListResult | null>(workspacesKeys.list("claude"));
    expect(next?.active_id).toBe(2);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: workspacesKeys.preview(2) });
  });

  it("rejects invalid workspace mutation inputs before service calls", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result: createResult } = renderHook(() => useWorkspaceCreateMutation(), { wrapper });
    await expect(
      createResult.current.mutateAsync({ cliKey: "unknown" as never, name: "W2" })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      createResult.current.mutateAsync({ cliKey: "claude", name: "   " })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      createResult.current.mutateAsync({
        cliKey: "claude",
        name: "x".repeat(MAX_WORKSPACE_NAME_CHARS + 1),
      })
    ).rejects.toThrow("workspace name is too long");
    expect(workspaceCreate).not.toHaveBeenCalled();

    const { result: renameResult } = renderHook(() => useWorkspaceRenameMutation(), { wrapper });
    await expect(
      renameResult.current.mutateAsync({ cliKey: "claude", workspaceId: 0, name: "W2" })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      renameResult.current.mutateAsync({ cliKey: "claude", workspaceId: 1, name: "bad\nname" })
    ).rejects.toThrow("workspace name contains control characters");
    expect(workspaceRename).not.toHaveBeenCalled();

    const { result: deleteResult } = renderHook(() => useWorkspaceDeleteMutation(), { wrapper });
    await expect(
      deleteResult.current.mutateAsync({ cliKey: "claude", workspaceId: Number.NaN })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(workspaceDelete).not.toHaveBeenCalled();

    const { result: applyResult } = renderHook(() => useWorkspaceApplyMutation(), { wrapper });
    await expect(
      applyResult.current.mutateAsync({ cliKey: "claude", workspaceId: -1 })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(workspaceApply).not.toHaveBeenCalled();
  });

  it("pickWorkspaceById returns the matching workspace or null", () => {
    const items: WorkspaceSummary[] = [
      { id: 1, cli_key: "claude", name: "W1", created_at: 0, updated_at: 0 },
      { id: 2, cli_key: "claude", name: "W2", created_at: 0, updated_at: 0 },
    ];

    expect(pickWorkspaceById(items, 2)?.name).toBe("W2");
    expect(pickWorkspaceById(items, null)).toBeNull();
    expect(pickWorkspaceById(items, 999)).toBeNull();
  });

  it("isWorkspacePreviewReady checks workspaceId match", () => {
    const preview: WorkspacePreview = {
      cli_key: "claude",
      from_workspace_id: 1,
      to_workspace_id: 2,
      prompts: { from_enabled: null, to_enabled: null, will_change: false },
      mcp: { from_enabled: [], to_enabled: [], added: [], removed: [] },
      skills: { from_enabled: [], to_enabled: [], added: [], removed: [] },
    };

    expect(isWorkspacePreviewReady(null, 2)).toBe(false);
    expect(isWorkspacePreviewReady(preview, null)).toBe(false);
    expect(isWorkspacePreviewReady(preview, 1)).toBe(false);
    expect(isWorkspacePreviewReady(preview, 2)).toBe(true);
  });
});
