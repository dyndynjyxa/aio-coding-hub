import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { settingsCircuitBreakerNoticeSet } from "../settingsCircuitBreakerNotice";
import { invokeTauriOrNull } from "../../tauriInvoke";

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

describe("services/settings/settingsCircuitBreakerNotice", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("circuit notice boom"));

    await expect(settingsCircuitBreakerNoticeSet(true)).rejects.toThrow("circuit notice boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存熔断提示设置失败",
      expect.objectContaining({
        cmd: "settings_circuit_breaker_notice_set",
        error: expect.stringContaining("circuit notice boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(settingsCircuitBreakerNoticeSet(true)).rejects.toThrow(
      "IPC_NULL_RESULT: settings_circuit_breaker_notice_set"
    );
  });
});
