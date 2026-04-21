import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { providerLimitUsageV1 } from "../providerLimitUsage";

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

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/providers/providerLimitUsage", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.providerLimitUsageV1).mockRejectedValueOnce(
      new Error("provider limit usage boom")
    );

    await expect(providerLimitUsageV1("claude")).rejects.toThrow("provider limit usage boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取 Provider 限额用量失败",
      expect.objectContaining({
        cmd: "provider_limit_usage_v1",
        error: expect.stringContaining("provider limit usage boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.providerLimitUsageV1).mockResolvedValueOnce(null as any);

    await expect(providerLimitUsageV1("claude")).rejects.toThrow(
      "IPC_NULL_RESULT: provider_limit_usage_v1"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.providerLimitUsageV1).mockResolvedValue({
      status: "ok",
      data: [] as any,
    });

    await providerLimitUsageV1("claude");
    expect(commands.providerLimitUsageV1).toHaveBeenCalledWith("claude");

    await providerLimitUsageV1(null);
    expect(commands.providerLimitUsageV1).toHaveBeenCalledWith(null);
  });
});
