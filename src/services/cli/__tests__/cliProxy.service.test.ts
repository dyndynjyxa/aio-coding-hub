import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  cliProxyRebindCodexHome,
  cliProxySetEnabled,
  cliProxyStatusAll,
  cliProxySyncEnabled,
  normalizeCliProxyBaseOrigin,
  validateCliProxyCliKey,
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
  beforeEach(() => {
    vi.clearAllMocks();
  });

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
    vi.mocked(commands.cliProxyStatusAll).mockResolvedValueOnce({
      status: "ok",
      data: null as any,
    });

    await expect(cliProxyStatusAll()).rejects.toThrow("IPC_NULL_RESULT: cli_proxy_status_all");
  });

  it("invokes generated commands with normalized args and typed payloads", async () => {
    vi.mocked(commands.cliProxyStatusAll).mockResolvedValueOnce({
      status: "ok",
      data: [
        {
          cli_key: "claude",
          enabled: false,
          base_origin: null,
          current_gateway_origin: null,
          applied_to_current_gateway: null,
        },
      ],
    });
    vi.mocked(commands.cliProxySetEnabled).mockResolvedValueOnce({
      status: "ok",
      data: {
        trace_id: "t1",
        cli_key: "claude",
        enabled: true,
        ok: true,
        error_code: null,
        message: "ok",
        base_origin: "http://127.0.0.1:37123",
      },
    });
    vi.mocked(commands.cliProxySyncEnabled).mockResolvedValueOnce({
      status: "ok",
      data: [
        {
          trace_id: "t2",
          cli_key: "codex",
          enabled: true,
          ok: true,
          error_code: null,
          message: "ok",
          base_origin: "http://127.0.0.1:37123",
        },
      ],
    });
    vi.mocked(commands.cliProxyRebindCodexHome).mockResolvedValueOnce({
      status: "ok",
      data: {
        trace_id: "t3",
        cli_key: "codex",
        enabled: true,
        ok: true,
        error_code: null,
        message: "ok",
        base_origin: "http://127.0.0.1:37123",
      },
    });

    const statuses = await cliProxyStatusAll();
    expect(commands.cliProxyStatusAll).toHaveBeenCalledWith();
    expect(statuses[0]?.cli_key).toBe("claude");

    const enabled = await cliProxySetEnabled({ cli_key: " claude " as never, enabled: true });
    expect(commands.cliProxySetEnabled).toHaveBeenCalledWith("claude", true);
    expect(enabled.cli_key).toBe("claude");

    const synced = await cliProxySyncEnabled(" http://127.0.0.1:37123/ ", { apply_live: false });
    expect(commands.cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:37123", false);
    expect(synced[0]?.cli_key).toBe("codex");

    const rebound = await cliProxyRebindCodexHome();
    expect(commands.cliProxyRebindCodexHome).toHaveBeenCalledWith();
    expect(rebound.cli_key).toBe("codex");
  });

  it("defaults cliProxySyncEnabled apply_live to null when omitted", async () => {
    vi.mocked(commands.cliProxySyncEnabled).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });

    await cliProxySyncEnabled("http://127.0.0.1:37123");

    expect(commands.cliProxySyncEnabled).toHaveBeenCalledWith("http://127.0.0.1:37123", null);
  });

  it("rejects invalid cli keys and base origins before generated commands", async () => {
    expect(validateCliProxyCliKey(" codex ")).toBe("codex");
    expect(normalizeCliProxyBaseOrigin(" http://127.0.0.1:37123/ ")).toBe("http://127.0.0.1:37123");
    expect(normalizeCliProxyBaseOrigin("https://example.com")).toBe("https://example.com");
    expect(() => validateCliProxyCliKey("unknown")).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeCliProxyBaseOrigin("")).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeCliProxyBaseOrigin("ftp://example.com")).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeCliProxyBaseOrigin("https://user@example.com")).toThrow(
      "SEC_INVALID_INPUT"
    );
    expect(() => normalizeCliProxyBaseOrigin("https://example.com/v1")).toThrow(
      "SEC_INVALID_INPUT"
    );

    await expect(
      cliProxySetEnabled({ cli_key: "unknown" as never, enabled: true })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(cliProxySyncEnabled("https://example.com/v1")).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );

    expect(commands.cliProxySetEnabled).not.toHaveBeenCalled();
    expect(commands.cliProxySyncEnabled).not.toHaveBeenCalled();
  });
});
