import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  gatewayCheckPortAvailable,
  gatewayCircuitResetCli,
  gatewayCircuitResetProvider,
  gatewayCircuitStatus,
  gatewaySessionsList,
  gatewayStart,
  gatewayStatus,
  gatewayStop,
  gatewayUpstreamProxyDetectIp,
  gatewayUpstreamProxyTest,
  gatewayUpstreamProxyValidate,
  type GatewayActiveSession,
  type GatewayProviderCircuitStatus,
  type GatewayStatus,
} from "../gateway";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      gatewayStatus: vi.fn(),
      gatewayStart: vi.fn(),
      gatewayStop: vi.fn(),
      gatewayCheckPortAvailable: vi.fn(),
      gatewaySessionsList: vi.fn(),
      gatewayCircuitStatus: vi.fn(),
      gatewayCircuitResetProvider: vi.fn(),
      gatewayCircuitResetCli: vi.fn(),
      gatewayUpstreamProxyValidate: vi.fn(),
      gatewayUpstreamProxyTest: vi.fn(),
      gatewayUpstreamProxyDetectIp: vi.fn(),
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

describe("services/gateway/gateway", () => {
  it("returns invoke result with generated ipc", async () => {
    const status: GatewayStatus = {
      running: true,
      port: 37123,
      base_url: "http://127.0.0.1:37123",
      listen_addr: "127.0.0.1:37123",
    };
    const sessions: GatewayActiveSession[] = [
      {
        cli_key: "claude",
        session_id: "session-1",
        session_suffix: "1",
        provider_id: 1,
        provider_name: "Provider-1",
        expires_at: 1,
        request_count: 2,
        total_input_tokens: 3,
        total_output_tokens: 4,
        total_cost_usd: 0.01,
        total_duration_ms: 20,
      },
    ];
    const circuits: GatewayProviderCircuitStatus[] = [
      {
        provider_id: 1,
        state: "OPEN",
        failure_count: 3,
        failure_threshold: 5,
        open_until: 100,
        cooldown_until: null,
      },
    ];

    vi.mocked(commands.gatewayStatus).mockResolvedValueOnce(status as any);
    vi.mocked(commands.gatewaySessionsList).mockResolvedValueOnce({ status: "ok", data: sessions });
    vi.mocked(commands.gatewayCircuitStatus).mockResolvedValueOnce({ status: "ok", data: circuits });

    await expect(gatewayStatus()).resolves.toEqual(status);
    await expect(gatewaySessionsList(20)).resolves.toEqual(sessions);
    await expect(gatewayCircuitStatus("claude")).resolves.toEqual(circuits);
  });

  it("passes gateway command args with stable contract fields", async () => {
    vi.mocked(commands.gatewayStart).mockResolvedValueOnce({
      status: "ok",
      data: { running: true } as any,
    });
    vi.mocked(commands.gatewayStop).mockResolvedValueOnce({
      status: "ok",
      data: { running: false } as any,
    });
    vi.mocked(commands.gatewayCheckPortAvailable).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });
    vi.mocked(commands.gatewaySessionsList).mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.gatewayCircuitStatus)
      .mockResolvedValueOnce({ status: "ok", data: [] as any })
      .mockResolvedValueOnce({ status: "ok", data: [] as any })
      .mockResolvedValueOnce({ status: "ok", data: [] as any });
    vi.mocked(commands.gatewayCircuitResetProvider).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });
    vi.mocked(commands.gatewayCircuitResetCli).mockResolvedValueOnce({
      status: "ok",
      data: 1,
    });
    vi.mocked(commands.gatewayUpstreamProxyValidate).mockResolvedValueOnce({
      status: "ok",
      data: null,
    });
    vi.mocked(commands.gatewayUpstreamProxyTest).mockResolvedValueOnce({
      status: "ok",
      data: null,
    });
    vi.mocked(commands.gatewayUpstreamProxyDetectIp).mockResolvedValueOnce({
      status: "ok",
      data: "203.0.113.42",
    });

    await gatewayStart(37123);
    await gatewayStop();
    await gatewayCheckPortAvailable(37123);
    await gatewaySessionsList(undefined);
    await gatewayCircuitStatus("claude");
    await gatewayCircuitStatus("codex");
    await gatewayCircuitStatus("gemini");
    await gatewayCircuitResetProvider(42);
    await gatewayCircuitResetCli("gemini");
    await gatewayUpstreamProxyValidate({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });
    await gatewayUpstreamProxyTest({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });
    await gatewayUpstreamProxyDetectIp({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });

    expect(commands.gatewayStart).toHaveBeenCalledWith(37123);
    expect(commands.gatewayStop).toHaveBeenCalledWith();
    expect(commands.gatewayCheckPortAvailable).toHaveBeenCalledWith(37123);
    expect(commands.gatewaySessionsList).toHaveBeenCalledWith(null);
    expect(commands.gatewayCircuitStatus).toHaveBeenCalledWith("claude");
    expect(commands.gatewayCircuitStatus).toHaveBeenCalledWith("codex");
    expect(commands.gatewayCircuitStatus).toHaveBeenCalledWith("gemini");
    expect(commands.gatewayCircuitResetProvider).toHaveBeenCalledWith(42);
    expect(commands.gatewayCircuitResetCli).toHaveBeenCalledWith("gemini");
    expect(commands.gatewayUpstreamProxyValidate).toHaveBeenCalledWith({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });
    expect(commands.gatewayUpstreamProxyTest).toHaveBeenCalledWith({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });
    expect(commands.gatewayUpstreamProxyDetectIp).toHaveBeenCalledWith({
      proxyUrl: "http://127.0.0.1:7890",
      proxyUsername: "proxy-user",
      proxyPassword: "secret",
    });
  });

  it("treats null results as success for void proxy commands", async () => {
    vi.mocked(commands.gatewayUpstreamProxyValidate).mockResolvedValueOnce({
      status: "ok",
      data: null,
    });
    vi.mocked(commands.gatewayUpstreamProxyTest).mockResolvedValueOnce({
      status: "ok",
      data: null,
    });

    await expect(
      gatewayUpstreamProxyValidate({
        proxyUrl: "http://127.0.0.1:7890",
        proxyUsername: "proxy-user",
        proxyPassword: "secret",
      })
    ).resolves.toBeNull();
    await expect(
      gatewayUpstreamProxyTest({
        proxyUrl: "http://127.0.0.1:7890",
        proxyUsername: "proxy-user",
        proxyPassword: "secret",
      })
    ).resolves.toBeNull();
  });

  it("returns proxy exit ip from generated command", async () => {
    vi.mocked(commands.gatewayUpstreamProxyDetectIp).mockResolvedValueOnce({
      status: "ok",
      data: "203.0.113.42",
    });

    await expect(
      gatewayUpstreamProxyDetectIp({
        proxyUrl: "http://127.0.0.1:7890",
        proxyUsername: "proxy-user",
        proxyPassword: "secret",
      })
    ).resolves.toBe("203.0.113.42");
  });

  it("rethrows invoke errors and logs details", async () => {
    vi.mocked(commands.gatewayStatus).mockRejectedValueOnce(new Error("boom"));

    await expect(gatewayStatus()).rejects.toThrow("boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "获取网关状态失败",
      expect.objectContaining({
        cmd: "gateway_status",
        error: expect.stringContaining("boom"),
      })
    );
  });

  it("treats null invoke result as error and logs", async () => {
    vi.mocked(commands.gatewayStatus).mockResolvedValueOnce(null as any);

    await expect(gatewayStatus()).rejects.toThrow("IPC_NULL_RESULT: gateway_status");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "获取网关状态失败",
      expect.objectContaining({
        cmd: "gateway_status",
        error: expect.stringContaining("IPC_NULL_RESULT: gateway_status"),
      })
    );
  });
});
