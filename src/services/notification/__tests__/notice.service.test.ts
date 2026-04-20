import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { noticeSend } from "../notice";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      noticeSend: vi.fn(),
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

describe("services/notification/notice", () => {
  it("maps true/false results as before", async () => {
    vi.mocked(commands.noticeSend)
      .mockResolvedValueOnce({ status: "ok", data: true })
      .mockResolvedValueOnce({ status: "ok", data: false });

    await expect(noticeSend({ level: "info", body: "ok" })).resolves.toBe(true);
    await expect(noticeSend({ level: "warning", body: "no" })).resolves.toBe(false);
  });

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.noticeSend).mockRejectedValueOnce(new Error("notice boom"));

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
    vi.mocked(commands.noticeSend).mockResolvedValueOnce(null as any);

    await expect(noticeSend({ level: "info", body: "x" })).rejects.toThrow(
      "IPC_NULL_RESULT: notice_send"
    );
  });
});
