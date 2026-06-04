import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  type ModelPriceAliases,
  type ModelPriceSummary,
  type ModelPricesSyncReport,
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
  notifyModelPricesUpdated,
  normalizeModelPriceAliases,
  subscribeModelPricesUpdated,
  validateModelPricesCliKey,
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

beforeEach(() => {
  vi.clearAllMocks();
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

function makeModelPriceAliases(overrides: Partial<ModelPriceAliases> = {}): ModelPriceAliases {
  return {
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

  it("maps generated list and alias payloads through generated authority", async () => {
    vi.mocked(commands.modelPricesList).mockResolvedValueOnce({
      status: "ok",
      data: [makeModelPriceSummary({ model: " claude-3-7-sonnet ", currency: " USD " })],
    });
    vi.mocked(commands.modelPriceAliasesGet).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPriceAliases({
        rules: [
          {
            cli_key: " codex " as never,
            match_type: "prefix",
            pattern: " gpt- ",
            target_model: " gpt-5 ",
            enabled: true,
          },
        ],
      }),
    });
    vi.mocked(commands.modelPriceAliasesSet).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPriceAliases({ version: 1 }),
    });
    vi.mocked(commands.modelPricesSyncBasellm).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPricesSyncReport(),
    });

    const rows = await modelPricesList(" claude " as never);
    const aliases = await modelPriceAliasesGet();
    const updated = await modelPriceAliasesSet(aliases!);
    const report = await modelPricesSyncBasellm(true);

    expect(rows?.[0]?.cli_key).toBe("claude");
    expect(rows?.[0]?.model).toBe("claude-3-7-sonnet");
    expect(rows?.[0]?.currency).toBe("USD");
    expect(aliases?.rules[0]?.cli_key).toBe("codex");
    expect(aliases?.rules[0]?.pattern).toBe("gpt-");
    expect(aliases?.rules[0]?.target_model).toBe("gpt-5");
    expect(updated?.version).toBe(1);
    expect(report).toEqual(expect.objectContaining({ status: "updated", inserted: 1, total: 1 }));
    expect(commands.modelPricesList).toHaveBeenCalledWith("claude");
    expect(commands.modelPriceAliasesSet).toHaveBeenCalledWith(aliases);
    expect(commands.modelPricesSyncBasellm).toHaveBeenCalledWith(true);
  });

  it("rejects invalid list keys and aliases before generated IPC", async () => {
    expect(validateModelPricesCliKey(" codex ")).toBe("codex");
    expect(() => validateModelPricesCliKey("unknown")).toThrow("SEC_INVALID_INPUT");

    await expect(modelPricesList("unknown" as never)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      modelPriceAliasesSet(
        makeModelPriceAliases({
          rules: [
            {
              cli_key: "codex",
              match_type: "exact",
              pattern: "gpt-*",
              target_model: "gpt-5",
              enabled: true,
            },
          ],
        })
      )
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.modelPricesList).not.toHaveBeenCalled();
    expect(commands.modelPriceAliasesSet).not.toHaveBeenCalled();
  });

  it("normalizes aliases locally for service and query callers", () => {
    expect(
      normalizeModelPriceAliases({
        version: 1,
        rules: [
          {
            cli_key: " gemini " as never,
            match_type: "wildcard",
            pattern: "gemini-*",
            target_model: "gemini-pro",
            enabled: true,
          },
        ],
      })
    ).toEqual({
      version: 1,
      rules: [
        {
          cli_key: "gemini",
          match_type: "wildcard",
          pattern: "gemini-*",
          target_model: "gemini-pro",
          enabled: true,
        },
      ],
    });

    expect(() => normalizeModelPriceAliases({ version: 2, rules: [] })).toThrow(
      "SEC_INVALID_INPUT"
    );
  });

  it("rejects invalid generated model price and sync payloads", async () => {
    vi.mocked(commands.modelPricesList).mockResolvedValueOnce({
      status: "ok",
      data: [makeModelPriceSummary({ id: 0 })],
    });

    await expect(modelPricesList("claude")).rejects.toThrow("IPC_INVALID_RESULT");

    vi.mocked(commands.modelPricesSyncBasellm).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPricesSyncReport({ inserted: 1, updated: 1, skipped: 0, total: 1 }),
    });

    await expect(modelPricesSyncBasellm(false)).rejects.toThrow("IPC_INVALID_RESULT");
  });

  it("isolates model price update subscribers when one fails", async () => {
    const throwingListener = vi.fn(() => {
      throw new Error("sync listener boom");
    });
    const healthyListener = vi.fn();
    const rejectingListener = vi.fn(() => Promise.reject(new Error("async listener boom")));

    const unsubscribeThrowing = subscribeModelPricesUpdated(throwingListener);
    const unsubscribeHealthy = subscribeModelPricesUpdated(healthyListener);
    const unsubscribeRejecting = subscribeModelPricesUpdated(rejectingListener);

    try {
      notifyModelPricesUpdated();

      expect(throwingListener).toHaveBeenCalledTimes(1);
      expect(healthyListener).toHaveBeenCalledTimes(1);
      expect(rejectingListener).toHaveBeenCalledTimes(1);
      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "模型定价更新订阅处理失败",
        { error: "Error: sync listener boom" },
        "model_prices"
      );

      await Promise.resolve();
      await Promise.resolve();

      expect(logToConsole).toHaveBeenCalledWith(
        "warn",
        "模型定价更新订阅处理失败",
        { error: "Error: async listener boom" },
        "model_prices"
      );
    } finally {
      unsubscribeThrowing();
      unsubscribeHealthy();
      unsubscribeRejecting();
    }
  });
});
