import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  GATEWAY_PORT_MAX,
  GATEWAY_PORT_MIN,
  GATEWAY_SESSIONS_MAX_LIMIT,
  GATEWAY_SESSIONS_MIN_LIMIT,
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
  normalizeGatewayPortAvailabilityCheck,
  normalizeGatewayPreferredPort,
  normalizeGatewayProviderId,
  normalizeGatewaySessionsLimit,
  validateGatewayCliKey,
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
    vi.mocked(commands.gatewayCircuitStatus).mockResolvedValueOnce({
      status: "ok",
      data: circuits,
    });

    await expect(gatewayStatus()).resolves.toEqual(status);
    await expect(gatewaySessionsList(20)).resolves.toEqual(sessions);
    await expect(gatewayCircuitStatus(" claude ")).resolves.toEqual(circuits);
    expect(commands.gatewayCircuitStatus).toHaveBeenCalledWith("claude");
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
    vi.mocked(commands.gatewaySessionsList).mockResolvedValueOnce({
      status: "ok",
      data: [] as any,
    });
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
    await gatewayCircuitResetCli(" gemini ");
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

  it("normalizes gateway sessions list limits before ipc", async () => {
    vi.mocked(commands.gatewaySessionsList).mockClear();
    vi.mocked(commands.gatewaySessionsList)
      .mockResolvedValueOnce({ status: "ok", data: [] as any })
      .mockResolvedValueOnce({ status: "ok", data: [] as any })
      .mockResolvedValueOnce({ status: "ok", data: [] as any });

    expect(normalizeGatewaySessionsLimit(undefined)).toBeNull();
    expect(normalizeGatewaySessionsLimit(null)).toBeNull();
    expect(normalizeGatewaySessionsLimit(0)).toBe(GATEWAY_SESSIONS_MIN_LIMIT);
    expect(normalizeGatewaySessionsLimit(999)).toBe(GATEWAY_SESSIONS_MAX_LIMIT);

    await gatewaySessionsList(0);
    await gatewaySessionsList(999);
    await gatewaySessionsList(42);

    expect(commands.gatewaySessionsList).toHaveBeenNthCalledWith(1, GATEWAY_SESSIONS_MIN_LIMIT);
    expect(commands.gatewaySessionsList).toHaveBeenNthCalledWith(2, GATEWAY_SESSIONS_MAX_LIMIT);
    expect(commands.gatewaySessionsList).toHaveBeenNthCalledWith(3, 42);
  });

  it("normalizes gateway port and provider id inputs before ipc", async () => {
    vi.mocked(commands.gatewayStart).mockClear();
    vi.mocked(commands.gatewayCheckPortAvailable).mockClear();
    vi.mocked(commands.gatewayCircuitResetProvider).mockClear();

    vi.mocked(commands.gatewayStart).mockResolvedValueOnce({
      status: "ok",
      data: { running: true } as any,
    });
    vi.mocked(commands.gatewayCheckPortAvailable).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });
    vi.mocked(commands.gatewayCircuitResetProvider).mockResolvedValueOnce({
      status: "ok",
      data: true,
    });

    expect(normalizeGatewayPreferredPort(null)).toBeNull();
    expect(normalizeGatewayPreferredPort(GATEWAY_PORT_MIN)).toBe(GATEWAY_PORT_MIN);
    expect(normalizeGatewayPreferredPort(GATEWAY_PORT_MAX)).toBe(GATEWAY_PORT_MAX);
    expect(normalizeGatewayPortAvailabilityCheck(80)).toBeNull();
    expect(normalizeGatewayProviderId(42)).toBe(42);

    await gatewayStart(GATEWAY_PORT_MIN);
    await expect(gatewayCheckPortAvailable(80)).resolves.toBe(false);
    await gatewayCheckPortAvailable(GATEWAY_PORT_MAX);
    await gatewayCircuitResetProvider(42);

    expect(commands.gatewayStart).toHaveBeenCalledWith(GATEWAY_PORT_MIN);
    expect(commands.gatewayCheckPortAvailable).toHaveBeenCalledTimes(1);
    expect(commands.gatewayCheckPortAvailable).toHaveBeenCalledWith(GATEWAY_PORT_MAX);
    expect(commands.gatewayCircuitResetProvider).toHaveBeenCalledWith(42);
  });

  it("rejects invalid gateway port and provider id inputs before ipc", async () => {
    vi.mocked(commands.gatewayStart).mockClear();
    vi.mocked(commands.gatewayCheckPortAvailable).mockClear();
    vi.mocked(commands.gatewayCircuitResetProvider).mockClear();

    await expect(gatewayStart(80)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewayStart(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewayCheckPortAvailable(1.5)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewayCircuitResetProvider(0)).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.gatewayStart).not.toHaveBeenCalled();
    expect(commands.gatewayCheckPortAvailable).not.toHaveBeenCalled();
    expect(commands.gatewayCircuitResetProvider).not.toHaveBeenCalled();
  });

  it("normalizes gateway circuit CLI keys before ipc", async () => {
    vi.mocked(commands.gatewayCircuitStatus).mockClear();
    vi.mocked(commands.gatewayCircuitResetCli).mockClear();
    vi.mocked(commands.gatewayCircuitStatus).mockResolvedValueOnce({ status: "ok", data: [] });
    vi.mocked(commands.gatewayCircuitResetCli).mockResolvedValueOnce({ status: "ok", data: 1 });

    expect(validateGatewayCliKey(" claude ")).toBe("claude");
    await gatewayCircuitStatus(" codex ");
    await gatewayCircuitResetCli(" gemini ");

    expect(commands.gatewayCircuitStatus).toHaveBeenCalledWith("codex");
    expect(commands.gatewayCircuitResetCli).toHaveBeenCalledWith("gemini");
  });

  it("rejects invalid gateway circuit CLI keys before ipc", async () => {
    vi.mocked(commands.gatewayCircuitStatus).mockClear();
    vi.mocked(commands.gatewayCircuitResetCli).mockClear();

    await expect(gatewayCircuitStatus("opencode")).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewayCircuitResetCli("opencode")).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.gatewayCircuitStatus).not.toHaveBeenCalled();
    expect(commands.gatewayCircuitResetCli).not.toHaveBeenCalled();
  });

  it("rejects invalid gateway sessions list limits before ipc", async () => {
    vi.mocked(commands.gatewaySessionsList).mockClear();

    await expect(gatewaySessionsList(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewaySessionsList(1.5)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(gatewaySessionsList(Number.POSITIVE_INFINITY)).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );

    expect(commands.gatewaySessionsList).not.toHaveBeenCalled();
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

  it("maps generated error envelopes at gateway service boundaries", async () => {
    vi.mocked(commands.gatewayStart).mockResolvedValueOnce({
      status: "error",
      error: "GW_UPSTREAM_TIMEOUT: upstream timed out",
    });

    await expect(gatewayStart(37123)).rejects.toThrow("GW_UPSTREAM_TIMEOUT");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "启动网关失败",
      expect.objectContaining({
        cmd: "gateway_start",
        args: { preferredPort: 37123 },
        error: expect.stringContaining("GW_UPSTREAM_TIMEOUT"),
      })
    );

    vi.mocked(commands.gatewayUpstreamProxyValidate).mockResolvedValueOnce({
      status: "error",
      error: "SEC_INVALID_INPUT: invalid proxy",
    });

    await expect(
      gatewayUpstreamProxyValidate({
        proxyUrl: "http://127.0.0.1:7890",
        proxyUsername: "proxy-user",
        proxyPassword: "secret",
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "代理配置验证失败",
      expect.objectContaining({
        cmd: "gateway_upstream_proxy_validate",
        args: {
          input: {
            proxyUrl: "http://127.0.0.1:7890",
            proxyUsername: "proxy-user",
            proxyPassword: "[REDACTED]",
          },
        },
        error: expect.stringContaining("SEC_INVALID_INPUT"),
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
