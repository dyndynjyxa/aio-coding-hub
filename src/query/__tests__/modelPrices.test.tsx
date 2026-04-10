import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ModelPriceAliases, ModelPricesSyncReport } from "../../services/usage/modelPrices";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
} from "../../services/usage/modelPrices";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { modelPricesKeys } from "../keys";
import {
  isModelPricesSyncNotModified,
  useModelPriceAliasesQuery,
  useModelPriceAliasesSetMutation,
  useModelPricesListQuery,
  useModelPricesSyncBasellmMutation,
  useModelPricesTotalCountQuery,
} from "../modelPrices";

vi.mock("../../services/usage/modelPrices", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/modelPrices")>(
    "../../services/usage/modelPrices"
  );
  return {
    ...actual,
    modelPricesList: vi.fn(),
    modelPricesSyncBasellm: vi.fn(),
    modelPriceAliasesGet: vi.fn(),
    modelPriceAliasesSet: vi.fn(),
  };
});

describe("query/modelPrices", () => {
  it("calls modelPricesList with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPricesListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(modelPricesList).toHaveBeenCalledWith("claude");
    });
  });

  it("useModelPricesListQuery enters error state when modelPricesList rejects", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesList).mockRejectedValue(new Error("model prices query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useModelPricesTotalCountQuery sums list lengths and returns null if any list is null", async () => {
    setTauriRuntime();

    vi.mocked(modelPricesList)
      .mockResolvedValueOnce([{ id: 1 } as any])
      .mockResolvedValueOnce([{ id: 2 } as any, { id: 3 } as any])
      .mockResolvedValueOnce([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesTotalCountQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.data).toBe(3);
    });

    vi.mocked(modelPricesList)
      .mockResolvedValueOnce(null as any)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    const client2 = createTestQueryClient();
    const wrapper2 = createQueryWrapper(client2);

    const { result: result2 } = renderHook(() => useModelPricesTotalCountQuery(), {
      wrapper: wrapper2,
    });
    await waitFor(() => {
      expect(result2.current.data).toBeNull();
    });
  });

  it("useModelPriceAliasesQuery calls modelPriceAliasesGet", async () => {
    setTauriRuntime();

    const aliases: ModelPriceAliases = { version: 1, rules: [] };
    vi.mocked(modelPriceAliasesGet).mockResolvedValue(aliases);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPriceAliasesQuery(), { wrapper });

    await waitFor(() => {
      expect(modelPriceAliasesGet).toHaveBeenCalled();
    });
  });

  it("useModelPriceAliasesSetMutation updates cache and invalidates aliases", async () => {
    setTauriRuntime();

    const updated: ModelPriceAliases = { version: 2, rules: [] };
    vi.mocked(modelPriceAliasesSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(modelPricesKeys.aliases(), { version: 1, rules: [] } as ModelPriceAliases);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPriceAliasesSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(updated);
    });

    expect(client.getQueryData(modelPricesKeys.aliases())).toEqual(updated);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.aliases() });
  });

  it("useModelPricesSyncBasellmMutation invalidates modelPricesKeys.all", async () => {
    setTauriRuntime();

    const report: ModelPricesSyncReport = {
      status: "updated",
      inserted: 1,
      updated: 0,
      skipped: 0,
      total: 1,
    };
    vi.mocked(modelPricesSyncBasellm).mockResolvedValue(report);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesSyncBasellmMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ force: true });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.all });
  });

  it("isModelPricesSyncNotModified detects not_modified reports", () => {
    expect(isModelPricesSyncNotModified(null)).toBe(false);
    expect(isModelPricesSyncNotModified({ status: "updated" } as any)).toBe(false);
    expect(isModelPricesSyncNotModified({ status: "not_modified" } as any)).toBe(true);
  });
});
