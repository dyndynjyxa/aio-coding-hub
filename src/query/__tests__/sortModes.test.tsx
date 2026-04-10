import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SortModeActiveRow } from "../../services/providers/sortModes";
import {
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProviderSetEnabled,
  sortModeProvidersList,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
} from "../../services/providers/sortModes";
import { createDeferred } from "../../test/utils/deferred";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { sortModesKeys } from "../keys";
import {
  sortModeProvidersQueryKey,
  useSortModeActiveListQuery,
  useSortModeActiveSetMutation,
  useSortModeCreateMutation,
  useSortModeDeleteMutation,
  useSortModeProviderSetEnabledMutation,
  useSortModeProvidersListQuery,
  useSortModeProvidersSetOrderMutation,
  useSortModeRenameMutation,
  useSortModesListQuery,
} from "../sortModes";

vi.mock("../../services/providers/sortModes", async () => {
  const actual = await vi.importActual<typeof import("../../services/providers/sortModes")>(
    "../../services/providers/sortModes"
  );
  return {
    ...actual,
    sortModesList: vi.fn(),
    sortModeActiveList: vi.fn(),
    sortModeActiveSet: vi.fn(),
    sortModeCreate: vi.fn(),
    sortModeRename: vi.fn(),
    sortModeDelete: vi.fn(),
    sortModeProvidersList: vi.fn(),
    sortModeProvidersSetOrder: vi.fn(),
    sortModeProviderSetEnabled: vi.fn(),
  };
});

describe("query/sortModes", () => {
  it("calls sortModesList and sortModeActiveList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(sortModesList).mockResolvedValue([]);
    vi.mocked(sortModeActiveList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSortModesListQuery(), { wrapper });
    renderHook(() => useSortModeActiveListQuery(), { wrapper });

    await waitFor(() => {
      expect(sortModesList).toHaveBeenCalled();
      expect(sortModeActiveList).toHaveBeenCalled();
    });
  });

  it("calls sortModeProvidersList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProvidersList).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSortModeProvidersListQuery({ modeId: 1, cliKey: "claude" }), { wrapper });

    await waitFor(() => {
      expect(sortModeProvidersList).toHaveBeenCalledWith({ mode_id: 1, cli_key: "claude" });
    });
  });

  it("useSortModesListQuery enters error state when sortModesList rejects", async () => {
    setTauriRuntime();

    vi.mocked(sortModesList).mockRejectedValue(new Error("sort modes query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModesListQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useSortModeActiveSetMutation optimistically updates activeList and invalidates on settle", async () => {
    setTauriRuntime();

    const previous: SortModeActiveRow[] = [
      { cli_key: "claude", mode_id: 1, updated_at: 0 },
      { cli_key: "gemini", mode_id: null, updated_at: 0 },
    ];
    const updated: SortModeActiveRow = { cli_key: "claude", mode_id: 2, updated_at: 123 };

    const deferred = createDeferred<SortModeActiveRow>();
    vi.mocked(sortModeActiveSet).mockImplementation(() => deferred.promise);

    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });

    act(() => {
      result.current.mutate({ cliKey: "claude", modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual([
      { ...previous[0], mode_id: 2 },
      previous[1],
    ]);

    deferred.resolve(updated);

    await act(async () => {
      await deferred.promise;
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual([updated, previous[1]]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("rolls back when sortModeActiveSet returns null", async () => {
    setTauriRuntime();

    const previous: SortModeActiveRow[] = [{ cli_key: "claude", mode_id: 1, updated_at: 0 }];

    vi.mocked(sortModeActiveSet).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual(previous);
  });

  it("invalidates even when service returns null and cache is missing", async () => {
    setTauriRuntime();

    vi.mocked(sortModeActiveSet).mockResolvedValue(null);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toBeUndefined();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("rolls back when sortModeActiveSet throws", async () => {
    setTauriRuntime();

    const previous: SortModeActiveRow[] = [{ cli_key: "claude", mode_id: 1, updated_at: 0 }];

    vi.mocked(sortModeActiveSet).mockRejectedValue(new Error("boom"));

    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      try {
        await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
      } catch {
        // expected
      }
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual(previous);
  });

  it("invalidates without updating cache when activeList is missing", async () => {
    setTauriRuntime();

    const updated: SortModeActiveRow = { cli_key: "claude", mode_id: 2, updated_at: 123 };
    vi.mocked(sortModeActiveSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toBeUndefined();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeCreateMutation invalidates list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeCreate).mockResolvedValue({
      id: 1,
      name: "Work",
      created_at: 0,
      updated_at: 0,
    } as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeCreateMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ name: "Work" });
    });

    expect(sortModeCreate).toHaveBeenCalledWith({ name: "Work" });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeRenameMutation invalidates list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeRename).mockResolvedValue({
      id: 2,
      name: "Life",
      created_at: 0,
      updated_at: 0,
    } as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeRenameMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ modeId: 2, name: "Life" });
    });

    expect(sortModeRename).toHaveBeenCalledWith({ mode_id: 2, name: "Life" });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeDeleteMutation invalidates list and activeList on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeDelete).mockResolvedValue(true as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ modeId: 3 });
    });

    expect(sortModeDelete).toHaveBeenCalledWith({ mode_id: 3 });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeProvidersSetOrderMutation invalidates the provider list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([
      { provider_id: 101, enabled: true },
    ] as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeProvidersSetOrderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        modeId: 3,
        cliKey: "codex",
        orderedProviderIds: [101],
      });
    });

    expect(sortModeProvidersSetOrder).toHaveBeenCalledWith({
      mode_id: 3,
      cli_key: "codex",
      ordered_provider_ids: [101],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: sortModeProvidersQueryKey(3, "codex"),
    });
  });

  it("useSortModeProviderSetEnabledMutation invalidates the provider list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProviderSetEnabled).mockResolvedValue({
      provider_id: 101,
      enabled: false,
    } as any);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeProviderSetEnabledMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        modeId: 4,
        cliKey: "gemini",
        providerId: 101,
        enabled: false,
      });
    });

    expect(sortModeProviderSetEnabled).toHaveBeenCalledWith({
      mode_id: 4,
      cli_key: "gemini",
      provider_id: 101,
      enabled: false,
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: sortModeProvidersQueryKey(4, "gemini"),
    });
  });
});
