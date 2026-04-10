import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
  notifyModelPricesUpdated,
  subscribeModelPricesUpdated,
} from "../modelPrices";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn(),
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
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("model prices boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(modelPricesList("claude")).rejects.toThrow("IPC_NULL_RESULT: model_prices_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({ version: 1, rules: [] } as any);

    await modelPricesList("claude");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("model_prices_list", { cliKey: "claude" });

    await modelPricesSyncBasellm(true);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("model_prices_sync_basellm", { force: true });

    await modelPriceAliasesGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("model_price_aliases_get");

    await modelPriceAliasesSet({ version: 2, rules: [] });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("model_price_aliases_set", {
      aliases: { version: 2, rules: [] },
    });
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
