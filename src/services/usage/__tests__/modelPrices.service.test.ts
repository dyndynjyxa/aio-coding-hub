import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  type ModelPriceAliases,
  type ModelPriceSummary,
  type ModelPricesSyncReport,
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesListAll,
  modelPricesSync,
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
      modelPricesListAll: vi.fn(),
      modelPricesSync: vi.fn(),
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
    vendor: "anthropic",
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
    unchanged: 0,
    total: 1,
    error: null,
    ...overrides,
  };
}

describe("services/usage/modelPrices", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.modelPricesListAll).mockRejectedValueOnce(new Error("model prices boom"));

    await expect(modelPricesListAll()).rejects.toThrow("model prices boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取模型价格列表失败",
      expect.objectContaining({
        cmd: "model_prices_list_all",
        error: expect.stringContaining("model prices boom"),
      })
    );
  });

  it("maps generated list and alias payloads through generated authority", async () => {
    vi.mocked(commands.modelPricesListAll).mockResolvedValueOnce({
      status: "ok",
      data: [
        makeModelPriceSummary({
          model: " claude-3-7-sonnet ",
          currency: " USD ",
          vendor: " deepseek ",
        }),
      ],
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
    vi.mocked(commands.modelPricesSync).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPricesSyncReport(),
    });

    const rows = await modelPricesListAll();
    const aliases = await modelPriceAliasesGet();
    const updated = await modelPriceAliasesSet(aliases!);
    const report = await modelPricesSync();

    expect(rows?.[0]?.cli_key).toBe("claude");
    expect(rows?.[0]?.model).toBe("claude-3-7-sonnet");
    expect(rows?.[0]?.currency).toBe("USD");
    expect(rows?.[0]?.vendor).toBe("deepseek");
    expect(aliases?.rules[0]?.cli_key).toBe("codex");
    expect(aliases?.rules[0]?.pattern).toBe("gpt-");
    expect(aliases?.rules[0]?.target_model).toBe("gpt-5");
    expect(updated?.version).toBe(1);
    expect(report).toEqual(expect.objectContaining({ status: "updated", inserted: 1, total: 1 }));
    expect(commands.modelPricesListAll).toHaveBeenCalledWith();
    expect(commands.modelPriceAliasesSet).toHaveBeenCalledWith(aliases);
    expect(commands.modelPricesSync).toHaveBeenCalledWith();
  });

  it("rejects invalid list keys and aliases before generated IPC", async () => {
    expect(validateModelPricesCliKey(" codex ")).toBe("codex");
    expect(() => validateModelPricesCliKey("unknown")).toThrow("SEC_INVALID_INPUT");

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
    vi.mocked(commands.modelPricesListAll).mockResolvedValueOnce({
      status: "ok",
      data: [makeModelPriceSummary({ id: 0 })],
    });

    await expect(modelPricesListAll()).rejects.toThrow("IPC_INVALID_RESULT");

    vi.mocked(commands.modelPricesListAll).mockResolvedValueOnce({
      status: "ok",
      data: [makeModelPriceSummary({ cli_key: "bogus" as never })],
    });

    await expect(modelPricesListAll()).rejects.toThrow("IPC_INVALID_LITERAL");

    vi.mocked(commands.modelPricesSync).mockResolvedValueOnce({
      status: "ok",
      data: makeModelPricesSyncReport({ status: "bogus" as never }),
    });

    await expect(modelPricesSync()).rejects.toThrow("IPC_INVALID_LITERAL");
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
