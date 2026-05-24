import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  providerLimitUsageV1,
  validateProviderLimitUsageCliKey,
  type ProviderLimitUsageRow,
} from "../providerLimitUsage";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      providerLimitUsageV1: vi.fn(),
    },
  };
});

function makeProviderLimitUsageRow(
  overrides: Partial<ProviderLimitUsageRow> = {}
): ProviderLimitUsageRow {
  return {
    cli_key: "claude",
    provider_id: 1,
    provider_name: "Fetch",
    enabled: true,
    limit_5h_usd: null,
    limit_daily_usd: null,
    daily_reset_mode: null,
    daily_reset_time: null,
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    usage_5h_usd: 0,
    usage_daily_usd: 0,
    usage_weekly_usd: 0,
    usage_monthly_usd: 0,
    usage_total_usd: 0,
    window_5h_start_ts: 0,
    window_daily_start_ts: 0,
    window_weekly_start_ts: 0,
    window_monthly_start_ts: 0,
    ...overrides,
  };
}

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/providers/providerLimitUsage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.providerLimitUsageV1).mockRejectedValueOnce(
      new Error("provider limit boom")
    );

    await expect(providerLimitUsageV1("claude")).rejects.toThrow("provider limit boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取 Provider 限额用量失败",
      expect.objectContaining({
        cmd: "provider_limit_usage_v1",
        error: expect.stringContaining("provider limit boom"),
      })
    );
  });

  it("maps generated rows and forwards nullable cliKey", async () => {
    vi.mocked(commands.providerLimitUsageV1)
      .mockResolvedValueOnce({
        status: "ok",
        data: [
          makeProviderLimitUsageRow({
            provider_name: " Fetch ",
            limit_daily_usd: 10,
            daily_reset_mode: " rolling " as never,
            daily_reset_time: " 00:00:00 ",
          }),
        ],
      })
      .mockResolvedValueOnce({ status: "ok", data: [] });

    const rows = await providerLimitUsageV1(" claude " as never);
    const allRows = await providerLimitUsageV1(null);

    expect(rows?.[0]?.cli_key).toBe("claude");
    expect(rows?.[0]?.provider_name).toBe("Fetch");
    expect(rows?.[0]?.daily_reset_mode).toBe("rolling");
    expect(rows?.[0]?.daily_reset_time).toBe("00:00:00");
    expect(allRows).toEqual([]);
    expect(commands.providerLimitUsageV1).toHaveBeenNthCalledWith(1, "claude");
    expect(commands.providerLimitUsageV1).toHaveBeenNthCalledWith(2, null);
  });

  it("rejects invalid cli keys before generated IPC", async () => {
    expect(validateProviderLimitUsageCliKey(null)).toBeNull();
    expect(validateProviderLimitUsageCliKey(" codex ")).toBe("codex");

    await expect(providerLimitUsageV1("unknown" as never)).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.providerLimitUsageV1).not.toHaveBeenCalled();
  });

  it("rejects invalid generated usage payloads before cache consumers", async () => {
    vi.mocked(commands.providerLimitUsageV1).mockResolvedValueOnce({
      status: "ok",
      data: [makeProviderLimitUsageRow({ usage_total_usd: Number.NaN })],
    });

    await expect(providerLimitUsageV1("claude")).rejects.toThrow("IPC_INVALID_RESULT");

    vi.mocked(commands.providerLimitUsageV1).mockResolvedValueOnce({
      status: "ok",
      data: [makeProviderLimitUsageRow({ provider_id: 0 })],
    });

    await expect(providerLimitUsageV1("claude")).rejects.toThrow("IPC_INVALID_RESULT");
  });
});
