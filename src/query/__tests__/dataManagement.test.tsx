import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { dbDiskUsageGet, requestLogsClearAll } from "../../services/app/dataManagement";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import {
  appAboutKeys,
  dataManagementKeys,
  gatewayKeys,
  modelPricesKeys,
  providersKeys,
  requestLogsKeys,
  settingsKeys,
  usageKeys,
} from "../keys";
import {
  APP_DATA_RESET_STOPPED_GATEWAY_STATUS,
  formatDbDiskUsageAvailable,
  isClearRequestLogsResult,
  resetAppDataQueryCaches,
  useDbDiskUsageQuery,
  useRequestLogsClearAllMutation,
} from "../dataManagement";

vi.mock("../../services/app/dataManagement", async () => {
  const actual = await vi.importActual<typeof import("../../services/app/dataManagement")>(
    "../../services/app/dataManagement"
  );
  return { ...actual, dbDiskUsageGet: vi.fn(), requestLogsClearAll: vi.fn() };
});

describe("query/dataManagement", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls dbDiskUsageGet with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(dbDiskUsageGet).mockResolvedValue({
      db_bytes: 1,
      wal_bytes: 2,
      shm_bytes: 3,
      total_bytes: 6,
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useDbDiskUsageQuery(), { wrapper });

    await waitFor(() => {
      expect(dbDiskUsageGet).toHaveBeenCalled();
    });
  });

  it("useDbDiskUsageQuery enters error state when dbDiskUsageGet rejects", async () => {
    setTauriRuntime();

    vi.mocked(dbDiskUsageGet).mockRejectedValue(new Error("db usage query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useDbDiskUsageQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useRequestLogsClearAllMutation invalidates dbDiskUsage + requestLogs", async () => {
    setTauriRuntime();

    vi.mocked(requestLogsClearAll).mockResolvedValue({
      request_logs_deleted: 1,
      request_attempt_logs_deleted: 2,
    });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useRequestLogsClearAllMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: dataManagementKeys.dbDiskUsage() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: requestLogsKeys.all });
  });

  it("resetAppDataQueryCaches removes reset-owned caches without invalidating", async () => {
    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    client.setQueryData(gatewayKeys.status(), {
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    });
    client.setQueryData(gatewayKeys.sessions(), [{ session_id: "session-1" }]);
    client.setQueryData(providersKeys.list("codex"), [{ id: 1 }]);
    client.setQueryData(requestLogsKeys.listAll(null), [{ id: 1 }]);
    client.setQueryData(usageKeys.summary("today", { cliKey: null }), { requests_total: 3 });
    client.setQueryData(modelPricesKeys.aliases(), { aliases: [] });
    client.setQueryData(settingsKeys.get(), { preferred_port: 37123 });
    client.setQueryData(dataManagementKeys.dbDiskUsage(), { total_bytes: 1024 });
    client.setQueryData(appAboutKeys.get(), { version: "keep" });

    await resetAppDataQueryCaches(client);

    expect(invalidateSpy).not.toHaveBeenCalled();
    expect(client.getQueryData(gatewayKeys.status())).toEqual(
      APP_DATA_RESET_STOPPED_GATEWAY_STATUS
    );
    expect(client.getQueryData(gatewayKeys.sessions())).toBeUndefined();
    expect(client.getQueryData(providersKeys.list("codex"))).toBeUndefined();
    expect(client.getQueryData(requestLogsKeys.listAll(null))).toBeUndefined();
    expect(client.getQueryData(usageKeys.summary("today", { cliKey: null }))).toBeUndefined();
    expect(client.getQueryData(modelPricesKeys.aliases())).toBeUndefined();
    expect(client.getQueryData(settingsKeys.get())).toBeUndefined();
    expect(client.getQueryData(dataManagementKeys.dbDiskUsage())).toBeUndefined();
    expect(client.getQueryData(appAboutKeys.get())).toEqual({ version: "keep" });
  });

  it("resetAppDataQueryCaches overwrites active db usage without refetching", async () => {
    setTauriRuntime();

    vi.mocked(dbDiskUsageGet).mockResolvedValue({
      db_bytes: 1,
      wal_bytes: 2,
      shm_bytes: 3,
      total_bytes: 6,
    });

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useDbDiskUsageQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.data?.total_bytes).toBe(6);
    });

    await act(async () => {
      await resetAppDataQueryCaches(client);
    });

    await waitFor(() => {
      expect(result.current.data).toEqual({
        db_bytes: 0,
        wal_bytes: 0,
        shm_bytes: 0,
        total_bytes: 0,
      });
    });
    expect(dbDiskUsageGet).toHaveBeenCalledTimes(1);
    expect(invalidateSpy).not.toHaveBeenCalled();
  });

  it("isClearRequestLogsResult validates result shape", () => {
    expect(isClearRequestLogsResult(null)).toBe(false);
    expect(isClearRequestLogsResult({} as any)).toBe(false);
    expect(
      isClearRequestLogsResult({ request_logs_deleted: -1, request_attempt_logs_deleted: 2 } as any)
    ).toBe(false);
    expect(
      isClearRequestLogsResult({
        request_logs_deleted: 1,
        request_attempt_logs_deleted: Number.NaN,
      } as any)
    ).toBe(false);
    expect(
      isClearRequestLogsResult({ request_logs_deleted: 1, request_attempt_logs_deleted: 2 })
    ).toBe(true);
  });

  it("formatDbDiskUsageAvailable returns total_bytes or null", () => {
    expect(formatDbDiskUsageAvailable(null)).toBeNull();
    expect(formatDbDiskUsageAvailable(undefined)).toBeNull();
    expect(formatDbDiskUsageAvailable({ total_bytes: 10 } as any)).toBe(10);
    expect(formatDbDiskUsageAvailable({ total_bytes: -1 } as any)).toBeNull();
    expect(
      formatDbDiskUsageAvailable({ total_bytes: Number.MAX_SAFE_INTEGER + 1 } as any)
    ).toBeNull();
  });
});
