import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT,
  REQUEST_ATTEMPT_LOGS_MAX_LIMIT,
  REQUEST_LOGS_DEFAULT_LIMIT,
  REQUEST_LOGS_MAX_LIMIT,
  REQUEST_LOG_TRACE_ID_MAX_LENGTH,
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAfterIdAll,
  requestLogsListAll,
  type RequestLogSummary,
} from "../../services/gateway/requestLogs";
import {
  createRequestLogDetail,
  createRequestLogSummary as createRequestLogSummaryFixture,
} from "../../services/gateway/requestLogFixtures";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { requestLogsKeys } from "../keys";
import {
  REQUEST_LOG_DETAIL_GC_TIME_MS,
  REQUEST_LOG_DETAIL_STALE_TIME_MS,
  useRequestAttemptLogsByTraceIdQuery,
  useRequestLogDetailQuery,
  useRequestLogsIncrementalRefreshMutation,
  useRequestLogsListAllQuery,
} from "../requestLogs";

vi.mock("../../services/gateway/requestLogs", async () => {
  const actual = await vi.importActual<typeof import("../../services/gateway/requestLogs")>(
    "../../services/gateway/requestLogs"
  );
  return {
    ...actual,
    requestLogsListAll: vi.fn(),
    requestLogsListAfterIdAll: vi.fn(),
    requestLogGet: vi.fn(),
    requestAttemptLogsByTraceId: vi.fn(),
  };
});

function makeRequestLogSummary(
  overrides: Parameters<typeof createRequestLogSummaryFixture>[0] = {}
): RequestLogSummary {
  const hasTimestampOverride = "created_at" in overrides || "created_at_ms" in overrides;
  return createRequestLogSummaryFixture({
    ...(hasTimestampOverride ? {} : { created_at_ms: 10_000, created_at: 10 }),
    ...overrides,
  });
}

describe("query/requestLogs", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("calls requestLogsListAll with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestLogsListAllQuery(10), { wrapper });

    await waitFor(() => {
      expect(requestLogsListAll).toHaveBeenCalledWith(10);
    });
  });

  it("normalizes request log list query limit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsListAllQuery(999), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(requestLogsListAll).toHaveBeenCalledWith(REQUEST_LOGS_MAX_LIMIT);
    expect(client.getQueryState(requestLogsKeys.listAll(REQUEST_LOGS_MAX_LIMIT))).toBeTruthy();
    expect(client.getQueryState(requestLogsKeys.listAll(999))).toBeUndefined();
  });

  it("uses the backend default request log list limit when omitted", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestLogsListAllQuery(), { wrapper });

    await waitFor(() => {
      expect(requestLogsListAll).toHaveBeenCalledWith(REQUEST_LOGS_DEFAULT_LIMIT);
    });
    expect(client.getQueryState(requestLogsKeys.listAll(REQUEST_LOGS_DEFAULT_LIMIT))).toBeTruthy();
  });

  it("sorts rows from the backend list-all query", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValue([
      makeRequestLogSummary({ id: 1, path: "/v1/messages", created_at: 10 }),
      makeRequestLogSummary({ id: 2, path: "/v1/messages", created_at: 11 }),
    ]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsListAllQuery(10), { wrapper });

    await waitFor(() => {
      expect(result.current.data?.map((row) => row.id)).toEqual([2, 1]);
    });
  });

  it("caps list-all query rows defensively when backend returns more than requested", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValue([
      makeRequestLogSummary({ id: 1, created_at: 10 }),
      makeRequestLogSummary({ id: 2, created_at: 12 }),
      makeRequestLogSummary({ id: 3, created_at: 11 }),
    ]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsListAllQuery(2), { wrapper });

    await waitFor(() => {
      expect(result.current.data?.map((row) => row.id)).toEqual([2, 3]);
    });
    expect(
      (client.getQueryData<RequestLogSummary[]>(requestLogsKeys.listAll(2)) ?? []).map(
        (row) => row.id
      )
    ).toEqual([2, 3]);
  });

  it("useRequestLogsListAllQuery enters error state when requestLogsListAll rejects", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockRejectedValue(new Error("request logs query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsListAllQuery(10), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestLogsListAllQuery(10, { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(requestLogsListAll).not.toHaveBeenCalled();
  });

  it("does not call requestLogGet when logId is null (even on manual refetch)", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogDetailQuery(null), { wrapper });
    await act(async () => {
      const res = await result.current.refetch();
      expect(res.data).toBeNull();
    });

    expect(requestLogGet).not.toHaveBeenCalled();
  });

  it("calls requestLogGet when logId is provided", async () => {
    setTauriRuntime();

    vi.mocked(requestLogGet).mockResolvedValue(createRequestLogDetail());

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestLogDetailQuery(1), { wrapper });

    await waitFor(() => {
      expect(requestLogGet).toHaveBeenCalledWith(1);
    });
  });

  it("does not call requestAttemptLogsByTraceId when traceId is null (even on manual refetch)", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestAttemptLogsByTraceIdQuery(null, 10), { wrapper });
    await act(async () => {
      const res = await result.current.refetch();
      expect(res.data).toBeNull();
    });

    expect(requestAttemptLogsByTraceId).not.toHaveBeenCalled();
  });

  it("calls requestAttemptLogsByTraceId when traceId is provided", async () => {
    setTauriRuntime();

    vi.mocked(requestAttemptLogsByTraceId).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestAttemptLogsByTraceIdQuery("trace-1", 10), { wrapper });

    await waitFor(() => {
      expect(requestAttemptLogsByTraceId).toHaveBeenCalledWith("trace-1", 10);
    });
  });

  it("normalizes trace id before request attempt fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(requestAttemptLogsByTraceId).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestAttemptLogsByTraceIdQuery(" trace-1 ", 10), { wrapper });

    await waitFor(() => {
      expect(requestAttemptLogsByTraceId).toHaveBeenCalledWith("trace-1", 10);
    });
    expect(client.getQueryState(requestLogsKeys.attemptsByTrace("trace-1", 10))).toBeTruthy();
    expect(client.getQueryState(requestLogsKeys.attemptsByTrace(" trace-1 ", 10))).toBeUndefined();
  });

  it("does not retain invalid trace id in request attempt query keys", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const tooLongTraceId = "t".repeat(REQUEST_LOG_TRACE_ID_MAX_LENGTH + 1);

    renderHook(() => useRequestAttemptLogsByTraceIdQuery(tooLongTraceId, 10), { wrapper });
    await Promise.resolve();

    expect(requestAttemptLogsByTraceId).not.toHaveBeenCalled();
    expect(
      client.getQueryState(requestLogsKeys.attemptsByTrace(tooLongTraceId, 10))
    ).toBeUndefined();
    expect(client.getQueryState(requestLogsKeys.attemptsByTrace(null, 10))).toBeTruthy();
  });

  it("normalizes request attempt log query limit for fetch and cache key", async () => {
    setTauriRuntime();

    vi.mocked(requestAttemptLogsByTraceId).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestAttemptLogsByTraceIdQuery("trace-1", 999), {
      wrapper,
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(requestAttemptLogsByTraceId).toHaveBeenCalledWith(
      "trace-1",
      REQUEST_ATTEMPT_LOGS_MAX_LIMIT
    );
    expect(
      client.getQueryState(
        requestLogsKeys.attemptsByTrace("trace-1", REQUEST_ATTEMPT_LOGS_MAX_LIMIT)
      )
    ).toBeTruthy();
    expect(client.getQueryState(requestLogsKeys.attemptsByTrace("trace-1", 999))).toBeUndefined();
  });

  it("uses the backend default request attempt log limit when omitted", async () => {
    setTauriRuntime();

    vi.mocked(requestAttemptLogsByTraceId).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useRequestAttemptLogsByTraceIdQuery("trace-1"), { wrapper });

    await waitFor(() => {
      expect(requestAttemptLogsByTraceId).toHaveBeenCalledWith(
        "trace-1",
        REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT
      );
    });
    expect(
      client.getQueryState(
        requestLogsKeys.attemptsByTrace("trace-1", REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT)
      )
    ).toBeTruthy();
  });

  it("uses short-lived cache options for heavy detail and attempt queries", async () => {
    setTauriRuntime();

    vi.mocked(requestLogGet).mockResolvedValue(createRequestLogDetail());
    vi.mocked(requestAttemptLogsByTraceId).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () => ({
        detail: useRequestLogDetailQuery(1),
        attempts: useRequestAttemptLogsByTraceIdQuery("trace-1", 10),
      }),
      { wrapper }
    );

    await waitFor(() => {
      expect(requestLogGet).toHaveBeenCalledWith(1);
      expect(requestAttemptLogsByTraceId).toHaveBeenCalledWith("trace-1", 10);
    });

    const detailQuery = client.getQueryCache().find({ queryKey: requestLogsKeys.detail(1) });
    const attemptsQuery = client
      .getQueryCache()
      .find({ queryKey: requestLogsKeys.attemptsByTrace("trace-1", 10) });
    const detailOptions = detailQuery?.options as
      | { staleTime?: unknown; gcTime?: unknown }
      | undefined;
    const attemptsOptions = attemptsQuery?.options as
      | { staleTime?: unknown; gcTime?: unknown }
      | undefined;
    expect(detailOptions?.staleTime).toBe(REQUEST_LOG_DETAIL_STALE_TIME_MS);
    expect(detailOptions?.gcTime).toBe(REQUEST_LOG_DETAIL_GC_TIME_MS);
    expect(attemptsOptions?.staleTime).toBe(REQUEST_LOG_DETAIL_STALE_TIME_MS);
    expect(attemptsOptions?.gcTime).toBe(REQUEST_LOG_DETAIL_GC_TIME_MS);
  });

  it("incremental refresh mutation keeps backend rows and cache stable on empty incremental items", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const listKey = requestLogsKeys.listAll(10);

    vi.mocked(requestLogsListAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 1, created_at: 9, created_at_ms: null }),
      makeRequestLogSummary({ id: 2, created_at: 10, created_at_ms: null }),
    ]);
    const { result } = renderHook(() => useRequestLogsIncrementalRefreshMutation(10), { wrapper });

    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("full");
      expect(res?.items?.map((row) => row.id)).toEqual([2, 1]);
    });
    expect((client.getQueryData<RequestLogSummary[]>(listKey) ?? []).map((row) => row.id)).toEqual([
      2, 1,
    ]);

    client.setQueryData(listKey, [makeRequestLogSummary({ id: 5, created_at: 10 })]);
    vi.mocked(requestLogsListAfterIdAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 6, created_at: 11 }),
      makeRequestLogSummary({ id: 7, created_at: 12 }),
    ]);
    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("incremental");
      expect(res?.items?.map((row) => row.id)).toEqual([7, 6]);
    });
    expect(
      (client.getQueryData<RequestLogSummary[]>(listKey) ?? []).some((row) => row.id === 6)
    ).toBe(true);
    expect(
      (client.getQueryData<RequestLogSummary[]>(listKey) ?? []).some((row) => row.id === 7)
    ).toBe(true);

    const nowSec2 = Math.floor(Date.now() / 1000);
    client.setQueryData(listKey, [
      makeRequestLogSummary({ id: 8, status: null, error_code: null, created_at: nowSec2 }),
    ]);
    vi.mocked(requestLogsListAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 8, status: 200, error_code: null, created_at: nowSec2 }),
    ]);
    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("full");
      expect(res?.items?.map((row) => row.id)).toEqual([8]);
    });
    expect((client.getQueryData<RequestLogSummary[]>(listKey) ?? [])[0]?.status).toBe(200);

    vi.mocked(requestLogsListAfterIdAll).mockResolvedValueOnce([]);
    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("incremental");
      expect(res?.items).toEqual([]);
    });
    expect(
      (client.getQueryData<RequestLogSummary[]>(listKey) ?? []).some((row) => row.id === 8)
    ).toBe(true);
  });

  it("caps full refresh mutation results before returning and caching them", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const listKey = requestLogsKeys.listAll(2);

    vi.mocked(requestLogsListAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 1, created_at: 10 }),
      makeRequestLogSummary({ id: 2, created_at: 12 }),
      makeRequestLogSummary({ id: 3, created_at: 11 }),
    ]);

    const { result } = renderHook(() => useRequestLogsIncrementalRefreshMutation(2), { wrapper });

    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("full");
      expect(res?.items.map((row) => row.id)).toEqual([2, 3]);
    });
    expect((client.getQueryData<RequestLogSummary[]>(listKey) ?? []).map((row) => row.id)).toEqual([
      2, 3,
    ]);
  });

  it("caps incremental refresh mutation results before returning and merging them", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const listKey = requestLogsKeys.listAll(2);
    client.setQueryData(listKey, [makeRequestLogSummary({ id: 1, created_at: 10 })]);

    vi.mocked(requestLogsListAfterIdAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 2, created_at: 12 }),
      makeRequestLogSummary({ id: 3, created_at: 13 }),
      makeRequestLogSummary({ id: 4, created_at: 11 }),
    ]);

    const { result } = renderHook(() => useRequestLogsIncrementalRefreshMutation(2), { wrapper });

    await act(async () => {
      const res = await result.current.mutateAsync();
      expect(res?.mode).toBe("incremental");
      expect(res?.items.map((row) => row.id)).toEqual([3, 2]);
    });
    expect((client.getQueryData<RequestLogSummary[]>(listKey) ?? []).map((row) => row.id)).toEqual([
      3, 2,
    ]);
  });

  it("normalizes incremental refresh mutation limit for fetch and cache writes", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsListAll).mockResolvedValueOnce([
      makeRequestLogSummary({ id: 1, created_at: 10 }),
    ]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsIncrementalRefreshMutation(999), {
      wrapper,
    });

    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(requestLogsListAll).toHaveBeenCalledWith(REQUEST_LOGS_MAX_LIMIT);
    expect(
      (
        client.getQueryData<RequestLogSummary[]>(requestLogsKeys.listAll(REQUEST_LOGS_MAX_LIMIT)) ??
        []
      ).map((row) => row.id)
    ).toEqual([1]);
    expect(client.getQueryData(requestLogsKeys.listAll(999))).toBeUndefined();
  });
});
