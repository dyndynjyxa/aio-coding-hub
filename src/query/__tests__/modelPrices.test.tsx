import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelPriceAliases, ModelPricesSyncReport } from "../../services/usage/modelPrices";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesListAll,
  modelPricesSync,
} from "../../services/usage/modelPrices";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { modelPricesKeys } from "../keys";
import {
  isModelPricesSyncNotModified,
  useModelPriceAliasesQuery,
  useModelPriceAliasesSetMutation,
  useModelPricesListAllQuery,
  useModelPricesSyncMutation,
} from "../modelPrices";

vi.mock("../../services/usage/modelPrices", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/modelPrices")>(
    "../../services/usage/modelPrices"
  );
  return {
    ...actual,
    modelPricesListAll: vi.fn(),
    modelPricesSync: vi.fn(),
    modelPriceAliasesGet: vi.fn(),
    modelPriceAliasesSet: vi.fn(),
  };
});

function makeModelPricesSyncReport(
  overrides: Partial<ModelPricesSyncReport> = {}
): ModelPricesSyncReport {
  return {
    status: "updated",
    inserted: 1,
    updated: 0,
    unchanged: 0,
    total: 1,
    error: null,
    ...overrides,
  };
}

describe("query/modelPrices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls modelPricesListAll with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesListAll).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPricesListAllQuery(), { wrapper });

    await waitFor(() => {
      expect(modelPricesListAll).toHaveBeenCalledWith();
    });
  });

  it("useModelPricesListAllQuery enters error state when modelPricesListAll rejects", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesListAll).mockRejectedValue(new Error("model prices query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesListAllQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
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

    const updated: ModelPriceAliases = {
      version: 1,
      rules: [
        {
          cli_key: " codex " as never,
          match_type: "prefix",
          pattern: " gpt- ",
          target_model: " gpt-5 ",
          enabled: true,
        },
      ],
    };
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
    expect(modelPriceAliasesSet).toHaveBeenCalledWith({
      version: 1,
      rules: [
        {
          cli_key: "codex",
          match_type: "prefix",
          pattern: "gpt-",
          target_model: "gpt-5",
          enabled: true,
        },
      ],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.aliases() });
  });

  it("useModelPricesSyncMutation runs a normal sync and invalidates modelPricesKeys.all", async () => {
    setTauriRuntime();

    const report = makeModelPricesSyncReport();
    vi.mocked(modelPricesSync).mockResolvedValue(report);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesSyncMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(modelPricesSync).toHaveBeenCalledWith();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.all });
  });

  it("isModelPricesSyncNotModified detects not_modified reports", () => {
    expect(isModelPricesSyncNotModified(null)).toBe(false);
    expect(isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "updated" }))).toBe(
      false
    );
    expect(
      isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "not_modified" }))
    ).toBe(true);
  });
});
