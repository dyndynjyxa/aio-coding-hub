import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import { providerLimitUsageV1 } from "../providerLimitUsage";

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

describe("services/providers/providerLimitUsage", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("provider limit usage boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(providerLimitUsageV1("claude")).rejects.toThrow(
      "IPC_NULL_RESULT: provider_limit_usage_v1"
    );
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue([] as any);

    await providerLimitUsageV1("claude");
    expect(invokeTauriOrNull).toHaveBeenCalledWith("provider_limit_usage_v1", {
      cliKey: "claude",
    });

    await providerLimitUsageV1(null);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("provider_limit_usage_v1", {
      cliKey: null,
    });
  });
});
