import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, createQueryWrapper } from "../../test/utils/reactQuery";
import { setTauriRuntime, clearTauriRuntime } from "../../test/utils/tauriRuntime";

vi.mock("../../services/cli/cliSessions", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/cliSessions")>(
    "../../services/cli/cliSessions"
  );
  return {
    ...actual,
    cliSessionsProjectsList: vi.fn(),
    cliSessionsSessionsList: vi.fn(),
    cliSessionsSessionDelete: vi.fn(),
    cliSessionsMessagesGet: vi.fn(),
    cliSessionsFolderLookupByIds: vi.fn(),
  };
});

import {
  useCliSessionsFolderLookupByIdsQuery,
  useCliSessionsProjectsListQuery,
  useCliSessionsSessionsListQuery,
  useCliSessionsSessionDeleteMutation,
  useCliSessionsMessagesInfiniteQuery,
} from "../cliSessions";
import {
  CLI_SESSIONS_MAX_PATH_CHARS,
  cliSessionsFolderLookupByIds,
  cliSessionsMessagesGet,
  cliSessionsProjectsList,
  cliSessionsSessionDelete,
  cliSessionsSessionsList,
} from "../../services/cli/cliSessions";
import { cliSessionsKeys } from "../keys";

describe("query/cliSessions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([]);
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([]);
    vi.mocked(cliSessionsSessionDelete).mockResolvedValue([]);
    vi.mocked(cliSessionsFolderLookupByIds).mockResolvedValue([]);
    vi.mocked(cliSessionsMessagesGet).mockResolvedValue({
      messages: [],
      total: 0,
      page: 0,
      page_size: 50,
      has_more: false,
    });
  });

  it("useCliSessionsProjectsListQuery renders", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useCliSessionsProjectsListQuery("claude"), { wrapper });
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    clearTauriRuntime();
  });

  it("useCliSessionsSessionsListQuery renders", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useCliSessionsSessionsListQuery("claude", "proj-1"), {
      wrapper,
    });
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    clearTauriRuntime();
  });

  it("useCliSessionsSessionsListQuery trims projectId for fetch and cache key", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useCliSessionsSessionsListQuery("claude", "  proj-1  "), {
      wrapper,
    });
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(cliSessionsSessionsList).toHaveBeenCalledWith("claude", "proj-1", undefined);
    expect(
      client.getQueryState(cliSessionsKeys.sessionsList("claude", "proj-1", undefined))
    ).toBeTruthy();
    expect(
      client.getQueryState(cliSessionsKeys.sessionsList("claude", "  proj-1  ", undefined))
    ).toBeUndefined();
    clearTauriRuntime();
  });

  it("normalizes wslDistro for fetches and cache keys", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(
      () => useCliSessionsSessionsListQuery("claude", "proj-1", { wslDistro: "  Ubuntu  " }),
      { wrapper }
    );
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(cliSessionsSessionsList).toHaveBeenCalledWith("claude", "proj-1", "Ubuntu");
    expect(
      client.getQueryState(cliSessionsKeys.sessionsList("claude", "proj-1", "Ubuntu"))
    ).toBeTruthy();
    expect(
      client.getQueryState(cliSessionsKeys.sessionsList("claude", "proj-1", "  Ubuntu  "))
    ).toBeUndefined();
    clearTauriRuntime();
  });

  it("useCliSessionsMessagesInfiniteQuery renders", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(
      () => useCliSessionsMessagesInfiniteQuery("claude", "/path/to/file.json"),
      { wrapper }
    );
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    clearTauriRuntime();
  });

  it("useCliSessionsMessagesInfiniteQuery trims filePath for fetch and cache key", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(
      () => useCliSessionsMessagesInfiniteQuery("claude", "  /path/to/file.json  "),
      { wrapper }
    );
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(cliSessionsMessagesGet).toHaveBeenCalledWith(
      expect.objectContaining({ filePath: "/path/to/file.json", page: 0, pageSize: 50 })
    );
    expect(
      client.getQueryState(
        cliSessionsKeys.messages("claude", "/path/to/file.json", true, undefined)
      )
    ).toBeTruthy();
    expect(
      client.getQueryState(
        cliSessionsKeys.messages("claude", "  /path/to/file.json  ", true, undefined)
      )
    ).toBeUndefined();
    clearTauriRuntime();
  });

  it("useCliSessionsFolderLookupByIdsQuery filters empty session ids for fetch and cache key", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(
      () =>
        useCliSessionsFolderLookupByIdsQuery([
          { source: "claude", session_id: " s1 " },
          { source: "codex", session_id: "   " },
        ]),
      { wrapper }
    );
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(cliSessionsFolderLookupByIds).toHaveBeenCalledWith(
      [{ source: "claude", session_id: "s1" }],
      undefined
    );
    expect(
      client.getQueryState(cliSessionsKeys.folderLookup(["claude:s1"], undefined))
    ).toBeTruthy();
    expect(
      client.getQueryState(cliSessionsKeys.folderLookup(["claude: s1 ", "codex:   "], undefined))
    ).toBeUndefined();
    clearTauriRuntime();
  });

  it("useCliSessionsSessionsListQuery disabled when empty projectId", () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useCliSessionsSessionsListQuery("claude", ""), { wrapper });
    // Should not fetch with empty projectId
    expect(result.current.fetchStatus).toBe("idle");
    clearTauriRuntime();
  });

  it("rejects oversized query-key text before service fetches", () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(
        () =>
          useCliSessionsSessionsListQuery("claude", "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1)),
        { wrapper }
      )
    ).toThrow("projectId is too long");
    expect(() =>
      renderHook(
        () =>
          useCliSessionsMessagesInfiniteQuery(
            "claude",
            "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1)
          ),
        { wrapper }
      )
    ).toThrow("filePath is too long");
    expect(() =>
      renderHook(
        () =>
          useCliSessionsFolderLookupByIdsQuery([
            { source: "claude", session_id: "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1) },
          ]),
        { wrapper }
      )
    ).toThrow("sessionId is too long");

    expect(cliSessionsSessionsList).not.toHaveBeenCalled();
    expect(cliSessionsMessagesGet).not.toHaveBeenCalled();
    expect(cliSessionsFolderLookupByIds).not.toHaveBeenCalled();
    clearTauriRuntime();
  });

  it("delete mutation normalizes cache keys and deleted paths", async () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const key = cliSessionsKeys.sessionsList("claude", "proj-1", "Ubuntu");
    client.setQueryData(key, [
      { source: "claude", file_path: "/f.json" },
      { source: "claude", file_path: "/keep.json" },
    ]);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const { result } = renderHook(() => useCliSessionsSessionDeleteMutation(), { wrapper });

    await result.current.mutateAsync({
      source: "claude",
      filePaths: [" /f.json "],
      projectId: "  proj-1  ",
      wslDistro: " Ubuntu ",
    });

    expect(cliSessionsSessionDelete).toHaveBeenCalledWith({
      source: "claude",
      filePaths: ["/f.json"],
      wslDistro: "Ubuntu",
    });
    expect(client.getQueryData<Array<{ file_path: string }>>(key)).toEqual([
      { source: "claude", file_path: "/keep.json" },
    ]);
    expect(
      client.getQueryState(cliSessionsKeys.sessionsList("claude", "  proj-1  ", " Ubuntu "))
    ).toBeUndefined();
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: cliSessionsKeys.sessionsList("claude", "proj-1", "Ubuntu"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: cliSessionsKeys.projectsList("claude", "Ubuntu"),
    });
    clearTauriRuntime();
  });
});
