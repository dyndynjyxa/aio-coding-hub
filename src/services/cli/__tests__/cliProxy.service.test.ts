import { describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { invokeTauriOrNull } from "../../tauriInvoke";
import {
  cliProxyRebindCodexHome,
  cliProxySetEnabled,
  cliProxyStatusAll,
  cliProxySyncEnabled,
} from "../cliProxy";

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

describe("services/cli/cliProxy", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("cli proxy boom"));

    await expect(cliProxyStatusAll()).rejects.toThrow("cli proxy boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取 CLI 代理状态失败",
      expect.objectContaining({
        cmd: "cli_proxy_status_all",
        error: expect.stringContaining("cli proxy boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValueOnce(null);

    await expect(cliProxyStatusAll()).rejects.toThrow("IPC_NULL_RESULT: cli_proxy_status_all");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(invokeTauriOrNull).mockResolvedValue([] as any);

    await cliProxyStatusAll();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_proxy_status_all");

    await cliProxySetEnabled({ cli_key: "claude", enabled: true });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_proxy_set_enabled", {
      cliKey: "claude",
      enabled: true,
    });

    await cliProxySyncEnabled("http://127.0.0.1:37123", { apply_live: false });
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_proxy_sync_enabled", {
      baseOrigin: "http://127.0.0.1:37123",
      applyLive: false,
    });

    await cliProxyRebindCodexHome();
    expect(invokeTauriOrNull).toHaveBeenCalledWith("cli_proxy_rebind_codex_home");
  });
});
