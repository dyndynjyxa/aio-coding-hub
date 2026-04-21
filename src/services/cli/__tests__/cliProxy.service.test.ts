import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  cliProxyRebindCodexHome,
  cliProxySetEnabled,
  cliProxyStatusAll,
  cliProxySyncEnabled,
} from "../cliProxy";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      cliProxyStatusAll: vi.fn(),
      cliProxySetEnabled: vi.fn(),
      cliProxySyncEnabled: vi.fn(),
      cliProxyRebindCodexHome: vi.fn(),
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

describe("services/cli/cliProxy", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.cliProxyStatusAll).mockRejectedValueOnce(new Error("cli proxy boom"));

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
    vi.mocked(commands.cliProxyStatusAll).mockResolvedValueOnce({ status: "ok", data: null as any });

    await expect(cliProxyStatusAll()).rejects.toThrow("IPC_NULL_RESULT: cli_proxy_status_all");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.cliProxyStatusAll).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.cliProxySetEnabled).mockResolvedValueOnce({
      status: "ok",
      data: { cli_key: "claude" } as any,
    });
    vi.mocked(commands.cliProxySyncEnabled).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });
    vi.mocked(commands.cliProxyRebindCodexHome).mockResolvedValueOnce({
      status: "ok",
      data: { cli_key: "codex" } as any,
    });

    await cliProxyStatusAll();
    expect(commands.cliProxyStatusAll).toHaveBeenCalledWith();

    await cliProxySetEnabled({ cli_key: "claude", enabled: true });
    expect(commands.cliProxySetEnabled).toHaveBeenCalledWith("claude", true);

    await cliProxySyncEnabled("http://127.0.0.1:37123", { apply_live: false });
    expect(commands.cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:37123", false);

    await cliProxyRebindCodexHome();
    expect(commands.cliProxyRebindCodexHome).toHaveBeenCalledWith();
  });

  it("defaults cliProxySyncEnabled apply_live to null when omitted", async () => {
    vi.mocked(commands.cliProxySyncEnabled).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });

    await cliProxySyncEnabled("http://127.0.0.1:37123");

    expect(commands.cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:37123", null);
  });
});
