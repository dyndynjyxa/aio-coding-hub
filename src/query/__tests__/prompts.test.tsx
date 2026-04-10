import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PromptSummary } from "../../services/workspace/prompts";
import {
  promptDelete,
  promptSetEnabled,
  promptUpsert,
  promptsList,
} from "../../services/workspace/prompts";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { promptsKeys } from "../keys";
import {
  usePromptDeleteMutation,
  usePromptSetEnabledMutation,
  usePromptUpsertMutation,
  usePromptsListQuery,
} from "../prompts";

vi.mock("../../services/workspace/prompts", async () => {
  const actual = await vi.importActual<typeof import("../../services/workspace/prompts")>(
    "../../services/workspace/prompts"
  );
  return {
    ...actual,
    promptsList: vi.fn(),
    promptUpsert: vi.fn(),
    promptSetEnabled: vi.fn(),
    promptDelete: vi.fn(),
  };
});

describe("query/prompts", () => {
  it("calls promptsList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(promptsList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => usePromptsListQuery(1), { wrapper });

    await waitFor(() => {
      expect(promptsList).toHaveBeenCalledWith(1);
    });
  });

  it("usePromptsListQuery enters error state when promptsList rejects", async () => {
    setTauriRuntime();

    vi.mocked(promptsList).mockRejectedValue(new Error("prompts query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => usePromptsListQuery(1), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("usePromptUpsertMutation inserts/updates cached list and invalidates", async () => {
    setTauriRuntime();

    const created: PromptSummary = {
      id: 10,
      workspace_id: 1,
      cli_key: "claude",
      name: "P1",
      content: "hi",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };

    vi.mocked(promptUpsert).mockResolvedValue(created);

    const client = createTestQueryClient();
    client.setQueryData(promptsKeys.list(1), []);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => usePromptUpsertMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        promptId: null,
        name: "P1",
        content: "hi",
        enabled: true,
      });
    });

    expect(client.getQueryData(promptsKeys.list(1))).toEqual([created]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: promptsKeys.list(1) });
  });

  it("usePromptSetEnabledMutation enforces at-most-one enabled in cached list", async () => {
    setTauriRuntime();

    const prev: PromptSummary[] = [
      {
        id: 1,
        workspace_id: 1,
        cli_key: "claude",
        name: "A",
        content: "a",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
      {
        id: 2,
        workspace_id: 1,
        cli_key: "claude",
        name: "B",
        content: "b",
        enabled: false,
        created_at: 0,
        updated_at: 0,
      },
    ];
    const updated: PromptSummary = { ...prev[1], enabled: true, updated_at: 1 };

    vi.mocked(promptSetEnabled).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(promptsKeys.list(1), prev);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => usePromptSetEnabledMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ promptId: 2, enabled: true });
    });

    expect(client.getQueryData(promptsKeys.list(1))).toEqual([
      { ...prev[0], enabled: false },
      updated,
    ]);
  });

  it("usePromptDeleteMutation removes item from cached list", async () => {
    setTauriRuntime();

    vi.mocked(promptDelete).mockResolvedValue(true);

    const prev: PromptSummary[] = [
      {
        id: 1,
        workspace_id: 1,
        cli_key: "claude",
        name: "A",
        content: "a",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
      {
        id: 2,
        workspace_id: 1,
        cli_key: "claude",
        name: "B",
        content: "b",
        enabled: false,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(promptsKeys.list(1), prev);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => usePromptDeleteMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(1);
    });

    expect(client.getQueryData(promptsKeys.list(1))).toEqual([prev[1]]);
  });
});
