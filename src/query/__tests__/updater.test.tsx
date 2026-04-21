import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { updaterCheck } from "../../services/app/updater";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { useUpdaterCheckQuery } from "../updater";

vi.mock("../../services/app/updater", async () => {
  const actual = await vi.importActual<typeof import("../../services/app/updater")>(
    "../../services/app/updater"
  );
  return {
    ...actual,
    updaterCheck: vi.fn(),
  };
});

describe("query/updater", () => {
  it("does not call updaterCheck when disabled by default", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useUpdaterCheckQuery(), { wrapper });
    await Promise.resolve();

    expect(updaterCheck).not.toHaveBeenCalled();
  });

  it("calls updaterCheck when enabled", async () => {
    setTauriRuntime();

    vi.mocked(updaterCheck).mockResolvedValue({
      rid: 1,
      version: "0.2.0",
      currentVersion: "0.1.0",
      date: "2026-01-01",
      body: "notes",
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useUpdaterCheckQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(updaterCheck).toHaveBeenCalledTimes(1);
    expect(result.current.data?.version).toBe("0.2.0");
  });

  it("useUpdaterCheckQuery enters error state when updaterCheck rejects", async () => {
    setTauriRuntime();

    vi.mocked(updaterCheck).mockRejectedValue(new Error("updater check boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useUpdaterCheckQuery({ enabled: true }), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });
});
