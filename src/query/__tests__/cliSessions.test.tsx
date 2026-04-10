import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, createQueryWrapper } from "../../test/utils/reactQuery";
import { setTauriRuntime, clearTauriRuntime } from "../../test/utils/tauriRuntime";

vi.mock("../../services/cli/cliSessions", () => ({
  cliSessionsProjectsList: vi.fn(),
  cliSessionsSessionsList: vi.fn(),
  cliSessionsMessagesGet: vi.fn(),
}));

import {
  useCliSessionsProjectsListQuery,
  useCliSessionsSessionsListQuery,
  useCliSessionsMessagesInfiniteQuery,
} from "../cliSessions";
import {
  cliSessionsMessagesGet,
  cliSessionsProjectsList,
  cliSessionsSessionsList,
} from "../../services/cli/cliSessions";

describe("query/cliSessions", () => {
  beforeEach(() => {
    vi.mocked(cliSessionsProjectsList).mockResolvedValue([]);
    vi.mocked(cliSessionsSessionsList).mockResolvedValue([]);
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

  it("useCliSessionsSessionsListQuery disabled when empty projectId", () => {
    setTauriRuntime();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useCliSessionsSessionsListQuery("claude", ""), { wrapper });
    // Should not fetch with empty projectId
    expect(result.current.fetchStatus).toBe("idle");
    clearTauriRuntime();
  });
});
