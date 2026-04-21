import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { wslConfigStatusGet, wslConfigureClients, wslDetect, wslHostAddressGet } from "../wsl";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      wslDetect: vi.fn(),
      wslHostAddressGet: vi.fn(),
      wslConfigStatusGet: vi.fn(),
      wslConfigureClients: vi.fn(),
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

describe("services/app/wsl", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.wslDetect).mockRejectedValueOnce(new Error("wsl boom"));

    await expect(wslDetect()).rejects.toThrow("wsl boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "检测 WSL 失败",
      expect.objectContaining({
        cmd: "wsl_detect",
        error: expect.stringContaining("wsl boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.wslDetect).mockResolvedValueOnce(null as any);

    await expect(wslDetect()).rejects.toThrow("IPC_NULL_RESULT: wsl_detect");
  });

  it("invokes wsl commands with expected parameters", async () => {
    vi.mocked(commands.wslDetect).mockResolvedValueOnce({ detected: true, distros: [] } as any);
    vi.mocked(commands.wslHostAddressGet).mockResolvedValueOnce("172.20.0.1" as any);
    vi.mocked(commands.wslConfigStatusGet)
      .mockResolvedValueOnce([] as any)
      .mockResolvedValueOnce([] as any);
    vi.mocked(commands.wslConfigureClients).mockResolvedValueOnce({ status: "ok", data: {} as any });

    await wslDetect();
    expect(commands.wslDetect).toHaveBeenCalledWith();

    await wslHostAddressGet();
    expect(commands.wslHostAddressGet).toHaveBeenCalledWith();

    await wslConfigStatusGet();
    expect(commands.wslConfigStatusGet).toHaveBeenCalledWith(null);

    await wslConfigStatusGet(["Ubuntu"]);
    expect(commands.wslConfigStatusGet).toHaveBeenCalledWith(["Ubuntu"]);

    await wslConfigureClients();
    expect(commands.wslConfigureClients).toHaveBeenCalledWith();
  });
});
