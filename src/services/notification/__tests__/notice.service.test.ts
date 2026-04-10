import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { noticeSend } from "../notice";
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

describe("services/notification/notice", () => {
  it("maps true/false results as before", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(true as any);
    await expect(noticeSend({ level: "info", body: "ok" })).resolves.toBe(true);

    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(false as any);
    await expect(noticeSend({ level: "warning", body: "no" })).resolves.toBe(false);
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("notice boom"));

    await expect(noticeSend({ level: "error", body: "x" })).rejects.toThrow("notice boom");

    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "发送系统通知失败",
      expect.objectContaining({
        cmd: "notice_send",
        error: expect.stringContaining("notice boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null as any);

    await expect(noticeSend({ level: "info", body: "x" })).rejects.toThrow(
      "IPC_NULL_RESULT: notice_send"
    );
  });
});
