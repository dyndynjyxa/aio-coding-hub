import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import { wslConfigStatusGet, wslConfigureClients, wslDetect, wslHostAddressGet } from "../wsl";

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

describe("services/app/wsl", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("wsl boom"));

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
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(wslDetect()).rejects.toThrow("IPC_NULL_RESULT: wsl_detect");
  });

  it("invokes wsl commands with expected parameters", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue({} as any);

    await wslDetect();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("wsl_detect");

    await wslHostAddressGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("wsl_host_address_get");

    await wslConfigStatusGet();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("wsl_config_status_get");

    await wslConfigStatusGet(["Ubuntu"]);
    expect(invokeTauriOrNull).toHaveBeenCalledWith("wsl_config_status_get", {
      distros: ["Ubuntu"],
    });

    await wslConfigureClients();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("wsl_configure_clients");
  });
});
