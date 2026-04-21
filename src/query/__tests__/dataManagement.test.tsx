import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { dbDiskUsageGet, requestLogsClearAll } from "../../services/app/dataManagement";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { dataManagementKeys, requestLogsKeys } from "../keys";
import {
  formatDbDiskUsageAvailable,
  isClearRequestLogsResult,
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

  it("isClearRequestLogsResult validates result shape", () => {
    expect(isClearRequestLogsResult(null)).toBe(false);
    expect(isClearRequestLogsResult({} as any)).toBe(false);
    expect(
      isClearRequestLogsResult({ request_logs_deleted: 1, request_attempt_logs_deleted: 2 })
    ).toBe(true);
  });

  it("formatDbDiskUsageAvailable returns total_bytes or null", () => {
    expect(formatDbDiskUsageAvailable(null)).toBeNull();
    expect(formatDbDiskUsageAvailable(undefined)).toBeNull();
    expect(formatDbDiskUsageAvailable({ total_bytes: 10 } as any)).toBe(10);
  });
});
