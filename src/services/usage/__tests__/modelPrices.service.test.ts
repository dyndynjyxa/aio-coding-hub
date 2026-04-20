import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
  notifyModelPricesUpdated,
  subscribeModelPricesUpdated,
} from "../modelPrices";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      modelPricesList: vi.fn(),
      modelPricesSyncBasellm: vi.fn(),
      modelPriceAliasesGet: vi.fn(),
      modelPriceAliasesSet: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/usage/modelPrices", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.modelPricesList).mockRejectedValueOnce(new Error("model prices boom"));

    await expect(modelPricesList("claude")).rejects.toThrow("model prices boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取模型价格列表失败",
      expect.objectContaining({
        cmd: "model_prices_list",
        error: expect.stringContaining("model prices boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.modelPricesList).mockResolvedValueOnce(null as any);

    await expect(modelPricesList("claude")).rejects.toThrow("IPC_NULL_RESULT: model_prices_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.modelPricesList).mockResolvedValue({ status: "ok", data: [] as any });
    vi.mocked(commands.modelPricesSyncBasellm).mockResolvedValue({
      status: "ok",
      data: { status: "updated", inserted: 0, updated: 0, skipped: 0, total: 0 } as any,
    });
    vi.mocked(commands.modelPriceAliasesGet).mockResolvedValue({
      status: "ok",
      data: { version: 1, rules: [] } as any,
    });
    vi.mocked(commands.modelPriceAliasesSet).mockResolvedValue({
      status: "ok",
      data: { version: 2, rules: [] } as any,
    });

    await modelPricesList("claude");
    expect(commands.modelPricesList).toHaveBeenCalledWith("claude");

    await modelPricesSyncBasellm(true);
    expect(commands.modelPricesSyncBasellm).toHaveBeenCalledWith(true);

    await modelPriceAliasesGet();
    expect(commands.modelPriceAliasesGet).toHaveBeenCalledWith();

    await modelPriceAliasesSet({ version: 2, rules: [] });
    expect(commands.modelPriceAliasesSet).toHaveBeenCalledWith({ version: 2, rules: [] });
  });

  it("subscribes/unsubscribes update listeners", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeModelPricesUpdated(listener);

    notifyModelPricesUpdated();
    expect(listener).toHaveBeenCalledTimes(1);

    unsubscribe();
    notifyModelPricesUpdated();
    expect(listener).toHaveBeenCalledTimes(1);
  });
});
