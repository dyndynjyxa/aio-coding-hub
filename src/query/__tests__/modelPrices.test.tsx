import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ModelPriceAliases,
  ModelPriceSummary,
  ModelPricesSyncReport,
} from "../../services/usage/modelPrices";
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

function makeModelPriceSummary(overrides: Partial<ModelPriceSummary> = {}): ModelPriceSummary {
  return {
    id: 1,
    cli_key: "claude",
    model: "claude-3-7-sonnet",
    currency: "USD",
    created_at: 1,
    updated_at: 2,
    ...overrides,
  };
}

function makeModelPricesSyncReport(
  overrides: Partial<ModelPricesSyncReport> = {}
): ModelPricesSyncReport {
  return {
    status: "updated",
    inserted: 1,
    updated: 0,
    skipped: 0,
    total: 1,
    ...overrides,
  };
}

describe("query/modelPrices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls modelPricesList with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPricesListQuery(" claude " as never), { wrapper });

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

  it("useModelPricesTotalCountQuery sums list lengths across all CLIs", async () => {
    setTauriRuntime();

    vi.mocked(modelPricesList)
      .mockResolvedValueOnce([makeModelPriceSummary({ id: 1 })])
      .mockResolvedValueOnce([makeModelPriceSummary({ id: 2 }), makeModelPriceSummary({ id: 3 })])
      .mockResolvedValueOnce([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesTotalCountQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.data).toBe(3);
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

  it("useModelPricesSyncBasellmMutation invalidates modelPricesKeys.all", async () => {
    setTauriRuntime();

    const report = makeModelPricesSyncReport();
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
    expect(isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "updated" }))).toBe(
      false
    );
    expect(
      isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "not_modified" }))
    ).toBe(true);
  });
});
